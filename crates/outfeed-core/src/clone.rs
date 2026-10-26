//! Suite cloning and bootstrap orchestrator.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::OutfeedError;
use crate::install::{SUITE_TOOLS, SuiteTool, install_tool};
use crate::mcp_sync::sync_antigravity_mcp;
use crate::model::InstallOpts;
use crate::progress::{ProgressReporter, RepoStatus};
use crate::suite_lock::SuiteLock;

/// Git protocol used for cloning suite repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GitProtocol {
    /// Secure Shell (git@github.com:gaugeblock/{repo}.git)
    #[default]
    Ssh,
    /// HTTPS (https://github.com/gaugeblock/{repo}.git)
    Https,
}

impl GitProtocol {
    /// Formats the clone remote URL for a given repository.
    pub fn url_for(&self, repo_name: &str) -> String {
        match self {
            GitProtocol::Ssh => format!("git@github.com:gaugeblock/{repo_name}.git"),
            GitProtocol::Https => format!("https://github.com/gaugeblock/{repo_name}.git"),
        }
    }
}

/// Status of a repository clone operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloneStatus {
    /// Repository cloned freshly from remote.
    Cloned,
    /// Existing local checkout detected and preserved.
    SkippedExisting,
    /// Aligned to exact commit SHA pinned in `suite.lock`.
    PinnedCommit(String),
}

/// Summary result for a single repository clone operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneSummary {
    /// Repository name.
    pub repo: String,
    /// Destination directory path.
    pub target_dir: PathBuf,
    /// Outcome of the clone operation.
    pub status: CloneStatus,
}

/// Execution options for suite clone commands.
#[derive(Debug, Clone, Default)]
pub struct CloneOpts {
    /// Clone all 10 suite repositories.
    pub all: bool,
    /// Pin checkouts to exact commit SHAs recorded in `suite.lock`.
    pub locked: bool,
    /// Remote git protocol to use.
    pub protocol: GitProtocol,
    /// Simulate clone operations without modifying disk.
    pub dry_run: bool,
    /// Automatically compile and install binaries to PATH.
    pub install: bool,
    /// Synchronize Antigravity and Cline MCP configuration files.
    pub sync_mcp: bool,
}

/// Clones a single member repository or aligns an existing checkout.
pub fn clone_tool(
    workspace_root: &Path,
    tool: &SuiteTool,
    opts: &CloneOpts,
    reporter: &dyn ProgressReporter,
) -> Result<CloneSummary, OutfeedError> {
    let target_dir = workspace_root.join(tool.name);
    reporter.on_start(tool.name, "cloning");

    if target_dir.exists() {
        if opts.locked
            && let Some(commit) = read_locked_commit(workspace_root, tool.name)
        {
            if !opts.dry_run {
                checkout_commit(&target_dir, &commit)?;
            }
            reporter.on_complete(
                tool.name,
                &RepoStatus::Success(format!("pinned to {commit}")),
            );
            return Ok(CloneSummary {
                repo: tool.name.to_string(),
                target_dir,
                status: CloneStatus::PinnedCommit(commit),
            });
        }

        reporter.on_complete(
            tool.name,
            &RepoStatus::Skipped("existing checkout preserved".to_string()),
        );
        return Ok(CloneSummary {
            repo: tool.name.to_string(),
            target_dir,
            status: CloneStatus::SkippedExisting,
        });
    }

    let url = opts.protocol.url_for(tool.name);
    if opts.dry_run {
        reporter.on_complete(
            tool.name,
            &RepoStatus::Success("simulated clone".to_string()),
        );
        return Ok(CloneSummary {
            repo: tool.name.to_string(),
            target_dir,
            status: CloneStatus::Cloned,
        });
    }

    let output = Command::new("git")
        .args(["clone", &url])
        .arg(&target_dir)
        .output()
        .map_err(|e| {
            OutfeedError::Git(format!(
                "Failed to execute git clone for {}: {e}",
                tool.name
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        reporter.on_complete(tool.name, &RepoStatus::Failed(stderr.to_string()));
        return Err(OutfeedError::Git(format!(
            "git clone {} failed: {stderr}",
            tool.name
        )));
    }

    if opts.locked
        && let Some(commit) = read_locked_commit(workspace_root, tool.name)
    {
        checkout_commit(&target_dir, &commit)?;
        reporter.on_complete(
            tool.name,
            &RepoStatus::Success(format!("pinned to {commit}")),
        );
        return Ok(CloneSummary {
            repo: tool.name.to_string(),
            target_dir,
            status: CloneStatus::PinnedCommit(commit),
        });
    }

    reporter.on_complete(tool.name, &RepoStatus::Success("cloned".to_string()));
    Ok(CloneSummary {
        repo: tool.name.to_string(),
        target_dir,
        status: CloneStatus::Cloned,
    })
}

/// Orchestrates cloning across suite repositories.
pub fn clone_suite(
    workspace_root: &Path,
    targets: &[&str],
    opts: &CloneOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<CloneSummary>, OutfeedError> {
    if targets.is_empty() && !opts.all {
        return Err(OutfeedError::Git(
            "Specify repository name(s) or use --all to clone all suite repositories".to_string(),
        ));
    }

    let selected_tools: Vec<_> = if opts.all {
        SUITE_TOOLS.to_vec()
    } else {
        SUITE_TOOLS
            .iter()
            .copied()
            .filter(|t| targets.contains(&t.name))
            .collect()
    };

    let mut summaries = Vec::new();
    for tool in &selected_tools {
        summaries.push(clone_tool(workspace_root, tool, opts, reporter)?);
    }

    if opts.install && !opts.dry_run {
        for tool in &selected_tools {
            let _ = install_tool(workspace_root, tool, &InstallOpts::default());
        }
    }

    if opts.sync_mcp && !opts.dry_run {
        let _ = sync_antigravity_mcp(workspace_root);
    }

    Ok(summaries)
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
