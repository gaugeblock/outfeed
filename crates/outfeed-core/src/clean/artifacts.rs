//! Artifact classification: which files under `target/` a prune may remove.
//!
//! Split out of `clean::mod` (GB-139) so the policy table and its predicate are reviewable
//! without the traversal that applies them.

use std::path::Path;

/// File extensions that carry reusable compilation artifacts and must survive pruning.
pub(crate) const PRESERVED_EXTENSIONS: &[&str] = &[
    "rlib", "rmeta", "so", "dylib", "dll", "a", "lib", "o", "obj", "d", "pdb", "msvc",
];

/// Returns true when an entry in `target/debug/deps` may be pruned.
///
/// Only relinkable outputs are prunable: extension-less executables (test and example binaries)
/// and directories. Compiled dependencies (`.rlib`, `.rmeta`), proc-macro libraries
/// (`.so`/`.dylib`) and dep-info files (`.d`) are preserved so a prune never triggers a rebuild
/// wave.
pub fn is_prunable_artifact(path: &Path) -> bool {
    if path.is_dir() {
        return true;
    }
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => !PRESERVED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()),
        None => true,
    }
}
