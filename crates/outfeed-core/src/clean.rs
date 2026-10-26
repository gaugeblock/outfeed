//! Workspace target cache hygiene, selective cleaning, and pruning.

use crate::error::OutfeedError;
use crate::model::{CleanOpts, CleanReport, DiskUsage, PruneOpts};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Recursively computes the byte size of a file or directory.
pub fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

/// Computes a breakdown of disk usage in the workspace target directory.
pub fn calculate_disk_usage(workspace_root: &Path) -> DiskUsage {
    let target = workspace_root.join("target");
    if !target.exists() {
        return DiskUsage::default();
    }

    let debug_bytes = dir_size(&target.join("debug"));
    let release_bytes = dir_size(&target.join("release"));
    let doc_bytes = dir_size(&target.join("doc"));
    let total_bytes = dir_size(&target);
    let other_bytes = total_bytes.saturating_sub(debug_bytes + release_bytes + doc_bytes);

    DiskUsage {
        debug_bytes,
        release_bytes,
        doc_bytes,
        other_bytes,
        total_bytes,
    }
}

/// Formats a byte quantity into a human-readable string (e.g. "14.24 GB", "240.50 MB").
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Selectively cleans workspace target artifacts according to options.
pub fn clean_workspace(workspace_root: &Path, opts: &CleanOpts) -> Result<CleanReport, OutfeedError> {
    let initial_usage = calculate_disk_usage(workspace_root);
    let target = workspace_root.join("target");
    if !target.exists() {
        return Ok(CleanReport {
            initial_usage,
            reclaimed_bytes: 0,
            paths_removed: Vec::new(),
            dry_run: opts.dry_run,
        });
    }

    let mut targets_to_clean: Vec<PathBuf> = Vec::new();
    if opts.clean_all {
        targets_to_clean.push(target.clone());
    } else if opts.clean_doc {
        let p = target.join("doc");
        if p.exists() {
            targets_to_clean.push(p);
        }
    } else if opts.clean_release {
        let p = target.join("release");
        if p.exists() {
            targets_to_clean.push(p);
        }
    } else {
        // Safe default: purge debug artifacts while strictly preserving target/release
        let debug = target.join("debug");
        if debug.exists() {
            targets_to_clean.push(debug);
        }
    }

    let mut reclaimed_bytes = 0;
    let mut paths_removed = Vec::new();

    for path in &targets_to_clean {
        let size = dir_size(path);
        reclaimed_bytes += size;
        paths_removed.push(path.display().to_string());

        if !opts.dry_run {
            if path.is_file() {
                fs::remove_file(path).map_err(OutfeedError::Io)?;
            } else {
                fs::remove_dir_all(path).map_err(OutfeedError::Io)?;
            }
        }
    }

    Ok(CleanReport {
        initial_usage,
        reclaimed_bytes,
        paths_removed,
        dry_run: opts.dry_run,
    })
}

/// Prunes test binaries and artifacts older than N days in target/debug.
pub fn prune_workspace(workspace_root: &Path, opts: &PruneOpts) -> Result<CleanReport, OutfeedError> {
    let initial_usage = calculate_disk_usage(workspace_root);
    let debug_deps = workspace_root.join("target").join("debug").join("deps");
    if !debug_deps.exists() {
        return Ok(CleanReport {
            initial_usage,
            reclaimed_bytes: 0,
            paths_removed: Vec::new(),
            dry_run: opts.dry_run,
        });
    }

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(opts.keep_days as u64 * 86400))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut reclaimed_bytes = 0;
    let mut paths_removed = Vec::new();

    let entries = fs::read_dir(&debug_deps).map_err(OutfeedError::Io)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(meta) = entry.metadata()
            && let Ok(modified) = meta.modified()
            && modified < cutoff
        {
            let size = if path.is_dir() { dir_size(&path) } else { meta.len() };
            reclaimed_bytes += size;
            paths_removed.push(path.display().to_string());

            if !opts.dry_run {
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    Ok(CleanReport {
        initial_usage,
        reclaimed_bytes,
        paths_removed,
        dry_run: opts.dry_run,
    })
}
