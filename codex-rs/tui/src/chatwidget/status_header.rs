use super::*;
use ratatui::text::Span;

pub(super) fn as_renderable(widget: &ChatWidget) -> RenderableItem<'_> {
    renderable(widget, /*fill_history*/ true)
}

pub(super) fn measure_renderable(widget: &ChatWidget) -> RenderableItem<'_> {
    renderable(widget, /*fill_history*/ false)
}

fn renderable(widget: &ChatWidget, fill_history: bool) -> RenderableItem<'_> {
    let active_cell_renderable = active_cell_renderable(widget);
    let active_cell_renderable = if fill_history {
        RenderableItem::Owned(Box::new(FillHeight::new(active_cell_renderable)))
    } else {
        active_cell_renderable
    };
    let bottom_section = bottom_section_renderable(widget);
    let mut flex = FlexRenderable::new();
    flex.push(/*flex*/ 1, active_cell_renderable);
    flex.push(
        /*flex*/ 0,
        RenderableItem::Owned(Box::new(bottom_section)),
    );
    RenderableItem::Owned(Box::new(flex))
}

fn active_cell_renderable(widget: &ChatWidget) -> RenderableItem<'_> {
    match &widget.active_cell {
        Some(cell) => RenderableItem::Borrowed(cell).inset(Insets::tlbr(
            /*top*/ 1, /*left*/ 0, /*bottom*/ 0, /*right*/ 0,
        )),
        None => RenderableItem::Owned(Box::new(())),
    }
}

fn bottom_section_renderable(widget: &ChatWidget) -> ColumnRenderable<'_> {
    let status_header = StatusHeaderBar::new(
        widget.model_display_name(),
        widget.effective_reasoning_effort(),
        widget.config.cwd.as_path(),
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
    directory: Option<String>,
    git_status: Option<GitStatusSummary>,
    rate_limit_summary: Option<String>,
}

struct FillHeight<'a> {
    child: RenderableItem<'a>,
}

impl<'a> FillHeight<'a> {
    fn new(child: RenderableItem<'a>) -> Self {
        Self { child }
    }
}

impl Renderable for FillHeight<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.child.render(area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        u16::MAX
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.child.cursor_pos(area)
    }
}

impl Renderable for StatusHeaderBar {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if let Some(line) = self.line() {
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
        let directory = cwd
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .or_else(|| {
                if cwd.as_os_str().is_empty() {
                    None
                } else {
                    Some("/".to_string())
                }
            })
            .filter(|label| !label.trim().is_empty());
        let rate_limit_summary = rate_limit_snapshot.and_then(|snapshot| {
            snapshot.primary.as_ref().map(|primary| {
                let remaining = (100.0 - primary.used_percent).clamp(0.0, 100.0).round() as i64;
                let mut summary = format!("{remaining}%");
                if let Some(resets_at) = primary.resets_at.as_ref()
                    && !resets_at.trim().is_empty()
                {
                    summary = format!("{summary} {resets_at}");
                }
                summary
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

    fn line(&self) -> Option<Line<'static>> {
        if !self.has_content() {
            return None;
        }

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

        if let Some(directory) = self.directory.as_ref() {
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
                segment.push(Span::from(format!(" ↑{ahead}")).green());
            }
            let behind = git_status.behind;
            if behind > 0 {
                segment.push(Span::from(format!(" ↓{behind}")).red());
            }
            let changed = git_status.changed;
            if changed > 0 {
                segment.push(Span::from(format!(" +{changed}")).yellow());
            }
            let untracked = git_status.untracked;
            if untracked > 0 {
                segment.push(Span::from(format!(" ?{untracked}")).red());
            }
            push_segment(segment);
        }

        if let Some(summary) = self.rate_limit_summary.as_ref() {
            push_segment(vec!["\u{f464} ".cyan(), Span::from(summary.clone()).cyan()]);
        }

        Some(Line::from(spans))
    }
}

fn format_model_label(model_name: &str, reasoning_effort: Option<ReasoningEffortConfig>) -> String {
    let effort_label = match reasoning_effort {
        Some(ReasoningEffortConfig::Minimal) => "minimal",
        Some(ReasoningEffortConfig::Low) => "low",
        Some(ReasoningEffortConfig::Medium) => "medium",
        Some(ReasoningEffortConfig::High) => "high",
        Some(ReasoningEffortConfig::XHigh) => "xhigh",
        Some(ReasoningEffortConfig::None) | None => "default",
    };
    if model_name.starts_with("codex-auto-") {
        model_name.to_string()
    } else {
        format!("{model_name} {effort_label}")
    }
}
