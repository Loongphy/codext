---
name: status-header-colors
description: Enforce the standard status header bar color scheme when implementing or editing the TUI status header (ratatui Line/Span), including model, directory, git status, and rate limit segments. Use whenever adding/updating a status header bar or related formatting.
---

# Status Header Colors

Apply this color scheme every time the status header bar is implemented or modified. Use Stylize helpers and keep the segment order/formatting consistent.

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
- Segment separator: " │ " in dim.

## Reference snippet (apply as-is)

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

## Usage notes

- Only change colors if this skill explicitly instructs it; do not introduce new colors.
- Keep the separator as dim to avoid competing with the segments.
- Prefer the exact icon codes shown above unless the feature removes a segment entirely.
