//! Status header row shown above the chat composer (bottom pane).
//!
//! # Layout
//!
//! The status header is rendered as the first child of the inner [`ColumnRenderable`]
//! inside [`bottom_section_renderable`], directly above the bottom pane.
//!
//! To keep the visual rhythm consistent across the TUI:
//!
//! - **Top spacing:** The enclosing bottom section already contributes the 1-line
//!   gap above this surface, so the status header itself does not add another top
//!   inset.
//! - **Left gutter:** The content is left-indented by [`LIVE_PREFIX_COLS`] columns,
//!   matching the gutter used by the configurable footer `/statusline` and history
//!   cells. The same constant (`crate::ui_consts::LIVE_PREFIX_COLS`) drives both
//!   indents, so any future change to the gutter width stays in sync automatically.

use std::path::Path;
use std::path::PathBuf;

use ratatui::text::Span;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::status::StatusAccountDisplay;
use crate::ui_consts::LIVE_PREFIX_COLS;
use codex_protocol::account::PlanType;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;

use super::*;

pub(super) fn renderable(widget: &ChatWidget) -> Option<RenderableItem<'static>> {
    let status_header = StatusHeaderBar::new(widget);
    if !status_header.has_content() {
        return None;
    }
    Some(
        RenderableItem::Owned(Box::new(status_header)).inset(Insets::tlbr(
            /*top*/ 0,
            /*left*/ LIVE_PREFIX_COLS,
            /*bottom*/ 1,
            /*right*/ 0,
        )),
    )
}

impl ChatWidget {
    pub(super) fn sync_status_header_git_status_poller(&mut self) {
        let cwd = self.status_line_cwd().to_path_buf();
        if self.status_header_git_status_cwd.as_ref() == Some(&cwd)
            && self.status_header_git_status_task.is_some()
        {
            return;
        }

        self.stop_status_header_git_status_poller();
        self.status_header_git_status = None;
        self.status_header_git_status_cwd = Some(cwd.clone());
        self.request_redraw();

        let app_event_tx = self.app_event_tx.clone();
        self.status_header_git_status_task = Some(tokio::spawn(async move {
            let mut last_summary: Option<crate::git_status::GitStatusSummary> = None;
            loop {
                let summary = crate::git_status::collect_git_status_summary(&cwd).await;
                if summary != last_summary {
                    last_summary.clone_from(&summary);
                    app_event_tx.send(AppEvent::StatusHeaderGitStatusUpdated {
                        cwd: cwd.clone(),
                        summary,
                    });
                }
                tokio::time::sleep(Duration::from_secs(/*secs*/ 15)).await;
            }
        }));
    }

    pub(crate) fn set_status_header_git_status(
        &mut self,
        cwd: PathBuf,
        summary: Option<crate::git_status::GitStatusSummary>,
    ) {
        if self.status_line_cwd() != cwd.as_path() {
            return;
        }
        self.status_header_git_status_cwd = Some(cwd);
        self.status_header_git_status = summary;
        self.request_redraw();
    }

    pub(super) fn stop_status_header_git_status_poller(&mut self) {
        if let Some(task) = self.status_header_git_status_task.take() {
            task.abort();
        }
    }
}

struct StatusHeaderBar {
    model_name: Option<String>,
    account_label: Option<String>,
    directories: Vec<PathBuf>,
    git_status: Option<crate::git_status::GitStatusSummary>,
    rate_limit_summary: Option<String>,
}

impl Renderable for StatusHeaderBar {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if let Some(line) = self.line(usize::from(area.width)) {
            line.render(area, buf);
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        if self.has_content() { 1 } else { 0 }
    }
}

impl StatusHeaderBar {
    fn new(widget: &ChatWidget) -> Self {
        let model_name = widget.model_display_name();
        let model_name = (!model_name.trim().is_empty())
            .then(|| format_model_label(model_name, widget.effective_reasoning_effort()));
        let mut directories = Vec::new();
        push_directory_context(&mut directories, widget.status_line_cwd());
        let rate_limit_snapshot = widget
            .rate_limit_snapshots_by_limit_id
            .get("codex")
            .or_else(|| widget.rate_limit_snapshots_by_limit_id.values().next());
        let rate_limit_summary = rate_limit_snapshot.and_then(|snapshot| {
            snapshot.primary.as_ref().map(|primary| {
                let remaining = (100.0 - primary.used_percent).clamp(0.0, 100.0).round() as i64;
                match primary.resets_at.as_deref() {
                    Some(resets_at) if !resets_at.trim().is_empty() => {
                        format!("{remaining}% {}", compact_reset_time(resets_at))
                    }
                    _ => format!("{remaining}%"),
                }
            })
        });
        Self {
            model_name,
            account_label: status_header_account_label(
                widget.status_account_display(),
                widget.current_plan_type(),
                widget.has_chatgpt_account(),
            ),
            directories,
            git_status: widget.status_header_git_status.clone(),
            rate_limit_summary,
        }
    }

