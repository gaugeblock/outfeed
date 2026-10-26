//! Meta-harness suite release orchestrator.

use std::fs;
use std::path::Path;
use std::process::Command;

use chrono::Utc;

use crate::error::OutfeedError;
use crate::git::{assert_clean_working_tree, assert_main_branch, get_latest_tag};
use crate::model::BumpType;
use crate::push::push_repo;
use crate::semver::apply_bump;
use crate::suite_lock::{
    SuiteReleaseOpts, SuiteReleaseSummary, SuiteToolEntry, generate_suite_lock, write_suite_lock,
};

/// Determines target suite version from explicit override or latest `suite-v*` git tag.
pub fn determine_suite_version(
    workspace_root: &Path,
    override_ver: Option<&str>,
) -> Result<String, OutfeedError> {
    if let Some(v) = override_ver {
        return Ok(v.trim_start_matches('v').to_string());
    }

    match get_latest_tag(workspace_root)? {
        Some(tag) => {
            let bare_version = tag.trim_start_matches("suite-v").trim_start_matches('v');
            apply_bump(bare_version, BumpType::PreReleasePatch)
        }
        None => Ok("0.1.0-beta.1".to_string()),
    }
}

/// Inserts a release section into root `CHANGELOG.md` if not already present.
pub fn update_root_changelog(
    workspace_root: &Path,
    version: &str,
    tools: &[SuiteToolEntry],
) -> Result<bool, OutfeedError> {
    let changelog_path = workspace_root.join("CHANGELOG.md");
    if !changelog_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&changelog_path)
        .map_err(|e| OutfeedError::Changelog(format!("Failed to read CHANGELOG.md: {e}")))?;

    let header_needle = format!("## [{version}]");
    if content.contains(&header_needle) {
        return Ok(false);
    }

    let mut tool_lines = String::new();
    for t in tools {
        tool_lines.push_str(&format!("  - `{}`: v{}\n", t.name, t.version));
    }

    let section = format!(
        "## [{version}] - {}\n\n### Synchronized Suite Release\n- Ecosystem release snapshot pinned in `suite.lock`:\n{tool_lines}\n---\n\n",
        Utc::now().format("%Y-%m-%d")
    );

    let updated = if let Some(pos) = content.find("## [") {
        format!("{}{}{}", &content[..pos], section, &content[pos..])
    } else {
        format!("{content}\n\n{section}")
    };

    fs::write(&changelog_path, updated)
        .map_err(|e| OutfeedError::Changelog(format!("Failed to update CHANGELOG.md: {e}")))?;

    Ok(true)
}

/// Orchestrates full suite release: generates `suite.lock`, updates changelog, commits, and tags.
pub fn release_suite(
    workspace_root: &Path,
    opts: &SuiteReleaseOpts,
) -> Result<SuiteReleaseSummary, OutfeedError> {
    if !opts.dry_run {
        assert_clean_working_tree(workspace_root)?;
        assert_main_branch(workspace_root)?;
    }

    let target_version = determine_suite_version(workspace_root, opts.target_version.as_deref())?;
    let tag = format!("suite-v{target_version}");
    let lock = generate_suite_lock(workspace_root, &target_version)?;
    let tool_count = lock.tools.len();
    let lock_path = workspace_root.join("suite.lock");

    if opts.dry_run {
        tracing::info!("[DRY RUN] Would write suite.lock for {tag} with {tool_count} tools");
        return Ok(SuiteReleaseSummary {
            target_version,
            tag,
            tool_count,
            lock_path: lock_path.to_string_lossy().to_string(),
            committed: false,
            pushed: false,
        });
    }

    write_suite_lock(workspace_root, &lock)?;
    let _ = update_root_changelog(workspace_root, &target_version, &lock.tools)?;

    commit_and_tag_suite(workspace_root, &target_version, &tag)?;

    let mut pushed = false;
    if !opts.skip_push {
        push_repo(workspace_root, &crate::model::PushOpts::default())?;
        pushed = true;
    }

    Ok(SuiteReleaseSummary {
        target_version,
        tag,
        tool_count,
        lock_path: lock_path.to_string_lossy().to_string(),
        committed: true,
        pushed,
    })
}

fn commit_and_tag_suite(
    workspace_root: &Path,
    version: &str,
    tag: &str,
) -> Result<(), OutfeedError> {
    let commit_msg = format!("chore(release): release suite-v{version}");

    let add = Command::new("git")
        .args(["add", "suite.lock", "CHANGELOG.md"])
        .current_dir(workspace_root)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to stage suite release files: {e}")))?;

    if !add.status.success() {
        return Err(OutfeedError::Git("Failed to stage suite.lock".to_string()));
    }

    let commit = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(workspace_root)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to commit suite release: {e}")))?;

    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr);
        return Err(OutfeedError::Git(format!("Suite commit failed: {stderr}")));
    }

    let tag_cmd = Command::new("git")
        .args(["tag", "-a", tag, "-m", &format!("{tag} ecosystem release")])
        .current_dir(workspace_root)
        .output()
        .map_err(|e| OutfeedError::Git(format!("Failed to create suite tag: {e}")))?;

    if !tag_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&tag_cmd.stderr);
        return Err(OutfeedError::Git(format!("Suite tag failed: {stderr}")));
    }

    Ok(())
}
