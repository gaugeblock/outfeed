#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Unit and integration tests for outfeed clean and prune cache hygiene.

use outfeed_core::clean::{calculate_disk_usage, clean_workspace, format_bytes, prune_workspace};
use outfeed_core::model::{CleanOpts, PruneOpts};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_calculate_disk_usage_and_formatting() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("target");
    let debug = target.join("debug");
    let release = target.join("release");
    let doc = target.join("doc");

    fs::create_dir_all(&debug).expect("create debug");
    fs::create_dir_all(&release).expect("create release");
    fs::create_dir_all(&doc).expect("create doc");

    fs::write(debug.join("test_bin"), vec![0u8; 1024]).expect("write debug");
    fs::write(release.join("release_bin"), vec![0u8; 2048]).expect("write release");
    fs::write(doc.join("index.html"), vec![0u8; 512]).expect("write doc");

    let usage = calculate_disk_usage(dir.path());
    assert_eq!(usage.debug_bytes, 1024);
    assert_eq!(usage.release_bytes, 2048);
    assert_eq!(usage.doc_bytes, 512);
    assert_eq!(usage.total_bytes, 3584);

    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(1024), "1.00 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
}

#[test]
fn test_clean_workspace_dry_run_and_safe_default() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("target");
    let debug = target.join("debug");
    let release = target.join("release");

    fs::create_dir_all(&debug).expect("create debug");
    fs::create_dir_all(&release).expect("create release");

    let debug_file = debug.join("test_bin");
    let release_file = release.join("app_bin");
    fs::write(&debug_file, vec![0u8; 4096]).expect("write debug");
    fs::write(&release_file, vec![0u8; 8192]).expect("write release");

    // 1. Dry run
    let dry_report = clean_workspace(
        dir.path(),
        &CleanOpts {
            dry_run: true,
            ..Default::default()
        },
    )
    .expect("dry run clean");

    assert!(dry_report.dry_run);
    assert_eq!(dry_report.reclaimed_bytes, 4096);
    assert!(
        debug_file.exists(),
        "debug file should still exist after dry run"
    );
    assert!(release_file.exists(), "release file should still exist");

    // 2. Safe default clean (purges debug, preserves release)
    let clean_report = clean_workspace(
        dir.path(),
        &CleanOpts {
            dry_run: false,
            ..Default::default()
        },
    )
    .expect("clean workspace");

    assert_eq!(clean_report.reclaimed_bytes, 4096);
    assert!(!debug_file.exists(), "debug file should be removed");
    assert!(release_file.exists(), "release binary must be preserved!");
}

#[test]
fn test_clean_workspace_all() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("target");
    fs::create_dir_all(target.join("debug")).expect("create debug");
    fs::write(target.join("debug").join("file.txt"), b"data").expect("write");

    let report = clean_workspace(
        dir.path(),
        &CleanOpts {
            clean_all: true,
            dry_run: false,
            ..Default::default()
        },
    )
    .expect("clean all");

    assert!(
        !target.exists(),
        "target directory should be completely gone"
    );
    assert_eq!(report.reclaimed_bytes, 4);
}

#[test]
fn test_prune_workspace() {
    let dir = tempdir().expect("tempdir");
    let debug_deps = dir.path().join("target").join("debug").join("deps");
    fs::create_dir_all(&debug_deps).expect("create deps");

    let candidate = debug_deps.join("old_test");
    fs::write(&candidate, b"old test content").expect("write candidate");

    // Prune with 0 keep days should catch everything
    let report = prune_workspace(
        dir.path(),
        &PruneOpts {
            keep_days: 0,
            dry_run: false,
        },
    )
    .expect("prune");

    assert_eq!(report.reclaimed_bytes, 16);
    assert!(!candidate.exists(), "candidate should be pruned");
}
