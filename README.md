# Codext

An opinionated Codex CLI. This is strictly a personal hobby project, forked from openai/codex.

![Preview](https://github.com/user-attachments/assets/23350e86-2597-48ea-82a6-378f8f01ac74)

## Quick Start

Choose one of these two ways:

* Install from npm:

```shell
npm i -g @loongphy/codext
```

* Build from source:

```shell
cd codex-rs
cargo run --bin codex
```

## Features

> Full change log: see [CHANGED.md](./CHANGED.md).

---

### TUI: Status Header

The TUI header now provides a comprehensive overview of your current workspace:

* **Context**: Displays the active model, effort level, and current working directory (`cwd`).
* **Git Status**: Real-time summary of your repository state.
* **Rate Limits**: Instant visibility into your API rate-limit status.

![Status Header Preview](https://github.com/user-attachments/assets/23350e86-2597-48ea-82a6-378f8f01ac74)

### Copy to Clipboard

* **`Ctrl+Shift+C`**: Copies the current draft to the system clipboard.
* **`Ctrl+C`**: Retains existing behavior; remains backward-compatible with legacy logic when the draft is empty.

### Prompt Queue on usage limit

![Prompt Queue](https://github.com/user-attachments/assets/534e927d-a306-4fef-b97c-629542bf8906)

This feature helps manage your follow-up messages when AI usage limits are reached:

* **Paused and Waiting**: When usage limits are triggered, messages in the queue will wait instead of being sent automatically.
* **Append While Usage limit**: Even when you are rate-limited, you can still press ` Tab ` to add more messages to the queue.
* **Auto-Send on Reset**: Once your quota becomes available again, Codex will automatically send the **first** queued message.

### Account Switching

![Account Changed](https://github.com/user-attachments/assets/35059463-b846-45c7-9d05-57a6e1082d8d)

Codex now monitors `auth.json` for external changes and automatically reloads authentication after external writes settle, providing seamless account switching without restarting.

* **TUI**: Watches `auth.json` for changes via filesystem notifications, with trailing debounce so reloads happen after writes settle. Auth is deferred until any active task completes; transient read errors do not clear cached auth.
* **App-server**: Reloads auth before `thread/start`, `thread/resume`, and `turn/start` when no turn is running, so the new account is picked up at the next safe request boundary.

This enables seamless auth refresh for TUI / Codex Client users when external tools update `auth.json`.

It also supports Codex App account switching via [codex-auth#103](https://github.com/Loongphy/codex-auth/pull/103).

### Automatic Resumption

When the TUI detects an account switch after hitting a usage limit, it automatically dispatches a recovery prompt to resume the interrupted task.

You can configure this behavior using `[tui].usage_limit_resume_prompt`:

* **Custom Prompt**: Define a specific string to be sent as the "resumption turn." This prompt will be used to signal the model to continue where it left off.
* **Disable**: Set to `""` (empty string) to disable this automatic recovery behavior entirely.
* **Default**: If left unset, the system uses the following built-in prompt:

  ```text
  Please continue from where the conversation left off after the usage limit reset or account switch.
  ```

  Example:

  ```toml
  [tui]
  usage_limit_resume_prompt = ""
  ```

### AGENTS.md auto reload

AGENTS.md and project-doc instructions are refreshed on each new user turn, and Codex shows an explicit warning when a refresh is applied.

## Project Goals

We will never merge code from the upstream repo; instead, we re-implement our changes on top of the latest upstream code.

Iteration flow (aligned with `.agents/skills/codex-upstream-reapply`):

```mermaid
flowchart TD
      A[Fetch upstream tags] --> B[Choose latest stable rust tag]
      B --> C[Create fresh branch from tag]
      C --> D[Generate old-branch reference bundle]
      D --> E[Read intent docs and old changes<br/>CHANGED.md / README.md / AGENTS.md<br/>bundle diff / changed-files / commits]
      E --> F[Re-implement required changes on fresh branch]
      F --> G[Build: cargo build -p codex-cli]
      G --> H[Review final diff against tag]
      H --> I[Push finished branch]
```

## Skills

When syncing to the latest upstream codex version, use `.agents/skills/codex-upstream-reapply` to re-implement our custom requirements on top of the newest code, avoiding merge conflicts from the old branch history.

Example:

```
$codex-upstream-reapply old_branch feat/rust-v0.130.0, new origin tag: rust-v0.131.0
```

## Credits

Status bar design reference: <https://linux.do/t/topic/1481797>
