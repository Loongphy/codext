# NPM/Release Carry-Over Rules

这个文档用于指导 `codex-upstream-reapply` 在处理 npm / release / CI 相关改动时，哪些内容应直接沿用旧分支，哪些删除要同步保留，哪些可以按当前 tag 的结构由 agent 自行判断。

上游 reapply 时，这份文档是 `docs/npm-release.md` 的 skill 内部执行版：

- `docs/npm-release.md` 说明“目标行为是什么”。
- 本文说明“reapply 时哪些 git changes 要如何处理”。

## Source of truth

- 行为目标：`docs/npm-release.md`
- 改动来源：`git diff BASE_COMMIT..OLD_BRANCH`
- 默认基线：`BASE_COMMIT="$(git merge-base TAG OLD_BRANCH)"`
- 如果 merge-base 不可靠，必须显式传 `--old-base-tag`

## Must Copy Directly

这些路径只要在 `OLD_BRANCH` 相对基线有新增或修改，就应直接把旧分支版本带到 `NEW_BRANCH`：

- `.github/workflows/**`
- `.github/actions/**`
- `.github/scripts/**`
- `ci/**`
- `codex-cli/package.json`
- `codex-cli/bin/**`
- `codex-cli/scripts/**`
- `docs/npm-release.md`

另外，以下文件仍按 skill 的固定 carry-over 规则处理，不依赖是否属于 npm/release：

- `README.md`
- `CHANGED.md`
- `.agents/skills/**`

## Must Keep Deletions

如果上述 npm / release / CI 范围内的路径在 `OLD_BRANCH` 的 git diff 里是删除状态，那么 `NEW_BRANCH` 也应删除，而不是因为新 tag 里还存在就自动保留。

这条规则特别适用于：

- 旧分支为了“只保留一个 codext release workflow”而删掉的多余 workflow
- 旧分支为了收敛发布链路而移除的旧 release job、旧 helper、旧 CI 入口

默认做法：

- `D <path>`：在 `NEW_BRANCH` 删除同一路径
- `R old -> new`：删除 `old`，复制 `new`

## Agent May Decide

下面这些内容不要求机械复制；agent 应基于当前 tag 的代码结构自行判断如何落地，但必须保持 `docs/npm-release.md` 里的行为目标不变：

- 当前 tag 如果把 release 逻辑拆分到不同 workflow / action / helper 文件，是否需要跟着调整文件组织
- 为了接入当前 tag 的新字段、新 action 版本、新环境变量命名而做的最小适配
- `README.md`、`CHANGED.md` 中围绕 release/npm 的措辞微调
- 如果当前 tag 已经自带等价能力，是否不再需要旧分支里的某个中间脚本或辅助步骤

判断原则：

- 优先保持“单一 codext release workflow + 自动发布 npm + 自动更新 GitHub Release”的行为
- 不为了机械保留旧文件组织而违背当前 tag 的结构
- 如果决定不直接复制某个旧实现，必须能说明新 tag 中的等价承接点在哪里

## Ask The User Instead Of Guessing

遇到这些情况不要自行拍板，应向用户确认：

- 想删掉的是不是用户仍在使用的发布入口
- upstream 新 tag 看起来恢复了某个旧 workflow，但你无法判断它是重新需要还是只是历史残留
- 发布包名、安装命令、dist-tag、release 产物矩阵要不要变
- 旧分支与当前 tag 在发布平台范围上明显冲突

## Current Branch Reference

当前分支相对 `rust-v0.117.0` 已确认的 npm/release/CI 相关 git changes 包括：

- 修改：`.github/workflows/rust-release.yml`
- 删除：`.github/workflows/bazel.yml`
- 删除：`.github/workflows/blob-size-policy.yml`
- 删除：`.github/workflows/cargo-deny.yml`
- 删除：`.github/workflows/ci.yml`
- 删除：`.github/workflows/cla.yml`
- 删除：`.github/workflows/close-stale-contributor-prs.yml`
- 删除：`.github/workflows/codespell.yml`
- 删除：`.github/workflows/issue-deduplicator.yml`
- 删除：`.github/workflows/issue-labeler.yml`
- 删除：`.github/workflows/rust-ci.yml`
- 删除：`.github/workflows/rust-release-argument-comment-lint.yml`
- 删除：`.github/workflows/rust-release-prepare.yml`
- 删除：`.github/workflows/rust-release-windows.yml`
- 删除：`.github/workflows/rust-release-zsh.yml`
- 删除：`.github/workflows/rusty-v8-release.yml`
- 删除：`.github/workflows/sdk.yml`
- 删除：`.github/workflows/v8-canary.yml`
- 修改：`codex-cli/bin/codex.js`
- 修改：`codex-cli/package.json`
- 修改：`codex-cli/scripts/build_npm_package.py`
- 修改：`codex-cli/scripts/install_native_deps.py`
- 新增：`docs/npm-release.md`
- 固定 carry-over：`README.md`
- 固定 carry-over：`CHANGED.md`

如果后续分支继续演进，应始终以 `git diff BASE_COMMIT..OLD_BRANCH` 的实时结果为准，而不是只看上面这份快照。
