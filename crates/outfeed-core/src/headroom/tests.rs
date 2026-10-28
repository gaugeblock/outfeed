//! Unit tests for the adaptive disk space headroom guard.

#![allow(clippy::expect_used)]

use super::*;
use tempfile::tempdir;

#[test]
fn test_prune_test_binaries_preserves_compiled_artifacts() {
    let dir = tempdir().expect("tempdir");
    let deps = dir.path().join("target").join("debug").join("deps");
    fs::create_dir_all(&deps).expect("create deps");

    let test_bin = deps.join("outfeed_core-1a2b3c4d5e6f");
    let example_bin = deps.join("miter_lookup-0f9e8d");
    let rlib = deps.join("liboutfeed_core-1a2b3c4d5e6f.rlib");
    let rmeta = deps.join("liboutfeed_core-1a2b3c4d5e6f.rmeta");
    let dep_info = deps.join("outfeed_core-1a2b3c4d5e6f.d");
    let proc_macro = deps.join("libserde_derive-8c7b6a5f.dylib");

    fs::write(&test_bin, vec![0u8; 1024]).expect("write test bin");
    fs::write(&example_bin, vec![0u8; 512]).expect("write example bin");
    fs::write(&rlib, vec![0u8; 2048]).expect("write rlib");
    fs::write(&rmeta, vec![0u8; 256]).expect("write rmeta");
    fs::write(&dep_info, b"dep-info").expect("write dep info");
    fs::write(&proc_macro, vec![0u8; 4096]).expect("write proc macro");

    let reclaimed = prune_test_binaries(dir.path()).expect("prune test binaries");

    assert_eq!(
        reclaimed, 1536,
        "only extension-less executables are counted"
    );
    assert!(!test_bin.exists(), "test binary should be pruned");
    assert!(!example_bin.exists(), "example binary should be pruned");
    assert!(rlib.exists(), "rlib must survive surgical pruning");
    assert!(rmeta.exists(), "rmeta must survive surgical pruning");
    assert!(dep_info.exists(), "dep-info must survive surgical pruning");
    assert!(proc_macro.exists(), "proc-macro library must survive");
}

#[test]
fn test_prune_and_package_are_noops_without_target_dir() {
    let dir = tempdir().expect("tempdir");
    assert_eq!(prune_test_binaries(dir.path()).expect("prune"), 0);
    assert_eq!(clean_target_package(dir.path()).expect("package"), 0);
}

#[test]
fn test_clean_target_package_removes_staging_dir() {
    let dir = tempdir().expect("tempdir");
    let package = dir.path().join("target").join("package");
    fs::create_dir_all(&package).expect("create package dir");
    fs::write(package.join("outfeed-0.1.0.crate"), vec![0u8; 256]).expect("write crate");

    let reclaimed = clean_target_package(dir.path()).expect("clean package");

    assert_eq!(reclaimed, 256);
    assert!(!package.exists(), "packaging staging should be removed");
}

#[test]
fn test_enforce_disk_headroom_retains_cache_with_ample_headroom() {
    let dir = tempdir().expect("tempdir");
    let status = enforce_disk_headroom_with_thresholds(dir.path(), 0, 0).expect("headroom pass");
    assert_eq!(status.action, HeadroomAction::Retained);
    assert_eq!(status.reclaimed_bytes, 0);
    assert_eq!(status.available_after, status.available_before);
    assert!(status.summary().contains("cache retained"));
}

#[test]
fn test_enforce_disk_headroom_escalates_to_emergency_clean() {
    let dir = tempdir().expect("tempdir");
    let deps = dir.path().join("target").join("debug").join("deps");
    fs::create_dir_all(&deps).expect("create deps");
    fs::write(deps.join("libcached-1a2b.rmeta"), vec![0u8; 128]).expect("write rmeta");

    let status =
        enforce_disk_headroom_with_thresholds(dir.path(), u64::MAX, u64::MAX).expect("pass");

    // Platforms without `df` report unlimited headroom and skip remediation entirely.
    if status.available_before == u64::MAX {
        return;
    }
    assert_eq!(status.action, HeadroomAction::EmergencyClean);
    assert!(!dir.path().join("target").join("debug").exists());
    assert!(status.summary().contains("emergency target clean"));
}
