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

    /// Internal: apply seccomp and `no_new_privs` in the already-sandboxed
    /// process, then exec the user command.
    ///
    /// This exists so we can run bubblewrap first (which may rely on setuid)
    /// and only tighten with seccomp after the filesystem view is established.
    #[arg(long = "apply-seccomp-then-exec", hide = true, default_value_t = false)]
    pub apply_seccomp_then_exec: bool,

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
/// 1. When needed, wrap the command with bubblewrap to construct the
///    filesystem view.
/// 2. Apply in-process restrictions (no_new_privs + seccomp).
/// 3. `execvp` into the final command.
pub fn run_main() -> ! {
    let LandlockCommand {
        sandbox_policy_cwd,
        sandbox_policy,
        apply_seccomp_then_exec,
        no_proc,
        command,
    } = LandlockCommand::parse();

    if command.is_empty() {
        panic!("No command specified to execute.");
    }

    // Inner stage: apply seccomp/no_new_privs after bubblewrap has already
    // established the filesystem view.
    if apply_seccomp_then_exec {
        if let Err(e) = apply_sandbox_policy_to_current_thread(&sandbox_policy, &sandbox_policy_cwd)
        {
            panic!("error applying Linux sandbox restrictions: {e:?}");
        }
        exec_or_panic(command);
    }

    let command = if sandbox_policy.has_full_disk_write_access() {
        if let Err(e) = apply_sandbox_policy_to_current_thread(&sandbox_policy, &sandbox_policy_cwd)
        {
            panic!("error applying Linux sandbox restrictions: {e:?}");
        }
        command
    } else {
        // Outer stage: bubblewrap first, then re-enter this binary in the
        // sandboxed environment to apply seccomp.
        ensure_bwrap_available();
        let inner = build_inner_seccomp_command(&sandbox_policy_cwd, &sandbox_policy, command);
        let options = BwrapOptions {
            mount_proc: !no_proc,
        };
        create_bwrap_command_args(inner, &sandbox_policy, &sandbox_policy_cwd, options)
            .unwrap_or_else(|err| panic!("error building bubblewrap command: {err:?}"))
    };

    if is_debug_bwrap_enabled() {
        // Debug-only visibility into the exact argv we are about to exec.
        eprintln!("codex-linux-sandbox exec argv: {command:?}");
    }

    exec_or_panic(command);
}

/// Build the inner command that applies seccomp after bubblewrap.
fn build_inner_seccomp_command(
    sandbox_policy_cwd: &PathBuf,
    sandbox_policy: &codex_core::protocol::SandboxPolicy,
    command: Vec<String>,
) -> Vec<String> {
    #[expect(clippy::expect_used)]
    let current_exe = std::env::current_exe().expect("Failed to resolve current executable path");
    let policy_json =
        serde_json::to_string(sandbox_policy).expect("Failed to serialize sandbox policy");

    let mut inner = vec![
        current_exe.to_string_lossy().to_string(),
        "--apply-seccomp-then-exec".to_string(),
        "--sandbox-policy-cwd".to_string(),
        sandbox_policy_cwd.to_string_lossy().to_string(),
        "--sandbox-policy".to_string(),
        policy_json,
        "--".to_string(),
    ];
    inner.extend(command);
    inner
}

/// Exec the provided argv, panicking with context if it fails.
fn exec_or_panic(command: Vec<String>) -> ! {
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
