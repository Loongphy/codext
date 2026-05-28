use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use anyhow::Context;
use anyhow::Result;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires tmux, python3, and a locally built codex binary; run with --ignored for the real TUI usage-limit smoke"]
async fn tmux_usage_limit_prompt_queue_does_not_post_responses() -> Result<()> {
    if cfg!(windows) {
        return Ok(());
    }
    if Command::new("tmux").arg("-V").output().is_err() {
        eprintln!("skipping usage-limit queue e2e because tmux is unavailable");
        return Ok(());
    }
    if Command::new("python3").arg("--version").output().is_err() {
        eprintln!("skipping usage-limit queue e2e because python3 is unavailable");
        return Ok(());
    }

    let repo_root = codex_utils_cargo_bin::repo_root()?;
    let script = repo_root.join("codex-rs/scripts/usage_limit_prompt_queue_e2e.py");
    let codex = codex_binary(&repo_root)?;

    checked_output(
        Command::new("python3")
            .arg(script)
            .arg("--codex-bin")
            .arg(codex),
    )?;
    Ok(())
}

fn codex_binary(repo_root: &Path) -> Result<PathBuf> {
    if let Ok(path) = codex_utils_cargo_bin::cargo_bin("codex") {
        return Ok(path);
    }

    let fallback = repo_root.join("codex-rs/target/debug/codex");
    anyhow::ensure!(
        fallback.is_file(),
        "codex binary is unavailable; run `cargo build -p codex-cli` first"
    );
    Ok(fallback)
}

fn checked_output(command: &mut Command) -> Result<Output> {
    let output = command
        .output()
        .with_context(|| format!("failed to run {command:?}"))?;
    anyhow::ensure!(
        output.status.success(),
        "command failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}
