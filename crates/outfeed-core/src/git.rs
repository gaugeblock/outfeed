//! Git operations and tree inspection utilities.

use crate::error::OutfeedError;
use std::path::Path;
use std::process::Command;

/// Asserts that the target repository has a completely clean working tree.
pub fn assert_clean_working_tree(repo_dir: &Path) -> Result<(), OutfeedError> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to invoke git status: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OutfeedError::Git(format!("git status failed: {stderr}")));
    }

    let status_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !status_text.is_empty() {
        return Err(OutfeedError::DirtyWorkingTree(status_text));
    }
    Ok(())
}

/// Asserts that the target repository (and any parent meta-harness repository) is on 'main' branch.
pub fn assert_main_branch(repo_dir: &Path) -> Result<(), OutfeedError> {
    let branch = get_current_branch(repo_dir)?;
    if branch != "main" {
        return Err(OutfeedError::InvalidBranch(format!(
            "{} is on branch '{branch}'",
            repo_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repository")
        )));
    }

    // If repo is nested inside an enclosing parent git repo (meta-harness workspace root),
    // ensure the parent is also on 'main' and clean.
    let abs_dir = repo_dir
        .canonicalize()
        .unwrap_or_else(|_| repo_dir.to_path_buf());

    if let Some(parent) = abs_dir.parent()
        && parent.join(".git").exists()
    {
        let parent_branch = get_current_branch(parent)?;
        if parent_branch != "main" {
            return Err(OutfeedError::InvalidBranch(format!(
                "workspace root is on branch '{parent_branch}'"
            )));
        }
        assert_clean_working_tree(parent)?;
    }

    Ok(())
}

/// Resolves current git branch name.
pub fn get_current_branch(repo_dir: &Path) -> Result<String, OutfeedError> {
    let sym = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(repo_dir)
        .output();

    if let Ok(out) = sym
        && out.status.success()
    {
        let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !branch.is_empty() {
            return Ok(branch);
        }
    }

    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to get current branch: {e}")))?;

    if !output.status.success() {
        return Err(OutfeedError::Git("Failed to determine branch".to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Resolves the latest annotated or lightweight git tag.
pub fn get_latest_tag(repo_dir: &Path) -> Result<Option<String>, OutfeedError> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to run git describe: {e}")))?;

    if !output.status.success() {
        // No tag found in repository
        return Ok(None);
    }
    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if tag.is_empty() {
        Ok(None)
    } else {
        Ok(Some(tag))
    }
}

/// Fetches commit list (SHA, summary, body) between two references.
pub fn get_commits_between(
    repo_dir: &Path,
    from_ref: Option<&str>,
    to_ref: &str,
) -> Result<Vec<(String, String, Option<String>)>, OutfeedError> {
    let range = match from_ref {
        Some(from) => format!("{from}..{to_ref}"),
        None => to_ref.to_string(),
    };

    let output = Command::new("git")
        .args(["log", "--format=%H%x1f%s%x1f%b%x1e", &range])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to get git log: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OutfeedError::Git(format!("git log failed: {stderr}")));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for record in raw.split('\x1e') {
        let trimmed = record.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('\x1f').collect();
        if parts.is_empty() {
            continue;
        }
        let sha = parts[0].trim().to_string();
        let summary = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
        let body = parts
            .get(2)
            .map(|b| b.trim().to_string())
            .filter(|b| !b.is_empty());

        commits.push((sha, summary, body));
    }

    Ok(commits)
}

/// Creates a release commit and annotated git tag.
pub fn create_release_commit_and_tag(
    repo_dir: &Path,
    version: &str,
) -> Result<(), OutfeedError> {
    let commit_msg = format!("chore(release): release v{version}");
    let tag_name = format!("v{version}");

    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("git add failed: {e}")))?;

    if !add.status.success() {
        return Err(OutfeedError::Git("Failed to stage release changes".to_string()));
    }

    let commit = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("git commit failed: {e}")))?;

    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr);
        return Err(OutfeedError::Git(format!("git commit failed: {stderr}")));
    }

    let tag = Command::new("git")
        .args(["tag", "-a", &tag_name, "-m", &tag_name])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("git tag failed: {e}")))?;

    if !tag.status.success() {
        let stderr = String::from_utf8_lossy(&tag.stderr);
        return Err(OutfeedError::Git(format!("git tag failed: {stderr}")));
    }

    Ok(())
}

