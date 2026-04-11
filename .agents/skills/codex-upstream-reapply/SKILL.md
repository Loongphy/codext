---
name: codex-upstream-reapply
description: 'Reapply a fork or secondary-development branch onto the latest stable rust-vX.Y.Z tag by creating a fresh branch from that tag and re-implementing the old branch intent without merge or rebase.'
---

# Codex Upstream Reapply

## Overview

用于“二开/魔改”场景的 tag 同步：默认按 `rust-*` 过滤拉取/查看 upstream tags，自动选择最新的稳定正式版 Rust tag（只接受 `rust-vX.Y.Z`，忽略 `-alpha`/`-beta`/`-rc`），并使用当前分支作为 `OLD_BRANCH`；然后从该 tag 创建新分支作为开发起点，再读取旧二开分支的 git changes 与意图 Markdown，在新分支上“重实现”需求（不 merge/rebase 旧分支历史）。

核心原则：`OLD_BRANCH` 的代码与提交历史只是参考材料，不是要直接照搬到 `NEW_BRANCH`。每次新的 upstream tag 都可能已经重构了相关模块，所以应当以 `CHANGED.md`、意图文档和旧分支行为为需求来源，基于当前 `TAG` 对应的代码结构重新实现。

## Default Mode（用户没指定参数时）

如果用户只是说类似 `$codex-upstream-reapply do it`，默认直接这样做，不再追问 tag / branch：

1. `REMOTE=upstream`
2. `TAG_PATTERN=rust-*`
3. `TAG=最新稳定正式版 Rust tag`
说明：只接受精确匹配 `rust-vX.Y.Z` 的 tag，例如 `rust-v0.117.0`；忽略 `rust-v0.117.0-alpha.1`
4. `OLD_BRANCH=当前分支`
说明：用 `git branch --show-current` 或等价命令获取
5. `NEW_BRANCH=feat/<TAG>`
6. 如果当前分支已经等于 `TAG` 或 `feat/<TAG>`，说明已经对齐到最新正式版，直接停止，不再继续重实现流程
7. 如果不一致，再执行后续 reapply 逻辑

## Inputs (每次明确这些东西)

- `REMOTE`：拉取 tags 的 remote（默认 `upstream`）
- `TAG_PATTERN`：tag 过滤规则（默认 `rust-*`）
- `TAG`：你选择的 tag 版本（作为新分支起点；默认取最新稳定正式版 `rust-vX.Y.Z`）
- `OLD_BRANCH`：原本二开的分支（包含改动 + 意图 Markdown；默认取“当前分支”）
- `NEW_BRANCH`：从 tag 新建的分支名（脚本默认 `feat/<tag-name>`）
- 可选：`OLD_BASE_TAG`（仅当基线推断不可靠时显式指定）

## Workflow (推荐：完全不 merge / 不 rebase 旧分支)

### 0) Acceptance criteria (必读)

- 禁止运行 `cargo test`（不需要写/跑测试）。
- 不得生成测试代码或快照文件：确保本次变更里没有新增/修改测试代码或 `*.snap`/`*.snap.new`。
- 禁止运行任何会检查/编译测试目标，或可能因此倒逼你修改测试代码的命令；包括但不限于 `cargo clippy --tests`、`cargo clippy --fix --tests`、`just fix`、`cargo insta ...`。
- 除非用户明确要求，不运行 `cargo fmt` / `just fmt` / `cargo clippy` / `cargo clippy --fix` / `just fix` 这类格式化、lint、自动修正命令；本 skill 默认只做必要的代码实现与 build 验证。
- 如果仓库通用 AGENTS/README/开发规范要求在大改后额外跑 `fmt` / `clippy` / `fix` / `test`，在本 skill 下默认跳过这些步骤，优先遵守“不改测试代码、只做 build 验证”的约束；如有例外必须先得到用户明确许可。
- 在 `NEW_BRANCH` 上保留并更新根目录 `AGENTS.md`：明确说明当前正在进行的是一次 upstream reapply 工作，禁止编写/修改测试代码，禁止执行任何 lint / format / auto-fix 命令，并注明本次验收标准以本 skill 的 Acceptance criteria 为准。
- 对于用户可见的 TUI 功能，如果 `codex-rs/tui` 与 `codex-rs/tui_app_server` 都存在对应的平行实现，则必须同步落地两边；不能只改其中一边就判定该需求已完成，除非 upstream 已明确删除其一，或你能在当前 tag 的代码里给出清晰的“不需要同步”的理由。
- 如果 `CHANGED.md` 记录的是这类共享 TUI 行为，文案应写成“用户可见行为要求”，并在需要时明确适用于 `tui` 与 `tui_app_server`，避免写成只对应某一个实现细节的说明。
- 在 `codex-rs` 目录下执行 `cargo build -p codex-cli`，确认能正常启动运行。
- 更新根目录 `README.md` 的 `Codex build` 徽章版本：使用选定 `TAG` 的版本号，并附加该 tag 指向的短 commit（例如 `rust-v0.94.0-dce99bc`）。推荐使用 `https://img.shields.io/static/v1?label=codex%20build&message=<tag>-<short_commit>&color=2ea043`。

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

