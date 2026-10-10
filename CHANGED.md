# Changes in This Fork

This file captures the fork-specific behavior reapplied on top of the current upstream tag.

## TUI composer draft clipboard shortcut

- Added `Ctrl+Shift+C` in the TUI composer to copy the current draft to the system clipboard when the input contains text.
- Existing `Ctrl+C` behavior stays unchanged.
- When the composer has no copyable text, `Ctrl+Shift+C` falls back to the existing `Ctrl+C` clear/interrupt/quit path.
- On WSL2, composer draft copy reuses the existing Windows clipboard fallback so copies still land in the Windows system clipboard.
- `Ctrl+Shift+C` now takes its own composer-copy path instead of falling through to the existing `Ctrl+C` clear/interrupt/quit behavior when draft text is present.
- Added footer shortcut help text for the new draft-copy binding.

## TUI resume picker rename

- Added `Ctrl+R` in the `/resume` picker to rename the highlighted session inline, reusing the picker search line as the editor. The draft starts empty so a new name can be typed directly.
- `Enter` submits through the existing `thread/name/set` app-server request; `Esc` cancels back to search mode.
- Empty or whitespace-only names are rejected inline; failed requests keep the editor open so the rename can be retried.
- On success the row updates in place, the search filter re-applies, and selection stays on the renamed thread when possible.
- The shortcut only claims `Ctrl+R` when no existing list keymap or custom binding uses it; archived sessions are read-only and cannot be renamed.

## TUI status header and polling

Implementation must follow the status-header skill .agents/skills/status-header/SKILL.md

- Added a status header above the composer in the app-server-backed `codex-rs/tui` surface. Segment order is fixed as model + reasoning effort, current directory, git branch/ahead/behind/changes, rate-limit remaining/reset time, then account identity.
- Status header account identity is the last segment without an icon: ChatGPT accounts render as `user@example.com(Pro)` and API-key auth renders as `API key`.
- Status header layout no longer adds its own top inset; it uses `Insets::tlbr(/*top*/ 0, …, /*bottom*/ 1, …)` and relies on the existing outer bottom-section gap above it so the spacing between `Working` and the header stays compact.
- Git status is collected in the background (15s interval, 2s timeout) and rendered when available.
- The directory segment represents the session/thread `cwd`, not a one-off tool `workdir`.
- When the session `cwd` changes (for example after switching into a new worktree), the git-status poller now rebinds to that new `cwd`, clears stale git state, and ignores late results from the previous `cwd`.

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

## TUI server-overload auto-resume

- When a turn fails with `ServerOverloaded`, the TUI automatically submits a `Continue` user turn so work resumes without manual intervention.
- Auto-resume is bounded: exponential backoff of 15s → 30s → 60s → 120s → 240s between attempts, stopping after 5 consecutive failures and leaving the error on screen for the user.
- The retry counter resets after any successfully completed turn; stale retry timers are discarded via a generation guard, so user intervention never triggers a late auto-Continue.
- Controlled by `[tui].server_overloaded_resume` (default `true`; set to `false` to disable).
- While auto-retries are pending, queued follow-up messages are held instead of being submitted into failing turns.

## App-server auth.json account switching

