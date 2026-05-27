---
name: codex-upstream-reapply
description: Reapply a fork or secondary-development branch onto a newer upstream `rust-vX.Y.Z` tag by starting from the new tag, generating reference material from the old branch, and re-implementing the intended behavior without merging or rebasing old history.
---

# Codex Upstream Reapply

Use this skill to sync a customization branch onto a newer upstream Rust tag.

The goal is not to carry old branch history forward.
The goal is to re-implement the real behavior and intent of the old branch on top of the selected `TAG`.

## Goal

The finished `NEW_BRANCH` should satisfy these conditions:

- It starts from the selected upstream `TAG`.
- The old branch `OLD_BRANCH` is reference material, not a merge or rebase target.
- Every old-branch delta relative to its original base has an outcome.
  Each one is either carried over automatically, re-implemented, or explicitly dropped with a reason.

## Defaults

When the user does not specify inputs, use these defaults:

- `REMOTE=upstream`
- `TAG_PATTERN=rust-*`
- `TAG=latest stable rust-vX.Y.Z release`
- `OLD_BRANCH=current branch`
- `NEW_BRANCH=feat/<tag>`
- `OLD_BASE_TAG` only when merge-base inference is unreliable

## Working rules

- Follow the repository's own `AGENTS.md`, README, and local guardrails first.
  This skill only adds the key reapply checkpoints.
  It does not invent a second set of universal test, format, or lint bans.
- Prefer re-implementing the behavior.
  Do not default to merge, rebase, or replaying old commits.
- Reuse the existing scripts and references whenever possible.
  If a rule is already encoded in a script, do not restate the whole workflow in prose.
- When a new upstream breakage pattern is discovered, encode it back into this skill's docs or scripts in the same wave.
- Ask the user only when old-branch intent, base selection, or upstream behavior creates a real semantic conflict.

## Workflow

### 1. Resolve the source and target

First resolve these inputs:

- `TAG`
- `OLD_BRANCH`
- `NEW_BRANCH`
- whether `OLD_BASE_TAG` needs to be explicit

Read `references/advanced.md` if you need worktree recipes, diff comparisons, or manual verification patterns.

### 2. Bootstrap the new branch and the reference bundle

Prefer `scripts/start_from_tag.sh`.

It handles these jobs:

- fetch and select the tag
- generate the reference bundle
- create `NEW_BRANCH` from `TAG`
- apply the fixed carry-over actions already encoded in the script

If you do not use this script, you must still reach the same outcome:

- `NEW_BRANCH` starts from `TAG`
- You have at least these reference artifacts
  - `META.md`
  - `changed-files.txt`
  - `diff.patch`
  - `diffstat.txt`
  - `commits.txt`
  - `coverage-checklist.md`

If merge-base inference looks suspicious, pass `--old-base-tag` explicitly.

### 3. Read intent before code

Understand what behavior must survive before you start writing code.

Start with these materials:

- `CHANGED.md`
- `README.md`
- intent documents on the old branch
- `coverage-checklist.md`, `changed-files.txt`, and `diff.patch` from the bundle
- `upstream-release-impact.md` from the bundle when npm/release rules apply

Old branch code and commit history are there to help you understand the requirement.
They are not the target shape of the new branch.

If the work touches npm, release, or CI changes, read `references/npm-release.md` first.

### 4. Re-implement on top of the selected tag

Implement the requirements in the structure of the selected `TAG`.

Use these decision rules:

- Preserve user-visible behavior and real intent first.
  Do not cling to the old file layout or old interfaces.
- Handle every path in `coverage-checklist.md` explicitly.
- If upstream already absorbed an old change, record it as `covered by upstream`.
- If `upstream-release-impact.md` shows release-critical upstream changes, review them against the release invariants before deciding whether to keep or adapt the old CI.
- If you decide not to port an old change, record the reason instead of silently dropping it.
- If you had to adapt to a new release-breaking upstream change, update the skill docs or scripts so the next reapply does not rediscover it from scratch.

### 5. Verify the reapply

When the reapply is done, verify at least these points:

- `NEW_BRANCH` really starts from the expected `TAG`.
- The final diff is a re-implementation on the new tag, not old history mixed forward.
- Every old-branch change listed in `coverage-checklist.md` has an outcome.
- If release or npm publish behavior is in scope, the branch still satisfies the release invariants that protect multi-platform build outputs and npm publish prerequisites.
- The repository-required verification for the touched surface has been executed.

The minimum default build check for this repository is:

```bash
cd codex-rs
cargo build -p codex-cli
```

If the repository, branch, or current task adds extra guardrails, keep following them.

Also do a final comparison:

```bash
git diff --stat TAG..NEW_BRANCH
git diff TAG..NEW_BRANCH
```

## References

- `scripts/start_from_tag.sh`
  Entry script for creating the new branch, generating the bundle, and reusing encoded carry-over rules.
- `scripts/prepare_reimplementation_bundle.sh`
  Use this when you only need the reference bundle.
- `references/advanced.md`
  Worktree, merge-base, and old-vs-new diff recipes.
- `references/npm-release.md`
  Read this only when the old branch includes npm, release, or CI related changes.
