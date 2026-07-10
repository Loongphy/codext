use std::path::PathBuf;

use codex_protocol::account::PlanType;
use ratatui::text::Span;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::status::StatusAccountDisplay;
use crate::ui_consts::LIVE_PREFIX_COLS;

use super::*;

pub(super) fn renderable(widget: &ChatWidget) -> Option<RenderableItem<'static>> {
    let header = StatusHeader::new(widget);
    header.has_content().then(|| {
        RenderableItem::Owned(Box::new(header)).inset(Insets::tlbr(
            /*top*/ 0,
            /*left*/ LIVE_PREFIX_COLS,
            /*bottom*/ 1,
            /*right*/ 0,
        ))
    })
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
            let mut previous = None;
            loop {
                let summary = crate::git_status::collect_git_status_summary(&cwd).await;
                if summary != previous {
                    previous.clone_from(&summary);
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

struct StatusHeader {
    model: Option<String>,
    directory: String,
    git: Option<crate::git_status::GitStatusSummary>,
    rate_limit: Option<String>,
    account: Option<String>,
}

impl StatusHeader {
    fn new(widget: &ChatWidget) -> Self {
        let name = widget.model_display_name();
        let model = (!name.trim().is_empty()).then(|| {
            let effort = ChatWidget::status_line_reasoning_effort_label(
                widget.effective_reasoning_effort().as_ref(),
            );
            if name.starts_with("codex-auto-") {
                name.to_string()
            } else {
                format!("{name} {effort}")
            }
        });
        let snapshot = widget
            .rate_limit_snapshots_by_limit_id
            .get("codex")
            .or_else(|| widget.rate_limit_snapshots_by_limit_id.values().next());
        let rate_limit = snapshot.and_then(|snapshot| {
            snapshot.primary.as_ref().map(|primary| {
                let remaining = (100.0 - primary.used_percent).clamp(0.0, 100.0).round() as i64;
                match primary.resets_at.as_deref().and_then(|reset| reset.split_once(' ')) {
                    Some((time, _)) => format!("{remaining}% {time}"),
                    None => format!("{remaining}%"),
                }
            })
        });
        Self {
            model,
            directory: crate::status::format_directory_display(widget.status_line_cwd(), None),
            git: widget.status_header_git_status.clone(),
            rate_limit,
            account: account_label(
                widget.status_account_display(),
                widget.current_plan_type(),
                widget.has_chatgpt_account(),
            ),
        }
    }

    fn has_content(&self) -> bool {
        self.model.is_some()
            || !self.directory.is_empty()
            || self.git.is_some()
            || self.rate_limit.is_some()
            || self.account.is_some()
    }

    fn line(&self, width: usize) -> Line<'static> {
        let mut spans = Vec::new();
        let mut push = |segment: Vec<Span<'static>>| {
            if !spans.is_empty() {
                spans.push(" │ ".dim());
            }
            spans.extend(segment);
        };
        if let Some(model) = &self.model {
            push(vec!["\u{ee9c} ".cyan(), Span::from(model.clone()).cyan()]);
        }
        if !self.directory.is_empty() {
            let available = width.saturating_sub(UnicodeWidthStr::width("\u{f07c} ")).max(8);
            let directory = crate::text_formatting::center_truncate_path(&self.directory, available);
            push(vec!["\u{f07c} ".yellow(), Span::from(directory).yellow()]);
        }
        if let Some(git) = &self.git {
            let mut segment = vec!["\u{f418} ".blue(), Span::from(git.branch.clone()).blue()];
            if git.ahead > 0 { segment.push(format!(" ↑{}", git.ahead).green()); }
            if git.behind > 0 { segment.push(format!(" ↓{}", git.behind).red()); }
            if git.changed > 0 { segment.push(format!(" +{}", git.changed).yellow()); }
            if git.untracked > 0 { segment.push(format!(" ?{}", git.untracked).red()); }
            push(segment);
        }
        if let Some(rate_limit) = &self.rate_limit {
            push(vec!["\u{f464} ".cyan(), Span::from(rate_limit.clone()).cyan()]);
        }
        if let Some(account) = &self.account {
            push(vec![Span::from(account.clone()).cyan()]);
        }
        Line::from(spans)
    }
}

impl Renderable for StatusHeader {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.line(usize::from(area.width)).render(area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        u16::from(self.has_content())
    }
}

fn account_label(
    account: Option<&StatusAccountDisplay>,
    plan: Option<PlanType>,
    has_chatgpt_account: bool,
) -> Option<String> {
    if !has_chatgpt_account {
        return Some("API key".to_string());
    }
    match account {
        Some(StatusAccountDisplay::ChatGpt { email, plan: label }) => match (email, label) {
            (Some(email), Some(plan)) => Some(format!("{email}({plan})")),
            (Some(email), None) => Some(format!(
                "{}({})",
                email,
                plan.map(crate::status::plan_type_display_name)?
            )),
            _ => None,
        },
        Some(StatusAccountDisplay::ApiKey) => Some("API key".to_string()),
        None => None,
    }
}
