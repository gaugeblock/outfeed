//! Suite cloning and bootstrap orchestrator.

use std::path::Path;

mod types;

pub use types::{CloneOpts, CloneStatus, CloneSummary, GitProtocol};

use crate::error::OutfeedError;
use crate::install::{SUITE_TOOLS, SuiteTool, install_tool};
use crate::mcp_sync::sync_antigravity_mcp;
use crate::model::InstallOpts;
use crate::progress::{ProgressReporter, RepoStatus};
use crate::suite_lock::SuiteLock;

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

    if let Err(error) = gaugeblock_git::Repository::clone(&url, &target_dir) {
        let message = error.to_string();
        reporter.on_complete(tool.name, &RepoStatus::Failed(message.clone()));
        return Err(OutfeedError::Git(format!(
            "git clone {} failed: {message}",
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
    gaugeblock_git::Repository::discover(repo_dir)
        .and_then(|repo| repo.checkout(commit_sha))
        .map_err(|e| OutfeedError::Git(format!("Failed to checkout {commit_sha}: {e}")))
}
