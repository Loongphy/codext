use codex_core::config::types::CollaborationModeOverrides;
use codex_core::models_manager::collaboration_mode_presets::collaboration_mode_presets_with_overrides;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::config_types::ModeKind;
use codex_protocol::openai_models::ReasoningEffort;

fn is_tui_mode(kind: ModeKind) -> bool {
    matches!(kind, ModeKind::Plan | ModeKind::Code)
}

fn filtered_presets(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Vec<CollaborationModeMask> {
    collaboration_mode_presets_with_overrides(base_model, base_effort, overrides)
        .into_iter()
        .filter(|mask| mask.mode.is_some_and(is_tui_mode))
        .collect()
}

pub(crate) fn presets_for_tui(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Vec<CollaborationModeMask> {
    filtered_presets(base_model, base_effort, overrides)
}

pub(crate) fn default_mask(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Option<CollaborationModeMask> {
    let presets = filtered_presets(base_model, base_effort, overrides);
    presets
        .iter()
        .find(|mask| mask.mode == Some(ModeKind::Code))
        .cloned()
        .or_else(|| presets.into_iter().next())
}

pub(crate) fn mask_for_kind(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
    kind: ModeKind,
) -> Option<CollaborationModeMask> {
    if !is_tui_mode(kind) {
        return None;
    }
    filtered_presets(base_model, base_effort, overrides)
        .into_iter()
        .find(|mask| mask.mode == Some(kind))
}

/// Cycle to the next collaboration mode preset in list order.
pub(crate) fn next_mask(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
    current: Option<&CollaborationModeMask>,
) -> Option<CollaborationModeMask> {
    let presets = filtered_presets(base_model, base_effort, overrides);
    if presets.is_empty() {
        return None;
    }
    let current_kind = current.and_then(|mask| mask.mode);
    let next_index = presets
        .iter()
        .position(|mask| mask.mode == current_kind)
        .map_or(0, |idx| (idx + 1) % presets.len());
    presets.get(next_index).cloned()
}

pub(crate) fn plan_mask(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Option<CollaborationModeMask> {
    mask_for_kind(base_model, base_effort, overrides, ModeKind::Plan)
}
