//! Is an installed binary older than the tree it was built from? (`GB-266`)
//!
//! Split out of `install/mod.rs` when that file passed the 250-line limit (`GB-291`). Staleness is its own
//! question with its own rule - the whole repository is scanned, not just the crate that owns the binary -
//! and it is answerable without knowing how installation works.

use std::path::{Path, PathBuf};

pub(super) fn binary_is_stale(repo_dir: &Path, bin_name: &str) -> bool {
    let Some(home) = std::env::var_os("HOME") else {
        return true;
    };
    let binary = PathBuf::from(home)
        .join(".cargo")
        .join("bin")
        .join(bin_name);
    let Ok(binary_mtime) = std::fs::metadata(&binary).and_then(|meta| meta.modified()) else {
        return true;
    };

    newest_mtime(repo_dir)
        .map(|source| source > binary_mtime)
        .unwrap_or(true)
}

/// Newest modification time anywhere under `dir`, skipping generated directories.
pub(super) fn newest_mtime(dir: &Path) -> Option<std::time::SystemTime> {
    const SKIP: [&str; 7] = [
        "target",
        ".git",
        "node_modules",
        "dist",
        ".gaugeblock",
        ".storypole",
        "worktrees",
    ];

    let mut newest: Option<std::time::SystemTime> = None;
    let mut consider = |candidate: std::time::SystemTime| {
        newest = Some(newest.map_or(candidate, |current| current.max(candidate)));
    };

    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if SKIP.contains(&name.as_str()) {
                continue;
            }
            if let Some(inner) = newest_mtime(&path) {
                consider(inner);
            }
        } else if let Ok(meta) = entry.metadata()
            && let Ok(mtime) = meta.modified()
        {
            consider(mtime);
        }
    }
    newest
}
