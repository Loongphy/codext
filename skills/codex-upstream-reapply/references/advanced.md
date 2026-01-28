# Advanced Recipes

## Two worktrees for side-by-side porting (recommended)

Use when:
- You want to read old code/docs while implementing on the new tag-based branch.
- You want to avoid constantly switching branches.

Example:

```bash
# In repo root (adjust paths as needed)
git fetch origin --tags --prune

# Old branch worktree (reference)
git worktree add /tmp/wt-old OLD_BRANCH

# New branch worktree (fresh branch from selected tag)
git worktree add -b NEW_BRANCH /tmp/wt-new TAG
```

Cleanup:

```bash
git worktree remove /tmp/wt-old
git worktree remove /tmp/wt-new
```

## Find the real delta of OLD_BRANCH (merge-base vs tag)

Use when:
- Your old branch was based on an older tag/commit and you want the exact “custom delta”.

Example:

```bash
BASE_COMMIT="$(git merge-base TAG OLD_BRANCH)"
git diff "${BASE_COMMIT}..OLD_BRANCH" > /tmp/old-delta.patch
git diff --name-status "${BASE_COMMIT}..OLD_BRANCH"
```

## Read old files without switching branches

Use when:
- You are on NEW_BRANCH but want to view old docs/code quickly.

```bash
git show OLD_BRANCH:path/to/file
```

For diffs:

```bash
git diff OLD_BRANCH -- path/to/file
```

## Compare “custom delta” old vs new

Use when:
- NEW_BRANCH is based on a selected tag and you want to verify your re-implementation covers the old intent.

```bash
OLD_BASE="$(git merge-base TAG OLD_BRANCH)"

# Old delta (against its original base)
git diff "${OLD_BASE}..OLD_BRANCH" > /tmp/old.patch

# New delta (against selected tag)
git diff TAG..NEW_BRANCH > /tmp/new.patch

# Optional quick check (names only)
git diff --name-status "${OLD_BASE}..OLD_BRANCH"
git diff --name-status TAG..NEW_BRANCH
```
