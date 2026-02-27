# Codex

![Codex build](https://img.shields.io/static/v1?label=codex%20build&message=v0.104.0-10553fa&color=2ea043)

![TUI](
https://github.com/user-attachments/assets/127abbc2-cb30-4d6e-8a81-ce707260c045)

This is strictly a personal hobby project, forked from [openai/codex](https://github.com/openai/codex).

## Quick Start

```shell
cd codex-rs
cargo run --bin codex
```

## Install From GitHub Release

### Bash (Linux/macOS)

```bash
sh -c "$(curl -fsSL https://raw.githubusercontent.com/Loongphy/codext/main/scripts/install.sh)"
```

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/Loongphy/codext/main/scripts/install.ps1 | iex
```

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

> [!IMPORTANT]
> **DO NOT USE IN PRODUCTION.**
> To keep upstream sync easy, we do not write test code for what we changed. This project is for experimental use only.

* **DX Focused:** Focus strictly on optimizing developer experience, **without adding new features**.
* **Upstream Sync:** We sync with the upstream repository regularly.

## What Changed

* Added a TUI status header with model/effort, cwd, git summary, and rate-limit status.
* Collaboration mode presets now accept per-mode overrides and default to the active `/model` settings.
* TUI watches `auth.json` for external login changes and reloads auth automatically (with a warning on account switch).
* Full change log: see [CHANGED.md](./CHANGED.md).

## AGENT Local development check

1. DO NOT update any test codes
2. After making code changes, verify the CLI still launches:

```shell
cd ./codex-rs
cargo run --bin codex
```

```toml
# config.toml
[collaboration_modes.plan]
model = "gpt-5.2-codex"
reasoning_effort = "xhigh"

[collaboration_modes.code]
model = "gpt-5.2-codex"
```

## Skills

When syncing to the latest upstream codex version, use `.agents/skills/codex-upstream-reapply` to re-implement our custom requirements on top of the newest code, avoiding merge conflicts from the old branch history.

Example:

```
$codex-upstream-reapply old_branch feat/rust-v0.94.0, new origin tag: rust-v0.98.0
```

## Credits

Status bar design reference: <https://linux.do/t/topic/1481797>
