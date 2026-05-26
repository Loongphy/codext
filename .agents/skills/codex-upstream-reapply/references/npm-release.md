# NPM release

Use this reference only when the old branch includes npm, release, or CI customization.
If this file exists on `OLD_BRANCH`, treat it as the source of truth for release behavior during the reapply.

## Release invariants

Protect these outcomes first:

- Supported platforms still produce the release binaries required by the branch
- The release workflow can fetch, verify, and package its build dependencies on those platforms
- Platform npm packages are published before the root package that depends on them
- Package identity stays aligned to `codext` unless the new upstream tag explicitly forces a rename
- The launcher and install scripts still resolve the intended platform package and native binary

If an upstream CI change does not threaten these invariants, it does not earn a change.

## Base

- Review old-branch changes from `git diff BASE_COMMIT..OLD_BRANCH`
- Default base is `BASE_COMMIT="$(git merge-base TAG OLD_BRANCH)"`
- If merge-base inference is unreliable, pass `--old-base-tag`

## Keep package identity from `OLD_BRANCH`

- Published package: `@loongphy/codext`
- Platform packages:
  - `@loongphy/codext-linux-x64`
  - `@loongphy/codext-linux-arm64`
  - `@loongphy/codext-darwin-x64`
  - `@loongphy/codext-darwin-arm64`
  - `@loongphy/codext-win32-x64`
- Installed command: `codext`
- NPM entry script: `codex-cli/bin/codex.js`
- The launcher may still execute `codex` or `codex.exe`

Keep user-facing strings aligned to `codext`.
For example, prefer `codext resume <session>`.

## Mandatory carry-over on `NEW_BRANCH`

Do these steps first.
They preserve the existing release pipeline before any upstream review starts.

1. Replace `.github/workflows/rust-release.yml` with the version from `OLD_BRANCH`.
2. Delete every other `.github/workflows/*` entry and keep only `rust-release.yml`.
3. Copy these paths directly from `OLD_BRANCH`:
   - `.github/actions/setup-rusty-v8-musl/action.yml`
   - `.github/scripts/install-musl-build-tools.sh`
   - `.github/scripts/rusty_v8_bazel.py`
   - `codex-cli/package.json`
   - `codex-cli/bin/codex.js`
   - `codex-cli/bin/rg`
   - `codex-cli/scripts/build_npm_package.py`
   - `codex-cli/scripts/install_native_deps.py`

Treat those paths as whole-file or whole-directory carry-over.
Do not re-derive publish names, dist-tags, or release text field by field.

## Review upstream impact after carry-over

After the mandatory carry-over, review `upstream-release-impact.md` from the bundle.
That artifact is generated from the upstream delta on release-critical paths between `BASE_COMMIT` and `TAG`.

Treat upstream changes as release-impacting when they affect:

- release workflow topology, platform matrix, artifact names, or publish order
- musl or V8 setup, dependency download, or checksum verification
- packaging scripts, launcher behavior, or native dependency install logic
- any path that would change whether multi-platform artifacts can be built or published correctly

If the upstream change does not affect the release invariants, keep the old release flow and do nothing.
If it does affect an invariant, adapt the minimum required pieces from upstream while preserving the established `codext` identity unless upstream forces a change.

## Evolve the skill when a new breakage mode appears

If upstream introduced a release-breaking pattern that this skill did not already cover, update the skill in the same wave:

- update this reference when the rule is procedural or judgment-heavy
- update `scripts/prepare_reimplementation_bundle.sh` when the critical-path review list is incomplete
- update `scripts/start_from_tag.sh` when the carry-over behavior itself must change

The next reapply should inherit the lesson structurally, not from memory.

## Ask the user only for real conflicts

- You are about to delete a publish entry that users may still rely on
- The new upstream tag adds a release or CI entry and you cannot tell whether it is required or obsolete
- The new upstream tag explicitly requires changing current publish names, install commands, dist-tags, or release artifact matrix
- The old branch and current tag clearly conflict on supported release platforms