    fn has_content(&self) -> bool {
        self.model_name.is_some()
            || self.account_label.is_some()
            || !self.directories.is_empty()
            || self.git_status.is_some()
            || self.rate_limit_summary.is_some()
    }

    fn line(&self, max_width: usize) -> Option<Line<'static>> {
        if !self.has_content() || max_width == 0 {
            return None;
        }

        let directory_width = max_width.saturating_sub(self.fixed_width()).max(1);
        let per_directory_width = directory_width
            .checked_div(self.directories.len().max(1))
            .unwrap_or(directory_width)
            .max(8);
        let directories = self
            .directories
            .iter()
            .map(|directory| compact_directory_display(directory.as_path(), per_directory_width))
            .collect::<Vec<_>>();

        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut push_segment = |segment: Vec<Span<'static>>| {
            if !spans.is_empty() {
                spans.push(" │ ".dim());
            }
            spans.extend(segment);
        };

        if let Some(model_name) = self.model_name.as_ref() {
            push_segment(vec!["\u{ee9c} ".cyan(), Span::from(model_name.clone()).cyan()]);
        }

        if !directories.is_empty() {
            let mut segment = vec!["\u{f07c} ".yellow()];
            for (idx, path) in directories.iter().enumerate() {
                if idx > 0 {
                    segment.push(" ".dim());
                }
                segment.push(Span::from(path.clone()).yellow());
            }
            push_segment(segment);
        }

        if let Some(git_status) = self.git_status.as_ref() {
            let mut segment = vec!["\u{f418} ".blue(), Span::from(git_status.branch.clone()).blue()];
            let ahead = git_status.ahead;
            if ahead > 0 {
                segment.push(format!(" ↑{ahead}").green());
            }
            let behind = git_status.behind;
            if behind > 0 {
                segment.push(format!(" ↓{behind}").red());
            }
            let changed = git_status.changed;
            if changed > 0 {
                segment.push(format!(" +{changed}").yellow());
            }
            let untracked = git_status.untracked;
            if untracked > 0 {
                segment.push(format!(" ?{untracked}").red());
            }
            push_segment(segment);
        }

        if let Some(summary) = self.rate_limit_summary.as_ref() {
            push_segment(vec!["\u{f464} ".cyan(), Span::from(summary.clone()).cyan()]);
        }

        if let Some(account_label) = self.account_label.as_ref() {
            push_segment(vec![Span::from(account_label.clone()).cyan()]);
        }

        Some(Line::from(spans))
    }

    fn fixed_width(&self) -> usize {
        let model_width = self
            .model_name
            .as_ref()
            .map(|model_name| UnicodeWidthStr::width("\u{ee9c} ") + model_name.width())
            .unwrap_or(0);
        let account_width = self
            .account_label
            .as_ref()
            .map(|account_label| account_label.width())
            .unwrap_or(0);
        let directory_width = if self.directories.is_empty() {
            0
        } else {
            UnicodeWidthStr::width("\u{f07c} ") + self.directories.len().saturating_sub(1)
        };
        let git_width = self
            .git_status
            .as_ref()
            .map(|git_status| {
                let mut width =
                    UnicodeWidthStr::width("\u{f418} ") + git_status.branch.as_str().width();
                let ahead = git_status.ahead;
                if ahead > 0 {
                    width += format!(" ↑{ahead}").width();
                }
                let behind = git_status.behind;
                if behind > 0 {
                    width += format!(" ↓{behind}").width();
                }
                let changed = git_status.changed;
                if changed > 0 {
                    width += format!(" +{changed}").width();
                }
                let untracked = git_status.untracked;
                if untracked > 0 {
                    width += format!(" ?{untracked}").width();
                }
                width
            })
            .unwrap_or(0);
        let rate_limit_width = self
            .rate_limit_summary
            .as_ref()
            .map(|summary| UnicodeWidthStr::width("\u{f464} ") + summary.width())
            .unwrap_or(0);
        let segment_count = usize::from(self.model_name.is_some())
            + usize::from(self.account_label.is_some())
            + usize::from(!self.directories.is_empty())
            + usize::from(self.git_status.is_some())
            + usize::from(self.rate_limit_summary.is_some());
        let separator_width = UnicodeWidthStr::width(" │ ") * segment_count.saturating_sub(1);

        model_width
            + account_width
            + directory_width
            + git_width
            + rate_limit_width
            + separator_width
    }
}

