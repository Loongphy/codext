use super::*;
use crate::app_event::AppEvent;
use crate::chatwidget::rate_limits::RateLimitErrorKind;
use crate::chatwidget::rate_limits::RateLimitSwitchPromptState;
use crate::chatwidget::rate_limits::RateLimitWarningState;
use codex_app_server_protocol::RateLimitReachedType;
use pretty_assertions::assert_eq;

// ---------------------------------------------------------------------------
// Test 1: on_rate_limit_error with UsageLimit kind emits usage limit message
// ---------------------------------------------------------------------------
#[tokio::test]
async fn usage_limit_error_emits_insert_history() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached. Increase your limits to continue.".to_string(),
    );

    // Should have emitted an InsertHistoryCell event with the error message
    let mut saw_error_in_history = false;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("usage limit") || text.contains("Usage limit") {
                saw_error_in_history = true;
            }
        }
    }
    assert!(
        saw_error_in_history,
        "expected usage limit error in history"
    );
}

// ---------------------------------------------------------------------------
// Test 2: on_rate_limit_snapshot with high usage then recovery
// ---------------------------------------------------------------------------
#[tokio::test]
async fn rate_limit_snapshot_recovery_no_error() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    // First: high usage
    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 95.0)));

    // Drain events
    while rx.try_recv().is_ok() {}

    // Second: recovery (low usage)
    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 10.0)));

    // Should not have emitted any error events on recovery
    let mut saw_error = false;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("error") || text.contains("Error") {
                saw_error = true;
            }
        }
    }
    assert!(!saw_error, "should not emit error on recovery");
}

// ---------------------------------------------------------------------------
// Test 3: Multiple rate limit snapshots don't accumulate errors
// ---------------------------------------------------------------------------
#[tokio::test]
async fn multiple_snapshots_no_error_accumulation() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    // Multiple snapshots at different levels
    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 50.0)));
    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 75.0)));
    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 90.0)));
    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 10.0)));

    // Should not have any error events
    let mut error_count = 0;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("error") || text.contains("Error") {
                error_count += 1;
            }
        }
    }
    assert_eq!(
        error_count, 0,
        "should not accumulate errors from snapshots"
    );
}

// ---------------------------------------------------------------------------
// Test 4: Multiple consecutive rate limit errors emit separate events
// ---------------------------------------------------------------------------
#[tokio::test]
async fn multiple_rate_limit_errors_emit_separately() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );
    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );

    let mut error_count = 0;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("usage limit") || text.contains("Usage limit") {
                error_count += 1;
            }
        }
    }
    assert_eq!(
        error_count, 2,
        "each error call should emit one history event"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Recovery after error clears state
// ---------------------------------------------------------------------------
#[tokio::test]
async fn recovery_after_error_clears_state() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    // Trigger error
    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );

    // Drain events
    while rx.try_recv().is_ok() {}

    // Recovery
    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 10.0)));

    // Should not have errors after recovery
    let mut error_count = 0;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("usage limit") || text.contains("Usage limit") {
                error_count += 1;
            }
        }
    }
    assert_eq!(error_count, 0, "recovery should not emit errors");
}

// ---------------------------------------------------------------------------
// Test 6: Workspace member credits depleted emits error with nudge
// ---------------------------------------------------------------------------
#[tokio::test]
async fn workspace_member_credits_depleted_emits_error() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::WorkspaceMemberCreditsDepleted);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Your workspace is out of credits. Add credits to continue using Codex.".to_string(),
    );

    let mut saw_credits_error = false;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("credits") || text.contains("workspace") {
                saw_credits_error = true;
            }
        }
    }
    assert!(saw_credits_error, "should emit workspace credits error");
}

