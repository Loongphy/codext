use codex_app_server::app_server_control_socket_path;
use codex_app_server_daemon::AuthReloadOutcome;
use codex_core::config::find_codex_home;

/// `codex app-server daemon reload-auth`: ask the running shared daemon to
/// reload `CODEX_HOME/auth.json` from storage. External account switchers
/// (codex-auth) run this after replacing auth.json so daemon-hosted sessions
/// pick up the new account without restarting the daemon.
pub(crate) async fn run_daemon_reload_auth() -> anyhow::Result<()> {
    let codex_home = find_codex_home()?;
    let socket_path = app_server_control_socket_path(codex_home.as_path())?;
    if !socket_path.as_path().exists() {
        anyhow::bail!("no app-server daemon socket at {}", socket_path.display());
    }
    match codex_app_server_daemon::request_auth_reload(socket_path.as_path()).await? {
        AuthReloadOutcome::Reloaded { changed: true } => {
            println!("Daemon reloaded auth; the account changed.");
        }
        AuthReloadOutcome::Reloaded { changed: false } => {
            println!("Daemon reloaded auth; the account is unchanged.");
        }
        AuthReloadOutcome::SkippedBusy => {
            println!("Daemon skipped the auth reload because a turn is running.");
        }
        AuthReloadOutcome::Unsupported => {
            anyhow::bail!(
                "the running app-server does not implement account/reload; it is not a codext daemon (stop it, or run codext with --no-daemon)"
            );
        }
    }
    Ok(())
}
