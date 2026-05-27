# NPM release

Use this reference only when the old branch includes npm, release, or CI customization.

## Goal

Keep packaging and npm publishing working on top of the selected upstream `TAG`.
Release workflow details may change between upstream tags, so preserve the intended package outcome instead of freezing an old CI shape.
The default GitHub Actions surface is a single workflow: `.github/workflows/rust-release.yml`.
Other workflows, checks, labels, stale automation, and test CI are out of scope unless packaging or npm publishing cannot work without adding a specific upstream piece.

## Sources of truth

- Use `README.md` for user-facing package and command expectations.
- Use `CHANGED.md` for detailed fork behavior and release notes.
- Use old-branch release files as reference material.
- Prefer `OLD_BRANCH`'s `.github/workflows/rust-release.yml` for GitHub Actions.
- Use `upstream-release-impact.md` to notice upstream changes that may affect packaging or npm publishing.

## Decision rules

- Keep only `.github/workflows/rust-release.yml` by default.
- Start from `OLD_BRANCH`'s `rust-release.yml`.
- If upstream changed release scripts, artifact names, platform setup, dependency download, checksum verification, or npm publish order, check whether the change affects package build or npm publish behavior for this fork.
- If it affects packaging or publishing, adapt the minimum needed upstream change while preserving the package identity and command behavior declared by the repo.
- If upstream requires another workflow or config file for packaging or npm publishing, introduce only that required piece and record why.
- If it does not affect packaging or publishing, leave the fork release behavior alone and ignore the upstream CI change.
- If upstream already implements a needed release fix, record it as `covered by upstream`.
- If the selected `TAG` requires a different implementation than `CHANGED.md` describes, keep the release outcome working and update `CHANGED.md` or `README.md` when their text would otherwise become misleading.

## Temporary notes

Record release decisions in the reference bundle's temporary notes, such as `reapply-notes.md`.
Include:

- upstream changes that were adapted
- upstream changes that were intentionally ignored
- package identity or command-name decisions
- README/CHANGED updates
- any packaging or npm publish risk left for the user to review

Summarize these notes to the user when the reapply is complete.

## Ask the user only for unresolved product choices

Ask only when the repo does not make the required package identity, command name, supported platforms, or publish target clear, and more than one choice would produce a different user-visible release outcome.
