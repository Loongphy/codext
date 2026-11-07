use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use codex_app_server_protocol::ClientInfo;
use codex_app_server_protocol::InitializeCapabilities;
use codex_app_server_protocol::InitializeParams;
use codex_app_server_protocol::InitializeResponse;
use codex_app_server_protocol::JSONRPCMessage;
use codex_app_server_protocol::JSONRPCNotification;
use codex_app_server_protocol::JSONRPCRequest;
use codex_app_server_protocol::RequestId;
use codex_uds::UnixStream;
use futures::SinkExt;
use futures::StreamExt;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::time::timeout;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::client_async;
use tokio_tungstenite::tungstenite::Message;

pub(crate) const CONTROL_SOCKET_RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);
const CLIENT_NAME: &str = "codex_app_server_daemon";
const INITIALIZE_REQUEST_ID: RequestId = RequestId::Integer(1);
const ACCOUNT_RELOAD_REQUEST_ID: RequestId = RequestId::Integer(2);
const JSONRPC_METHOD_NOT_FOUND: i64 = -32601;
const AUTH_RELOAD_TIMEOUT: Duration = Duration::from_secs(15);

/// Outcome of an `account/reload` request sent to a running app-server daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthReloadOutcome {
    /// The daemon reloaded auth storage; `changed` says whether the auth
    /// snapshot actually changed.
    Reloaded { changed: bool },
    /// The daemon skipped the reload because a turn is running on a client
    /// connected to it.
    SkippedBusy,
    /// The running daemon does not implement `account/reload` (for example a
    /// stock upstream Codex daemon, which lacks this Codext-only RPC).
    Unsupported,
}

/// Ask the app-server daemon listening on `socket_path` to reload auth from
/// storage. External account switchers (codex-auth) call this after replacing
/// `CODEX_HOME/auth.json` so a long-lived daemon picks up the new account.
pub async fn request_auth_reload(socket_path: &Path) -> Result<AuthReloadOutcome> {
    timeout(AUTH_RELOAD_TIMEOUT, request_auth_reload_inner(socket_path))
        .await
        .with_context(|| {
            format!(
                "timed out reloading auth on app-server control socket {}",
                socket_path.display()
            )
        })?
}

