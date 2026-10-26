//! Plain text milestone reporter for non-interactive / CI / piped environments.

use outfeed_core::{ProgressReporter, RepoStatus};

/// Non-TTY stream reporter that emits discrete log lines to stderr.
#[derive(Debug, Default)]
pub struct StreamReporter;

impl ProgressReporter for StreamReporter {
    fn on_start(&self, repo: &str, action: &str) {
        eprintln!("➜ [{repo}] {action}...");
    }

    fn on_update(&self, repo: &str, message: &str) {
        eprintln!("  [{repo}] {message}");
    }

    fn on_complete(&self, repo: &str, status: &RepoStatus) {
        match status {
            RepoStatus::Success(msg) => eprintln!("✔ [{repo}] {msg}"),
            RepoStatus::Skipped(msg) => eprintln!("⊘ [{repo}] {msg}"),
            RepoStatus::Failed(err) => eprintln!("✖ [{repo}] FAILED: {err}"),
        }
    }
}
