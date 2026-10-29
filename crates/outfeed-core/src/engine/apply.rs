//! Applying a computed release plan: manifest rewrites, verification, and tagging.
//!
//! Split out of `engine::mod` (GB-139): the planner decides *what* to release, this module carries
//! it out.

use std::path::Path;
use std::process::Command;

use crate::error::OutfeedError;
use crate::git::{assert_clean_working_tree, assert_main_branch, create_release_commit_and_tag};
use crate::manifest::{apply_manifest_diffs, rollback_manifest_diffs};
use crate::model::ReleasePlan;
use crate::progress::{NoopProgress, ProgressReporter};
use crate::storypole::{certify_release_ticket, create_release_ticket, find_storypole_root};
use crate::verify_runner::{TestRunner, run_workspace_tests};

/// Executes a release plan with transactional safety and verification gating.
pub fn apply_release(
    repo_dir: &Path,
    plan: &ReleasePlan,
    verify_tier4: bool,
    skip_storypole: bool,
) -> Result<(), OutfeedError> {
    apply_release_with_progress(repo_dir, plan, verify_tier4, skip_storypole, &NoopProgress)
}

/// Executes a release plan with granular progress events and verification gating.
pub fn apply_release_with_progress(
    repo_dir: &Path,
    plan: &ReleasePlan,
    verify_tier4: bool,
    skip_storypole: bool,
    reporter: &dyn ProgressReporter,
) -> Result<(), OutfeedError> {
    assert_clean_working_tree(repo_dir)?;
    assert_main_branch(repo_dir)?;

    let repo_name = &plan.repo_name;

    // 1. Write manifests & changelog
    reporter.on_update(repo_name, "updating manifests & changelog...");
    apply_manifest_diffs(repo_dir, &plan.manifest_diffs)?;

    // 2. Synchronize Cargo.lock with the bumped manifests (no compilation)
    reporter.on_update(repo_name, "synchronizing Cargo.lock...");
    if let Err(err) = sync_cargo_lock(repo_dir) {
        let _ = rollback_manifest_diffs(repo_dir, &plan.manifest_diffs);
        return Err(err);
    }

    // 3. Optional Tier 4 verification (captured to prevent terminal corruption)
    if verify_tier4
        && let Err(err) = run_workspace_tests(repo_dir, TestRunner::detect(), repo_name, reporter)
    {
        let _ = rollback_manifest_diffs(repo_dir, &plan.manifest_diffs);
        return Err(err);
    }

    // 4. Closed-loop Storypole integration
    if !skip_storypole && let Some(sp_root) = find_storypole_root(repo_dir) {
        reporter.on_update(repo_name, "certifying release in Storypole...");
        if let Ok(ticket_key) = create_release_ticket(
            &sp_root,
            &plan.repo_name,
            &plan.target_version,
            &plan.bump_rationale,
            &plan.referenced_tickets,
        ) {
            let _ = certify_release_ticket(&sp_root, &ticket_key, &plan.target_version);
        }
    }

    // 5. Git commit & tag
    reporter.on_update(
        repo_name,
        &format!("committing and tagging v{}...", plan.target_version),
    );
    create_release_commit_and_tag(repo_dir, &plan.target_version)?;

    Ok(())
}

/// Synchronizes `Cargo.lock` with the freshly bumped manifests without compiling.
///
/// `cargo update --workspace --offline` rewrites the lockfile entries for workspace members only,
/// which is all a version bump needs. If the offline registry cache is unavailable it falls back to
/// a `cargo check --workspace` resolve so an offline machine can still cut a release.
fn sync_cargo_lock(repo_dir: &Path) -> Result<(), OutfeedError> {
    let mut update = Command::new("cargo");
    update
        .args(["update", "--workspace", "--offline"])
        .current_dir(repo_dir);
    if let Ok(output) = update.output()
        && output.status.success()
    {
        return Ok(());
    }

    let mut check = Command::new("cargo");
    check.args(["check", "--workspace"]).current_dir(repo_dir);
    let output = check.output().map_err(OutfeedError::Io)?;
    if output.status.success() {
        return Ok(());
    }

    Err(OutfeedError::Manifest(
        "Cargo.lock".to_string(),
        format!(
            "Cargo.lock synchronization failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    ))
}
