//! Placeholder crate scaffolding for name reservation (`OF-5`).
//!
//! The placeholder is deliberately dull: one package, one empty lib, no dependencies. What matters is
//! that it parses as a standalone workspace member and that its metadata is already final, because
//! crates.io fixes metadata per published version and the first publish is permanent.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::OutfeedError;

use super::identity::Identity;

/// Version every placeholder is published at.
///
/// Alpha, never beta: a reservation is not a release, and `GB-127` draws the same line for namespace
/// claims.
pub const PLACEHOLDER_VERSION: &str = "0.1.0-alpha.1";

/// Staging directory used when the caller does not name one. Already gitignored in this repository.
pub const DEFAULT_STAGING_DIR: &str = "reserve-staging";

/// Everything a placeholder manifest needs that cannot be changed after publication.
#[derive(Debug, Clone)]
pub struct ScaffoldOpts {
    /// Directory the placeholder crates are written into.
    pub staging_dir: PathBuf,
    /// Who is claiming the names: homepage, license and repository template (`GB-364`).
    pub identity: Identity,
    /// Per-name repository overrides, for crates whose home is not `{name}`.
    pub repository_overrides: BTreeMap<String, String>,
}

impl ScaffoldOpts {
    /// Resolves the repository URL for one name, preferring an explicit override.
    #[must_use]
    pub fn repository_for(&self, name: &str) -> String {
        self.repository_overrides
            .get(name)
            .cloned()
            .unwrap_or_else(|| self.identity.repository_template.replace("{name}", name))
    }
}

/// Which placeholders were written, and which were already on disk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScaffoldOutcome {
    /// Placeholder directories created by this run.
    pub written: Vec<PathBuf>,
    /// Placeholder directories that already existed and were left alone.
    pub skipped: Vec<PathBuf>,
}

/// Writes one placeholder crate per name into the staging directory.
///
/// An existing directory is skipped rather than overwritten: a placeholder may already carry a
/// corrected license or description, and silently replacing it would lose that edit.
///
/// # Errors
///
/// [`OutfeedError::Reserve`] when a placeholder cannot be written.
pub fn scaffold_placeholders(
    names: &[String],
    opts: &ScaffoldOpts,
) -> Result<ScaffoldOutcome, OutfeedError> {
    let mut outcome = ScaffoldOutcome::default();
    for name in names {
        let dir = opts.staging_dir.join(name);
        if dir.is_dir() {
            outcome.skipped.push(dir);
            continue;
        }
        write_placeholder(&dir, name, opts)?;
        outcome.written.push(dir);
    }
    Ok(outcome)
}

/// Detects a staging directory that the parent workspace would enrol as a member.
///
/// Returns a remediation hint when the parent manifest is a workspace that does not already exclude
/// the staging path. An empty `[workspace]` table in each placeholder is usually enough, but the
/// measured case needed both, so the caller is told rather than left to discover it.
#[must_use]
pub fn parent_exclude_hint(staging_dir: &Path, parent_manifest: &Path) -> Option<String> {
    let manifest = fs::read_to_string(parent_manifest).ok()?;
    if !manifest.contains("[workspace]") {
        return None;
    }
    let staging = staging_dir.to_string_lossy();
    if manifest.contains(&format!("\"{staging}\"")) {
        return None;
    }
    Some(format!(
        "add \"{staging}\" to [workspace] exclude in the parent manifest"
    ))
}

fn write_placeholder(dir: &Path, name: &str, opts: &ScaffoldOpts) -> Result<(), OutfeedError> {
    fs::create_dir_all(dir.join("src")).map_err(|err| io_error(dir, err))?;

    let manifest = format!(
        "# Reserved by `outfeed reserve scaffold` (OF-5).\n\
         #\n\
         # This crate exists only to hold the name on crates.io until the real crate publishes its\n\
         # first release; it is replaced at that point. crates.io fixes a version's metadata at\n\
         # publication time, so the repository URL and description below must already be the final\n\
         # ones - they cannot be corrected in this version later.\n\
         [package]\n\
         name = \"{name}\"\n\
         version = \"{version}\"\n\
         edition = \"2024\"\n\
         license = \"{license}\"\n\
         repository = \"{repository}\"\n\
         homepage = \"{homepage}\"\n\
         description = \"Namespace reservation placeholder for `{name}`; superseded by the first real release.\"\n\
         \n\
         # An empty workspace table keeps this crate standalone. Without it cargo enrols it into the\n\
         # workspace above it and refuses to publish a member that is not in publish order.\n\
         [workspace]\n",
        version = PLACEHOLDER_VERSION,
        license = opts.identity.license,
        repository = opts.repository_for(name),
        homepage = opts.identity.homepage,
    );
    write_file(&dir.join("Cargo.toml"), &manifest)?;

    let lib = format!(
        "//! Namespace reservation placeholder for `{name}`.\n\
         //!\n\
         //! Published by `outfeed reserve` (OF-5) so the name is held on crates.io until the real\n\
         //! crate is ready. Nothing depends on this crate; it is retired when it is replaced.\n\
         \n\
         /// Returns the name this placeholder reserved.\n\
         #[must_use]\n\
         pub fn reserved_name() -> &'static str {{\n    \"{name}\"\n}}\n"
    );
    write_file(&dir.join("src").join("lib.rs"), &lib)?;

    let readme = format!(
        "# {name}\n\nReserved namespace. The crate of this name is not published yet; this placeholder holds the name on crates.io.\n\nHomepage: <{homepage}>\n",
        homepage = opts.identity.homepage
    );
    write_file(&dir.join("README.md"), &readme)?;

    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<(), OutfeedError> {
    fs::write(path, contents).map_err(|err| io_error(path, err))
}

fn io_error(path: &Path, err: std::io::Error) -> OutfeedError {
    OutfeedError::Reserve(format!("cannot write {}: {err}", path.display()))
}
