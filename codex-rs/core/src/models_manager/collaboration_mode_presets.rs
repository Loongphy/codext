use crate::config::types::CollaborationModeOverride;
use crate::config::types::CollaborationModeOverrides;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::config_types::ModeKind;
use codex_protocol::openai_models::ReasoningEffort;

const COLLABORATION_MODE_PLAN: &str = include_str!("../../templates/collaboration_mode/plan.md");
const COLLABORATION_MODE_CODE: &str = include_str!("../../templates/collaboration_mode/code.md");
const COLLABORATION_MODE_PAIR_PROGRAMMING: &str =
    include_str!("../../templates/collaboration_mode/pair_programming.md");
const COLLABORATION_MODE_EXECUTE: &str =
    include_str!("../../templates/collaboration_mode/execute.md");

pub(super) fn builtin_collaboration_mode_presets() -> Vec<CollaborationModeMask> {
    vec![
        plan_preset(),
        code_preset(),
        pair_programming_preset(),
        execute_preset(),
    ]
}

/// Build collaboration mode presets, applying optional overrides and base defaults.
pub fn collaboration_mode_presets_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Vec<CollaborationModeMask> {
    let base_model = base_model.trim();
    let base_model = (!base_model.is_empty()).then(|| base_model.to_string());
    let presets = builtin_collaboration_mode_presets();
    presets
        .into_iter()
        .map(|preset| {
            let override_for_mode = override_for_mode(overrides, &preset);
            apply_overrides(preset, base_model.as_ref(), base_effort, override_for_mode)
        })
        .collect()
}

#[cfg(any(test, feature = "test-support"))]
pub fn test_builtin_collaboration_mode_presets() -> Vec<CollaborationModeMask> {
    builtin_collaboration_mode_presets()
}

fn plan_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: "Plan".to_string(),
        mode: Some(ModeKind::Plan),
        model: None,
        reasoning_effort: Some(Some(ReasoningEffort::Medium)),
        developer_instructions: Some(Some(COLLABORATION_MODE_PLAN.to_string())),
    }
}

fn code_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: "Code".to_string(),
        mode: Some(ModeKind::Code),
        model: None,
        reasoning_effort: Some(Some(ReasoningEffort::Medium)),
        developer_instructions: Some(Some(COLLABORATION_MODE_CODE.to_string())),
    }
}

fn pair_programming_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: "Pair Programming".to_string(),
        mode: Some(ModeKind::PairProgramming),
        model: None,
        reasoning_effort: Some(Some(ReasoningEffort::Medium)),
        developer_instructions: Some(Some(COLLABORATION_MODE_PAIR_PROGRAMMING.to_string())),
    }
}

fn execute_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: "Execute".to_string(),
        mode: Some(ModeKind::Execute),
        model: None,
        reasoning_effort: Some(Some(ReasoningEffort::XHigh)),
        developer_instructions: Some(Some(COLLABORATION_MODE_EXECUTE.to_string())),
    }
}

fn override_for_mode<'a>(
    overrides: Option<&'a CollaborationModeOverrides>,
    mode: &CollaborationModeMask,
) -> Option<&'a CollaborationModeOverride> {
    let overrides = overrides?;
    match mode.mode {
        Some(ModeKind::Plan) => overrides.plan.as_ref(),
        Some(ModeKind::Code) => overrides.code.as_ref(),
        _ => None,
    }
}

fn apply_overrides(
    mut mode: CollaborationModeMask,
    base_model: Option<&String>,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverride>,
) -> CollaborationModeMask {
    let model = overrides
        .and_then(|entry| entry.model.clone())
        .or_else(|| base_model.cloned());
    let effort = overrides
        .and_then(|entry| entry.reasoning_effort)
        .or(base_effort);
    let should_override_effort =
        overrides.and_then(|entry| entry.reasoning_effort).is_some() || base_effort.is_some();

    let effort_update = should_override_effort.then_some(effort);
    if let Some(model) = model {
        mode.model = Some(model);
    }
    if let Some(effort) = effort_update {
        mode.reasoning_effort = Some(effort);
    }
    mode
}