async fn request_auth_reload_inner(socket_path: &Path) -> Result<AuthReloadOutcome> {
    let mut websocket = connect(socket_path).await?;
    initialize(&mut websocket, /*experimental_api*/ true).await?;
    let initialized = JSONRPCMessage::Notification(JSONRPCNotification {
        method: "initialized".to_string(),
        params: None,
    });
    send_message(&mut websocket, &initialized)
        .await
        .context("failed to send initialized notification")?;
    let request = JSONRPCMessage::Request(JSONRPCRequest {
        id: ACCOUNT_RELOAD_REQUEST_ID,
        method: "account/reload".to_string(),
        params: None,
        trace: None,
    });
    send_message(&mut websocket, &request)
        .await
        .context("failed to send account/reload request")?;
    let outcome = loop {
        match read_message(&mut websocket).await? {
            JSONRPCMessage::Response(response) if response.id == ACCOUNT_RELOAD_REQUEST_ID => {
                let parsed: codex_app_server_protocol::ReloadAccountResponse =
                    serde_json::from_value(response.result)
                        .context("failed to parse account/reload response")?;
                break if parsed.auth_reload_skipped {
                    AuthReloadOutcome::SkippedBusy
                } else {
                    AuthReloadOutcome::Reloaded {
                        changed: parsed.auth_changed,
                    }
                };
            }
            JSONRPCMessage::Error(error) if error.id == ACCOUNT_RELOAD_REQUEST_ID => {
                if error.error.code == JSONRPC_METHOD_NOT_FOUND {
                    break AuthReloadOutcome::Unsupported;
                }
                return Err(anyhow!("account/reload failed: {}", error.error.message));
            }
            _ => {}
        }
    };
    websocket.close(None).await.ok();
    Ok(outcome)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProbeInfo {
    pub(crate) app_server_version: String,
}

pub(crate) async fn probe(socket_path: &Path) -> Result<ProbeInfo> {
    timeout(CONTROL_SOCKET_RESPONSE_TIMEOUT, probe_inner(socket_path))
        .await
        .with_context(|| {
            format!(
                "timed out probing app-server control socket {}",
                socket_path.display()
            )
        })?
}

async fn probe_inner(socket_path: &Path) -> Result<ProbeInfo> {
    let mut websocket = connect(socket_path).await?;

    let initialize_response = initialize(&mut websocket, /*experimental_api*/ false).await?;
    let initialized = JSONRPCMessage::Notification(JSONRPCNotification {
        method: "initialized".to_string(),
        params: None,
    });
    send_message(&mut websocket, &initialized)
        .await
        .context("failed to send initialized notification")?;
    websocket.close(None).await.ok();

    Ok(ProbeInfo {
        app_server_version: parse_version_from_user_agent(&initialize_response.user_agent)?,
    })
}

pub(crate) async fn connect(socket_path: &Path) -> Result<WebSocketStream<UnixStream>> {
    connect_at(socket_path, "ws://localhost/").await
}

async fn connect_at(socket_path: &Path, url: &str) -> Result<WebSocketStream<UnixStream>> {
    let stream = UnixStream::connect(socket_path)
        .await
        .with_context(|| format!("failed to connect to {}", socket_path.display()))?;
    let (websocket, _response) = client_async(url, stream)
        .await
        .with_context(|| format!("failed to upgrade {}", socket_path.display()))?;
    Ok(websocket)
}

#[cfg(windows)]
pub(crate) async fn request_shutdown(socket_path: &Path, pid: u32) -> Result<()> {
    timeout(CONTROL_SOCKET_RESPONSE_TIMEOUT, async {
        let mut websocket = connect_at(socket_path, "ws://localhost/daemon/shutdown").await?;
        websocket
            .send(Message::Text(pid.to_string().into()))
            .await?;
        let reply = websocket
            .next()
            .await
            .context("shutdown socket closed without acknowledgment")??;
        anyhow::ensure!(
            matches!(reply, Message::Text(ack) if ack == pid.to_string()),
            "shutdown acknowledgment did not match the managed process {pid}"
        );
        websocket.close(None).await?;
        Ok(())
    })
    .await
    .context("timed out waiting for managed app-server shutdown acknowledgment")?
}

pub(crate) async fn initialize<S>(
    websocket: &mut WebSocketStream<S>,
    experimental_api: bool,
) -> Result<InitializeResponse>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let initialize = JSONRPCMessage::Request(JSONRPCRequest {
        id: INITIALIZE_REQUEST_ID,
        method: "initialize".to_string(),
        params: Some(serde_json::to_value(InitializeParams {
            client_info: ClientInfo {
                name: CLIENT_NAME.to_string(),
                title: Some("Codex App Server Daemon".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: if experimental_api {
                Some(InitializeCapabilities {
                    experimental_api: true,
                    ..Default::default()
                })
            } else {
                None
            },
        })?),
        trace: None,
    });
    send_message(websocket, &initialize)
        .await
        .context("failed to send initialize request")?;

    let response = loop {
        let message = timeout(CONTROL_SOCKET_RESPONSE_TIMEOUT, read_message(websocket))
            .await
            .context("timed out waiting for initialize response")??;
        if let JSONRPCMessage::Response(response) = message
            && response.id == INITIALIZE_REQUEST_ID
        {
            break response;
        }
    };
    serde_json::from_value::<InitializeResponse>(response.result)
        .context("failed to parse initialize response")
}

pub(crate) async fn send_message<S>(
    websocket: &mut WebSocketStream<S>,
    message: &JSONRPCMessage,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    websocket
        .send(Message::Text(serde_json::to_string(message)?.into()))
        .await?;
    Ok(())
}

pub(crate) async fn read_message<S>(websocket: &mut WebSocketStream<S>) -> Result<JSONRPCMessage>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        let frame = websocket
            .next()
            .await
            .ok_or_else(|| anyhow!("app-server closed the control socket"))??;
        let Message::Text(payload) = frame else {
            continue;
        };
        return serde_json::from_str::<JSONRPCMessage>(&payload)
            .context("failed to parse app-server JSON-RPC message");
    }
}

fn parse_version_from_user_agent(user_agent: &str) -> Result<String> {
    let (_originator, rest) = user_agent
        .split_once('/')
        .ok_or_else(|| anyhow!("app-server user-agent omitted version separator"))?;
    let version = rest
        .split_whitespace()
        .next()
        .filter(|version| !version.is_empty())
        .ok_or_else(|| anyhow!("app-server user-agent omitted version"))?;
    Ok(version.to_string())
}

#[cfg(all(test, unix))]
mod tests {
    use pretty_assertions::assert_eq;

    use super::parse_version_from_user_agent;

    #[test]
    fn parses_version_from_codex_user_agent() {
        assert_eq!(
            parse_version_from_user_agent(
                "codex_app_server_daemon/1.2.3 (Linux 6.8.0; x86_64) codex_cli_rs/1.2.3",
            )
            .expect("version"),
            "1.2.3"
        );
    }

    #[test]
    fn rejects_user_agent_without_version() {
        assert!(parse_version_from_user_agent("codex_app_server_daemon").is_err());
    }
}