fn push_directory_context(directories: &mut Vec<PathBuf>, path: &Path) {
    if crate::status::format_directory_display(path, None)
        .trim()
        .is_empty()
    {
        return;
    }
    directories.push(path.to_path_buf());
}

fn compact_directory_display(directory: &Path, available_width: usize) -> String {
    let full_directory = crate::status::format_directory_display(directory, None);
    if UnicodeWidthStr::width(full_directory.as_str()) <= available_width {
        return full_directory;
    }

    let separator = std::path::MAIN_SEPARATOR;
    let separator_string = separator.to_string();
    let has_leading_separator = full_directory.starts_with(separator);
    let segments: Vec<&str> = full_directory
        .split(separator)
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return crate::status::format_directory_display(directory, Some(available_width));
    }

    let join_segments = |leading_separator: bool, segments: &[&str]| {
        let joined = segments.join(separator_string.as_str());
        if leading_separator {
            format!("{separator}{joined}")
        } else {
            joined
        }
    };
    let mut candidates = vec![full_directory.clone()];
    let push_candidate = |candidates: &mut Vec<String>, candidate: String| {
        if !candidate.is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    };

    let prefix_count = if has_leading_separator {
        1
    } else if segments
        .first()
        .is_some_and(|segment| *segment == "~" || segment.ends_with(':'))
    {
        std::cmp::min(2, segments.len())
    } else {
        1
    };
    let last_segment = segments.last().copied().unwrap_or_default();
    if segments.len() > prefix_count {
        let prefix = join_segments(has_leading_separator, &segments[..prefix_count]);
        push_candidate(
            &mut candidates,
            format!("{prefix}{separator}...{separator}{last_segment}"),
        );
    }
    if segments.len() >= 2 {
        push_candidate(
            &mut candidates,
            join_segments(false, &segments[segments.len() - 2..]),
        );
    }
    push_candidate(&mut candidates, format!("...{separator}{last_segment}"));

    candidates
        .into_iter()
        .find(|candidate| UnicodeWidthStr::width(candidate.as_str()) <= available_width)
        .unwrap_or_else(|| {
            crate::text_formatting::center_truncate_path(
                &format!("...{separator}{last_segment}"),
                available_width,
            )
        })
}

fn format_model_label(model_name: &str, reasoning_effort: Option<ReasoningEffortConfig>) -> String {
    let effort_label = ChatWidget::status_line_reasoning_effort_label(reasoning_effort.as_ref());
    if model_name.starts_with("codex-auto-") {
        model_name.to_string()
    } else {
        format!("{model_name} {effort_label}")
    }
}

fn status_header_account_label(
    account_display: Option<&StatusAccountDisplay>,
    plan_type: Option<PlanType>,
    has_chatgpt_account: bool,
) -> Option<String> {
    if !has_chatgpt_account {
        return Some("API key".to_string());
    }

    let (email, display_plan) = match account_display {
        Some(StatusAccountDisplay::ChatGpt { email, plan }) => (email.as_deref(), plan.as_deref()),
        Some(StatusAccountDisplay::ApiKey) => return Some("API key".to_string()),
        None => return None,
    };

    let plan = match display_plan {
        Some(plan) => Some(plan.to_string()),
        None => plan_type.map(crate::status::plan_type_display_name),
    };

    match (email, plan) {
        (Some(email), Some(plan)) => Some(format!("{email}({plan})")),
        (Some(email), None) => Some(email.to_string()),
        (None, Some(plan)) => Some(plan),
        (None, None) => None,
    }
}

fn compact_reset_time(resets_at: &str) -> &str {
    resets_at
        .split_once(' ')
        .map_or(resets_at, |(time, _)| time)
}
