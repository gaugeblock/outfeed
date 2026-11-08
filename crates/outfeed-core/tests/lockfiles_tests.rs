//! Integration tests for the `outfeed lockfiles` hygiene check (GB-152).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::Command;

use outfeed_core::LockfileReport;
use tempfile::tempdir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git run");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
}

fn init_repo(dir: &Path) {
    fs::create_dir_all(dir.join("src")).expect("create repo");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    fs::write(dir.join("src").join("lib.rs"), "//! probe\n").expect("lib");
    git(dir, &["init", "-q", "-b", "main"]);
    let ok = Command::new("cargo")
        .arg("generate-lockfile")
        .current_dir(dir)
        .output()
        .expect("generate lockfile");
    assert!(ok.status.success(), "generate-lockfile failed: {ok:?}");
}

#[test]
fn test_reports_tracked_and_untracked_repositories() {
    let root = tempdir().expect("tempdir");

    // A repository that tracks its lockfile.
    let good = root.path().join("good");
    init_repo(&good);
    git(
        &good,
        &["-c", "user.name=t", "-c", "user.email=t@e", "add", "."],
    );

    // A repository that ignores it.
    let bad = root.path().join("bad");
    init_repo(&bad);
    fs::write(bad.join(".gitignore"), "Cargo.lock\n").expect("gitignore");
    git(
        &bad,
        &["-c", "user.name=t", "-c", "user.email=t@e", "add", "."],
    );

    // A directory that is not a standalone repository is ignored entirely.
    let plain = root.path().join("plain");
    fs::create_dir_all(&plain).expect("plain dir");
    fs::write(plain.join("Cargo.toml"), "[workspace]\n").expect("plain manifest");

    let report: LockfileReport = outfeed_core::check_lockfiles(root.path()).expect("check");
    let names: Vec<&str> = report
        .repositories
        .iter()
        .map(|r| r.repo.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["bad", "good"],
        "only git repositories are checked"
    );

    let good_entry = report
        .repositories
        .iter()
        .find(|r| r.repo == "good")
        .unwrap();
    assert!(good_entry.tracked, "good tracks its lockfile");
    assert!(good_entry.fresh, "good's lockfile is current");
    assert_eq!(good_entry.status(), "ok");

    let bad_entry = report
        .repositories
        .iter()
        .find(|r| r.repo == "bad")
        .unwrap();
    assert!(!bad_entry.tracked, "bad ignores its lockfile");
    assert_eq!(bad_entry.status(), "untracked");

    assert!(!report.is_ok());
    assert_eq!(report.failures().len(), 1);

    let rendered = outfeed_core::format_lockfile_report(&report);
    assert!(rendered.contains("1 of 2 repositories failed"));
    assert!(rendered.contains("untracked"));
}

#[test]
fn test_empty_workspace_is_a_failure_not_a_pass() {
    // This asserted the opposite until GB-327, and the assertion was the defect stated as a rule: an
    // empty workspace reported "All 0 repositories track a fresh Cargo.lock" and exited 0.
    let root = tempdir().expect("tempdir");
    let error =
        outfeed_core::check_lockfiles(root.path()).expect_err("nothing checked is not a pass");
    assert!(matches!(
        error,
        outfeed_core::OutfeedError::NoRepositories { .. }
    ));
}
