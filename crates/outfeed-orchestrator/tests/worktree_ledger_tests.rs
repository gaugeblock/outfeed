//! A linked worktree must resolve to the same ledger as its main checkout (`GB-305`).
//!
//! This exists because the fix in `GB-304` was behavioural and no test in the suite could see it. The old
//! `find_storypole_root` walked upwards for the nearest `.gaugeblock`, which from inside a worktree finds
//! nothing at all - the directory lives in the main checkout, and the walk goes up past a sibling rather
//! than across to it. The consequence was silent: a release run from a worktree simply recorded no ticket.
//!
//! A test with one repository in scope cannot distinguish the two implementations. This one creates two.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use outfeed_orchestrator::backlog::find_storypole_root;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

/// Runs git in `dir`, panicking with stdout on failure - a silently broken fixture is worse than a failing
/// test, because it makes the assertion look like a product defect.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git spawns");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A repository with one commit and a storypole ledger in its `.gaugeblock`.
fn repo_with_ledger(root: &Path) {
    fs::create_dir_all(root).expect("create repo dir");
    git(root, &["init", "-b", "main"]);
    git(root, &["config", "user.name", "Test User"]);
    git(root, &["config", "user.email", "test@example.com"]);
    fs::write(root.join("README.md"), "# Suite Root\n").expect("write readme");
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "chore: init suite root"]);

    let ledger_dir = root.join(".gaugeblock").join("storypole");
    fs::create_dir_all(&ledger_dir).expect("create ledger dir");
    fs::write(ledger_dir.join("ledger.jsonl"), "").expect("write ledger");
}

#[test]
fn the_main_checkout_resolves_to_its_own_ledger() {
    let tmp = tempdir().expect("tempdir");
    let main = tmp.path().join("main");
    repo_with_ledger(&main);

    let resolved = find_storypole_root(&main).expect("the main checkout has a ledger");
    assert_eq!(
        std::fs::canonicalize(resolved).unwrap(),
        std::fs::canonicalize(&main).unwrap()
    );
}

#[test]
fn a_linked_worktree_resolves_to_the_main_checkout() {
    let tmp = tempdir().expect("tempdir");
    let main = tmp.path().join("main");
    let worktree = tmp.path().join("wt");
    repo_with_ledger(&main);

    git(
        &main,
        &[
            "worktree",
            "add",
            &worktree.to_string_lossy(),
            "-b",
            "feature",
        ],
    );

    // The worktree has no `.gaugeblock` of its own - that is the whole point. An upward walk from here
    // finds nothing, which is how a release run in a worktree came to record no ticket at all.
    assert!(!worktree.join(".gaugeblock").exists());

    let from_worktree = find_storypole_root(&worktree)
        .expect("a worktree must find the main checkout's ledger, not nothing");
    assert_eq!(
        std::fs::canonicalize(from_worktree).unwrap(),
        std::fs::canonicalize(&main).unwrap(),
        "the ledger belongs to the main checkout, and a worktree must not grow a second one"
    );
}
