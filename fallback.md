# Fallbacks

## Automatic approval review fallback to manual user approval

- Reason: automatic approval review can time out or fail internally (for example due to reviewer-side usage limits). In those cases, returning a hard denial would regress the older manual approval path for sandbox escalation.
- Protected callers or data: live sandbox approval requests flowing through `codex-rs/core/src/tools/orchestrator.rs` into the TUI/app-server approval UI. No persisted data is involved.
- Removal conditions: remove this fallback once automatic approval review can reliably distinguish internal reviewer failures from real denials, or once product requirements explicitly drop manual approval fallback for reviewer-unavailable cases.
