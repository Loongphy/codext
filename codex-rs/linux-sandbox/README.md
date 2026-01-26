# codex-linux-sandbox

This crate is responsible for producing:

- a `codex-linux-sandbox` standalone executable for Linux that is bundled with the Node.js version of the Codex CLI
- a lib crate that exposes the business logic of the executable as `run_main()` so that
  - the `codex-exec` CLI can check if its arg0 is `codex-linux-sandbox` and, if so, execute as if it were `codex-linux-sandbox`
  - this should also be true of the `codex` multitool CLI

On Linux, the sandbox helper expects `bwrap` (bubblewrap) to be available on `PATH`.

**Current Behavior**
- The helper applies `PR_SET_NO_NEW_PRIVS` and a seccomp network filter in-process.
- Filesystem restrictions are enforced by `bwrap`.
- The filesystem is read-only by default via `--ro-bind / /`.
- Writable roots are layered with `--bind <root> <root>`.
- Protected subpaths under writable roots (for example `.git`, resolved `gitdir:`, and `.codex`) are re-applied as read-only via `--ro-bind`.
- Symlink-in-path and non-existent protected paths inside writable roots are blocked by mounting `/dev/null` on the symlink or first missing component.
- The helper isolates the PID namespace via `--unshare-pid`.
- By default it mounts a fresh `/proc` via `--proc /proc`, but you can skip this in restrictive container environments with `--no-proc`.

**Notes**
- The CLI surface still uses legacy names like `codex debug landlock`, but the filesystem sandboxing is bubblewrap-based.
- See `docs/linux_sandbox.md` for the full Linux sandbox semantics.