### 2) Fetch tags & resolve TAG

```bash
git fetch upstream 'refs/tags/rust-*:refs/tags/rust-*' --prune
git for-each-ref --sort=-v:refname --format='%(refname:short)' 'refs/tags/rust-*'
```

如只想先查看远端候选而不先写入本地 tags，也可以：

```bash
git ls-remote --tags --refs upstream 'rust-*'
```

默认取最新稳定正式版 `TAG`：

```bash
git for-each-ref --sort=-v:refname --format='%(refname:short)' 'refs/tags/rust-*' \
  | grep -E '^rust-v[0-9]+\.[0-9]+\.[0-9]+$' \
  | head -n 1
```

如果用户明确指定了 tag，再按用户指定值覆盖默认值。

### 3) Generate a re-implementation bundle & create NEW_BRANCH

用脚本生成“重实现材料包”（默认输出到 `/tmp/codex-upstream-reapply/...`），并从 `TAG` 创建 `NEW_BRANCH`：

```bash
# 默认模式：自动选择最新稳定 Rust tag + 当前分支作为 OLD_BRANCH
bash .agents/skills/codex-upstream-reapply/scripts/start_from_tag.sh \
  --remote upstream
```

如需覆盖默认值，再显式传参：

```bash
bash .agents/skills/codex-upstream-reapply/scripts/start_from_tag.sh \
  --remote upstream \
  --tag TAG \
  --old-branch OLD_BRANCH
```

脚本默认只 fetch `rust-*` tags，并自动选择最新稳定正式版；如确需放宽范围，再显式传 `--tag-pattern <glob>`。

它会记录：

- `OLD_BRANCH` 相对 `TAG` 的 `merge-base`（作为改动基线）
- 变更文件清单、diff patch、commit 列表
-（默认）复制所有“变更过的 Markdown 意图文档”的旧版内容到 bundle 里
-（可选）用 `--copy-all` 复制所有变更文件的旧版内容（用于离线阅读）
并且会固定复制 `OLD_BRANCH` 的 `README.md`、`CHANGED.md`、`.agents/skills/` 到 `NEW_BRANCH`；对于 npm / release / CI 相关改动，则会按 `OLD_BRANCH` 相对基线 tag 的 git changes 自动搬运，包括删除。只要 `OLD_BRANCH` 带有 `references/npm-release.md` 对应的 skill 规则，就必须执行 npm release 文档里定义的强制动作，而不是只把它当成“默认原则”。

如果分支上包含 codext npm / release 相关改动，必须先看 `references/npm-release.md`。这份文档明确要求：在 `NEW_BRANCH` 上用 `OLD_BRANCH` 的 `rust-release.yml` 覆盖当前 tag 分支内容，删除其他 workflow，并直接复制 `.github/scripts/install-musl-build-tools.sh`、`.github/scripts/rusty_v8_bazel.py`、`codex-cli/package.json`、`codex-cli/bin/codex.js`、`codex-cli/bin/rg`、`codex-cli/scripts/build_npm_package.py`、`codex-cli/scripts/install_native_deps.py`；这些是必做项，不是建议。只有这些动作完成后，才允许评估上游 / 新 tag 额外新增或改动的 CI 是否要合并或忽略。

