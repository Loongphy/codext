# NPM Release Reapply Rules

## Package identity

- npm package: `@loongphy/codext`
- Platform packages: `@loongphy/codext-{linux-x64,linux-arm64,darwin-x64,darwin-arm64,win32-x64}`
- User command: `codext` (not `codex`)
- Native binary inside vendor: `vendor/<target>/bin/codex` / `bin/codex.exe` (canonical codex-package layout)
- All user-facing text (tooltips, resume hints, README) must say `codext`

## Canonical package layout (required)

Since upstream made the app-server daemon the default and seeded it from a
self-contained local package, every npm platform package and release archive
must vendor a complete codex-package root per target:

```text
vendor/<target>/
  codex-package.json                 # layoutVersion/target/variant/entrypoint/...
  bin/codex(.exe)                    # entrypoint
  bin/codex-code-mode-host(.exe)
  codex-path/rg(.exe)
  codex-resources/bwrap              # Linux only; bytes must match CODEX_BWRAP_SHA256
  codex-resources/zsh/bin/zsh        # bundled zsh (non-Windows, when manifest has the platform)
  codex-resources/codex-command-runner.exe           # Windows only
  codex-resources/codex-windows-sandbox-setup.exe    # Windows only
```

Build these trees with `scripts/build_codex_package.py` (upstream tooling) and
pass `--package-version` the codext release version (e.g. `0.160.0-<sha>`). The
pre-release suffix is required: it keeps `prepare_install` from marking the
managed daemon release as latest-channel stable, so a codext-seeded daemon is
not auto-replaced by upstream standalone updates.

`codex-cli/bin/codex.js` must execute `vendor/<target>/bin/codex` and prepend
`vendor/<target>/codex-path` to `PATH`. Any layout that places the entrypoint
outside `bin/` or omits `codex-package.json` fails daemon seeding with
"this CLI has no complete local package".

## Mandatory copy from OLD_BRANCH

Use the OLD_BRANCH release workflow as the `F_OLD` packaging baseline, then apply the release CI sync procedure before pushing. Copy these codext packaging files from OLD_BRANCH:

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

Read [release-ci-sync.md](release-ci-sync.md) together with the npm release work, immediately before the final commit and push. It defines the three-way `U_OLD` / `F_OLD` / `U_NEW` comparison, the build/GitHub Release/npm Release scope, model-driven application of upstream changes, and the required final report.

## After mandatory steps

Only then evaluate upstream TAG's new/changed CI files. If they don't affect the release pipeline, ignore them. If they must be merged, do minimal integration without changing package names or command names.
