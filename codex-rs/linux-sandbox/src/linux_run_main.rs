use clap::Parser;
use std::ffi::CString;
use std::path::PathBuf;

use crate::bwrap::BwrapOptions;
use crate::bwrap::create_bwrap_command_args;
use crate::landlock::apply_sandbox_policy_to_current_thread;

#[derive(Debug, Parser)]
/// CLI surface for the Linux sandbox helper.
///
/// The type name remains `LandlockCommand` for compatibility with existing
/// wiring, but the filesystem sandbox now uses bubblewrap.
pub struct LandlockCommand {
    /// It is possible that the cwd used in the context of the sandbox policy
    /// is different from the cwd of the process to spawn.
    #[arg(long = "sandbox-policy-cwd")]
    pub sandbox_policy_cwd: PathBuf,

    #[arg(long = "sandbox-policy")]
    pub sandbox_policy: codex_core::protocol::SandboxPolicy,

    /// When set, skip mounting a fresh `/proc` even though PID isolation is
    /// still enabled. This is primarily intended for restrictive container
    /// environments that deny `--proc /proc`.
    #[arg(long = "no-proc", default_value_t = false)]
    pub no_proc: bool,

    /// Full command args to run under the Linux sandbox helper.
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

/// Entry point for the Linux sandbox helper.
///
/// The sequence is:
/// 1. Apply in-process restrictions (no_new_privs + seccomp).
/// 2. Wrap the command with bubblewrap when disk writes are restricted.
/// 3. `execvp` into the final command.
pub fn run_main() -> ! {
    let LandlockCommand {
        sandbox_policy_cwd,
        sandbox_policy,
        no_proc,
        command,
    } = LandlockCommand::parse();

    if command.is_empty() {
        panic!("No command specified to execute.");
    }

    // When disk writes are restricted, bubblewrap is required to construct the
    // read-only-by-default filesystem view. Fail fast with an actionable
    // message before applying other restrictions.
    if !sandbox_policy.has_full_disk_write_access() {
        ensure_bwrap_available();
    }

    if let Err(e) = apply_sandbox_policy_to_current_thread(&sandbox_policy, &sandbox_policy_cwd) {
        panic!("error applying Linux sandbox restrictions: {e:?}");
    }

    let command = if sandbox_policy.has_full_disk_write_access() {
        command
    } else {
        let options = BwrapOptions {
            mount_proc: !no_proc,
        };
        create_bwrap_command_args(command, &sandbox_policy, &sandbox_policy_cwd, options)
            .unwrap_or_else(|err| panic!("error building bubblewrap command: {err:?}"))
    };

    if is_debug_bwrap_enabled() {
        // Debug-only visibility into the exact argv we are about to exec.
        eprintln!("codex-linux-sandbox exec argv: {command:?}");
    }

    #[expect(clippy::expect_used)]
    let c_command =
        CString::new(command[0].as_str()).expect("Failed to convert command to CString");
    #[expect(clippy::expect_used)]
    let c_args: Vec<CString> = command
        .iter()
        .map(|arg| CString::new(arg.as_str()).expect("Failed to convert arg to CString"))
        .collect();

    let mut c_args_ptrs: Vec<*const libc::c_char> = c_args.iter().map(|arg| arg.as_ptr()).collect();
    c_args_ptrs.push(std::ptr::null());

    unsafe {
        libc::execvp(c_command.as_ptr(), c_args_ptrs.as_ptr());
    }

    // If execvp returns, there was an error.
    let err = std::io::Error::last_os_error();
    panic!("Failed to execvp {}: {err}", command[0].as_str());
}

/// Ensure the `bwrap` binary is available on PATH when the sandbox needs it.
fn ensure_bwrap_available() {
    if which::which("bwrap").is_ok() {
        return;
    }

    panic!(
        "bubblewrap (bwrap) is required for Linux filesystem sandboxing but was not found on PATH.\n\
Install it and retry. Examples:\n\
- Debian/Ubuntu: apt-get install bubblewrap\n\
- Fedora/RHEL: dnf install bubblewrap\n\
- Arch: pacman -S bubblewrap\n\
If you are running the Codex Node package, ensure bwrap is installed on the host system."
    );
}

/// Returns true when debug logging of the bwrap argv should be enabled.
///
/// This is intentionally controlled via an environment variable so we do not
/// need to thread additional flags through `codex-core` while debugging Linux
/// sandbox failures on devboxes.
fn is_debug_bwrap_enabled() -> bool {
    matches!(
        std::env::var("CODEX_LINUX_SANDBOX_DEBUG"),
        Ok(value) if value == "1" || value.eq_ignore_ascii_case("true")
    )
}
