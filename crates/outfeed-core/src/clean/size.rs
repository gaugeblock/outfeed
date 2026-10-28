//! Byte-size accounting helpers shared by clean and prune operations.

use crate::model::DiskUsage;
use std::fs;
use std::path::Path;

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
