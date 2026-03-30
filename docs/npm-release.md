# NPM release requirements

This document records the NPM release requirements confirmed in the recent conversation.

## Goal

The repository should keep a single GitHub Actions workflow dedicated to NPM publishing.

## Workflow requirements

- Keep only one release workflow for NPM publishing.
- The workflow should trigger on every push to `main`.
- Manual triggering via `workflow_dispatch` is acceptable.
- The workflow does not need to run tests.
- The workflow does not need to run unrelated CI checks.
- The workflow only needs to build successfully and publish the NPM package.
- The workflow should keep publishing the newest package under the `latest` dist-tag.

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
