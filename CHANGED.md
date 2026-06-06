# Changes in This Fork

This file captures the fork-specific behavior reapplied on top of the current upstream tag.

## TUI composer draft clipboard shortcut

- Added `Ctrl+Shift+C` in the TUI composer to copy the current draft to the system clipboard when the input contains text.
- Existing `Ctrl+C` behavior stays unchanged.
- When the composer has no copyable text, `Ctrl+Shift+C` falls back to the existing `Ctrl+C` clear/interrupt/quit path.
- On WSL2, composer draft copy reuses the existing Windows clipboard fallback so copies still land in the Windows system clipboard.
- `Ctrl+Shift+C` now takes its own composer-copy path instead of falling through to the existing `Ctrl+C` clear/interrupt/quit behavior when draft text is present.
- Added footer shortcut help text for the new draft-copy binding.

## TUI status header and polling

Implementation must follow the status-header skill .agents/skills/status-header/SKILL.md

- Added a status header above the composer in the app-server-backed `codex-rs/tui` surface. Segment order is fixed as model + reasoning effort, current directory, git branch/ahead/behind/changes, rate-limit remaining/reset time, then account identity.
- Status header account identity is the last segment without an icon: ChatGPT accounts render as `user@example.com(Pro)` and API-key auth renders as `API key`.
- Status header layout no longer adds its own top inset; it uses `Insets::tlbr(/*top*/ 0, …, /*bottom*/ 1, …)` and relies on the existing outer bottom-section gap above it so the spacing between `Working` and the header stays compact.
- Git status is collected in the background (15s interval, 2s timeout) and rendered when available.
- The directory segment represents the session/thread `cwd`, not a one-off tool `workdir`.
- When the session `cwd` changes (for example after switching into a new worktree), the git-status poller now rebinds to that new `cwd`, clears stale git state, and ignores late results from the previous `cwd`.
- ChatGPT `5h` / weekly usage-limit snapshots in the TUI now refresh in the background every 15 seconds, so the header and any `/statusline` limit items keep moving while the UI is otherwise idle.

## TUI auth.json watcher

- The running TUI now watches `CODEX_HOME/auth.json` and reloads auth when the file changes.
- Watch notifications are now trailing-debounced so reload happens after writes settle, reducing partial-file reads.
- If `auth.json` changes while the TUI still has an active task/turn running, auth reload is deferred until that work fully finishes; Codex does not hot-swap auth in the middle of the running task.
- Auth reload failures no longer clear cached auth (so transient parse/read errors do not appear as a logout).
- On auth reload failure, the TUI retries every 5 seconds for up to 3 attempts before surfacing a final warning.
- When the account identity changes, the TUI surfaces a warning in the transcript (including old/new emails when available).
- Auth change warnings now show the account plan type (e.g., Plus/Team/Free/Pro) instead of the generic ChatGPT label.
- Rate-limit state and polling are refreshed after auth changes so the header reflects the new account.
- That post-task auth refresh also resets cached rate-limit warning/prompt state for the new auth snapshot, so stale usage-limit/UI state from the previous auth context does not keep re-triggering after the reload.
- The TUI now supports `[tui].usage_limit_resume_prompt` for the synthetic recovery user turn sent after `UsageLimitExceeded`. If the field is unset, Codext uses the built-in default recovery prompt; if the field is set to an empty string, Codext disables the automatic recovery turn.
- When a turn hits `UsageLimitExceeded`, the TUI now queues that synthetic recovery turn ahead of other queued user input. If an `auth.json` reload is also pending, the reload still runs first, and only then does Codext submit the recovery turn before draining later queued inputs.
- After a turn stops on `UsageLimitExceeded`, Codext now keeps that synthetic recovery turn parked until the next `auth.json` reload that actually changes account identity, so switching accounts can continue the interrupted task without a manual resend.
- If the user manually submits a new message before that auth reload arrives, Codext clears the parked usage-limit recovery turn instead of replaying the stale synthetic prompt later.

## TUI queued messages after usage-limit exhaustion

- When a turn ends because quota/rate limit is exhausted, Codext pauses queued-message autosend instead of draining already queued Tab follow-ups into more failed turns.
- While autosend is paused, pressing Tab still queues new messages even when no turn is currently running.
- When a later Codex rate-limit snapshot shows quota available again, Codext resumes autosend and submits exactly the first queued user message; any additional queued messages remain queued for normal FIFO draining after that turn completes.
- If both a parked usage-limit recovery prompt and user-queued follow-ups exist when quota recovers, the user-queued follow-up wins and the stale synthetic recovery prompt is cleared.

## App-server auth.json account switching

- The app-server now reloads auth from storage before `thread/start`, `thread/resume`, and `turn/start` when no turn is running.
- This change supports Codex App account switching through [Loongphy/codex-auth#103](https://github.com/Loongphy/codex-auth/pull/103), allowing the app-server to pick up the newly selected account at the next safe request boundary.
- Auth is still not hot-swapped in the middle of an active turn; reload is skipped while `running_turn_count` is nonzero and the next request boundary gets the new auth.
- ChatGPT account/workspace switches inside the same auth mode are treated as auth changes by comparing the refresh-relevant auth snapshot, not only the top-level auth mode.
- When a reload changes auth, loaded threads invalidate their cached model transport state so a reused WebSocket session created under the previous account is not used for the next turn.
- The app-server also refreshes cloud requirements/default residency state and emits `AccountUpdated` after a changed reload so app UI account state follows the new snapshot.
- Reapply notes: keep `reload_auth_from_storage_if_idle` wired into all three request entry points, preserve the idle guard, and preserve the invalidation chain `ThreadManager::invalidate_model_transport_caches` -> `CodexThread::invalidate_model_transport_cache` -> `ModelClient::invalidate_cached_transport_state`.
- Reapply validation: run `cargo test --offline -p codex-login reload_detects_chatgpt_workspace_switch`, `cargo test -p codex-core invalidate_cached_transport_state_clears_cached_websocket_session`, and `cargo check --offline -p codex-app-server --lib` after merging upstream auth/app-server changes.

## TUI exit resume command

- Added a fork requirement that user-facing resume hints use `codext resume <session>` / `codext resume <thread-name>` instead of `codex resume ...`.
- This includes the final resume hint shown after exiting the TUI and other resume guidance surfaced inside the TUI.
