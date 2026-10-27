//! Options controlling install, push, publish, and macro release execution.

use serde::{Deserialize, Serialize};

/// Options controlling CLI binary installation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallOpts {
    /// Build with debug profile instead of release.
    pub debug: bool,
    /// Force rebuild/reinstall even if versions match.
    pub force: bool,
}

/// Options controlling git remote push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushOpts {
    /// Git remote name (defaults to "origin").
    pub remote: String,
    /// Simulate without pushing to remote.
    pub dry_run: bool,
}

impl Default for PushOpts {
    fn default() -> Self {
        Self {
            remote: "origin".to_string(),
            dry_run: false,
        }
    }
}

/// Options controlling crates.io publishing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishOpts {
    /// Target registry (defaults to crates.io).
    pub registry: Option<String>,
    /// Simulate publication without uploading.
    pub dry_run: bool,
    /// Delay in seconds between intra-repo dependent crates.
    pub delay_secs: u64,
}

impl Default for PublishOpts {
    fn default() -> Self {
        Self {
            registry: None,
            dry_run: false,
            delay_secs: 15,
        }
    }
}

/// Options controlling the orchestrated release macro.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseOpts {
    /// Simulate release without modifying disk or pushing.
    pub dry_run: bool,
    /// Skip installing binary locally.
    pub skip_install: bool,
    /// Skip pushing commits and tags to remote.
    pub skip_push: bool,
    /// Skip publishing crates to registry.
    pub skip_publish: bool,
    /// Bypass LLM release note synthesis and use deterministic formatting.
    pub no_llm: bool,
    /// Skip Tier 4 verification test suite during release.
    pub no_verify: bool,
    /// Maximum concurrent release jobs (default: 1).
    pub jobs: usize,
    /// Clean intermediate compilation artifacts (target/debug, target/package) between suite repositories.
    pub clean_between_repos: bool,
    /// Optional minimum required disk space in bytes for pre-flight check.
    pub min_disk_bytes: Option<u64>,
    /// Skip pre-flight disk space headroom check.
    pub skip_disk_check: bool,
}

impl Default for ReleaseOpts {
    fn default() -> Self {
        Self {
            dry_run: false,
            skip_install: false,
            skip_push: false,
            skip_publish: false,
            no_llm: false,
            no_verify: false,
            jobs: 1,
            clean_between_repos: true,
            min_disk_bytes: None,
            skip_disk_check: false,
        }
    }
}
