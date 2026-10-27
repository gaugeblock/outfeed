//! Conventional commit models and metadata.

use serde::{Deserialize, Serialize};

/// Conventional commit classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConventionalKind {
    /// New feature (feat)
    Feat,
    /// Bug fix (fix)
    Fix,
    /// Refactoring (refactor)
    Refactor,
    /// Performance enhancement (perf)
    Perf,
    /// Documentation (docs)
    Docs,
    /// Code style / formatting (style)
    Style,
    /// Chore or maintenance (chore)
    Chore,
    /// Continuous integration (ci)
    Ci,
    /// Build system or dependencies (build)
    Build,
    /// Test suite (test)
    Test,
    /// Other / unrecognized kind
    Other(String),
}

/// Parsed git commit metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Short or full git commit SHA.
    pub sha: String,
    /// Raw commit summary line.
    pub summary: String,
    /// Optional commit body.
    pub body: Option<String>,
    /// Conventional commit kind.
    pub kind: ConventionalKind,
    /// Optional scope (e.g. "sawhorse-core").
    pub scope: Option<String>,
    /// Whether commit introduces breaking API or contract change.
    pub is_breaking: bool,
    /// Breaking change description if present.
    pub breaking_description: Option<String>,
    /// Referenced Storypole ticket keys (e.g. ["GB-41", "GB-45"]).
    pub referenced_tickets: Vec<String>,
}
