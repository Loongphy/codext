use codex_config::types::CollaborationModeOverride;
use codex_config::types::CollaborationModeOverrides;
use codex_collaboration_mode_templates::DEFAULT as COLLABORATION_MODE_DEFAULT;
use codex_collaboration_mode_templates::PLAN as COLLABORATION_MODE_PLAN;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::TUI_VISIBLE_COLLABORATION_MODES;
use codex_protocol::openai_models::ReasoningEffort;
use codex_utils_template::Template;
use std::sync::LazyLock;

const KNOWN_MODE_NAMES_TEMPLATE_KEY: &str = "KNOWN_MODE_NAMES";
const REQUEST_USER_INPUT_AVAILABILITY_TEMPLATE_KEY: &str = "REQUEST_USER_INPUT_AVAILABILITY";
const ASKING_QUESTIONS_GUIDANCE_TEMPLATE_KEY: &str = "ASKING_QUESTIONS_GUIDANCE";
static COLLABORATION_MODE_DEFAULT_TEMPLATE: LazyLock<Template> = LazyLock::new(|| {
    Template::parse(COLLABORATION_MODE_DEFAULT)
        .unwrap_or_else(|err| panic!("collaboration mode default template must parse: {err}"))
});

/// Stores feature flags that control collaboration-mode behavior.
///
/// Keep mode-related flags here so new collaboration-mode capabilities can be
/// added without large cross-cutting diffs to constructor and call-site
/// signatures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CollaborationModesConfig {
    /// Enables `request_user_input` availability in Default mode.
    pub default_mode_request_user_input: bool,
}

pub fn builtin_collaboration_mode_presets(
    collaboration_modes_config: CollaborationModesConfig,
) -> Vec<CollaborationModeMask> {
    vec![plan_preset(), default_preset(collaboration_modes_config)]
}

pub fn collaboration_mode_presets_with_overrides(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
) -> Vec<CollaborationModeMask> {
    collaboration_mode_presets_with_overrides_and_config(
        base_model,
        base_effort,
        overrides,
        CollaborationModesConfig::default(),
    )
}

pub fn collaboration_mode_presets_with_overrides_and_config(
    base_model: &str,
    base_effort: Option<ReasoningEffort>,
    overrides: Option<&CollaborationModeOverrides>,
    collaboration_modes_config: CollaborationModesConfig,
) -> Vec<CollaborationModeMask> {
    let base_model = base_model.trim();
    let base_model = (!base_model.is_empty()).then_some(base_model);

    builtin_collaboration_mode_presets(collaboration_modes_config)
        .into_iter()
        .map(|preset| {
            let override_for_mode = override_for_mode(overrides, &preset);
            apply_overrides(preset, base_model, base_effort, override_for_mode)
        })
        .collect()
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

fn default_preset(collaboration_modes_config: CollaborationModesConfig) -> CollaborationModeMask {
    CollaborationModeMask {
        name: ModeKind::Default.display_name().to_string(),
        mode: Some(ModeKind::Default),
        model: None,
        reasoning_effort: None,
        developer_instructions: Some(Some(default_mode_instructions(collaboration_modes_config))),
    }
}

fn default_mode_instructions(collaboration_modes_config: CollaborationModesConfig) -> String {
    let known_mode_names = format_mode_names(&TUI_VISIBLE_COLLABORATION_MODES);
    let request_user_input_availability = request_user_input_availability_message(
        ModeKind::Default,
        collaboration_modes_config.default_mode_request_user_input,
    );
    let asking_questions_guidance = asking_questions_guidance_message(
        collaboration_modes_config.default_mode_request_user_input,
    );
    COLLABORATION_MODE_DEFAULT_TEMPLATE
        .render([
            (KNOWN_MODE_NAMES_TEMPLATE_KEY, known_mode_names.as_str()),
            (
                REQUEST_USER_INPUT_AVAILABILITY_TEMPLATE_KEY,
                request_user_input_availability.as_str(),
            ),
            (
                ASKING_QUESTIONS_GUIDANCE_TEMPLATE_KEY,
                asking_questions_guidance.as_str(),
            ),
        ])
        .unwrap_or_else(|err| panic!("collaboration mode default template must render: {err}"))
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

fn request_user_input_availability_message(
    mode: ModeKind,
    default_mode_request_user_input: bool,
) -> String {
    let mode_name = mode.display_name();
    if mode.allows_request_user_input()
        || (default_mode_request_user_input && mode == ModeKind::Default)
    {
        format!("The `request_user_input` tool is available in {mode_name} mode.")
    } else {
        format!(
            "The `request_user_input` tool is unavailable in {mode_name} mode. If you call it while in {mode_name} mode, it will return an error."
        )
    }
}

fn asking_questions_guidance_message(default_mode_request_user_input: bool) -> String {
    if default_mode_request_user_input {
        "In Default mode, strongly prefer making reasonable assumptions and executing the user's request rather than stopping to ask questions. If you absolutely must ask a question because the answer cannot be discovered from local context and a reasonable assumption would be risky, prefer using the `request_user_input` tool rather than writing a multiple choice question as a textual assistant message. Never write a multiple choice question as a textual assistant message.".to_string()
    } else {
        "In Default mode, strongly prefer making reasonable assumptions and executing the user's request rather than stopping to ask questions. If you absolutely must ask a question because the answer cannot be discovered from local context and a reasonable assumption would be risky, ask the user directly with a concise plain-text question. Never write a multiple choice question as a textual assistant message.".to_string()
    }
}

fn override_for_mode<'a>(
    overrides: Option<&'a CollaborationModeOverrides>,
    mode: &CollaborationModeMask,
) -> Option<&'a CollaborationModeOverride> {
    let overrides = overrides?;
    match mode.mode {
        Some(ModeKind::Plan) => overrides.plan.as_ref(),
        Some(ModeKind::Default) => overrides.code.as_ref(),
        Some(ModeKind::PairProgramming | ModeKind::Execute) | None => None,
    }
}

fn apply_overrides(
    mut preset: CollaborationModeMask,
    base_model: Option<&str>,
    base_effort: Option<ReasoningEffort>,
    override_for_mode: Option<&CollaborationModeOverride>,
) -> CollaborationModeMask {
    let override_model = override_for_mode.and_then(|value| value.model.as_deref());
    let override_effort = override_for_mode.and_then(|value| value.reasoning_effort);

    preset.model = override_model
        .or(base_model)
        .map(std::borrow::ToOwned::to_owned);
    preset.reasoning_effort = Some(Some(
        override_effort.unwrap_or_else(|| {
            preset
                .reasoning_effort
                .flatten()
                .or(base_effort)
                .unwrap_or(ReasoningEffort::Medium)
        }),
    ));
    preset
}

#[cfg(test)]
#[path = "collaboration_mode_presets_tests.rs"]
mod tests;
