# Codext

An opinionated Codex CLI. This is strictly a personal hobby project, forked from openai/codex.

![Codex build](https://img.shields.io/static/v1?label=codex%20build&message=rust-v0.130.0-58573da43a&color=2ea043)

> ![Status Header Preview](https://github.com/user-attachments/assets/23350e86-2597-48ea-82a6-378f8f01ac74)

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

Here is the optimized version in English, structured for better readability and professional presentation:

---

### TUI: Status Header
The TUI header now provides a comprehensive overview of your current workspace:
* **Context**: Displays the active model, effort level, and current working directory (`cwd`).
* **Git Status**: Real-time summary of your repository state.
* **Rate Limits**: Instant visibility into your API rate-limit status.
> ![Status Header Preview](https://github.com/user-attachments/assets/23350e86-2597-48ea-82a6-378f8f01ac74)

### Copy to Clipboard

* **`Ctrl+Shift+C`**: Copies the current draft to the system clipboard.
* **`Ctrl+C`**: Retains existing behavior; remains backward-compatible with legacy logic when the draft is empty.

### Automatic Account switch
TUI now monitors `auth.json` for external changes, automatically reloads authentication after external writes settle.

Fully compatible with [codex-auth](https://github.com/Loongphy/codex-auth) for seamless external login management.

### Automatic Resumption
When the TUI detects an account switch after hitting a usage limit, it automatically dispatches a recovery prompt to resume the interrupted task.

You can configure this behavior using `[tui].usage_limit_resume_prompt`:
* **Custom Prompt**: Define a specific string to be sent as the "resumption turn." This prompt will be used to signal the model to continue where it left off.
* **Disable**: Set to `""` (empty string) to disable this automatic recovery behavior entirely.
* **Default**: If left unset, the system uses the following built-in prompt:
  ```text
  The previous turn stopped because the active account hit a usage limit. Any pending auth reload has already been applied. Please continue the previous coding task from where it stopped, and use apply_patch for any required file edits.
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
    A[Freeze old branch: commit changes + intent docs] --> B[Fetch upstream tags]
    B --> C[Pick tag + create new branch from tag]
    C --> D[Generate reimplementation bundle]
    D --> E[Read old branch + bundle for intent]
    E --> F[Re-implement changes on new branch]
    F --> G[Sanity check diffs vs tag]
    G --> H[Force-push to fork main]
```

## Skills

When syncing to the latest upstream codex version, use `.agents/skills/codex-upstream-reapply` to re-implement our custom requirements on top of the newest code, avoiding merge conflicts from the old branch history.

Example:

```
$codex-upstream-reapply old_branch feat/rust-v0.94.0, new origin tag: rust-v0.98.0
```

## Credits

Status bar design reference: <https://linux.do/t/topic/1481797>
