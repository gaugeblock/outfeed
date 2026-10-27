//! Crates.io publication engine with topological sorting and index propagation delay.

use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use crate::cmd::run_captured;
use crate::error::OutfeedError;
use crate::model::PublishOpts;
use crate::progress::{NoopProgress, ProgressReporter, RepoStatus};

pub use crate::publish_order::{MemberCrate, resolve_publish_order};

/// Publishes all crates within a repository workspace in topological order using default reporter.
pub fn publish_repo(repo_dir: &Path, opts: &PublishOpts) -> Result<Vec<String>, OutfeedError> {
    publish_repo_with_progress(repo_dir, opts, &NoopProgress)
}

/// Publishes all crates within a repository workspace with progress reporting.
pub fn publish_repo_with_progress(
    repo_dir: &Path,
    opts: &PublishOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<String>, OutfeedError> {
    let order = resolve_publish_order(repo_dir)?;
    let mut published = Vec::new();

    // 1. Dry run gate across all crates
    for member in &order {
        reporter.on_start(&member.name, "validating publish dry-run");
        let mut cmd = Command::new("cargo");
        cmd.arg("publish")
            .arg("--dry-run")
            .arg("-p")
            .arg(&member.name)
            .current_dir(repo_dir);
        if let Some(ref reg) = opts.registry {
            cmd.arg("--registry").arg(reg);
        }
        let out = run_captured(&mut cmd)?;
        if !out.success {
            let err_text = out.combined_output();
            if err_text.contains("is already uploaded")
                || err_text.contains("already uploaded")
                || err_text.contains("already exists on crates.io index")
            {
                reporter.on_complete(
                    &member.name,
                    &RepoStatus::Skipped("already published".to_string()),
                );
                continue;
            }
            reporter.on_complete(&member.name, &RepoStatus::Failed(err_text.clone()));
            return Err(OutfeedError::Publish(format!(
                "cargo publish --dry-run failed for {}:\n{err_text}",
                member.name
            )));
        }
        reporter.on_complete(
            &member.name,
            &RepoStatus::Success("dry-run verified".to_string()),
        );
    }

    if opts.dry_run {
        return Ok(order.into_iter().map(|m| m.name).collect());
    }

    // 2. Real publication with index delay
    let total = order.len();
    for (idx, member) in order.into_iter().enumerate() {
        reporter.on_start(&member.name, "publishing to registry");
        let mut cmd = Command::new("cargo");
        cmd.arg("publish")
            .arg("-p")
            .arg(&member.name)
            .current_dir(repo_dir);
        if let Some(ref reg) = opts.registry {
            cmd.arg("--registry").arg(reg);
        }
        let out = run_captured(&mut cmd)?;
        if !out.success {
            let text = out.combined_output();
            if text.contains("is already uploaded")
                || text.contains("already uploaded")
                || text.contains("already exists on crates.io index")
            {
                reporter.on_complete(
                    &member.name,
                    &RepoStatus::Skipped("already published".to_string()),
                );
                published.push(member.name);
                continue;
            }
            reporter.on_complete(&member.name, &RepoStatus::Failed(text.clone()));
            return Err(OutfeedError::Publish(format!(
                "cargo publish failed for {}:\n{text}",
                member.name
            )));
        }
        reporter.on_complete(&member.name, &RepoStatus::Success("published".to_string()));
        published.push(member.name.clone());

        // Delay between dependent crates if not the last crate
        if idx + 1 < total && opts.delay_secs > 0 {
            reporter.on_update(
                &member.name,
                &format!("waiting {}s for index propagation", opts.delay_secs),
            );
            sleep(Duration::from_secs(opts.delay_secs));
        }
    }

    Ok(published)
}

/// Publishes all member repositories in topological order using default reporter.
pub fn publish_all(
    workspace_root: &Path,
    opts: &PublishOpts,
) -> Result<Vec<(String, Vec<String>)>, OutfeedError> {
    publish_all_with_progress(workspace_root, opts, &NoopProgress)
}

/// Publishes all member repositories in topological order with progress reporting.
pub fn publish_all_with_progress(
    workspace_root: &Path,
    opts: &PublishOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<(String, Vec<String>)>, OutfeedError> {
    let mut results = Vec::new();
    for tool in crate::install::SUITE_TOOLS {
        let repo_dir = workspace_root.join(tool.name);
        if !repo_dir.is_dir() {
            continue;
        }
        let published = publish_repo_with_progress(&repo_dir, opts, reporter)?;
        results.push((tool.name.to_string(), published));
    }
    Ok(results)
}
