//! Clone vocabulary: protocol, per-repository status, summary and options.
//!
//! Split out of `clone::mod` (GB-139) so the wire-ish types are readable apart from the cloning.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Git protocol used for cloning suite repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GitProtocol {
    /// Secure Shell (git@github.com:gaugeblock/{repo}.git)
    #[default]
    Ssh,
    /// HTTPS (https://github.com/gaugeblock/{repo}.git)
    Https,
}

impl GitProtocol {
    /// Formats the clone remote URL for a given repository.
    pub fn url_for(&self, repo_name: &str) -> String {
        match self {
            GitProtocol::Ssh => format!("git@github.com:gaugeblock/{repo_name}.git"),
            GitProtocol::Https => format!("https://github.com/gaugeblock/{repo_name}.git"),
        }
    }
}

/// Status of a repository clone operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloneStatus {
    /// Repository cloned freshly from remote.
    Cloned,
    /// Existing local checkout detected and preserved.
    SkippedExisting,
    /// Aligned to exact commit SHA pinned in `suite.lock`.
    PinnedCommit(String),
}

/// Summary result for a single repository clone operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneSummary {
    /// Repository name.
    pub repo: String,
    /// Destination directory path.
    pub target_dir: PathBuf,
    /// Outcome of the clone operation.
    pub status: CloneStatus,
}

/// Execution options for suite clone commands.
#[derive(Debug, Clone, Default)]
pub struct CloneOpts {
    /// Clone all 10 suite repositories.
    pub all: bool,
    /// Pin checkouts to exact commit SHAs recorded in `suite.lock`.
    pub locked: bool,
    /// Remote git protocol to use.
    pub protocol: GitProtocol,
    /// Simulate clone operations without modifying disk.
    pub dry_run: bool,
    /// Automatically compile and install binaries to PATH.
    pub install: bool,
    /// Synchronize Antigravity and Cline MCP configuration files.
    pub sync_mcp: bool,
}
