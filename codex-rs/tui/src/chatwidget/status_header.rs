use super::*;
use ratatui::text::Span;
use std::path::PathBuf;
use unicode_width::UnicodeWidthStr;

pub(super) fn as_renderable(widget: &ChatWidget) -> RenderableItem<'_> {
    let items = vec![
        active_cell_renderable(widget),
        active_hook_cell_renderable(widget),
        RenderableItem::Owned(Box::new(bottom_section_renderable(widget))),
    ];
    RenderableItem::Owned(Box::new(ColumnRenderable::with(items)))
}

fn active_cell_renderable(widget: &ChatWidget) -> RenderableItem<'_> {
    match &widget.active_cell {
        Some(cell) => RenderableItem::Borrowed(cell).inset(Insets::tlbr(
            /*top*/ 1, /*left*/ 0, /*bottom*/ 0, /*right*/ 0,
        )),
        None => RenderableItem::Owned(Box::new(())),
    }
}

fn active_hook_cell_renderable(widget: &ChatWidget) -> RenderableItem<'_> {
    match &widget.active_hook_cell {
        Some(cell) if cell.should_render() => RenderableItem::Borrowed(cell).inset(Insets::tlbr(
            /*top*/ 1, /*left*/ 0, /*bottom*/ 0, /*right*/ 0,
        )),
        _ => RenderableItem::Owned(Box::new(())),
    }
}

fn bottom_section_renderable(widget: &ChatWidget) -> ColumnRenderable<'_> {
    let status_header = StatusHeaderBar::new(
        widget.model_display_name(),
        widget.effective_reasoning_effort(),
        widget.status_line_cwd(),
        widget.git_status.clone(),
        widget
            .rate_limit_snapshots_by_limit_id
            .get("codex")
            .or_else(|| widget.rate_limit_snapshots_by_limit_id.values().next()),
    );
    let mut items: Vec<RenderableItem<'_>> = Vec::new();
    if status_header.has_content() {
        items.push(RenderableItem::Owned("".into()));
        items.push(RenderableItem::Owned(Box::new(status_header)));
        items.push(RenderableItem::Owned("".into()));
    }
    items.push(
        RenderableItem::Borrowed(&widget.bottom_pane).inset(Insets::tlbr(
            /*top*/ 1, /*left*/ 0, /*bottom*/ 0, /*right*/ 0,
        )),
    );
    ColumnRenderable::with(items)
}

struct StatusHeaderBar {
    model_name: Option<String>,
    directory: Option<PathBuf>,
    git_status: Option<GitStatusSummary>,
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
    fn new(
        model_name: &str,
        reasoning_effort: Option<ReasoningEffortConfig>,
        cwd: &Path,
        git_status: Option<GitStatusSummary>,
        rate_limit_snapshot: Option<&RateLimitSnapshotDisplay>,
    ) -> Self {
        let model_name = (!model_name.trim().is_empty())
            .then(|| format_model_label(model_name, reasoning_effort));
        let directory = (!crate::status::format_directory_display(cwd, /*max_width*/ None)
            .trim()
            .is_empty())
        .then(|| cwd.to_path_buf());
        let rate_limit_summary = rate_limit_snapshot.and_then(|snapshot| {
            snapshot.primary.as_ref().map(|primary| {
                let remaining = (100.0 - primary.used_percent).clamp(0.0, 100.0).round() as i64;
                match primary.resets_at.as_deref() {
                    Some(resets_at) if !resets_at.trim().is_empty() => {
                        format!("{remaining}% {resets_at}")
                    }
                    _ => format!("{remaining}%"),
                }
            })
        });
        Self {
            model_name,
            directory,
            git_status,
            rate_limit_summary,
        }
    }

    fn has_content(&self) -> bool {
        self.model_name.is_some()
            || self.directory.is_some()
            || self.git_status.is_some()
            || self.rate_limit_summary.is_some()
    }

    fn line(&self, max_width: usize) -> Option<Line<'static>> {
        if !self.has_content() || max_width == 0 {
            return None;
        }

        let directory = self.directory.as_ref().map(|directory| {
            let available_width = max_width.saturating_sub(self.fixed_width()).max(1);
            let full_directory =
                crate::status::format_directory_display(directory, /*max_width*/ None);
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
                    format!("{prefix}{separator}…{separator}{last_segment}"),
                );
            }
            if segments.len() >= 2 {
                push_candidate(
                    &mut candidates,
                    join_segments(false, &segments[segments.len() - 2..]),
                );
            }
            push_candidate(&mut candidates, format!("…{separator}{last_segment}"));

            candidates
                .into_iter()
                .find(|candidate| UnicodeWidthStr::width(candidate.as_str()) <= available_width)
                .unwrap_or_else(|| {
                    crate::text_formatting::center_truncate_path(
                        &format!("…{separator}{last_segment}"),
                        available_width,
                    )
                })
        });

        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut push_segment = |segment: Vec<Span<'static>>| {
            if !spans.is_empty() {
                spans.push(" │ ".dim());
            }
            spans.extend(segment);
        };

        if let Some(model_name) = self.model_name.as_ref() {
            push_segment(vec![
                "\u{ee9c} ".cyan(),
                Span::from(model_name.clone()).cyan(),
            ]);
        }

        if let Some(directory) = directory.as_ref() {
            push_segment(vec![
                "\u{f07c} ".yellow(),
                Span::from(directory.clone()).yellow(),
            ]);
        }

        if let Some(git_status) = self.git_status.as_ref() {
            let mut segment = vec![
                "\u{f418} ".blue(),
                Span::from(git_status.branch.clone()).blue(),
            ];
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

        Some(Line::from(spans))
    }

    fn fixed_width(&self) -> usize {
        let model_width = self
            .model_name
            .as_ref()
            .map(|model_name| UnicodeWidthStr::width("\u{ee9c} ") + model_name.width())
            .unwrap_or(0);
        let directory_width = self
            .directory
            .as_ref()
            .map(|_| UnicodeWidthStr::width("\u{f07c} "))
            .unwrap_or(0);
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
            + usize::from(self.directory.is_some())
            + usize::from(self.git_status.is_some())
            + usize::from(self.rate_limit_summary.is_some());
        let separator_width = UnicodeWidthStr::width(" │ ") * segment_count.saturating_sub(1);

        model_width + directory_width + git_width + rate_limit_width + separator_width
    }
}

fn format_model_label(model_name: &str, reasoning_effort: Option<ReasoningEffortConfig>) -> String {
    let effort_label = ChatWidget::status_line_reasoning_effort_label(reasoning_effort);
    if model_name.starts_with("codex-auto-") {
        model_name.to_string()
    } else {
        format!("{model_name} {effort_label}")
    }
}