如果这一步复制了 `README.md` 到 `NEW_BRANCH`，则紧接着必须更新 `NEW_BRANCH` 根目录 `AGENTS.md`，补充一段当前任务说明，至少包含这些信息：

- 当前正在进行 `TAG` 对应的 upstream reapply / re-implementation 工作。
- 本次只允许修改实现代码与必要文档，不写、不改任何测试代码或 snapshot。
- 本次不执行任何 lint / format / auto-fix 命令（例如 `cargo fmt`、`just fmt`、`cargo clippy`、`just fix`）。
- 本次是否完成，以本 skill 的 “Acceptance criteria” 为唯一验收标准。

推荐把这段说明写成显式的临时工作约束，方便后续同线程/同分支继续协作时不偏离边界。

如果基线推断可疑（脚本会提示），请显式指定旧分支基线 tag：

```bash
bash .agents/skills/codex-upstream-reapply/scripts/start_from_tag.sh \
  --remote upstream --tag TAG \
  --old-base-tag rust-vX.Y.Z
```

### 4) Read OLD_BRANCH as reference (理解需求与意图，而不是直接套 patch)

从 bundle 里先读清楚“要实现什么”，再开始在 `NEW_BRANCH` 上写代码。

重点：

- `OLD_BRANCH` 的实现、diff、提交记录只用于帮助理解需求，不应直接 `cherry-pick`、照搬旧提交历史，或把旧分支当成目标代码树覆盖到新分支上。
- `CHANGED.md` 应视为需求清单的第一参考来源；旧分支代码只是帮助你理解这些需求当时是如何落地的。
- 对 TUI 相关需求，不要默认只看 `codex-rs/tui`。先确认当前 tag 下 `codex-rs/tui` 与 `codex-rs/tui_app_server` 是否都存在对应 surface，以及 `codex` 默认 interactive 入口实际会分发到哪一条链路，再决定需要同步重实现的范围。
- 若 upstream 在新 `TAG` 中已经重构相关模块，应优先适配当前 codebase 的结构，在当前实现方式下重新落地相同需求，而不是强行维持旧文件组织或旧接口。
- 最终目标是“在当前 codebase 上实现同样的需求”，不是“让新分支长得像旧分支的提交历史”。

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
- 以 `CHANGED.md` 中记录的变动为主线逐项核对，确认每项需求都在当前 codebase 上重新实现。
- 让意图文档与实现保持一致（必要时更新 Markdown）。
- 不跑测试；不要生成或更新任何测试文件/快照文件。

### 5.1) Status header 规范（改动 TUI 状态栏时）

- 状态栏是共享 TUI surface：如果 `codex-rs/tui` 与 `codex-rs/tui_app_server` 都渲染了这一层，默认两边都要同步修改，不能只改经典 `tui`。
- 具体图标、颜色、segment 顺序、rate-limit summary 格式与刷新语义，统一遵循 `status-header` skill；这里不要再维护第二份会漂移的细节规范。
- 如果当前仓库的 TUI 样式规范、lint 或现有封装与状态栏 skill 的示例写法冲突，优先遵循仓库本身的规则，但要保持相同的用户可见效果；不要为了强行对齐示例而引入 `clippy` 警告/报错，或去修改测试代码。

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

### 8) Final release build (codex-rs)

所有重实现修改完成并确认后，再执行最后收尾构建：

```bash
cd codex-rs
cargo build -p codex-cli --release
```

## How changes are computed from OLD_BRANCH

默认用以下方式推断旧分支的“改动基线”：

```bash
BASE_COMMIT="$(git merge-base TAG OLD_BRANCH)"
git diff "${BASE_COMMIT}..OLD_BRANCH"
```

如果推断结果可疑（例如 `OLD_BRANCH` 的历史标记与 `TAG` 不一致），脚本会停止并要求你明确指定：

```bash
--old-base-tag rust-vX.Y.Z
```

这样可以准确得到 “从指定 Rust tag 到 OLD_BRANCH 的全部二开变更”。