// ---------------------------------------------------------------------------
// Test 7: Workspace owner credits depleted emits error
// ---------------------------------------------------------------------------
#[tokio::test]
async fn workspace_owner_credits_depleted_emits_error() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::WorkspaceOwnerCreditsDepleted);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "You're out of credits. Your workspace is out of credits.".to_string(),
    );

    let mut saw_error = false;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("credits") || text.contains("out of credits") {
                saw_error = true;
            }
        }
    }
    assert!(saw_error, "should emit workspace owner credits error");
}

// ---------------------------------------------------------------------------
// Test 8: Workspace member usage limit reached emits error with nudge
// ---------------------------------------------------------------------------
#[tokio::test]
async fn workspace_member_usage_limit_emits_error() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached. You've reached your usage limit.".to_string(),
    );

    let mut saw_error = false;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("usage limit") || text.contains("Usage limit") {
                saw_error = true;
            }
        }
    }
    assert!(saw_error, "should emit workspace member usage limit error");
}

// ---------------------------------------------------------------------------
// Test 9: Rate limit warnings at 75% threshold
// ---------------------------------------------------------------------------
#[tokio::test]
async fn rate_limit_warning_at_75_percent() {
    let mut state = RateLimitWarningState::default();
    let warnings = state.take_warnings(Some(75.0), Some(10079), Some(75.0), Some(10079));

    assert!(!warnings.is_empty(), "should emit warning at 75% usage");
    assert!(
        warnings.iter().any(|w| w.contains("less than")),
        "warning should mention 'less than', got: {:?}",
        warnings
    );
}

// ---------------------------------------------------------------------------
// Test 10: Rate limit warnings at 90% threshold
// ---------------------------------------------------------------------------
#[tokio::test]
async fn rate_limit_warning_at_90_percent() {
    let mut state = RateLimitWarningState::default();
    let warnings = state.take_warnings(Some(90.0), Some(10079), Some(90.0), Some(10079));

    assert!(!warnings.is_empty(), "should emit warning at 90% usage");
    assert!(
        warnings.iter().any(|w| w.contains("less than")),
        "warning should mention 'less than', got: {:?}",
        warnings
    );
}

// ---------------------------------------------------------------------------
// Test 11: Server overloaded uses different error kind
// ---------------------------------------------------------------------------
#[tokio::test]
async fn server_overloaded_emits_error() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_error(
        RateLimitErrorKind::ServerOverloaded,
        "Selected model is at capacity. Please try a different model.".to_string(),
    );

    let mut saw_error = false;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("capacity") || text.contains("overloaded") || text.contains("model") {
                saw_error = true;
            }
        }
    }
    assert!(saw_error, "should emit server overloaded error");
}

// ---------------------------------------------------------------------------
// Test 12: Generic rate limit error
// ---------------------------------------------------------------------------
#[tokio::test]
async fn generic_rate_limit_error_emits_error() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_error(
        RateLimitErrorKind::Generic,
        "Rate limit exceeded. Please try again later.".to_string(),
    );

    let mut saw_error = false;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(_) = event {
            saw_error = true;
        }
    }
    assert!(saw_error, "should emit generic rate limit error");
}

// ---------------------------------------------------------------------------
// Test 13: Rate limit switch prompt stays idle below threshold
// ---------------------------------------------------------------------------
#[tokio::test]
async fn rate_limit_switch_prompt_idle_below_threshold() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 50.0)));

    assert!(
        matches!(
            chat.rate_limit_switch_prompt,
            RateLimitSwitchPromptState::Idle
        ),
        "switch prompt should be idle at 50%"
    );
}

// ---------------------------------------------------------------------------
// Test 14: Usage limit reached type propagates correctly
// ---------------------------------------------------------------------------
#[tokio::test]
async fn usage_limit_reached_type_propagates() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::RateLimitReached);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "You've hit your usage limit.".to_string(),
    );

    let mut saw_error = false;
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let text = format!("{:?}", cell);
            if text.contains("usage limit") || text.contains("hit your") {
                saw_error = true;
            }
        }
    }
    assert!(
        saw_error,
        "should emit usage limit error for RateLimitReached type"
    );
}
