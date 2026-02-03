---
name: codex-upstream-reapply
description: "Tag-based upstream sync for a fork/secondary-development repo: fetch tags, let the user choose a tag, create a fresh branch from that tag, then read the old customization branch’s git changes + intent Markdown to re-implement the requirements on the new branch (no merge/rebase of the old branch)."
---

# Codex Upstream Reapply

## Overview

用于“二开/魔改”场景的 tag 同步：先 `git fetch upstream --tags` 拉取 tags，让用户选择一个 tag 版本，从该 tag 创建新分支作为开发起点；然后读取旧二开分支的 git changes 与意图 Markdown，在新分支上“重实现”需求（不 merge/rebase 旧分支历史）。

## Inputs (每次明确这些东西)

- `REMOTE`：拉取 tags 的 remote（默认 `upstream`）
- `TAG`：你选择的 tag 版本（作为新分支起点）
- `OLD_BRANCH`：原本二开的分支（包含改动 + 意图 Markdown；默认取“当前分支”）
- `NEW_BRANCH`：从 tag 新建的分支名（脚本默认 `feat/<tag-name>`）
- 可选：`OLD_BASE_TAG`（仅当基线推断不可靠时显式指定）

## Workflow (推荐：完全不 merge / 不 rebase 旧分支)

### 0) Acceptance criteria (必读)

- 禁止运行 `cargo test`（不需要写/跑测试）。
- 不得生成测试代码或快照文件：确保本次变更里没有新增/修改测试代码或 `*.snap`/`*.snap.new`。
- 在 `codex-rs` 目录下执行 `cargo build -p codex-cli`，确认能正常启动运行。

### 0) One-time setup（如果还没有）

确认是否已有 `origin`（fork）和 `upstream`（openai/codex），如没有再添加；已有就跳过 `remote add`：

```bash
git remote -v
git remote add origin <ORIGIN_GIT_URL>
git remote add upstream https://github.com/openai/codex.git
```

### 1) Freeze OLD_BRANCH (把现有改动“固化”为可回看的参考)

- 把工作区改动都提交到 `OLD_BRANCH`（包括你写的意图 Markdown）。
- 建议把 `OLD_BRANCH` 推到你的 fork 远端（例如 `origin`），避免本地丢失。
- 可选：打一个 snapshot tag/branch，方便以后回溯。

### 2) Fetch tags & choose TAG

```bash
git fetch upstream --tags --prune
git for-each-ref --sort=-creatordate --format='%(creatordate:iso8601) %(refname:short)' refs/tags
```

让用户从列表中选择一个 `TAG`（例如 `v0.2`）。

### 3) Generate a re-implementation bundle & create NEW_BRANCH

用脚本生成“重实现材料包”（默认输出到 `/tmp/codex-upstream-reapply/...`），并从 `TAG` 创建 `NEW_BRANCH`：

```bash
# 建议在 OLD_BRANCH 上执行；省略 --old-branch 时默认使用当前分支作为 OLD_BRANCH
bash ~/.codex/skills/codex-upstream-reapply/scripts/start_from_tag.sh \
  --remote upstream --tag TAG
```

它会记录：

- `OLD_BRANCH` 相对 `TAG` 的 `merge-base`（作为改动基线）
- 变更文件清单、diff patch、commit 列表
-（默认）复制所有“变更过的 Markdown 意图文档”的旧版内容到 bundle 里
-（可选）用 `--copy-all` 复制所有变更文件的旧版内容（用于离线阅读）
并且会把 `OLD_BRANCH` 的 `README.md` 与 `CHANGED.md` 原样复制到 `NEW_BRANCH`（不改内容；如有差异会自动提交一次）。

如果基线推断可疑（脚本会提示），请显式指定旧分支基线 tag：

```bash
bash ~/.codex/skills/codex-upstream-reapply/scripts/start_from_tag.sh \
  --remote upstream --tag TAG \
  --old-base-tag v0.1
```

### 4) Read OLD_BRANCH as reference (理解需求与意图，而不是直接套 patch)

从 bundle 里先读清楚“要实现什么”，再开始在 `NEW_BRANCH` 上写代码。

常用命令（在 `NEW_BRANCH` 上也能直接读取旧分支文件）：

```bash
git show OLD_BRANCH:path/to/file
git diff OLD_BRANCH -- path/to/file
```

如果你需要“旧分支相对当时基线的真实改动”，用 bundle 里的 `BASE_COMMIT`（在 `META.md` 里）：

```bash
git diff BASE_COMMIT..OLD_BRANCH -- path/to/file
```

### 5) Re-implement on NEW_BRANCH

- 按“需求点/模块”拆分小 commit 逐步实现。
- 让意图文档与实现保持一致（必要时更新 Markdown）。
- 不跑测试；不要生成或更新任何测试文件/快照文件。

### 6) Build (codex-rs)

在 `codex-rs` 目录下执行：

```bash
cargo build -p codex-cli
```

### 7) Sanity checks

比较“你最终在新分支做了哪些改动”（相对 `TAG`）：

```bash
git diff --stat TAG..NEW_BRANCH
git diff TAG..NEW_BRANCH
```

对照旧分支材料包，确认需求点都覆盖到即可（不要求 diff 完全一致）。

更多对照方式（worktree、merge-base 对照等）见 `references/advanced.md`。

## How changes are computed from OLD_BRANCH

默认用以下方式推断旧分支的“改动基线”：

```bash
BASE_COMMIT="$(git merge-base TAG OLD_BRANCH)"
git diff "${BASE_COMMIT}..OLD_BRANCH"
```

如果推断结果可疑（例如 `OLD_BRANCH` 的历史标记与 `TAG` 不一致），脚本会停止并要求你明确指定：

```bash
--old-base-tag v0.1
```

这样可以准确得到 “从 v0.1 到 OLD_BRANCH 的全部二开变更”。
