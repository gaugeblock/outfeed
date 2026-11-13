//! Namespace reservation against crates.io (`OF-5`).
//!
//! crates.io has no reservation API. A name is held only by publishing a version, and a published
//! version can be yanked but never deleted, so the first publish of a name is permanent. Reserving a
//! name therefore means publishing a placeholder crate at an alpha version, with the metadata the
//! real crate will carry, because metadata is immutable per version.
//!
//! Three steps, which every project in the suite used to rewrite by hand as a shell script:
//! [`check_names`] asks the registry what is free, [`scaffold_placeholders`] writes the placeholder
//! crates, and [`publish`] sends them.

mod publish;
mod scaffold;

pub use publish::{PublishOutcome, PublishSummary, publish, publish_placeholders};
pub use scaffold::{
    DEFAULT_STAGING_DIR, Identity, PLACEHOLDER_VERSION, ScaffoldOpts, ScaffoldOutcome,
    parent_exclude_hint, scaffold_placeholders,
};

use crate::error::OutfeedError;
use crate::index_poll::{IndexLookup, lookup_index};
use serde::{Deserialize, Serialize};

/// What the registry says about one candidate name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameState {
    /// The registry answered 404, so the name is available to claim.
    Free,
    /// A version is already published, so the name is held.
    Taken,
    /// The registry did not answer, so the state of the name is unknown.
    Unknown,
}

impl NameState {
    /// Stable label used in the table and in the JSON payload.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Taken => "taken",
            Self::Unknown => "unknown",
        }
    }
}

/// One row of a reservation check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameStatus {
    /// Crate name that was queried.
    pub name: String,
    /// What the registry said about it.
    pub state: NameState,
    /// Highest published version, when the registry reported one.
    pub published_version: Option<String>,
}

/// Result of [`check_names`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReserveCheckReport {
    /// One entry per requested name, in request order.
    pub entries: Vec<NameStatus>,
}

impl ReserveCheckReport {
    /// Names the registry reports as free, in request order.
    #[must_use]
    pub fn free(&self) -> Vec<&str> {
        self.names_in_state(NameState::Free)
    }

    /// Names the registry reports as already taken, in request order.
    #[must_use]
    pub fn taken(&self) -> Vec<&str> {
        self.names_in_state(NameState::Taken)
    }

    /// Names whose state could not be determined because the registry did not answer.
    #[must_use]
    pub fn unknown(&self) -> Vec<&str> {
        self.names_in_state(NameState::Unknown)
    }

    fn names_in_state(&self, state: NameState) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|entry| entry.state == state)
            .map(|entry| entry.name.as_str())
            .collect()
    }

    /// True when every queried name is free. Unknown names are not free.
    #[must_use]
    pub fn all_free(&self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.state == NameState::Free)
    }
}

/// Queries the registry for every name, without publishing anything.
///
/// A registry that does not answer yields [`NameState::Unknown`] for that name rather than a
/// failure, so one flaky request cannot hide the answer for the other fifty.
///
/// # Errors
///
/// Currently infallible; the signature keeps the caller's error path uniform with the other two
/// reservation steps.
pub fn check_names(names: &[String]) -> Result<ReserveCheckReport, OutfeedError> {
    let entries = names
        .iter()
        .map(|name| {
            let (state, published_version) = match lookup_index(name) {
                IndexLookup::Missing => (NameState::Free, None),
                IndexLookup::Present(version) => (NameState::Taken, version),
                IndexLookup::Unreachable => (NameState::Unknown, None),
            };
            NameStatus {
                name: name.clone(),
                state,
                published_version,
            }
        })
        .collect();
    Ok(ReserveCheckReport { entries })
}

#[cfg(test)]
mod tests;
