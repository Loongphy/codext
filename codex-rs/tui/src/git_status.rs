use std::path::Path;

use tokio::process::Command;
use tokio::time::Duration;
use tokio::time::timeout;

const GIT_STATUS_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitStatusSummary {
    pub(crate) branch: String,
    pub(crate) changed: usize,
    pub(crate) untracked: usize,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
}

pub(crate) async fn collect_git_status_summary(cwd: &Path) -> Option<GitStatusSummary> {
    let output = run_git_command(&["rev-parse", "--is-inside-work-tree"], cwd).await?;
    if !output.status.success() {
        return None;
    }

    let status_output = run_git_command(
        &["status", "--porcelain=2", "-z", "--untracked-files=normal"],
        cwd,
    )
    .await?;
    if !status_output.status.success() {
        return None;
    }

    let (changed, untracked) = parse_porcelain_counts(&status_output.stdout);

    let branch = match current_branch_name(cwd).await {
        Some(branch) => branch,
        None => {
            let sha = short_head_sha(cwd).await;
            sha.map(|sha| format!("detached@{sha}"))
                .unwrap_or_else(|| "detached".to_string())
        }
    };

    let ahead = count_commits(cwd, "@{u}..HEAD").await.unwrap_or(0);
    let behind = count_commits(cwd, "HEAD..@{u}").await.unwrap_or(0);

    Some(GitStatusSummary {
        branch,
        changed,
        untracked,
        ahead,
        behind,
    })
}

fn parse_porcelain_counts(output: &[u8]) -> (usize, usize) {
    let mut changed = 0;
    let mut untracked = 0;

    for entry in output.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }

        match entry[0] {
            b'?' => untracked += 1,
            b'!' => {}
            b'1' | b'2' | b'u' => changed += 1,
            _ => {}
        }
    }

    (changed, untracked)
}

async fn current_branch_name(cwd: &Path) -> Option<String> {
    let output = run_git_command(&["branch", "--show-current"], cwd).await?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn short_head_sha(cwd: &Path) -> Option<String> {
    let output = run_git_command(&["rev-parse", "--short", "HEAD"], cwd).await?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn count_commits(cwd: &Path, range: &str) -> Option<usize> {
    let output = run_git_command(&["rev-list", "--count", range], cwd).await?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .and_then(|value| value.trim().parse().ok())
}

async fn run_git_command(args: &[&str], cwd: &Path) -> Option<std::process::Output> {
    let output = timeout(
        GIT_STATUS_TIMEOUT,
        Command::new("git").args(args).current_dir(cwd).output(),
    )
    .await
    .ok()?
    .ok()?;

    Some(output)
}
