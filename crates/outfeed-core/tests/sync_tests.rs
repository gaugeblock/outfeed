#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Integration tests for workspace sync and audit.

use std::process::Command;

use outfeed_core::progress::NoopProgress;
use outfeed_core::sync::{audit_repo_sync, sync_suite, sync_tool};
use outfeed_core::sync_model::{BranchDivergence, SyncOpts, is_working_tree_dirty};
use tempfile::tempdir;

fn init_dummy_git_repo(path: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.name", "Tester"])
        .current_dir(path)
        .output()
        .expect("git config user.name failed");

    Command::new("git")
        .args(["config", "user.email", "tester@example.com"])
        .current_dir(path)
        .output()
        .expect("git config user.email failed");

    std::fs::write(path.join("README.md"), "# Test\n").expect("write failed");

    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(path)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(path)
        .output()
        .expect("git commit failed");
}

#[test]
fn test_is_working_tree_dirty_detection() {
    let temp = tempdir().expect("failed to create tempdir");
    init_dummy_git_repo(temp.path());

    assert!(!is_working_tree_dirty(temp.path()).expect("dirty check failed"));

    // Modify file
    std::fs::write(temp.path().join("README.md"), "# Modified\n").expect("write failed");
    assert!(is_working_tree_dirty(temp.path()).expect("dirty check failed"));
}

#[test]
fn test_audit_repo_sync_clean_repo() {
    let temp = tempdir().expect("failed to create tempdir");
    init_dummy_git_repo(temp.path());

    let status = audit_repo_sync(temp.path()).expect("audit failed");
    assert_eq!(status.branch, "main");
    assert!(!status.dirty);
    assert_eq!(status.divergence, BranchDivergence::Untracked);
}

#[test]
fn test_sync_tool_status_only_mode() {
    let temp = tempdir().expect("failed to create tempdir");
    let miter_dir = temp.path().join("miter");
    std::fs::create_dir(&miter_dir).expect("create dir failed");
    init_dummy_git_repo(&miter_dir);

    let tool = outfeed_core::install::SUITE_TOOLS[0]; // miter
    let opts = SyncOpts {
        status_only: true,
        ..Default::default()
    };

    let status = sync_tool(temp.path(), &tool, &opts, &NoopProgress).expect("sync_tool failed");
    assert_eq!(status.repo, "miter");
    assert_eq!(status.branch, "main");
    assert!(status.action_taken.is_none());
}

#[test]
fn test_sync_tool_dirty_skipped() {
    let temp = tempdir().expect("failed to create tempdir");
    let miter_dir = temp.path().join("miter");
    std::fs::create_dir(&miter_dir).expect("create dir failed");
    init_dummy_git_repo(&miter_dir);

    // Make dirty
    std::fs::write(miter_dir.join("README.md"), "# Dirty\n").expect("write failed");

    let tool = outfeed_core::install::SUITE_TOOLS[0];
    let opts = SyncOpts {
        status_only: false,
        ..Default::default()
    };

    let status = sync_tool(temp.path(), &tool, &opts, &NoopProgress).expect("sync_tool failed");
    assert!(status.dirty);
    assert!(
        status
            .action_taken
            .unwrap()
            .contains("dirty working tree; skipping sync")
    );
}

#[test]
fn test_sync_suite_empty_or_dry_run() {
    let temp = tempdir().expect("failed to create tempdir");
    let opts = SyncOpts {
        all: true,
        status_only: true,
        ..Default::default()
    };

    let statuses = sync_suite(temp.path(), &[], &opts, &NoopProgress).expect("sync_suite failed");
    assert!(statuses.is_empty());
}
