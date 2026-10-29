//! Read-only history queries: tags, HEAD, and the commit range since a tag.
//!
//! Split out of `git::mod` (GB-139) so the mutating git operations and the queries that feed the
//! release plan sit in separate files.

use std::path::Path;
use std::process::Command;

use crate::error::OutfeedError;

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

/// Resolves the current HEAD commit full SHA.
pub fn get_head_commit_sha(repo_dir: &Path) -> Result<String, OutfeedError> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to get HEAD commit: {e}")))?;

    if !output.status.success() {
        return Err(OutfeedError::Git(
            "Failed to determine HEAD commit".to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
        let summary = parts
            .get(1)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let body = parts
            .get(2)
            .map(|b| b.trim().to_string())
            .filter(|b| !b.is_empty());

        commits.push((sha, summary, body));
    }

    Ok(commits)
}
