//! CLI command execution handlers for outfeed.

pub mod apply;
pub mod install;
pub mod plan;
pub mod publish;
pub mod push;
pub mod release;
pub mod status;

use std::path::{Path, PathBuf};

/// Resolves a repository name or path against the root meta-harness workspace.
pub fn resolve_repo_dir(base: &Path, input: &str) -> PathBuf {
    let p = PathBuf::from(input);
    if p.is_dir() {
        p
    } else {
        base.join(input)
    }
}

