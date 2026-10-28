#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Unit and integration tests for outfeed clean and prune cache hygiene.

use outfeed_core::clean::{
    calculate_disk_usage, clean_workspace, clean_workspace_with_progress, format_bytes,
    prune_workspace,
};
use outfeed_core::model::{CleanOpts, PruneOpts};
use outfeed_core::progress::{ProgressReporter, RepoStatus};
use std::fs;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
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

#[derive(Debug, Default)]
struct CountingReporter {
    completions: AtomicUsize,
    messages: Mutex<Vec<String>>,
}

impl ProgressReporter for CountingReporter {
    fn on_start(&self, _repo: &str, _action: &str) {}

    fn on_update(&self, _repo: &str, _message: &str) {}

    fn on_complete(&self, _repo: &str, status: &RepoStatus) {
        self.completions.fetch_add(1, Ordering::SeqCst);
        if let RepoStatus::Success(message) = status
            && let Ok(mut messages) = self.messages.lock()
        {
            messages.push(message.clone());
        }
    }
}

#[test]
fn test_clean_workspace_emits_a_single_completion_event() {
    let dir = tempdir().expect("tempdir");
    let debug = dir.path().join("target").join("debug");
    fs::create_dir_all(&debug).expect("create debug");
    fs::write(debug.join("artifact"), vec![0u8; 512]).expect("write artifact");

    let reporter = CountingReporter::default();
    let report = clean_workspace_with_progress(
        dir.path(),
        &CleanOpts {
            dry_run: false,
            ..Default::default()
        },
        &reporter,
    )
    .expect("clean workspace");

    assert_eq!(report.reclaimed_bytes, 512);
    assert_eq!(
        reporter.completions.load(Ordering::SeqCst),
        1,
        "a clean pass must report exactly one completion event"
    );
    let messages = reporter.messages.lock().expect("messages");
    assert_eq!(messages.len(), 1);
    assert!(
        messages[0].starts_with("reclaimed"),
        "the single event must use clean wording, got {:?}",
        messages[0]
    );
}
