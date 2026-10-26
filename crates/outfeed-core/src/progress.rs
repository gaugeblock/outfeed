//! Progress reporter trait and event models.

use std::sync::Arc;

/// Status outcome of an operation on a tool or repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoStatus {
    /// Operation completed successfully with optional summary detail.
    Success(String),
    /// Operation skipped (e.g. up to date or already published).
    Skipped(String),
    /// Operation failed with error description.
    Failed(String),
}

/// Abstract progress reporter trait for long-running operations.
pub trait ProgressReporter: Send + Sync {
    /// Called when an operation begins for a repository or tool.
    fn on_start(&self, repo: &str, action: &str);

    /// Called to update current action status for an active repository.
    fn on_update(&self, repo: &str, message: &str);

    /// Called when an operation completes for a repository.
    fn on_complete(&self, repo: &str, status: &RepoStatus);
}

/// No-op progress reporter that drops all events.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProgress;

impl ProgressReporter for NoopProgress {
    fn on_start(&self, _repo: &str, _action: &str) {}
    fn on_update(&self, _repo: &str, _message: &str) {}
    fn on_complete(&self, _repo: &str, _status: &RepoStatus) {}
}

impl<T: ProgressReporter + ?Sized> ProgressReporter for Arc<T> {
    fn on_start(&self, repo: &str, action: &str) {
        (**self).on_start(repo, action);
    }
    fn on_update(&self, repo: &str, message: &str) {
        (**self).on_update(repo, message);
    }
    fn on_complete(&self, repo: &str, status: &RepoStatus) {
        (**self).on_complete(repo, status);
    }
}

