# NPM Release Reapply Rules

## Package identity

- npm package: `@loongphy/codext`
- Platform packages: `@loongphy/codext-{linux-x64,linux-arm64,darwin-x64,darwin-arm64,win32-x64}`
- User command: `codext` (not `codex`)
- Native binary inside vendor: `codex` / `codex.exe` (unchanged)
- All user-facing text (tooltips, resume hints, README) must say `codext`

## Mandatory copy from OLD_BRANCH

Overwrite NEW_BRANCH with OLD_BRANCH versions of these files:

1. `.github/workflows/rust-release.yml`
2. `.github/scripts/install-musl-build-tools.sh`
3. `.github/scripts/rusty_v8_bazel.py`
4. `codex-cli/package.json`
5. `codex-cli/bin/codex.js`
6. `codex-cli/bin/rg`
7. `codex-cli/scripts/build_npm_package.py`
8. `codex-cli/scripts/install_native_deps.py`

## Mandatory deletes

Delete all `.github/workflows/*` that OLD_BRANCH deleted (i.e. workflows carried over from the upstream tag but not needed by this fork). Do not blindly delete workflows that upstream TAG newly added — evaluate those after the mandatory steps.

## Verify release workflow compatibility

After copying rust-release.yml from OLD_BRANCH, check upstream TAG's release CI. Actively adapt to upstream's current structure — do not cling to OLD_BRANCH's layout if upstream has moved on. The goal: working release pipeline with current upstream tooling, plus our fork-specific names (package, command, dist-tag).

Trace every runtime binary from build to package before committing:

1. Compare the upstream TAG workflow's `cargo --bin` matrix with the fork workflow.
2. Follow each binary through build output, uploaded artifact, standalone archive, vendor tree, and npm platform package.
3. Include `codex-code-mode-host` anywhere the CLI is shipped. It must sit beside `codex`/`codext`; `cargo build -p codex-cli` does not build it.
   Follow upstream's workspace-level release build and pass `--bin codex --bin codex-code-mode-host` in the same Cargo invocation. Do not split them into separate `-p` commands: target-specific feature resolution must remain shared.
4. Keep Windows-only helpers (`codex-windows-sandbox-setup.exe`, `codex-command-runner.exe`) and Linux resources such as `bwrap` aligned with the current release scripts.
5. Preserve `CARGO_NET_GIT_FETCH_WITH_CLI: "true"` in release jobs. The workspace contains git dependencies with nested submodules, and macOS runners can fail when Cargo uses libgit2/SecureTransport.

Run the static parity audit after the carry-over copy:

```bash
bash .agents/skills/codex-upstream-reapply/scripts/check_release_artifact_parity.sh
```

For local execution, build the CLI and companion host with the same profile:

```bash
cd codex-rs
cargo build --release --bin codex --bin codex-code-mode-host
```

Debug builds work the same way; `--release` is not what makes code mode available.

## After mandatory steps

Only then evaluate upstream TAG's new/changed CI files. If they don't affect the release pipeline, ignore them. If they must be merged, do minimal integration without changing package names or command names.
