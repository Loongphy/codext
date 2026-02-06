# Changes in This Fork

This file captures the full set of changes currently in the working tree.

## TUI status header and polling

- Added a status header above the composer that surfaces model + reasoning effort, current directory, git branch/ahead/behind/changes, and rate-limit remaining/reset time.
- Git status is collected in the background (5s interval, 2s timeout) and rendered when available.
- Rate-limit polling is now more frequent (15s) so the header stays fresh.

## TUI auth.json watcher

- The running TUI now watches `CODEX_HOME/auth.json` and reloads auth when the file changes.
- When the account identity changes, the TUI surfaces a warning in the transcript (including old/new emails when available).
- Auth change warnings now show the account plan type (e.g., Plus/Team/Free/Pro) instead of the generic ChatGPT label.
- Rate-limit state and polling are refreshed after auth changes so the header reflects the new account.

## Collaboration modes and config overrides

- Added `collaboration_modes` config overrides with per-mode `model` and `reasoning_effort` fields (plan/code).
- Collaboration mode presets now derive defaults from `/model` + reasoning effort and apply the optional overrides.
- The app-server collaboration-mode list uses these overrides and the resolved base model so UI and API stay aligned.
- Built-in Plan preset keeps `medium` reasoning effort by default, while allowing per-mode override via config.
