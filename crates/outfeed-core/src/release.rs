//! Orchestrated release workflow combining plan, apply, install, push, and publish.

use std::path::Path;

use crate::engine::{apply_release, plan_release};
use crate::error::OutfeedError;
use crate::install::{install_tool, SUITE_TOOLS};
use crate::model::{InstallOpts, PushOpts, PublishOpts, ReleaseOpts};
use crate::publish::publish_repo;
use crate::push::push_repo;

/// Summary of orchestrated release execution.
#[derive(Debug, Clone)]
pub struct ReleaseSummary {
    /// Target repository name.
    pub repo_name: String,
    /// Final released version.
    pub target_version: String,
    /// Whether the binary was installed locally.
    pub installed: bool,
    /// Whether commits and tags were pushed to Git remote.
    pub pushed: bool,
    /// List of crates published to registry.
    pub published_crates: Vec<String>,
}

/// Executes the full release lifecycle on a repository.
pub async fn release_repo(
    workspace_root: &Path,
    repo_name: &str,
    opts: &ReleaseOpts,
) -> Result<Option<ReleaseSummary>, OutfeedError> {
    let repo_dir = workspace_root.join(repo_name);
    if !repo_dir.exists() {
        return Err(OutfeedError::Git(format!("Repository not found: {repo_name}")));
    }

    // 1. Plan
    let plan = plan_release(&repo_dir, None, false).await?;
    if plan.commits.is_empty() {
        return Ok(None);
    }

    let target_version = plan.target_version.clone();

    // 2. Apply (or simulate if dry_run)
    if opts.dry_run {
        tracing::info!("[DRY RUN] Would release {} v{}", repo_name, target_version);
        return Ok(Some(ReleaseSummary {
            repo_name: repo_name.to_string(),
            target_version,
            installed: !opts.skip_install,
            pushed: !opts.skip_push,
            published_crates: vec!["[simulated]".to_string()],
        }));
    }

    apply_release(&repo_dir, &plan, true, false)?;

    // 3. Install locally
    let mut installed = false;
    if !opts.skip_install
        && let Some(tool) = SUITE_TOOLS.iter().find(|t| t.name == repo_name)
    {
        install_tool(workspace_root, tool, &InstallOpts::default())?;
        installed = true;
    }

    // 4. Push to Git remote
    let mut pushed = false;
    if !opts.skip_push {
        push_repo(&repo_dir, &PushOpts::default())?;
        pushed = true;
    }

    // 5. Publish to registry
    let mut published_crates = Vec::new();
    if !opts.skip_publish {
        published_crates = publish_repo(&repo_dir, &PublishOpts::default())?;
    }

    Ok(Some(ReleaseSummary {
        repo_name: repo_name.to_string(),
        target_version,
        installed,
        pushed,
        published_crates,
    }))
}
