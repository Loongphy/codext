use super::*;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde_json::json;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test]
async fn usage_limit_error_pauses_existing_queue() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());

    chat.on_task_started();
    tab_queue(&mut chat, "queued while running");
    assert_eq!(
        chat.queued_user_message_texts(),
        vec!["queued while running"]
    );
    assert_no_submit_op(&mut op_rx);

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );

    assert!(chat.input_queue.suppress_queue_autosend);
    assert_eq!(
        chat.queued_user_message_texts(),
        vec!["queued while running"]
    );
    assert_no_submit_op(&mut op_rx);
}

#[tokio::test]
async fn tab_queues_while_usage_limit_paused_and_idle() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());

    chat.on_task_started();
    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );
    assert!(!chat.is_user_turn_pending_or_running());

    tab_queue(&mut chat, "queued while limited");

    assert_eq!(
        chat.queued_user_message_texts(),
        vec!["queued while limited"]
    );
    assert_no_submit_op(&mut op_rx);
}

#[tokio::test]
async fn available_rate_limit_snapshot_resumes_first_queued_message_only() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());

    chat.on_task_started();
    tab_queue(&mut chat, "first queued");
    tab_queue(&mut chat, "second queued");
    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );

    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 100.0)));
    assert_eq!(
        chat.queued_user_message_texts(),
        vec!["first queued", "second queued"]
    );
    assert_no_submit_op(&mut op_rx);

    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 80.0)));

    assert_next_user_turn_text(&mut op_rx, "first queued");
    assert_eq!(chat.queued_user_message_texts(), vec!["second queued"]);
    assert_no_submit_op(&mut op_rx);
}

#[tokio::test]
async fn mock_home_backend_usage_limit_flow_queues_tab_until_available_snapshot()
-> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let server = MockServer::start().await;
    write_mock_usage_config(codex_home.path(), &server.uri())?;
    mount_usage_snapshot(
        &server,
        "/api/codex/usage/exhausted",
        /*used_percent*/ 100,
    )
    .await;
    mount_usage_snapshot(
        &server,
        "/api/codex/usage/available",
        /*used_percent*/ 63,
    )
    .await;

    let config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
        .build()
        .await?;
    assert_eq!(
        config.codex_home.to_path_buf(),
        codex_home.path().to_path_buf()
    );
    assert_eq!(config.chatgpt_base_url, server.uri());

    let exhausted = fetch_mock_usage_snapshot(&server, "exhausted").await?;
    let available = fetch_mock_usage_snapshot(&server, "available").await?;

    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.config = config;
    chat.thread_id = Some(ThreadId::new());
    chat.on_task_started();
    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "mock backend returned UsageLimitExceeded".to_string(),
    );
    chat.on_rate_limit_snapshot(Some(exhausted));

    tab_queue(&mut chat, "continue after reset");
    tab_queue(&mut chat, "do not auto-send this one");

    assert_eq!(
        chat.queued_user_message_texts(),
        vec!["continue after reset", "do not auto-send this one"]
    );
    assert_no_submit_op(&mut op_rx);

    chat.on_rate_limit_snapshot(Some(available));

    assert_next_user_turn_text(&mut op_rx, "continue after reset");
    assert_eq!(
        chat.queued_user_message_texts(),
        vec!["do not auto-send this one"]
    );
    assert_no_submit_op(&mut op_rx);

    let requests = server.received_requests().await.unwrap_or_default();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.url.path().to_string())
            .collect::<Vec<_>>(),
        vec![
            "/api/codex/usage/exhausted".to_string(),
            "/api/codex/usage/available".to_string(),
        ]
    );

    Ok(())
}

fn tab_queue(chat: &mut ChatWidget, text: &str) {
    chat.bottom_pane
        .set_composer_text(text.to_string(), Vec::new(), Vec::new());
    chat.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
}

fn assert_next_user_turn_text(
    op_rx: &mut tokio::sync::mpsc::UnboundedReceiver<Op>,
    expected: &str,
) {
    match next_submit_op(op_rx) {
        Op::UserTurn { items, .. } => assert_eq!(
            items,
            vec![UserInput::Text {
                text: expected.to_string(),
                text_elements: Vec::new(),
            }]
        ),
        other => panic!("expected Op::UserTurn, got {other:?}"),
    }
}

async fn mount_usage_snapshot(server: &MockServer, endpoint: &str, used_percent: i32) {
    Mock::given(method("GET"))
        .and(path(endpoint))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "plan_type": "pro",
            "rate_limit": {
                "allowed": used_percent < 100,
                "limit_reached": used_percent >= 100,
                "primary_window": {
                    "used_percent": used_percent,
                    "limit_window_seconds": 3600,
                    "reset_at": 1779902400
                }
            }
        })))
        .mount(server)
        .await;
}

async fn fetch_mock_usage_snapshot(
    server: &MockServer,
    state: &str,
) -> anyhow::Result<RateLimitSnapshot> {
    let response: MockUsageResponse = reqwest::Client::new()
        .get(format!("{}/api/codex/usage/{state}", server.uri()))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(response.into_snapshot())
}

fn write_mock_usage_config(codex_home: &std::path::Path, server_uri: &str) -> std::io::Result<()> {
    std::fs::write(
        codex_home.join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
chatgpt_base_url = "{server_uri}"
model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for usage-limit queue test"
base_url = "{server_uri}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
}

#[derive(Deserialize)]
struct MockUsageResponse {
    rate_limit: MockRateLimit,
    rate_limit_reached_type: Option<MockRateLimitReachedType>,
}

impl MockUsageResponse {
    fn into_snapshot(self) -> RateLimitSnapshot {
        RateLimitSnapshot {
            limit_id: Some("codex".to_string()),
            limit_name: None,
            primary: self.rate_limit.primary_window.map(Into::into),
            secondary: None,
            credits: None,
            plan_type: Some(PlanType::Pro),
            rate_limit_reached_type: self
                .rate_limit_reached_type
                .and_then(MockRateLimitReachedType::into_protocol),
        }
    }
}

#[derive(Deserialize)]
struct MockRateLimit {
    primary_window: Option<MockRateLimitWindow>,
}

#[derive(Deserialize)]
struct MockRateLimitWindow {
    used_percent: i32,
    limit_window_seconds: i64,
    reset_at: Option<i64>,
}

impl From<MockRateLimitWindow> for RateLimitWindow {
    fn from(window: MockRateLimitWindow) -> Self {
        Self {
            used_percent: window.used_percent,
            window_duration_mins: Some(window.limit_window_seconds / 60),
            resets_at: window.reset_at,
        }
    }
}

#[derive(Deserialize)]
struct MockRateLimitReachedType {
    #[serde(rename = "type")]
    kind: String,
}

impl MockRateLimitReachedType {
    fn into_protocol(self) -> Option<RateLimitReachedType> {
        match self.kind.as_str() {
            "workspace_member_usage_limit_reached" => {
                Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached)
            }
            _ => None,
        }
    }
}
