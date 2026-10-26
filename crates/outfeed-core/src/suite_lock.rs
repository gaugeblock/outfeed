//! Data structures and manifest generation for `suite.lock`.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::OutfeedError;
use crate::git::{get_head_commit_sha, get_latest_tag};
use crate::install::SUITE_TOOLS;
use crate::manifest::get_repo_version;

/// Top-level metadata for `suite.lock`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteMetadata {
    /// Meta-harness ecosystem release version.
    pub version: String,
    /// Timestamp of lockfile generation in UTC ISO 8601.
    pub created_at: String,
    /// Canonical organization or meta-harness name.
    pub meta_harness: String,
}

/// Pinned state for a member repository in `suite.lock`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteToolEntry {
    /// Member repository name.
    pub name: String,
    /// Current package version declared in repository's root Cargo.toml.
    pub version: String,
    /// Exact git commit SHA currently checked out.
    pub commit: String,
    /// Most recent release tag, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Canonical schema for `suite.lock`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteLock {
    /// Ecosystem release metadata.
    pub suite: SuiteMetadata,
    /// Pinned member tool snapshots.
    pub tools: Vec<SuiteToolEntry>,
}

/// Execution options for suite release orchestration.
#[derive(Debug, Clone, Default)]
pub struct SuiteReleaseOpts {
    /// Optional explicit version string overriding auto-computed increment.
    pub target_version: Option<String>,
    /// Simulate suite release without modifying disk or committing/tagging.
    pub dry_run: bool,
    /// Skip pushing commit and tag to Git remote.
    pub skip_push: bool,
}

/// Summary result of suite release execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteReleaseSummary {
    /// Target suite release version.
    pub target_version: String,
    /// Annotated git tag created on meta-harness.
    pub tag: String,
    /// Number of member tools pinned in suite.lock.
    pub tool_count: usize,
    /// Path to written suite.lock manifest.
    pub lock_path: String,
    /// Whether commit and tag were created.
    pub committed: bool,
    /// Whether commit and tag were pushed to remote.
    pub pushed: bool,
}

/// Generates the canonical `SuiteLock` snapshot across all 10 member repositories.
pub fn generate_suite_lock(
    workspace_root: &Path,
    suite_version: &str,
) -> Result<SuiteLock, OutfeedError> {
    let mut tools = Vec::new();

    for tool in SUITE_TOOLS {
        let repo_dir = workspace_root.join(tool.name);
        if !repo_dir.exists() {
            continue;
        }

        let version = get_repo_version(&repo_dir)?;
        let commit = get_head_commit_sha(&repo_dir)?;
        let tag = get_latest_tag(&repo_dir)?;

        tools.push(SuiteToolEntry {
            name: tool.name.to_string(),
            version,
            commit,
            tag,
        });
    }

    Ok(SuiteLock {
        suite: SuiteMetadata {
            version: suite_version.to_string(),
            created_at: Utc::now().to_rfc3339(),
            meta_harness: "gaugeblock".to_string(),
        },
        tools,
    })
}

/// Writes `SuiteLock` to `suite.lock` at `workspace_root`.
pub fn write_suite_lock(workspace_root: &Path, lock: &SuiteLock) -> Result<PathBuf, OutfeedError> {
    let lock_path = workspace_root.join("suite.lock");
    let content = toml::to_string_pretty(lock).map_err(|e| {
        OutfeedError::Manifest(
            "suite.lock".to_string(),
            format!("Serialization failed: {e}"),
        )
    })?;

    fs::write(&lock_path, content).map_err(|e| {
        OutfeedError::Manifest("suite.lock".to_string(), format!("Write failed: {e}"))
    })?;

    Ok(lock_path)
}
