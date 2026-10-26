//! Dynamic MultiProgress reporter using `indicatif` for interactive TTY terminals.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use outfeed_core::{ProgressReporter, RepoStatus};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Interactive terminal progress reporter holding multiple spinners.
pub struct MultiProgressReporter {
    mp: Arc<MultiProgress>,
    bars: Mutex<HashMap<String, ProgressBar>>,
    active_style: ProgressStyle,
    success_style: ProgressStyle,
    skipped_style: ProgressStyle,
    failed_style: ProgressStyle,
}

impl MultiProgressReporter {
    /// Creates a new interactive multi-progress reporter.
    pub fn new() -> Self {
        let mp = Arc::new(MultiProgress::new());

        let active_style = ProgressStyle::with_template("{spinner:.cyan} {prefix:.bold} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);

        let success_style = ProgressStyle::with_template("{prefix:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());

        let skipped_style = ProgressStyle::with_template("{prefix:.dim} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());

        let failed_style = ProgressStyle::with_template("{prefix:.red} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());

        Self {
            mp,
            bars: Mutex::new(HashMap::new()),
            active_style,
            success_style,
            skipped_style,
            failed_style,
        }
    }
}

impl Default for MultiProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for MultiProgressReporter {
    fn on_start(&self, repo: &str, action: &str) {
        if let Ok(mut bars) = self.bars.lock() {
            let pb = self.mp.add(ProgressBar::new_spinner());
            pb.set_style(self.active_style.clone());
            pb.set_prefix(repo.to_string());
            pb.set_message(action.to_string());
            pb.enable_steady_tick(Duration::from_millis(80));
            bars.insert(repo.to_string(), pb);
        }
    }

    fn on_update(&self, repo: &str, message: &str) {
        if let Ok(bars) = self.bars.lock()
            && let Some(pb) = bars.get(repo)
        {
            pb.set_message(message.to_string());
        }
    }

    fn on_complete(&self, repo: &str, status: &RepoStatus) {
        if let Ok(mut bars) = self.bars.lock()
            && let Some(pb) = bars.remove(repo)
        {
            match status {
                RepoStatus::Success(msg) => {
                    pb.set_style(self.success_style.clone());
                    pb.set_prefix(format!("✔ {repo}:"));
                    pb.finish_with_message(msg.clone());
                }
                RepoStatus::Skipped(msg) => {
                    pb.set_style(self.skipped_style.clone());
                    pb.set_prefix(format!("⊘ {repo}:"));
                    pb.finish_with_message(msg.clone());
                }
                RepoStatus::Failed(err) => {
                    pb.set_style(self.failed_style.clone());
                    pb.set_prefix(format!("✖ {repo}:"));
                    pb.finish_with_message(format!("FAILED: {err}"));
                }
            }
        }
    }
}
