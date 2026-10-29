//! Orchestrated release workflow combining plan, apply, install, push, and publish.

use std::path::{Path, PathBuf};

mod pipeline;

use crate::engine::plan_release;
use crate::error::OutfeedError;
use crate::model::{ReleaseOpts, ReleasePlan};
use crate::progress::{ProgressReporter, RepoStatus};
use pipeline::finish_planned_release;

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
    let (repo_dir, actual_name) = resolve_repo(workspace_root, repo_name)?;

    reporter.on_start(&actual_name, "evaluating commits and changelog...");

    // 1. Plan (dry-run uses deterministic formatting to simulate pipeline without remote overhead)
    let no_llm = opts.no_llm || opts.dry_run;
    guard_disk_headroom(workspace_root, opts)?;
    // Idempotency: an override naming a version that already carries a tag is a completed release,
    // so a second pass is a clean no-op rather than a duplicate tag attempt.
    if let Some(version) = opts.override_version.as_deref() {
        let tag = format!("v{}", version.trim_start_matches('v'));
        if crate::git::get_latest_tag(&repo_dir)?.as_deref() == Some(tag.as_str()) {
            reporter.on_complete(
                &actual_name,
                &RepoStatus::Skipped(format!("already released as {tag}")),
            );
            return Ok(None);
        }
    }

    let plan = match plan_release(&repo_dir, opts.override_version.as_deref(), false, no_llm).await
    {
        Ok(p) => p,
        Err(OutfeedError::Git(ref msg)) if msg.contains("No unreleased commits") => {
            reporter.on_complete(
                &actual_name,
                &RepoStatus::Skipped("clean (no unreleased commits)".to_string()),
            );
            return Ok(None);
        }
        Err(err) => {
            reporter.on_complete(&actual_name, &RepoStatus::Failed(err.to_string()));
            return Err(err);
        }
    };
    if plan.commits.is_empty() {
        reporter.on_complete(
            &actual_name,
            &RepoStatus::Skipped("clean (no unreleased commits)".to_string()),
        );
        return Ok(None);
    }

    finish_planned_release(
        workspace_root,
        &repo_dir,
        &actual_name,
        &plan,
        opts,
        reporter,
    )
    .map(Some)
}

/// Resolves a repository directory and its display name from a plan entry.
fn resolve_repo(workspace_root: &Path, repo_name: &str) -> Result<(PathBuf, String), OutfeedError> {
    let repo_dir = if repo_name == "." || repo_name.is_empty() {
        workspace_root.to_path_buf()
    } else {
        let candidate = Path::new(repo_name);
        if candidate.is_dir() {
            candidate.to_path_buf()
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
            .to_string()
    } else {
        repo_name.to_string()
    };
    Ok((repo_dir, actual_name))
}

/// Applies the single-repository disk headroom pre-flight unless it is disabled.
fn guard_disk_headroom(workspace_root: &Path, opts: &ReleaseOpts) -> Result<(), OutfeedError> {
    if opts.skip_disk_check {
        return Ok(());
    }
    let required = opts
        .min_disk_bytes
        .unwrap_or(crate::disk_space::DEFAULT_MIN_SINGLE_REPO_DISK_BYTES);
    crate::disk_space::check_disk_space_headroom(workspace_root, required)?;
    Ok(())
}

/// Releases a repository from a plan that Phase 2 already computed.
///
/// Reusing the suite plan avoids a second `git log` walk and a second changelog synthesis for
/// every repository in `outfeed release --all`.
pub async fn release_repo_with_plan(
    workspace_root: &Path,
    repo_name: &str,
    plan: &ReleasePlan,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Option<ReleaseSummary>, OutfeedError> {
    let (repo_dir, actual_name) = resolve_repo(workspace_root, repo_name)?;
    reporter.on_start(&actual_name, "reusing suite plan (no re-planning)...");
    guard_disk_headroom(workspace_root, opts)?;
    finish_planned_release(
        workspace_root,
        &repo_dir,
        &actual_name,
        plan,
        opts,
        reporter,
    )
    .map(Some)
}

pub use crate::suite_exec::{execute_suite_plan, release_all};
