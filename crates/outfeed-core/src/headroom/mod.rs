//! Adaptive disk space headroom guard with tiered, cache-preserving pruning.
//!
//! Cache retention is the default across a suite release: build artifacts survive between
//! repositories so optimised dependencies (`syn`, `tokio`, `serde`) and proc-macros are
//! compiled once and reused. Safety comes from measurement instead of static reservation —
//! free space is sampled before and after every repository, and remediation escalates only as
//! far as the measured headroom requires:
//!
//! 1. `>= 3.5 GB` free — cache fully retained.
//! 2. `< 3.5 GB` free — stale test executables and `target/package` staging are pruned
//!    surgically (rlibs, rmeta files and proc-macro libraries survive, so no rebuild wave).
//! 3. `< 2.0 GB` free — emergency clean of `target/debug` and `target/package`, guaranteeing
//!    disk exhaustion never halts a release.

use std::fs;
use std::path::Path;

use crate::clean::{dir_size, format_bytes};
use crate::disk_space::{
    ADAPTIVE_PRUNE_DISK_THRESHOLD_BYTES, EMERGENCY_CLEAN_DISK_THRESHOLD_BYTES,
    clean_repo_target_cache, get_available_disk_space,
};
use crate::error::OutfeedError;

/// Remediation tier selected by the adaptive headroom guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadroomAction {
    /// Headroom was sufficient; the compilation cache is fully retained.
    Retained,
    /// Stale test executables and packaging staging were pruned surgically.
    Pruned,
    /// Headroom stayed critical; the intermediate target cache was cleaned.
    EmergencyClean,
}

impl HeadroomAction {
    /// Returns the human-readable remediation label used in progress reports.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retained => "cache retained",
            Self::Pruned => "pruned stale test binaries",
            Self::EmergencyClean => "emergency target clean",
        }
    }
}

/// Result of a single adaptive headroom enforcement pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadroomStatus {
    /// Free bytes measured before remediation.
    pub available_before: u64,
    /// Free bytes measured after remediation.
    pub available_after: u64,
    /// Bytes reclaimed by this pass.
    pub reclaimed_bytes: u64,
    /// Remediation tier that was applied.
    pub action: HeadroomAction,
}

impl HeadroomStatus {
    /// Renders a single-line summary suitable for a progress reporter update.
    pub fn summary(&self) -> String {
        if self.action == HeadroomAction::Retained {
            return format!(
                "disk headroom ok ({} free, cache retained)",
                format_bytes(self.available_after)
            );
        }
        format!(
            "adaptive disk guard: {} ({} reclaimed, {} free)",
            self.action.as_str(),
            format_bytes(self.reclaimed_bytes),
            format_bytes(self.available_after)
        )
    }
}

/// Measures free space and applies adaptive remediation using the default thresholds.
pub fn enforce_disk_headroom(workspace_root: &Path) -> Result<HeadroomStatus, OutfeedError> {
    enforce_disk_headroom_with_thresholds(
        workspace_root,
        ADAPTIVE_PRUNE_DISK_THRESHOLD_BYTES,
        EMERGENCY_CLEAN_DISK_THRESHOLD_BYTES,
    )
}

/// Measures free space and applies adaptive remediation using explicit thresholds.
///
/// `prune_threshold_bytes` is the floor that triggers surgical pruning and
/// `emergency_threshold_bytes` is the floor that escalates to a full target cache clean.
pub fn enforce_disk_headroom_with_thresholds(
    workspace_root: &Path,
    prune_threshold_bytes: u64,
    emergency_threshold_bytes: u64,
) -> Result<HeadroomStatus, OutfeedError> {
    let available_before = get_available_disk_space(workspace_root)?;
    if available_before >= prune_threshold_bytes {
        return Ok(HeadroomStatus {
            available_before,
            available_after: available_before,
            reclaimed_bytes: 0,
            action: HeadroomAction::Retained,
        });
    }

    // Tier 1: surgical prune. Proc-macro libraries, rlibs and rmeta files are preserved, so
    // the retained cache still short-circuits the next repository's compilation.
    let mut reclaimed = prune_test_binaries(workspace_root)?;
    reclaimed = reclaimed.saturating_add(clean_target_package(workspace_root)?);
    let after_prune = get_available_disk_space(workspace_root)?;

    // Tier 2: escalate only inside the emergency band.
    if after_prune < emergency_threshold_bytes {
        reclaimed = reclaimed.saturating_add(clean_repo_target_cache(workspace_root)?);
        let after_clean = get_available_disk_space(workspace_root)?;
        return Ok(HeadroomStatus {
            available_before,
            available_after: after_clean,
            reclaimed_bytes: reclaimed,
            action: HeadroomAction::EmergencyClean,
        });
    }

    Ok(HeadroomStatus {
        available_before,
        available_after: after_prune,
        reclaimed_bytes: reclaimed,
        action: HeadroomAction::Pruned,
    })
}

/// Prunes extension-less executables (test and example binaries) from `target/debug/deps`.
///
/// Compiled artifacts carry extensions (`.rlib`, `.rmeta`, `.d`, `.dylib`/`.so`, `.o`), so
/// they survive the sweep; only relinkable executables are discarded.
pub fn prune_test_binaries(workspace_root: &Path) -> Result<u64, OutfeedError> {
    let deps = workspace_root.join("target").join("debug").join("deps");
    if !deps.exists() {
        return Ok(0);
    }

    let mut reclaimed: u64 = 0;
    let entries = fs::read_dir(&deps).map_err(OutfeedError::Io)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() || path.extension().is_some() {
            continue;
        }
        reclaimed = reclaimed.saturating_add(metadata.len());
        fs::remove_file(&path).map_err(OutfeedError::Io)?;
    }

    Ok(reclaimed)
}

/// Removes the `target/package` staging directory produced by `cargo package`/`publish`.
pub fn clean_target_package(workspace_root: &Path) -> Result<u64, OutfeedError> {
    let package_dir = workspace_root.join("target").join("package");
    if !package_dir.exists() {
        return Ok(0);
    }

    let reclaimed = dir_size(&package_dir);
    fs::remove_dir_all(&package_dir).map_err(OutfeedError::Io)?;
    Ok(reclaimed)
}

#[cfg(test)]
mod tests;
