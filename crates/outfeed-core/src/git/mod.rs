//! Git operations and tree inspection utilities.
//!
//! These are the suite's git operations for outfeed. The process spawning now lives in
//! `gaugeblock-git` (`GB-131` step 6), which is the one owner of "spawn git and parse stdout"; this
//! module keeps outfeed's own error types and policy - what counts as a clean tree, which branch a
//! release may run on - and delegates the mechanics.
//!
//! One call intentionally still spawns git: `git add -A` in the release path. `GB-150` is the record of
//! what un-scoped staging costs, and replacing it means deciding what a release commit may contain,
//! which is a behaviour change rather than a refactor. It is tracked rather than quietly kept.

mod history;

use std::path::Path;

use crate::error::OutfeedError;
use gaugeblock_git::Repository;
pub use history::{get_commits_between, get_head_commit_sha, get_latest_tag};

/// Opens the repository at `dir`, mapping the crate error into outfeed's.
fn open(repo_dir: &Path) -> Result<Repository, OutfeedError> {
    Repository::discover(repo_dir).map_err(|error| OutfeedError::Git(format!("{error}")))
}

/// Asserts that the target repository has a completely clean working tree.
pub fn assert_clean_working_tree(repo_dir: &Path) -> Result<(), OutfeedError> {
    let repo = open(repo_dir)?;
    let dirty = repo
        .is_dirty()
        .map_err(|error| OutfeedError::Git(format!("{error}")))?;
    if dirty {
        return Err(OutfeedError::DirtyWorkingTree(
            "working tree has uncommitted changes".to_string(),
        ));
    }
    Ok(())
}

/// Asserts that the target repository (and any parent meta-harness repository) is on 'main' branch.
pub fn assert_main_branch(repo_dir: &Path) -> Result<(), OutfeedError> {
    let branch = get_current_branch(repo_dir)?;
    if branch != "main" {
        return Err(OutfeedError::InvalidBranch(format!(
            "{} is on branch '{branch}'",
            repo_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repository")
        )));
    }

    // If repo is nested inside an enclosing parent git repo (meta-harness workspace root),
    // ensure the parent is also on 'main' and clean.
    let abs_dir = repo_dir
        .canonicalize()
        .unwrap_or_else(|_| repo_dir.to_path_buf());

    if let Some(parent) = abs_dir.parent()
        && parent.join(".git").exists()
    {
        let parent_branch = get_current_branch(parent)?;
        if parent_branch != "main" {
            return Err(OutfeedError::InvalidBranch(format!(
                "workspace root is on branch '{parent_branch}'"
            )));
        }
        assert_clean_working_tree(parent)?;
    }

    Ok(())
}

/// Resolves current git branch name.
///
/// A detached `HEAD` is an error here: a release from a detached head would tag a commit no branch
/// points at, so `None` from the crate becomes a refusal rather than a placeholder string.
pub fn get_current_branch(repo_dir: &Path) -> Result<String, OutfeedError> {
    let repo = open(repo_dir)?;
    match repo
        .current_branch()
        .map_err(|error| OutfeedError::Git(format!("Failed to get current branch: {error}")))?
    {
        Some(branch) => Ok(branch),
        None => Err(OutfeedError::InvalidBranch(
            "HEAD is detached; a release needs a branch".to_string(),
        )),
    }
}

/// Stages `paths`, commits with `message`, and cuts annotated tag `tag` with `tag_message`.
///
/// Shared by the single-repository and suite release paths, which previously carried two
/// near-identical copies of this subprocess sequence.
pub fn stage_commit_and_tag(
    repo_dir: &Path,
    paths: &[&str],
    message: &str,
    tag: &str,
    tag_message: &str,
) -> Result<(), OutfeedError> {
    let repo = open(repo_dir)?;

    // Deliberate exception: `-A` means "stage everything", which is the hazard `GB-150` records. It is
    // kept because deciding what a release commit may contain is a behaviour change, not a refactor,
    // and it is tracked as such rather than migrated silently.
    // Through `gaugeblock-git` for the same reason as the log call: only that crate spawns git, and
    // this one had the dependency without using it for the command (`GB-305`).
    let mut add_args = vec!["add"];
    add_args.extend_from_slice(paths);
    gaugeblock_git::run_git(repo_dir, &add_args)
        .map_err(|e| OutfeedError::Git(format!("git add failed: {e}")))?;

    repo.commit(message)
        .map_err(|error| OutfeedError::Git(format!("git commit failed: {error}")))?;
    repo.tag_annotated(tag, tag_message)
        .map_err(|error| OutfeedError::Git(format!("git tag failed: {error}")))?;
    Ok(())
}

/// Creates a release commit and annotated git tag.
pub fn create_release_commit_and_tag(repo_dir: &Path, version: &str) -> Result<(), OutfeedError> {
    let tag = format!("v{version}");
    stage_commit_and_tag(
        repo_dir,
        &["-A"],
        &format!("chore(release): release v{version}"),
        &tag,
        &tag,
    )
}
