//! Git push operations for branches and release tags.

use std::path::Path;
use std::process::Command;

use crate::error::OutfeedError;
use crate::git::{assert_clean_working_tree, get_current_branch, get_latest_tag};
use crate::install::SUITE_TOOLS;
use crate::model::PushOpts;

/// Push result description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushOutcome {
    /// Remote pushed to.
    pub remote: String,
    /// Branch pushed.
    pub branch: String,
    /// Tag pushed, if any.
    pub tag: Option<String>,
    /// Whether this was a simulated dry-run.
    pub dry_run: bool,
}

/// Pushes the active branch and release tag for a repository.
pub fn push_repo(
    repo_dir: &Path,
    opts: &PushOpts,
) -> Result<PushOutcome, OutfeedError> {
    assert_clean_working_tree(repo_dir)?;
    let branch = get_current_branch(repo_dir)?;
    let tag = get_latest_tag(repo_dir)?;

    if opts.dry_run {
        return Ok(PushOutcome {
            remote: opts.remote.clone(),
            branch,
            tag,
            dry_run: true,
        });
    }

    // Push branch
    let branch_status = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("push")
        .arg(&opts.remote)
        .arg(&branch)
        .status()
        .map_err(|e| OutfeedError::Push(format!("git push branch failed: {e}")))?;

    if !branch_status.success() {
        return Err(OutfeedError::Push(format!(
            "git push {} {} exited with status {}",
            opts.remote, branch, branch_status
        )));
    }

    // Push latest tag if present
    if let Some(ref t) = tag {
        let tag_status = Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .arg("push")
            .arg(&opts.remote)
            .arg(t)
            .status()
            .map_err(|e| OutfeedError::Push(format!("git push tag failed: {e}")))?;

        if !tag_status.success() {
            return Err(OutfeedError::Push(format!(
                "git push {} {} exited with status {}",
                opts.remote, t, tag_status
            )));
        }
    }

    Ok(PushOutcome {
        remote: opts.remote.clone(),
        branch,
        tag,
        dry_run: false,
    })
}

/// Executes git push across all repositories in the suite.
pub fn push_all(
    workspace_root: &Path,
    opts: &PushOpts,
) -> Result<Vec<(String, PushOutcome)>, OutfeedError> {
    let mut results = Vec::new();
    for tool in SUITE_TOOLS {
        let repo_dir = workspace_root.join(tool.name);
        if repo_dir.exists() && repo_dir.join(".git").exists() {
            let outcome = push_repo(&repo_dir, opts)?;
            results.push((tool.name.to_string(), outcome));
        }
    }
    Ok(results)
}

