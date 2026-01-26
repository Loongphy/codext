# Linux Sandbox (bubblewrap + seccomp)

The Linux sandbox helper (`codex-linux-sandbox`) combines:
- in-process restrictions (`PR_SET_NO_NEW_PRIVS` and seccomp), and
- bubblewrap (`bwrap`) for filesystem isolation.

This mirrors the macOS Seatbelt semantics as closely as practical while
remaining unprivileged.

## Requirements

- `bwrap` must be installed and available on `PATH`.

## Filesystem Semantics

When disk writes are restricted (`read-only` or `workspace-write`), the helper
builds the filesystem view with bubblewrap in this order:

1. `--ro-bind / /` makes the entire filesystem read-only.
2. `--bind <root> <root>` re-enables writes for each writable root.
3. `--ro-bind <subpath> <subpath>` re-applies read-only protections under
   writable roots so protected paths win.
4. `--bind /dev/null /dev/null` preserves the common sink.

Writable roots and protected subpaths are derived from
`SandboxPolicy::get_writable_roots_with_cwd()`.

Protected subpaths include:
- top-level `.git` (directory or pointer file),
- the resolved `gitdir:` target for worktrees and submodules, and
- top-level `.codex`.

### Deny-path Hardening

To reduce symlink and path-creation attacks inside writable roots:
- If any component of a protected path is a symlink within a writable root, the
  helper mounts `/dev/null` on that symlink.
- If a protected path does not exist, the helper mounts `/dev/null` on the
  first missing path component (when it is within a writable root).

## Process and Network Semantics

- The helper isolates the PID namespace via `--unshare-pid`.
- By default it mounts a fresh `/proc` via `--proc /proc`.
- In restrictive container environments, you can skip the `/proc` mount with
  the helper flag `--no-proc` while still keeping PID isolation enabled.
- Network restrictions are enforced with seccomp when network access is
  disabled.

## Notes

- The CLI still exposes legacy names such as `codex debug landlock`, but the
  filesystem sandbox is bubblewrap-based.
- Landlock helpers remain in the codebase as legacy/backup utilities but are
  not currently used for filesystem enforcement.
