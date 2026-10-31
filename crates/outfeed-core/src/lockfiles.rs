//! Lockfile hygiene: every standalone repository must track a fresh `Cargo.lock`.
//!
//! The suite ships binaries, so the lockfile is what makes a published build reproducible. A
//! repository that stops tracking it re-resolves its dependency graph on every clone, which is how a
//! gated build and a user's build quietly diverge. A repository that tracks a stale one publishes a
//! graph that no longer matches its manifests.
//!
//! This lives in `outfeed-core` rather than a shell script so the check is part of the delivery
//! engine: `scripts/check-lockfiles.sh` is a wrapper over `outfeed lockfiles`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::OutfeedError;

/// What the check found for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoLockfile {
    /// Repository directory name.
    pub repo: String,
    /// Whether `git ls-files` reports `Cargo.lock` as tracked.
    pub tracked: bool,
    /// Whether `cargo metadata --locked` accepts the lockfile as current.
    pub fresh: bool,
}

impl RepoLockfile {
    /// True when the repository satisfies both requirements.
    pub fn is_ok(&self) -> bool {
        self.tracked && self.fresh
    }

    /// Short single-word status used in the text report.
    pub fn status(&self) -> &'static str {
        match (self.tracked, self.fresh) {
            (true, true) => "ok",
            (false, _) => "untracked",
            (true, false) => "stale",
        }
    }
}

/// Result of checking every standalone repository under a workspace root.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockfileReport {
    /// One entry per discovered repository, in directory order.
    pub repositories: Vec<RepoLockfile>,
}

impl LockfileReport {
    /// True when every repository tracks a fresh lockfile.
    pub fn is_ok(&self) -> bool {
        self.repositories.iter().all(RepoLockfile::is_ok)
    }

    /// Repositories that failed the check.
    pub fn failures(&self) -> Vec<&RepoLockfile> {
        self.repositories.iter().filter(|r| !r.is_ok()).collect()
    }
}

/// Discovers repositories under `workspace_root` and checks each one's lockfile.
///
/// A directory counts as a repository when it holds a `Cargo.toml` **and** owns a `.git`
/// entry — `.git` is a directory for a clone and a file for a worktree, so both are accepted.
/// Hidden directories, `target/` and the repository the workspace root itself belongs to are
/// skipped.
pub fn check_lockfiles(workspace_root: &Path) -> Result<LockfileReport, OutfeedError> {
    let mut repositories = Vec::new();
    let entries = fs::read_dir(workspace_root).map_err(OutfeedError::Io)?;

    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !path.is_dir() || name.starts_with('.') || name == "target" {
            continue;
        }
        if path.join("Cargo.toml").is_file() && path.join(".git").exists() {
            candidates.push(path);
        }
    }
    candidates.sort();

    for repo_dir in candidates {
        let repo = repo_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        repositories.push(RepoLockfile {
            tracked: is_lockfile_tracked(&repo_dir),
            fresh: is_lockfile_fresh(&repo_dir),
            repo,
        });
    }

    Ok(LockfileReport { repositories })
}

/// Whether git tracks `Cargo.lock` in `repo_dir`.
fn is_lockfile_tracked(repo_dir: &Path) -> bool {
    gaugeblock_git::Repository::discover(repo_dir)
        .and_then(|repo| gaugeblock_git::is_tracked(&repo, Path::new("Cargo.lock")))
        .unwrap_or(false)
}

/// Whether `cargo metadata --locked` accepts the lockfile as current.
fn is_lockfile_fresh(repo_dir: &Path) -> bool {
    Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(repo_dir)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Renders the report as an aligned text table.
pub fn format_lockfile_report(report: &LockfileReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{:<14} {:<12} {:<8}\n",
        "REPOSITORY", "LOCKFILE", "FRESH"
    ));
    out.push_str(&format!("{}\n", "-".repeat(36)));
    for repo in &report.repositories {
        out.push_str(&format!(
            "{:<14} {:<12} {:<8}\n",
            repo.repo,
            if repo.tracked { "tracked" } else { "untracked" },
            if repo.fresh { "yes" } else { "no" }
        ));
    }
    out.push('\n');
    if report.is_ok() {
        out.push_str(&format!(
            "All {} repositories track a fresh Cargo.lock.\n",
            report.repositories.len()
        ));
    } else {
        out.push_str(&format!(
            "{} of {} repositories failed the lockfile check.\n",
            report.failures().len(),
            report.repositories.len()
        ));
    }
    out
}
