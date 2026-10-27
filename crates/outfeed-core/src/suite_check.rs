//! Phase 1: Parallel pre-flight validation across all suite repositories.

use std::path::Path;

use crate::error::OutfeedError;
use crate::git::get_current_branch;
use crate::install::SUITE_TOOLS;
use crate::sync_model::is_working_tree_dirty;

/// Result of pre-flight check on an individual repository.
#[derive(Debug, Clone)]
pub struct RepoPreflightCheck {
    /// Target repository name.
    pub repo_name: String,
    /// Whether the repository exists on disk.
    pub exists: bool,
    /// Current Git branch.
    pub branch: String,
    /// Whether branch is 'main'.
    pub is_main: bool,
    /// Whether working tree is clean.
    pub is_clean: bool,
}

impl RepoPreflightCheck {
    /// Returns true if this repository passed all pre-flight invariants.
    pub fn is_ok(&self) -> bool {
        self.exists && self.is_main && self.is_clean
    }
}

/// Executes pre-flight validation across workspace root and all member repositories.
pub fn check_suite_preflight(
    workspace_root: &Path,
) -> Result<Vec<RepoPreflightCheck>, OutfeedError> {
    // 1. Check workspace root
    let root_branch = get_current_branch(workspace_root)?;
    if root_branch != "main" {
        return Err(OutfeedError::InvalidBranch(format!(
            "Invalid branch 'workspace root is on branch '{root_branch}'': releases must occur on 'main'"
        )));
    }
    if is_working_tree_dirty(workspace_root)? {
        return Err(OutfeedError::DirtyWorkingTree(
            "Workspace root working tree is dirty. Stash or commit changes before releasing."
                .to_string(),
        ));
    }

    // 2. Check each member repository
    let mut checks = Vec::new();
    let mut failed = false;

    for tool in SUITE_TOOLS {
        let repo_dir = workspace_root.join(tool.name);
        if !repo_dir.exists() {
            continue;
        }

        let branch = get_current_branch(&repo_dir).unwrap_or_else(|_| "unknown".to_string());
        let is_main = branch == "main";
        let is_clean = !is_working_tree_dirty(&repo_dir).unwrap_or(true);

        if !is_main || !is_clean {
            failed = true;
        }

        checks.push(RepoPreflightCheck {
            repo_name: tool.name.to_string(),
            exists: true,
            branch,
            is_main,
            is_clean,
        });
    }

    if failed {
        let mut report =
            String::from("Pre-flight check failed for one or more suite repositories:\n");
        report.push_str(&format!(
            "{:<14} {:<15} {:<10} {:<10}\n",
            "REPOSITORY", "BRANCH", "PORCELAIN", "STATUS"
        ));
        report.push_str(&format!("{}\n", "-".repeat(52)));
        for c in &checks {
            let status = if c.is_ok() { "OK" } else { "FAIL" };
            let clean_str = if c.is_clean { "clean" } else { "DIRTY" };
            report.push_str(&format!(
                "{:<14} {:<15} {:<10} {:<10}\n",
                c.repo_name, c.branch, clean_str, status
            ));
        }
        return Err(OutfeedError::Git(report));
    }

    Ok(checks)
}
