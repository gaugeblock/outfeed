//! Orchestrated release workflow combining plan, apply, install, push, and publish.

use std::path::Path;

use crate::engine::{apply_release_with_progress, plan_release};
use crate::error::OutfeedError;
use crate::install::{SUITE_TOOLS, install_tool};
use crate::model::{InstallOpts, PublishOpts, PushOpts, ReleaseOpts};
use crate::progress::{ProgressReporter, RepoStatus};
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
    reporter: &dyn ProgressReporter,
) -> Result<Option<ReleaseSummary>, OutfeedError> {
    let repo_dir = if repo_name == "." || repo_name.is_empty() {
        workspace_root.to_path_buf()
    } else {
        let p = Path::new(repo_name);
        if p.is_dir() {
            p.to_path_buf()
        } else {
            workspace_root.join(repo_name)
        }
    };
    if !repo_dir.exists() {
        return Err(OutfeedError::Git(format!(
            "Repository not found: {repo_name}"
        )));
    }

    let actual_name = if repo_name == "." || repo_name.is_empty() {
        repo_dir
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(repo_name)
    } else {
        repo_name
    };

    reporter.on_start(actual_name, "evaluating commits and changelog...");

    // 1. Plan (dry-run uses deterministic formatting to simulate pipeline without remote overhead)
    let no_llm = opts.no_llm || opts.dry_run;
    let plan = match plan_release(&repo_dir, None, false, no_llm).await {
        Ok(p) => p,
        Err(OutfeedError::Git(ref msg)) if msg.contains("No unreleased commits") => {
            reporter.on_complete(
                actual_name,
                &RepoStatus::Skipped("clean (no unreleased commits)".to_string()),
            );
            return Ok(None);
        }
        Err(err) => {
            reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
            return Err(err);
        }
    };
    if plan.commits.is_empty() {
        reporter.on_complete(
            actual_name,
            &RepoStatus::Skipped("clean (no unreleased commits)".to_string()),
        );
        return Ok(None);
    }

    let target_version = plan.target_version.clone();
    reporter.on_update(actual_name, &format!("planning v{target_version}..."));

    // 2. Apply (or simulate if dry_run)
    if opts.dry_run {
        tracing::info!(
            "[DRY RUN] Would release {} v{}",
            actual_name,
            target_version
        );
        reporter.on_complete(
            actual_name,
            &RepoStatus::Success(format!("[dry-run] proposed v{target_version}")),
        );
        return Ok(Some(ReleaseSummary {
            repo_name: actual_name.to_string(),
            target_version,
            installed: !opts.skip_install,
            pushed: !opts.skip_push,
            published_crates: vec!["[simulated]".to_string()],
        }));
    }

    if let Err(err) =
        apply_release_with_progress(&repo_dir, &plan, !opts.no_verify, false, reporter)
    {
        reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
        return Err(err);
    }

    // 3. Install locally
    let mut installed = false;
    if !opts.skip_install
        && let Some(tool) = SUITE_TOOLS.iter().find(|t| t.name == actual_name)
    {
        reporter.on_update(actual_name, "compiling and installing binary...");
        if let Err(err) = install_tool(workspace_root, tool, &InstallOpts::default()) {
            reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
            return Err(err);
        }
        installed = true;
    }

    // 4. Push to Git remote
    let mut pushed = false;
    if !opts.skip_push {
        reporter.on_update(actual_name, "pushing to Git remote...");
        if let Err(err) = push_repo(&repo_dir, &PushOpts::default()) {
            reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
            return Err(err);
        }
        pushed = true;
    }

    // 5. Publish to registry
    let mut published_crates = Vec::new();
    if !opts.skip_publish {
        reporter.on_update(actual_name, "publishing crates to registry...");
        published_crates = match publish_repo(&repo_dir, &PublishOpts::default()) {
            Ok(crates) => crates,
            Err(err) => {
                reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
                return Err(err);
            }
        };
    }

    reporter.on_complete(
        actual_name,
        &RepoStatus::Success(format!("released v{target_version}")),
    );

    Ok(Some(ReleaseSummary {
        repo_name: actual_name.to_string(),
        target_version,
        installed,
        pushed,
        published_crates,
    }))
}

/// Executes the release lifecycle across all suite repositories using the 3-phase pipeline.
pub async fn release_all(
    workspace_root: &Path,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<ReleaseSummary>, OutfeedError> {
    // Phase 1: Pre-flight validation
    crate::suite_check::check_suite_preflight(workspace_root)?;

    // Phase 2: Upfront Suite Release Plan
    let plan = crate::suite_plan::plan_suite_releases(workspace_root, opts.no_llm).await?;

    // Phase 3: Execute Suite Plan
    execute_suite_plan(workspace_root, &plan, opts, reporter).await
}

/// Executes Phase 3 on a pre-compiled SuiteReleasePlan in topological order.
pub async fn execute_suite_plan(
    workspace_root: &Path,
    plan: &crate::suite_plan::SuiteReleasePlan,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<ReleaseSummary>, OutfeedError> {
    let mut summaries = Vec::new();
    for entry in &plan.entries {
        if entry.action == crate::suite_plan::SuitePlanAction::SkipClean {
            continue;
        }
        if let Some(summary) =
            release_repo(workspace_root, &entry.repo_name, opts, reporter).await?
        {
            summaries.push(summary);
        }
    }
    Ok(summaries)
}
