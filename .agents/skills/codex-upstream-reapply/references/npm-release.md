# NPM release

这个文档用于指导 `codex-upstream-reapply` 在处理 npm / release / CI 相关改动时，哪些内容必须直接沿用旧分支，哪些删除要同步保留，以及只有在这些必做项执行完之后，才去评估上游 / 新 tag 自己新增或改动的 CI。

上游 reapply 时，本文本身就是 npm release 的唯一规则来源。

## Source of truth

- 行为目标：本文
- 改动来源：`git diff BASE_COMMIT..OLD_BRANCH`
- 默认基线：`BASE_COMMIT="$(git merge-base TAG OLD_BRANCH)"`
- 如果 merge-base 不可靠，必须显式传 `--old-base-tag`

## Package identity

这些命名默认直接沿用 `OLD_BRANCH` 当前已经确认过的实现，不重新发明：

- 顶层发布包名：`@loongphy/codext`
- 平台包名：
  - `@loongphy/codext-linux-x64`
  - `@loongphy/codext-darwin-x64`
  - `@loongphy/codext-darwin-arm64`
  - `@loongphy/codext-win32-x64`
- 用户安装后的命令名：`codext`
- NPM 入口脚本：`codex-cli/bin/codex.js`
- 入口脚本最终拉起的原生二进制名：
  - Unix: `codex`
  - Windows: `codex.exe`

这里要明确区分：

- npm registry 上的顶层包名是 `@loongphy/codext`
- shell 里用户执行的命令名是 `codext`
- `codext` 对应的是 `@loongphy/codext` 包里的 JS launcher，不要求 vendor 内原生二进制也改名
- 当前允许 launcher 最终去执行 vendor 内的 `codex` / `codex.exe`
- 只要 launcher 解析的是 `@loongphy/codext-*` 这些 scoped 平台包，而不是 `@openai/codex-*`，就不会和 `@openai/codex` 混用

如果 upstream / 新 tag 没有明确要求变更这些名称，就不要在 reapply 时改动它们。

## Must execute on the new tag branch

只要 `OLD_BRANCH` 存在本文对应的 skill 规则，就视为启用这套 codext npm release reapply 规则。此时在 `NEW_BRANCH` 上必须执行这些动作：

1. 用 `OLD_BRANCH` 的 `.github/workflows/rust-release.yml` 覆盖 `NEW_BRANCH` 当前内容。
2. 删除 `NEW_BRANCH` 下其他所有 `.github/workflows/*`，只保留 `rust-release.yml`。
3. 直接复制这些路径的 `OLD_BRANCH` 版本：
   - `.github/scripts/install-musl-build-tools.sh`
   - `.github/scripts/rusty_v8_bazel.py`
   - `codex-cli/package.json`
   - `codex-cli/bin/codex.js`
   - `codex-cli/bin/rg`
   - `codex-cli/scripts/build_npm_package.py`
   - `codex-cli/scripts/install_native_deps.py`

上面这些都是必做项，不是建议，也不是“默认情况下尽量这样做”。执行完之前，不要去讨论新 tag 的结构要不要沿用。

这些路径里的内容按“整份文件/目录直接复制”处理，不单独重推导其中的细节。也就是说，下列内容都以 `OLD_BRANCH` 文件内容为准：

- 当前分支已经改过的 workflow / job / step 配置
- release workflow 名称、release tag/asset 命名、发包入口名称
- NPM package name、platform package name、bin 名称、安装命令、dist-tag
- 为了发包链路落地而改过的脚本参数、环境变量名、文案

## Only review upstream/new-tag deltas after the mandatory steps

先执行完上面的必做项。只有在这些动作都完成后，如果 `TAG` / upstream 相对旧基线额外带来了新的或变动的 CI / release 文件，才评估这些 upstream 变化要不要合并进当前分支方案。

默认处理顺序：

- 先保留 `OLD_BRANCH` 已验证过的 CI / release / npm 配置，不因为 `TAG` 文件结构不同就回退成上游写法
- 再单独看 `TAG` / upstream 新增或改动的 CI，决定是继续忽略、局部吸收，还是手动合并
- 如果 upstream 变化没有影响当前分支既有发包链路，就保持当前分支方案不动
- 如果必须吸收 upstream 变化，也只做最小合并；不要顺手改掉当前分支已经确认的包名、命令名、release 命名

可用的核对方式：

- 旧分支已有改动：`git diff --name-status BASE_COMMIT..OLD_BRANCH -- .github/workflows/rust-release.yml .github/scripts/install-musl-build-tools.sh .github/scripts/rusty_v8_bazel.py codex-cli/package.json codex-cli/bin/codex.js codex-cli/bin/rg codex-cli/scripts/build_npm_package.py codex-cli/scripts/install_native_deps.py`
- 新 tag / upstream 额外变化：`git diff --name-status BASE_COMMIT..TAG -- .github/workflows/rust-release.yml .github/scripts/install-musl-build-tools.sh .github/scripts/rusty_v8_bazel.py codex-cli/package.json codex-cli/bin/codex.js codex-cli/bin/rg codex-cli/scripts/build_npm_package.py codex-cli/scripts/install_native_deps.py`

## Ask the user only for real conflicts

只有遇到这些“上游新增/变动 CI 后仍然无法安全决定”的情况，再向用户确认：

- 想删掉的是不是用户仍在使用的发布入口
- upstream 新 tag 新增了新的 release / CI 入口，但你无法判断它是必须接入还是历史残留
- upstream 新 tag 明确要求当前分支现有发包命名、安装命令、dist-tag、release 产物矩阵发生变化
- 旧分支与当前 tag 在发布平台范围上明显冲突
