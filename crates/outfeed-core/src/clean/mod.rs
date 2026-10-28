//! Workspace target cache hygiene, selective cleaning, and pruning.

mod size;

pub use size::{calculate_disk_usage, dir_size, format_bytes};

use crate::error::OutfeedError;
use crate::model::{CleanOpts, CleanReport, PruneOpts};
use crate::progress::{NoopProgress, ProgressReporter, RepoStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Selectively cleans workspace target artifacts according to options with a default reporter.
pub fn clean_workspace(
    workspace_root: &Path,
    opts: &CleanOpts,
) -> Result<CleanReport, OutfeedError> {
    clean_workspace_with_progress(workspace_root, opts, &NoopProgress)
}

/// Selectively cleans workspace target artifacts with progress reporting.
pub fn clean_workspace_with_progress(
    workspace_root: &Path,
    opts: &CleanOpts,
    reporter: &dyn ProgressReporter,
) -> Result<CleanReport, OutfeedError> {
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

    let action_desc = if opts.dry_run {
        "auditing cache"
    } else {
        "cleaning cache"
    };
    reporter.on_start("workspace", action_desc);

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

    let result_msg = if opts.dry_run {
        format!("audited: {} reclaimable", format_bytes(reclaimed_bytes))
    } else {
        format!("reclaimed {}", format_bytes(reclaimed_bytes))
    };
    reporter.on_complete("workspace", &RepoStatus::Success(result_msg));

    Ok(CleanReport {
        initial_usage,
        reclaimed_bytes,
        paths_removed,
        dry_run: opts.dry_run,
    })
}

/// Prunes test binaries and artifacts older than N days with default reporter.
pub fn prune_workspace(
    workspace_root: &Path,
    opts: &PruneOpts,
) -> Result<CleanReport, OutfeedError> {
    prune_workspace_with_progress(workspace_root, opts, &NoopProgress)
}

/// Prunes test binaries and artifacts older than N days with progress reporting.
pub fn prune_workspace_with_progress(
    workspace_root: &Path,
    opts: &PruneOpts,
    reporter: &dyn ProgressReporter,
) -> Result<CleanReport, OutfeedError> {
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

    reporter.on_start(
        "workspace",
        if opts.dry_run {
            "auditing stale artifacts"
        } else {
            "pruning stale artifacts"
        },
    );

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
            let size = if path.is_dir() {
                dir_size(&path)
            } else {
                meta.len()
            };
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

    let result_msg = if opts.dry_run {
        format!("audited: {} prunable", format_bytes(reclaimed_bytes))
    } else {
        format!("pruned {}", format_bytes(reclaimed_bytes))
    };
    reporter.on_complete("workspace", &RepoStatus::Success(result_msg));

    Ok(CleanReport {
        initial_usage,
        reclaimed_bytes,
        paths_removed,
        dry_run: opts.dry_run,
    })
}
