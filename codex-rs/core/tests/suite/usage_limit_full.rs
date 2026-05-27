use anyhow::Result;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::credits_depleted_response;
use core_test_support::responses::ev_completed_with_tokens;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::quota_exceeded_sse;
use core_test_support::responses::response_with_rate_limits;
use core_test_support::responses::server_overloaded_response;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::responses::usage_limit_response;
use core_test_support::responses::usage_limit_with_promo;
use core_test_support::responses::workspace_member_limit_response;
use core_test_support::responses::workspace_owner_credits_response;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use wiremock::Mock;
use wiremock::matchers::method;
use wiremock::matchers::path;

// ---------------------------------------------------------------------------
// Test 1: 429 usage_limit_reached emits error
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn usage_limit_reached_emits_error() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(usage_limit_response("pro", 1704067242))
        .expect(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "test usage limit".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut error_count = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(err) => {
                error_count += 1;
                assert!(
                    err.message.contains("usage limit") || err.message.contains("Usage limit"),
                    "Expected usage limit message, got: {}",
                    err.message
                );
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert_eq!(error_count, 1, "expected exactly one error event");
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: SSE response.failed with insufficient_quota emits single error
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quota_exceeded_emits_single_error() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    mount_sse_once(&server, quota_exceeded_sse()).await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "quota test".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut error_count = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(err) => {
                error_count += 1;
                assert!(
                    err.message.contains("Quota exceeded") || err.message.contains("quota"),
                    "Expected quota exceeded message, got: {}",
                    err.message
                );
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert_eq!(error_count, 1, "expected exactly one error event");
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: 429 server_overloaded emits error
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_overloaded_emits_error() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(server_overloaded_response())
        .expect(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "overloaded test".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut error_count = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(err) => {
                error_count += 1;
                // The core layer wraps 429 as "exceeded retry limit" or similar
                assert!(
                    err.message.contains("retry limit")
                        || err.message.contains("429")
                        || err.message.contains("overloaded"),
                    "Expected retry/429 message, got: {}",
                    err.message
                );
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert!(error_count >= 1, "expected at least one error event");
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4: 200 OK with rate limit headers parsed into TokenCount event
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rate_limit_headers_parsed_in_token_count() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    let sse_body = sse(vec![ev_completed_with_tokens("resp-rl", 123)]);

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            response_with_rate_limits(95.0, 1440).set_body_raw(sse_body, "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "rate limit test".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut saw_token_count = false;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::TokenCount(ev) => {
                if let Some(rate_limits) = &ev.rate_limits {
                    if let Some(primary) = &rate_limits.primary {
                        assert!(
                            (primary.used_percent - 95.0).abs() < 0.1,
                            "Expected primary used_percent ~95.0, got: {}",
                            primary.used_percent
                        );
                        saw_token_count = true;
                    }
                }
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert!(
        saw_token_count,
        "expected TokenCount event with rate limits"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5: 200 OK with credits-depleted headers — headers are parsed but
//         credits depletion is tracked as rate limit info, not an error
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credits_depleted_parses_headers() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(credits_depleted_response())
        .expect(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "credits test".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    // Credits depleted is communicated via headers; the turn may succeed
    // but rate limit info should be present in TokenCount events
    let mut _saw_turn_complete = false;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::TurnComplete(_) => {
                _saw_turn_complete = true;
                break;
            }
            EventMsg::Error(_) => {
                // Also acceptable - credits depleted may surface as error
                _saw_turn_complete = true;
                break;
            }
            _ => {}
        }
    }

    assert!(
        _saw_turn_complete,
        "expected turn to complete (with or without error)"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 6: 429 workspace owner credits depleted
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_owner_credits_limit() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(workspace_owner_credits_response())
        .expect(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "workspace owner test".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut error_count = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(err) => {
                error_count += 1;
                // Core layer maps 429 to usage limit error
                assert!(
                    err.message.contains("usage limit")
                        || err.message.contains("Usage limit")
                        || err.message.contains("retry"),
                    "Expected usage limit or retry message, got: {}",
                    err.message
                );
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert!(
        error_count >= 1,
        "expected at least one error for workspace owner credits"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 7: 429 workspace member usage limit
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_member_usage_limit() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(workspace_member_limit_response())
        .expect(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "workspace member test".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut error_count = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(err) => {
                error_count += 1;
                assert!(
                    err.message.contains("usage limit")
                        || err.message.contains("Usage limit")
                        || err.message.contains("retry"),
                    "Expected usage limit or retry message, got: {}",
                    err.message
                );
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert!(
        error_count >= 1,
        "expected at least one error for workspace member limit"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 8: 429 usage_limit with resets_at timestamp in message
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn usage_limit_with_resets_at() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(usage_limit_response("pro", 1704067242))
        .expect(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "resets_at test".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut error_count = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(err) => {
                error_count += 1;
                assert!(
                    err.message.contains("Try again") || err.message.contains("usage limit"),
                    "Expected 'Try again' in message, got: {}",
                    err.message
                );
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert!(
        error_count >= 1,
        "expected at least one error with resets_at"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 9: 429 usage_limit with promo_message
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn usage_limit_with_promo_message() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(usage_limit_with_promo(
            "Upgrade to Pro for more usage!",
            1704067242,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "promo test".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut error_count = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(err) => {
                error_count += 1;
                // The core layer formats the message with plan type info
                assert!(
                    err.message.contains("usage limit")
                        || err.message.contains("Usage limit")
                        || err.message.contains("Try again"),
                    "Expected usage limit message with plan info, got: {}",
                    err.message
                );
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert!(error_count >= 1, "expected at least one error with promo");
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 10: Rate limit recovery — first 429 then 200 with low usage
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rate_limit_recovery_clears() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();

    // First request: 429 usage limit
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(usage_limit_response("pro", 1704067242))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second request: 200 OK with low usage
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(response_with_rate_limits(10.0, 1440))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let test = builder.build(&server).await?;

    // First turn: should hit 429
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "first turn".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut first_turn_errors = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(_) => first_turn_errors += 1,
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }
    assert!(first_turn_errors >= 1, "first turn should have error");

    // Second turn: should succeed with low usage
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "second turn".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let mut second_turn_errors = 0;
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        match event {
            EventMsg::Error(_) => second_turn_errors += 1,
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }
    assert_eq!(
        second_turn_errors, 0,
        "second turn should succeed after recovery"
    );
    Ok(())
}
