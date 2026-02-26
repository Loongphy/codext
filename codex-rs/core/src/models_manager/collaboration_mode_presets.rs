use crate::config::types::CollaborationModeOverride;
use crate::config::types::CollaborationModeOverrides;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::TUI_VISIBLE_COLLABORATION_MODES;
use codex_protocol::openai_models::ReasoningEffort;

const COLLABORATION_MODE_PLAN: &str = include_str!("../../templates/collaboration_mode/plan.md");
const COLLABORATION_MODE_DEFAULT: &str =
    include_str!("../../templates/collaboration_mode/default.md");
const KNOWN_MODE_NAMES_PLACEHOLDER: &str = "{{KNOWN_MODE_NAMES}}";
const REQUEST_USER_INPUT_AVAILABILITY_PLACEHOLDER: &str = "{{REQUEST_USER_INPUT_AVAILABILITY}}";

pub(crate) fn builtin_collaboration_mode_presets() -> Vec<CollaborationModeMask> {
    vec![plan_preset(), default_preset()]
}

/// Build collaboration mode presets, applying optional overrides and base defaults.
pub fn collaboration_mode_presets_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Vec<CollaborationModeMask> {
    let base_model = base_model.trim();
    let base_model = (!base_model.is_empty()).then(|| base_model.to_string());
    builtin_collaboration_mode_presets()
        .into_iter()
        .map(|preset| {
            let override_for_mode = override_for_mode(overrides, &preset);
            apply_overrides(preset, base_model.as_ref(), base_effort, override_for_mode)
        })
        .collect()
}

#[cfg(test)]
pub fn test_builtin_collaboration_mode_presets() -> Vec<CollaborationModeMask> {
    builtin_collaboration_mode_presets()
}
fn plan_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: ModeKind::Plan.display_name().to_string(),
        mode: Some(ModeKind::Plan),
        model: None,
        reasoning_effort: Some(Some(ReasoningEffort::Medium)),
        developer_instructions: Some(Some(COLLABORATION_MODE_PLAN.to_string())),
    }
}

fn default_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: ModeKind::Default.display_name().to_string(),
        mode: Some(ModeKind::Default),
        model: None,
        reasoning_effort: None,
        developer_instructions: Some(Some(default_mode_instructions())),
    }
}

fn default_mode_instructions() -> String {
    let known_mode_names = format_mode_names(&TUI_VISIBLE_COLLABORATION_MODES);
    let request_user_input_availability =
        request_user_input_availability_message(ModeKind::Default);
    COLLABORATION_MODE_DEFAULT
        .replace(KNOWN_MODE_NAMES_PLACEHOLDER, &known_mode_names)
        .replace(
            REQUEST_USER_INPUT_AVAILABILITY_PLACEHOLDER,
            &request_user_input_availability,
        )
}

fn format_mode_names(modes: &[ModeKind]) -> String {
    let mode_names: Vec<&str> = modes.iter().map(|mode| mode.display_name()).collect();
    match mode_names.as_slice() {
        [] => "none".to_string(),
        [mode_name] => (*mode_name).to_string(),
        [first, second] => format!("{first} and {second}"),
        [..] => mode_names.join(", "),
    }
}

fn request_user_input_availability_message(mode: ModeKind) -> String {
    let mode_name = mode.display_name();
    if mode.allows_request_user_input() {
        format!("The `request_user_input` tool is available in {mode_name} mode.")
    } else {
        format!(
            "The `request_user_input` tool is unavailable in {mode_name} mode. If you call it while in {mode_name} mode, it will return an error."
        )
    }
}

fn override_for_mode<'a>(
    overrides: Option<&'a CollaborationModeOverrides>,
    mode: &CollaborationModeMask,
) -> Option<&'a CollaborationModeOverride> {
    let overrides = overrides?;
    match mode.mode {
        Some(ModeKind::Plan) => overrides.plan.as_ref(),
        Some(ModeKind::Default | ModeKind::PairProgramming | ModeKind::Execute) => {
            overrides.code.as_ref()
        }
        None => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn preset_names_use_mode_display_names() {
        assert_eq!(plan_preset().name, ModeKind::Plan.display_name());
        assert_eq!(default_preset().name, ModeKind::Default.display_name());
    }

    #[test]
    fn default_mode_instructions_replace_mode_names_placeholder() {
        let default_instructions = default_preset()
            .developer_instructions
            .expect("default preset should include instructions")
            .expect("default instructions should be set");

        assert!(!default_instructions.contains(KNOWN_MODE_NAMES_PLACEHOLDER));
        assert!(!default_instructions.contains(REQUEST_USER_INPUT_AVAILABILITY_PLACEHOLDER));

        let known_mode_names = format_mode_names(&TUI_VISIBLE_COLLABORATION_MODES);
        let expected_snippet = format!("Known mode names are {known_mode_names}.");
        assert!(default_instructions.contains(&expected_snippet));

        let expected_availability_message =
            request_user_input_availability_message(ModeKind::Default);
        assert!(default_instructions.contains(&expected_availability_message));
    }
}
