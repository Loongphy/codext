---
name: status-header
description: 'Enforce the standard TUI status header layout, icons, colors, and rate-limit summary format, and keep equivalent TUI surfaces aligned when more than one exists.'
---

# Status Header

## Scope

If multiple TUI surfaces render the same header (e.g. `codex-rs/tui` and `codex-rs/tui_app_server`), keep them aligned. Check current dispatch path before deciding which to edit.

## Layout

Header sits above the chat composer inside the bottom section, but below run-state/status indicator
surfaces such as `Working`, unified exec footer, pending approvals, and queued-input previews.
Do not wrap the entire bottom pane with the status header; inject the header into the bottom-pane
composition immediately before the composer so active task state remains above it.

- Top inset: `Insets::tlbr(0, LIVE_PREFIX_COLS, 1, 0)`
- Left gutter: `LIVE_PREFIX_COLS` columns
- Outer spacing above the bottom section already provides the single-row gap above the header; do not add another header-local top spacer.
- When no content: bottom pane gets 1-line top inset instead

## Segment order (fixed)

model → directory → git → rate limit → account

Omit unavailable segments without reordering.

## Icons (Nerd Font v3, never emoji)

| Segment    | Codepoint  | Color    |
|------------|------------|----------|
| Model      | `\u{ee9c}` | cyan     |
| Directory  | `\u{f07c}` | yellow   |
| Git        | `\u{f418}` | blue     |
| Rate limit | `\u{f464}` | cyan     |

Width calc: `UnicodeWidthStr::width("\u{ee9c} ")` (icon + space).

## Colors

- Model: icon + label, cyan
- Directory: icon + path, yellow
- Git: icon + branch blue, ↑ahead green, ↓behind red, +changed yellow, ?untracked red
- Rate limit: icon + summary cyan (format: `95% 23:19`)
- Account: label only cyan, always last. ChatGPT: `user@example.com(Pro)`, API key: `API key`
- Separator: ` │ ` dim

## Example

```rust
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
        if idx > 0 { segment.push(" ".dim()); }
        segment.push(Span::from(path.clone()).yellow());
    }
    push_segment(segment);
}

if let Some(git) = self.git_status.as_ref() {
    let mut segment = vec!["\u{f418} ".blue(), Span::from(git.branch.clone()).blue()];
    if git.ahead > 0 { segment.push(format!(" ↑{}", git.ahead).green()); }
    if git.behind > 0 { segment.push(format!(" ↓{}", git.behind).red()); }
    if git.changed > 0 { segment.push(format!(" +{}", git.changed).yellow()); }
    if git.untracked > 0 { segment.push(format!(" ?{}", git.untracked).red()); }
    push_segment(segment);
}

if let Some(summary) = self.rate_limit_summary.as_ref() {
    push_segment(vec!["\u{f464} ".cyan(), Span::from(summary.clone()).cyan()]);
}

if let Some(label) = self.account_label.as_ref() {
    push_segment(vec![Span::from(label.clone()).cyan()]);
}
```

## Async refresh

- Rate limits: 15s background poll, redraw after each snapshot update
- Git status: 15s poll keyed by session `cwd`; retarget on cwd change, clear stale state, ignore late results
- Directory = session/thread `cwd`, not tool `workdir`

## Checklist

- [ ] All icons are Nerd Font codepoints, not emoji
- [ ] Directory uses yellow, not magenta
- [ ] Git changed count uses yellow, not magenta
- [ ] Width calculations match actual icon codepoints
- [ ] Segment order correct
- [ ] Separator is dim
