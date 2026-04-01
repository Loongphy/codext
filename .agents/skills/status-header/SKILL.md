---
name: status-header
description: 'Enforce the standard TUI status header layout, icons, colors, and rate-limit summary format, and keep equivalent tui and tui_app_server surfaces aligned.'
---

# Status Header

Apply these conventions every time the status header bar is implemented or modified. Treat this skill as defining user-visible behavior, not as permission to update only one code path. Use Stylize helpers and keep the segment order/formatting consistent.

## Scope and synchronization

- Before editing the header, identify every implementation that renders the same user-visible surface. In this repo that usually means both `codex-rs/tui` and `codex-rs/tui_app_server`.
- If both implementations expose the same header, keep them aligned. Do not mark the task complete after changing only one side unless the other side has been intentionally removed upstream or there is a documented reason not to sync it.
- Do not assume the classic `tui` is the runtime path users see. Check the current dispatch path for the target tag/config before deciding which implementation to edit.
- Match behavior first, not plumbing. The classic `tui` may use local polling, while `tui_app_server` may use bootstrap data or app-server events; either is acceptable as long as the rendered header stays behaviorally aligned and fresh.

## Required color mapping

- Model segment: icon + label in cyan.
- Directory segment: icon + path in yellow.
- Git segment:
  - icon + branch in blue
  - ahead count in green
  - behind count in red
  - changed count in yellow
  - untracked count in red
- Rate limit segment: icon + summary in cyan.
  - Summary format: `95% 23:19`
- Segment separator: " │ " in dim.

## Reference snippet (behavioral template, adapt to local architecture)

```rust
let mut spans: Vec<Span<'static>> = Vec::new();
let mut push_segment = |segment: Vec<Span<'static>>| {
    if !spans.is_empty() {
        spans.push(" │ ".dim());
    }
    spans.extend(segment);
};

if let Some(model_name) = self.model_name.as_ref() {
    let label = format_model_label(model_name);
    push_segment(vec!["\u{ee9c} ".cyan(), Span::from(label).cyan()]);
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
```

Use the snippet as a template for segment order, icon usage, and color intent. Adapt field names,
ownership, helper selection, and refresh wiring to the local module instead of cargo-culting the
exact code.

## Usage notes

- Only change colors if this skill explicitly instructs it; do not introduce new colors.
- Keep the separator as dim to avoid competing with the segments.
- Prefer the exact icon codes shown above unless the feature removes a segment entirely.
- If a repo-level lint, style rule, or existing helper abstraction rejects the exact method calls in
  the snippet, keep the same visual result using the repo-approved mechanism instead of forcing the
  snippet verbatim.
- If a status-header segment depends on background-polled or async state (for example rate-limit
  data fetched from `/usage`), the update path must explicitly request a redraw/frame after the
  cached state changes so the header updates while the UI is otherwise idle.
- The redraw requirement applies to every implementation that renders the header. If `tui` and
  `tui_app_server` both show the header, each side needs its own refresh path and redraw trigger.
- For `tui_app_server`, do not assume the rate-limit source is local `/usage` polling; event-driven
  or bootstrap-fed data is acceptable if it keeps the header equivalently fresh.
