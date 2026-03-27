use codex_core::config::types::CollaborationModeOverrides;
use codex_core::models_manager::collaboration_mode_presets::collaboration_mode_presets_with_overrides;
#[cfg(test)]
use codex_core::models_manager::manager::ModelsManager;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::config_types::ModeKind;
use codex_protocol::openai_models::ReasoningEffort;

#[cfg(test)]
fn filtered_presets(models_manager: &ModelsManager) -> Vec<CollaborationModeMask> {
    models_manager
        .list_collaboration_modes()
        .into_iter()
        .filter(|mask| mask.mode.is_some_and(ModeKind::is_tui_visible))
        .collect()
}

fn filtered_presets_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Vec<CollaborationModeMask> {
    collaboration_mode_presets_with_overrides(base_model, base_effort, overrides)
        .into_iter()
        .filter(|mask| mask.mode.is_some_and(ModeKind::is_tui_visible))
        .collect()
}

pub(crate) fn presets_for_tui_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Vec<CollaborationModeMask> {
    filtered_presets_with_overrides(base_model, base_effort, overrides)
}

#[cfg(test)]
pub(crate) fn default_mask(models_manager: &ModelsManager) -> Option<CollaborationModeMask> {
    let presets = filtered_presets(models_manager);
    presets
        .iter()
        .find(|mask| mask.mode == Some(ModeKind::Default))
        .cloned()
        .or_else(|| presets.into_iter().next())
}

pub(crate) fn default_mask_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Option<CollaborationModeMask> {
    let presets = filtered_presets_with_overrides(base_model, base_effort, overrides);
    presets
        .iter()
        .find(|mask| mask.mode == Some(ModeKind::Default))
        .cloned()
        .or_else(|| presets.into_iter().next())
}

#[cfg(test)]
pub(crate) fn mask_for_kind(
    models_manager: &ModelsManager,
    kind: ModeKind,
) -> Option<CollaborationModeMask> {
    if !kind.is_tui_visible() {
        return None;
    }
    filtered_presets(models_manager)
        .into_iter()
        .find(|mask| mask.mode == Some(kind))
}

pub(crate) fn mask_for_kind_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
    kind: ModeKind,
) -> Option<CollaborationModeMask> {
    if !kind.is_tui_visible() {
        return None;
    }

    filtered_presets_with_overrides(base_model, base_effort, overrides)
        .into_iter()
        .find(|mask| mask.mode == Some(kind))
}

/// Cycle to the next collaboration mode preset in list order.
pub(crate) fn next_mask_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
    current: Option<&CollaborationModeMask>,
) -> Option<CollaborationModeMask> {
    let presets = filtered_presets_with_overrides(base_model, base_effort, overrides);
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

#[cfg(test)]
pub(crate) fn default_mode_mask(models_manager: &ModelsManager) -> Option<CollaborationModeMask> {
    mask_for_kind(models_manager, ModeKind::Default)
}

pub(crate) fn default_mode_mask_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Option<CollaborationModeMask> {
    mask_for_kind_with_overrides(base_model, base_effort, overrides, ModeKind::Default)
}

#[cfg(test)]
pub(crate) fn plan_mask(models_manager: &ModelsManager) -> Option<CollaborationModeMask> {
    mask_for_kind(models_manager, ModeKind::Plan)
}

pub(crate) fn plan_mask_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Option<CollaborationModeMask> {
    mask_for_kind_with_overrides(base_model, base_effort, overrides, ModeKind::Plan)
}
