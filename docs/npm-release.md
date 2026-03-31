# NPM release requirements

This document records the codext release requirements confirmed in the recent conversation.

## Goal

The repository should keep a single GitHub Actions workflow dedicated to codext release publishing.

## Workflow requirements

- Keep only one release workflow for codext publishing.
- The workflow should trigger on every push to `main`.
- Manual triggering via `workflow_dispatch` is acceptable.
- The workflow does not need to run tests.
- The workflow does not need to run unrelated CI checks.
- The workflow only needs to build successfully and publish the NPM package plus GitHub Release assets.
- The workflow should keep publishing the newest package under the `latest` dist-tag.
- The workflow should also create or update a GitHub Release on every push to `main`.

## Package identity

- The published package name must be `@loongphy/codext`.
- Global installation must work with:

```bash
npm i -g @loongphy/codext
```

- The installed command must be `codext`.
- Running the CLI after installation must work with:

```bash
codext
```

## Package layout

- Keep the existing NPM packaging model of one entry package plus platform-specific payload packages.
- Users install one top-level package: `@loongphy/codext`.
- Platform-specific native payloads are published separately and selected at runtime based on the current platform and architecture.
- This layout is acceptable because it avoids downloading every platform binary on every install and keeps install size smaller.
- The current supported platform payloads are:
  - `linux-x64`
  - `darwin-x64`
  - `darwin-arm64`
  - `win32-x64`

## GitHub Release assets

- Each push to `main` should create or update a GitHub Release for the current build.
- Each new GitHub Release should be marked as the repository `latest` release.
- The GitHub release tag should be derived from the computed release version for that commit.
- The release notes should begin with an upstream changelog link rendered as
  `codex-v<base_version>`, pointing to
  `https://github.com/openai/codex/releases/tag/rust-v<base_version>`.
- GitHub release assets should only contain codext CLI binaries for the supported platforms.
- The released archive set should match the supported platforms:
  - `codext-linux-x64-<version>.tar.gz`
  - `codext-darwin-x64-<version>.tar.gz`
  - `codext-darwin-arm64-<version>.tar.gz`
  - `codext-win32-x64-<version>.zip`

## Versioning requirements

- Every publish must use a unique NPM version so fixes can be released repeatedly from the same base version.
- A commit-hash suffix is acceptable for uniqueness.
- Example versions discussed in the conversation:
  - `0.117.0-e293`
  - `0.117.0-8e93`
- For NPM publishing, use the semver string without a leading `v`.
- Using a hash suffix is enough to avoid the "same version cannot be published twice" problem.
- The `latest` dist-tag should always point to the newest published build.

## Version ordering caveat

- NPM accepts semver prerelease versions with suffixes such as `0.117.0-e293`.
- Do not rely on hash-only suffixes for semantic ordering.
- The source of truth for "newest install" should be the `latest` dist-tag, not lexical comparison between hash suffixes.

## Scope limits

- Only NPM release related files should be changed for this work.
- Do not modify unrelated helper scripts, container scripts, or other non-NPM functionality as part of this release setup.
- In particular, avoid broad cleanup outside the NPM publishing path unless explicitly requested.
