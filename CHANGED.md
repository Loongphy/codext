# Changes in This Fork

This file captures the full set of changes currently in the working tree.

## TUI composer draft clipboard shortcut

- Added `Ctrl+Shift+C` in the TUI composer to copy the current draft to the system clipboard when the input contains text.
- Existing `Ctrl+C` behavior stays unchanged.
- When the composer has no copyable text, `Ctrl+Shift+C` falls back to the existing `Ctrl+C` clear/interrupt/quit path.
- On WSL2, composer draft copy reuses the existing Windows clipboard fallback so copies still land in the Windows system clipboard.
- `Ctrl+Shift+C` now takes its own composer-copy path instead of falling through to the existing `Ctrl+C` clear/interrupt/quit behavior when draft text is present.
- Added footer shortcut help text for the new draft-copy binding.
- `rust-v0.118.0` removed the old `tui_app_server` crate upstream, so this behavior now lives in the app-server-backed `codex-rs/tui` surface only.

## TUI status header and polling

- Added a status header above the composer in the app-server-backed `codex-rs/tui` surface. It shows model + reasoning effort, current directory, git branch/ahead/behind/changes, and rate-limit remaining/reset time.
- Git status is collected in the background (15s interval, 2s timeout) and rendered when available.
- ChatGPT `5h` / weekly usage-limit snapshots in the TUI now refresh in the background every 15 seconds, so the header and any `/statusline` limit items keep moving while the UI is otherwise idle.
- `rust-v0.118.0` removed the old `tui_app_server` crate upstream, so the reapply keeps only the surviving TUI path aligned with the status-header skill.

## TUI auth.json watcher

- The running TUI now watches `CODEX_HOME/auth.json` and reloads auth when the file changes.
- Watch notifications are now trailing-debounced so reload happens after writes settle, reducing partial-file reads.
- Auth reload failures no longer clear cached auth (so transient parse/read errors do not appear as a logout).
- On auth reload failure, the TUI retries every 5 seconds for up to 3 attempts before surfacing a final warning.
- When the account identity changes, the TUI surfaces a warning in the transcript (including old/new emails when available).
- Auth change warnings now show the account plan type (e.g., Plus/Team/Free/Pro) instead of the generic ChatGPT label.
- Rate-limit state and polling are refreshed after auth changes so the header reflects the new account.

## Collaboration modes and config overrides

- Added `collaboration_modes` config overrides with per-mode `model` and `reasoning_effort` fields (plan/code).
- Collaboration mode presets now derive defaults from `/model` + reasoning effort and apply the optional overrides.
- The app-server collaboration-mode list uses these overrides and the resolved base model so UI and API stay aligned.
- Built-in Plan preset keeps `medium` reasoning effort by default, while allowing per-mode override via config.

## AGENTS.md reload semantics

- On each new user turn, Codex now checks whether project docs (`AGENTS.md` hierarchy) changed.
- If changed, it reloads instructions before creating the turn, so updates made during a running turn take effect on the next turn.
- When a reload happens, Codex emits an explicit warning in the transcript:
  `AGENTS.md instructions changed. Reloaded and applied starting this turn.`

## TUI exit resume command

- Added a fork requirement that the final resume hint shown after exiting Codex TUI uses `codext resume <session>` instead of `codex resume <session>`.
