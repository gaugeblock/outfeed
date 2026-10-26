#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Integration tests for suite clone orchestration and repository alignment.

use outfeed_core::clone::{CloneOpts, CloneStatus, GitProtocol, clone_suite};
use outfeed_core::progress::NoopProgress;
use outfeed_core::suite_lock::{SuiteLock, SuiteMetadata, SuiteToolEntry, write_suite_lock};
use tempfile::tempdir;

#[test]
fn test_git_protocol_url_generation() {
    assert_eq!(
        GitProtocol::Ssh.url_for("miter"),
        "git@github.com:gaugeblock/miter.git"
    );
    assert_eq!(
        GitProtocol::Https.url_for("benchwork"),
        "https://github.com/gaugeblock/benchwork.git"
    );
}

#[test]
fn test_clone_suite_empty_targets_without_all_fails() {
    let temp = tempdir().expect("failed to create tempdir");
    let res = clone_suite(temp.path(), &[], &CloneOpts::default(), &NoopProgress);
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("Specify repository name(s) or use --all"));
}

#[test]
fn test_clone_suite_dry_run_all() {
    let temp = tempdir().expect("failed to create tempdir");
    let opts = CloneOpts {
        all: true,
        dry_run: true,
        protocol: GitProtocol::Https,
        ..Default::default()
    };

    let summaries =
        clone_suite(temp.path(), &[], &opts, &NoopProgress).expect("clone_suite dry-run failed");
    assert_eq!(summaries.len(), 10);
    for s in &summaries {
        assert_eq!(s.status, CloneStatus::Cloned);
    }
}

#[test]
fn test_clone_existing_repo_skipped() {
    let temp = tempdir().expect("failed to create tempdir");
    let miter_dir = temp.path().join("miter");
    std::fs::create_dir(&miter_dir).expect("failed to create miter dir");

    let opts = CloneOpts {
        dry_run: true,
        ..Default::default()
    };

    let summaries =
        clone_suite(temp.path(), &["miter"], &opts, &NoopProgress).expect("clone_suite failed");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].status, CloneStatus::SkippedExisting);
}

#[test]
fn test_clone_with_suite_lock_pinning() {
    let temp = tempdir().expect("failed to create tempdir");
    let miter_dir = temp.path().join("miter");
    std::fs::create_dir(&miter_dir).expect("failed to create miter dir");

    let lock = SuiteLock {
        suite: SuiteMetadata {
            version: "0.1.0-beta.3".to_string(),
            created_at: "2026-10-26T12:00:00Z".to_string(),
            meta_harness: "gaugeblock".to_string(),
        },
        tools: vec![SuiteToolEntry {
            name: "miter".to_string(),
            version: "0.1.0-beta.1".to_string(),
            commit: "deadbeefcafebabe1234567890".to_string(),
            tag: Some("v0.1.0-beta.1".to_string()),
        }],
    };
    write_suite_lock(temp.path(), &lock).expect("failed to write suite.lock");

    let opts = CloneOpts {
        locked: true,
        dry_run: true,
        ..Default::default()
    };

    let summaries =
        clone_suite(temp.path(), &["miter"], &opts, &NoopProgress).expect("clone_suite failed");
    assert_eq!(summaries.len(), 1);
    assert_eq!(
        summaries[0].status,
        CloneStatus::PinnedCommit("deadbeefcafebabe1234567890".to_string())
    );
}
