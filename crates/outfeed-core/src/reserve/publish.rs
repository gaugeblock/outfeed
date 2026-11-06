//! Publishing placeholders to claim their names (`OF-5`).
//!
//! A reservation publish is not a release. It sends one alpha version per placeholder, skips names
//! that are already held, honours a 429's reset timestamp through [`crate::rate_limit`], and keeps
//! going past an individual failure so that one bad name cannot abandon the rest.

use std::path::Path;
use std::process::Command;

use crate::cmd::run_captured;
use crate::error::OutfeedError;
use crate::progress::{NoopProgress, ProgressReporter, RepoStatus};
use crate::rate_limit::run_with_rate_limit_retry;

/// What happened to one name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    /// The placeholder was published and the name is now held.
    Published,
    /// The name was already taken, so nothing was published.
    AlreadyTaken(String),
    /// Publication failed; the name is still free (or still not ours).
    Failed(String),
}

/// Per-name results of a reservation publish.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublishSummary {
    /// One entry per requested name, in request order.
    pub outcomes: Vec<(String, PublishOutcome)>,
}

impl PublishSummary {
    /// Names that this run successfully published.
    #[must_use]
    pub fn published(&self) -> Vec<&str> {
        self.matching(|outcome| matches!(outcome, PublishOutcome::Published))
    }

    /// Names that were already held before this run.
    #[must_use]
    pub fn already_taken(&self) -> Vec<&str> {
        self.matching(|outcome| matches!(outcome, PublishOutcome::AlreadyTaken(_)))
    }

    /// Names that remain unpublished because something failed.
    #[must_use]
    pub fn failed(&self) -> Vec<&str> {
        self.matching(|outcome| matches!(outcome, PublishOutcome::Failed(_)))
    }

    fn matching(&self, predicate: impl Fn(&PublishOutcome) -> bool) -> Vec<&str> {
        self.outcomes
            .iter()
            .filter(|(_, outcome)| predicate(outcome))
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

/// Publishes each placeholder in `staging` that has not already been claimed.
///
/// Names are attempted in the order given and a failure does not stop the run, because the ten-minute
/// per-crate window measured for new names means a single aborted run would otherwise waste the whole
/// window.
///
/// # Errors
///
/// [`OutfeedError::Reserve`] when a placeholder directory is missing or `cargo` cannot be spawned at
/// all. A publish that runs and fails is recorded in the summary instead.
pub fn publish_placeholders(
    staging: &Path,
    names: &[String],
    registry: Option<&str>,
    reporter: &dyn ProgressReporter,
) -> Result<PublishSummary, OutfeedError> {
    let mut summary = PublishSummary::default();
    for name in names {
        let dir = staging.join(name);
        if !dir.is_dir() {
            summary.outcomes.push((
                name.clone(),
                PublishOutcome::Failed(format!("no placeholder at {}", dir.display())),
            ));
            continue;
        }

        reporter.on_start(name, "publishing reservation placeholder");
        let output = run_with_rate_limit_retry(name, reporter, || {
            let mut cmd = Command::new("cargo");
            cmd.arg("publish").arg("--no-verify").current_dir(&dir);
            if let Some(registry) = registry {
                cmd.arg("--registry").arg(registry);
            }
            run_captured(&mut cmd)
        })?;

        let outcome = if output.success {
            reporter.on_complete(name, &RepoStatus::Success("name reserved".to_string()));
            PublishOutcome::Published
        } else {
            let text = output.combined_output();
            if text.contains("already uploaded")
                || text.contains("already exists on crates.io index")
            {
                reporter.on_complete(name, &RepoStatus::Skipped("already taken".to_string()));
                PublishOutcome::AlreadyTaken(text)
            } else {
                reporter.on_complete(name, &RepoStatus::Failed(text.clone()));
                PublishOutcome::Failed(text)
            }
        };
        summary.outcomes.push((name.clone(), outcome));
    }
    Ok(summary)
}

/// Publishes placeholders, for callers that print their own summary.
///
/// # Errors
///
/// As [`publish_placeholders`].
pub fn publish(
    staging: &Path,
    names: &[String],
    registry: Option<&str>,
) -> Result<PublishSummary, OutfeedError> {
    publish_placeholders(staging, names, registry, &NoopProgress)
}
