# Changes in This Fork

This file captures the full set of changes currently in the working tree.

## TUI status header and polling
- Added a status header above the composer that surfaces model + reasoning effort, current directory, git branch/ahead/behind/changes, and rate-limit remaining/reset time.
- Git status is collected in the background (5s interval, 2s timeout) and rendered when available.
- Rate-limit polling is now more frequent (15s) so the header stays fresh.

## TUI auth.json watcher
- The running TUI now watches `CODEX_HOME/auth.json` and reloads auth when the file changes.
- When the account identity changes, the TUI surfaces a warning in the transcript (including old/new emails when available).
- Rate-limit state and polling are refreshed after auth changes so the header reflects the new account.

## Collaboration modes and config overrides
- Added `collaboration_modes` config overrides with per-mode `model` and `reasoning_effort` fields (plan/code).
- Collaboration mode presets now derive defaults from `/model` + reasoning effort and apply the optional overrides.
- The app-server collaboration-mode list uses these overrides and the resolved base model so UI and API stay aligned.
- Built-in presets now set Plan to `medium` reasoning effort and Execute to `xhigh`.

## Codex home fallback for writes
- Added `resolve_codex_home_for_writes`, which falls back to a temp `codex` dir if the default home is not writable and `CODEX_HOME` is unset.
- TUI startup and CLI alias helper placement now use this fallback.

## Misc TUI layout/behavior
- History wrapping uses the larger of viewport width and terminal width to keep scrollback consistent.
- Bottom-pane ordering now places the unified exec footer above the status line.
- The automatic “Implement this plan?” prompt after a plan update has been removed.
