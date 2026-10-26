//! Multi-repository audit and synchronization orchestrator.

use std::path::Path;
use std::process::Command;

use crate::error::OutfeedError;
use crate::git::{get_current_branch, get_head_commit_sha};
use crate::install::{SUITE_TOOLS, SuiteTool};
use crate::progress::{ProgressReporter, RepoStatus};
use crate::suite_lock::SuiteLock;
use crate::sync_model::{RepoSyncStatus, SyncOpts, calculate_divergence, is_working_tree_dirty};

/// Audits a repository's branch, dirty status, and remote divergence.
pub fn audit_repo_sync(repo_dir: &Path) -> Result<RepoSyncStatus, OutfeedError> {
    let repo_name = repo_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo")
        .to_string();

    let branch = get_current_branch(repo_dir).unwrap_or_else(|_| "HEAD".to_string());
    let head_sha = get_head_commit_sha(repo_dir).unwrap_or_else(|_| "unknown".to_string());
    let dirty = is_working_tree_dirty(repo_dir)?;
    let divergence = calculate_divergence(repo_dir, &branch);

    Ok(RepoSyncStatus {
        repo: repo_name,
        path: repo_dir.to_path_buf(),
        branch,
        head_sha,
        dirty,
        divergence,
        action_taken: None,
    })
}

/// Synchronizes or audits a single repository.
pub fn sync_tool(
    workspace_root: &Path,
    tool: &SuiteTool,
    opts: &SyncOpts,
    reporter: &dyn ProgressReporter,
) -> Result<RepoSyncStatus, OutfeedError> {
    let repo_dir = workspace_root.join(tool.name);
    if !repo_dir.exists() {
        return Err(OutfeedError::Git(format!(
            "Repository {} not found at {}",
            tool.name,
            repo_dir.display()
        )));
    }

    reporter.on_start(tool.name, "auditing");
    let mut status = audit_repo_sync(&repo_dir)?;

    if opts.status_only {
        reporter.on_complete(tool.name, &RepoStatus::Success("audited".to_string()));
        return Ok(status);
    }

    if status.dirty {
        let msg = "dirty working tree; skipping sync".to_string();
        status.action_taken = Some(msg.clone());
        reporter.on_complete(tool.name, &RepoStatus::Skipped(msg));
        return Ok(status);
    }

    if opts.locked {
        if let Some(commit) = read_locked_commit(workspace_root, tool.name) {
            if status.head_sha == commit {
                status.action_taken = Some("already aligned with suite.lock".to_string());
                reporter.on_complete(tool.name, &RepoStatus::Skipped("in sync".to_string()));
            } else if opts.dry_run {
                status.action_taken = Some(format!("would checkout pinned commit {commit}"));
                reporter.on_complete(tool.name, &RepoStatus::Success("simulated pin".to_string()));
            } else {
                checkout_commit(&repo_dir, &commit)?;
                status.head_sha = commit.clone();
                status.action_taken = Some(format!("checked out pinned commit {commit}"));
                reporter.on_complete(
                    tool.name,
                    &RepoStatus::Success(format!("pinned to {commit}")),
                );
            }
        }
        return Ok(status);
    }

    // Default: fast-forward pull on main
    if status.branch != "main" {
        let msg = format!("on branch '{}'; fast-forward skipped", status.branch);
        status.action_taken = Some(msg.clone());
        reporter.on_complete(tool.name, &RepoStatus::Skipped(msg));
        return Ok(status);
    }

    if opts.dry_run {
        status.action_taken = Some("would fast-forward pull from origin/main".to_string());
        reporter.on_complete(
            tool.name,
            &RepoStatus::Success("simulated pull".to_string()),
        );
        return Ok(status);
    }

    let pull_res = Command::new("git")
        .args(["pull", "--ff-only", "origin", "main"])
        .current_dir(&repo_dir)
        .output();

    match pull_res {
        Ok(out) if out.status.success() => {
            status.head_sha = get_head_commit_sha(&repo_dir)?;
            status.action_taken = Some("fast-forward updated from origin/main".to_string());
            reporter.on_complete(tool.name, &RepoStatus::Success("pulled".to_string()));
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            status.action_taken = Some(format!("pull failed: {stderr}"));
            reporter.on_complete(tool.name, &RepoStatus::Failed(stderr.to_string()));
        }
        Err(e) => {
            status.action_taken = Some(format!("pull error: {e}"));
            reporter.on_complete(tool.name, &RepoStatus::Failed(e.to_string()));
        }
    }

    Ok(status)
}

/// Audits or synchronizes selected suite repositories.
pub fn sync_suite(
    workspace_root: &Path,
    targets: &[&str],
    opts: &SyncOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<RepoSyncStatus>, OutfeedError> {
    let selected_tools: Vec<_> = if opts.all || targets.is_empty() {
        SUITE_TOOLS
            .iter()
            .copied()
            .filter(|t| workspace_root.join(t.name).exists())
            .collect()
    } else {
        SUITE_TOOLS
            .iter()
            .copied()
            .filter(|t| targets.contains(&t.name) && workspace_root.join(t.name).exists())
            .collect()
    };

    let mut statuses = Vec::new();
    for tool in &selected_tools {
        statuses.push(sync_tool(workspace_root, tool, opts, reporter)?);
    }

    Ok(statuses)
}

fn read_locked_commit(workspace_root: &Path, repo_name: &str) -> Option<String> {
    let lock_path = workspace_root.join("suite.lock");
    let content = std::fs::read_to_string(lock_path).ok()?;
    let lock: SuiteLock = toml::from_str(&content).ok()?;
    lock.tools
        .into_iter()
        .find(|t| t.name == repo_name)
        .map(|t| t.commit)
}

fn checkout_commit(repo_dir: &Path, commit_sha: &str) -> Result<(), OutfeedError> {
    let output = Command::new("git")
        .args(["checkout", commit_sha])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to checkout {commit_sha}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OutfeedError::Git(format!("git checkout failed: {stderr}")));
    }
    Ok(())
}