- The app-server now reloads auth from storage before `thread/start`, `thread/resume`, and `turn/start` when no turn is running.
- This change supports Codex App account switching through [Loongphy/codex-auth#103](https://github.com/Loongphy/codex-auth/pull/103), allowing the app-server to pick up the newly selected account at the next safe request boundary.
- Auth is still not hot-swapped in the middle of an active turn; reload is skipped while `running_turn_count` is nonzero and the next request boundary gets the new auth.
- ChatGPT account/workspace switches inside the same auth mode are treated as auth changes by comparing the refresh-relevant auth snapshot, not only the top-level auth mode.
- When a reload changes auth, loaded threads invalidate their cached model transport state so a reused WebSocket session created under the previous account is not used for the next turn.
- The app-server also refreshes cloud requirements/default residency state and emits `AccountUpdated` after a changed reload so app UI account state follows the new snapshot.
- The TUI drives its `auth.json` watcher through a dedicated `account/reload` RPC (response: account snapshot + `authChanged`), which performs the same idle-guarded storage reload. Earlier reapplies extended `account/read` with a `reloadAuthFromStorage` param and an `authChanged` response field; the dedicated RPC keeps upstream `GetAccountParams`/`GetAccountResponse` wire shapes untouched.
- `ReloadAccountResponse` carries `authReloadSkipped`: it is `true` when the request reached the app-server but the storage reload was skipped because `running_turn_count` is nonzero (a turn running in *any* client of a shared daemon). Callers must not treat `authChanged=false` alone as "no account change".
- When the TUI sees `authReloadSkipped`, it retries the reload every 5s for up to 24 attempts instead of dropping the pending account switch; if the daemon is still busy after that, the next request-boundary `reload_auth_from_storage_if_idle` still applies the switch.
- The fork feature `codext_auth_reload` (Stable, default on) marks an app-server that implements `account/reload`. It is part of the TUI `SERVER_FEATURES` compatibility check (`tui/src/daemon_startup.rs`), so attaching to a daemon that does not report it — a stock upstream daemon or a pre-reapply codext daemon — is treated as incompatible: implicit socket attach falls back to embedded mode, and the managed-daemon path offers the existing restart/embedded recovery. This prevents the silent "account switch has no effect" failure when a foreign daemon owns `app-server-control.sock`. Disabling the feature also disables the auth.json watcher entirely.
- `codext app-server daemon reload-auth` asks the running daemon on `app-server-control.sock` to perform the idle-guarded `account/reload` without a TUI. It reports whether the daemon reloaded (and whether auth changed), was skipped-busy, or does not implement the RPC (foreign daemon). `codex_app_server_daemon::request_auth_reload` is the library entry point; external account switchers such as codex-auth invoke the subcommand after replacing `auth.json`.
- Reapply notes: keep `reload_auth_from_storage_if_idle` wired into all three request entry points, preserve the idle guard, preserve the `auth_reload_skipped` response field and its busy-retry path in `handle_auth_file_changed`, keep `Feature::CodextAuthReload` in `SERVER_FEATURES`/`allowed_feature`, and preserve the invalidation chain `ThreadManager::invalidate_model_transport_caches` -> `CodexThread::invalidate_model_transport_cache` -> `ModelClient::invalidate_cached_transport_state`.

## TUI exit resume command

- Added a fork requirement that user-facing resume hints use `codext resume <session>` / `codext resume <thread-name>` instead of `codex resume ...`.
- This includes the final resume hint shown after exiting the TUI and other resume guidance surfaced inside the TUI.

## Update channel points at Codext releases

- Latest-version discovery queries `https://api.github.com/repos/Loongphy/codext/releases/latest` and the `@loongphy/codext` npm registry entry instead of `openai/codex` / `@openai/codex`.
- Codext release tags are `codext-v<base>-<sha>`; `extract_version_from_latest_tag` accepts both `codext-v` and `rust-v` prefixes so upstream tests and tooling stay valid.
- Update prompts compare only the base version triple (`is_newer_release` strips the `-<sha>` suffix), so pushes that rebuild the same upstream base do not retrigger the prompt.
- `codext update` and the in-TUI "Update now" action run `npm/bun/vp/pnpm install -g @loongphy/codext`; release-note and install links point at `Loongphy/codext`.
- `codext doctor` update diagnostics report the Codext channel too (latest-release probe, tag parsing, and npm-family update labels).
- Homebrew cask, standalone installer, and daemon update variants still reference upstream artifacts; they are unreachable for Codext installs because no Codext cask or standalone installer exists.
- Reapply notes: the Windows-only "absolute update command" error keeps the upstream docs URL because an existing test pins the message; snapshot files under `tui/src/snapshots` still contain upstream update URLs on purpose.

## Release artifact parity

- Release builds and npm platform packages ship `codex-code-mode-host` beside the `codext` CLI binary so code mode can start from installed and locally packaged artifacts.
- The upstream release matrix is audited during reapply instead of assuming that copying the previous fork workflow preserves all companion binaries.
- The upstream `codex-responses-api-proxy` package/binary is intentionally not shipped: it is a standalone debugging proxy, not a companion required by `codext` or `codex-code-mode-host`.

## Canonical package layout

- Every npm platform package and GitHub release archive vendors a complete codex-package root per target: `bin/codex` + `bin/codex-code-mode-host`, `codex-path/rg`, `codex-resources/` (`bwrap` on Linux, `zsh/bin/zsh` where the manifest provides it, `codex-command-runner.exe` + `codex-windows-sandbox-setup.exe` on Windows), and `codex-package.json` metadata. This is the layout the app-server daemon requires when seeding its managed install; anything less fails startup with "this CLI has no complete local package" on fresh installs.
- Package trees are assembled with upstream's `scripts/build_codex_package.py`, so npm staging, release archives, and `codex-cli/scripts/install_native_deps.py` all produce the same layout.
- `codex-cli/bin/codex.js` executes `vendor/<target>/bin/codex` and prepends `vendor/<target>/codex-path` to `PATH`.
- Linux release builds compile `--bin bwrap`, strip it, and export `CODEX_BWRAP_SHA256` before building `codex`, so the digest embedded in the CLI matches the shipped `codex-resources/bwrap` bytes.
- `codex-package.json` records the codext release version (`<base>-<sha>` prerelease), which keeps the seeded daemon release off upstream's latest-channel auto-update path.
- Unix release archives ship `bin/codext` as a hardlink to `bin/codex`; the Windows zip adds a `bin/codext.exe` alias. Keeping the alias inside `bin/` preserves package-layout detection, and the hardlink extracts on Windows without the symlink privileges a root symlink would require. A root-level executable copy would not be recognized as a package entrypoint.
