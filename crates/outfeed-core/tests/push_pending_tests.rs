//! Regression tests for GB-153: a release pass must not leave a clean repository unpushed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use outfeed_core::push::is_ahead_of_remote;
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(
        status.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&status.stderr)
    );
}

/// Creates a repository with one commit and returns its tempdir.
fn repo_with_commit() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    git(tmp.path(), &["init", "--initial-branch=main"]);
    git(tmp.path(), &["config", "user.email", "test@gaugeblock.dev"]);
    git(tmp.path(), &["config", "user.name", "Test"]);
    std::fs::write(tmp.path().join("README.md"), "hello\n").expect("write");
    git(tmp.path(), &["add", "README.md"]);
    git(tmp.path(), &["commit", "-m", "chore: init"]);
    tmp
}

#[test]
fn a_repository_with_no_remote_cannot_be_ahead() {
    let repo = repo_with_commit();
    assert!(
        !is_ahead_of_remote(repo.path()).expect("query"),
        "a repository with no remote has nowhere to push, so it is not pending"
    );
}

#[test]
fn a_branch_that_was_never_pushed_is_ahead() {
    let repo = repo_with_commit();
    let remote = tempfile::tempdir().expect("remote");
    git(remote.path(), &["init", "--bare", "--initial-branch=main"]);
    git(
        repo.path(),
        &[
            "remote",
            "add",
            "origin",
            remote.path().to_str().expect("utf8"),
        ],
    );

    assert!(
        is_ahead_of_remote(repo.path()).expect("query"),
        "a configured remote with no upstream ref means the commits exist only locally"
    );
}

#[test]
fn a_pushed_branch_is_not_ahead_until_local_moves() {
    let repo = repo_with_commit();
    let remote = tempfile::tempdir().expect("remote");
    git(remote.path(), &["init", "--bare", "--initial-branch=main"]);
    git(
        repo.path(),
        &[
            "remote",
            "add",
            "origin",
            remote.path().to_str().expect("utf8"),
        ],
    );
    git(repo.path(), &["push", "-u", "origin", "main"]);
    assert!(
        !is_ahead_of_remote(repo.path()).expect("query"),
        "a fully pushed branch is not pending work"
    );

    std::fs::write(repo.path().join("NOTES.md"), "more\n").expect("write");
    git(repo.path(), &["add", "NOTES.md"]);
    git(repo.path(), &["commit", "-m", "docs: note"]);
    assert!(
        is_ahead_of_remote(repo.path()).expect("query"),
        "a local commit the remote lacks is pending work - the case a release pass used to skip"
    );
}
