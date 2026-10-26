//! Data types and divergence calculations for multi-repo sync.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::OutfeedError;

/// Branch alignment status relative to upstream tracking remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchDivergence {
    /// In sync with upstream tracking branch.
    UpToDate,
    /// Local branch has unpushed commits.
    Ahead(usize),
    /// Local branch is behind upstream remote.
    Behind(usize),
    /// Local and upstream have diverged.
    Diverged {
        /// Number of unpushed local commits.
        ahead: usize,
        /// Number of unpulled upstream commits.
        behind: usize,
    },
    /// HEAD is detached (e.g. checked out at specific commit or tag).
    Detached,
    /// Upstream remote tracking branch not configured.
    Untracked,
}

/// Audit and synchronization snapshot for a single member repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoSyncStatus {
    /// Member repository name.
    pub repo: String,
    /// Absolute path to repository checkout.
    pub path: PathBuf,
    /// Current checked out branch name or HEAD.
    pub branch: String,
    /// Current commit SHA.
    pub head_sha: String,
    /// Whether working tree has uncommitted modifications.
    pub dirty: bool,
    /// Alignment with upstream tracking remote.
    pub divergence: BranchDivergence,
    /// Action taken or skipped rationale during sync.
    pub action_taken: Option<String>,
}

/// Execution options for sync and audit operations.
#[derive(Debug, Clone, Default)]
pub struct SyncOpts {
    /// Sync or audit all 10 suite repositories.
    pub all: bool,
    /// Align local checkouts to exact commits recorded in `suite.lock`.
    pub locked: bool,
    /// Only inspect and report status without performing pulls or checkouts.
    pub status_only: bool,
    /// Simulate sync operations without modifying checkouts.
    pub dry_run: bool,
}

/// Checks whether a repository has uncommitted changes in its working tree.
pub fn is_working_tree_dirty(repo_dir: &Path) -> Result<bool, OutfeedError> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to run git status: {e}")))?;

    if !output.status.success() {
        return Err(OutfeedError::Git("git status failed".to_string()));
    }
    Ok(!output.stdout.is_empty())
}

/// Calculates divergence between the local branch and its upstream tracking counterpart.
pub fn calculate_divergence(repo_dir: &Path, branch: &str) -> BranchDivergence {
    if branch == "HEAD" {
        return BranchDivergence::Detached;
    }

    let upstream = format!("origin/{branch}");
    let output = Command::new("git")
        .args([
            "rev-list",
            "--left-right",
            "--count",
            &format!("{branch}...{upstream}"),
        ])
        .current_dir(repo_dir)
        .output();

    let Ok(out) = output else {
        return BranchDivergence::Untracked;
    };

    if !out.status.success() {
        return BranchDivergence::Untracked;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() != 2 {
        return BranchDivergence::Untracked;
    }

    let ahead: usize = parts[0].parse().unwrap_or(0);
    let behind: usize = parts[1].parse().unwrap_or(0);

    match (ahead, behind) {
        (0, 0) => BranchDivergence::UpToDate,
        (a, 0) => BranchDivergence::Ahead(a),
        (0, b) => BranchDivergence::Behind(b),
        (a, b) => BranchDivergence::Diverged {
            ahead: a,
            behind: b,
        },
    }
}
