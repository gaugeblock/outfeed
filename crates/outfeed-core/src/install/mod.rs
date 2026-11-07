//! Binary compilation and installation into global PATH (`~/.cargo/bin`).

use std::path::Path;
use std::process::Command;

mod registry;

pub use registry::{SUITE_TOOLS, SuiteTool};

use crate::cmd::run_captured;
use crate::error::OutfeedError;
use crate::manifest::get_repo_version;
use crate::model::InstallOpts;
use crate::progress::{NoopProgress, ProgressReporter, RepoStatus};

/// Result of installing a tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    /// Newly built and installed.
    Installed {
        /// The installed version string.
        version: String,
        /// Captured output from cargo install.
        output: String,
    },
    /// Version already matches, skipped.
    Skipped {
        /// The skipped version string.
        version: String,
    },
}

mod staleness;
use staleness::binary_is_stale;

/// Installs a single tool by name using default progress reporter.
pub fn install_tool(
    workspace_root: &Path,
    tool: &SuiteTool,
    opts: &InstallOpts,
) -> Result<InstallOutcome, OutfeedError> {
    install_tool_with_progress(workspace_root, tool, opts, &NoopProgress)
}

/// Installs a single tool by name with progress reporting.
pub fn install_tool_with_progress(
    workspace_root: &Path,
    tool: &SuiteTool,
    opts: &InstallOpts,
    reporter: &dyn ProgressReporter,
) -> Result<InstallOutcome, OutfeedError> {
    // A library-only repository ships no binary; there is nothing to install.
    if tool.bin_name.is_empty() {
        reporter.on_complete(tool.name, &RepoStatus::Skipped("library-only".to_string()));
        return Ok(InstallOutcome::Skipped {
            version: get_repo_version(&workspace_root.join(tool.name))
                .unwrap_or_else(|_| "unknown".to_string()),
        });
    }

    let crate_path = workspace_root.join(tool.rel_path);
    if !crate_path.exists() {
        return Err(OutfeedError::Install(format!(
            "Crate path does not exist: {}",
            crate_path.display()
        )));
    }

    let repo_dir = workspace_root.join(tool.name);
    let target_version = get_repo_version(&repo_dir).unwrap_or_else(|_| "unknown".to_string());

    // A `--debug` install exists to pick up source edits during development, and those never change the
    // version - so the version gate is exactly the wrong test for them. Compare mtimes instead, and when
    // the version matches but the binary is current, say *why* it was skipped: "up-to-date" reads as
    // "your build is current" and cost a debugging detour the first time it lied (`GB-264`).
    let source_is_newer = opts.debug && binary_is_stale(&repo_dir, tool.bin_name);

    if !opts.force
        && !source_is_newer
        && let Some(installed) = get_installed_version(tool.bin_name)
        && installed == target_version
    {
        reporter.on_complete(
            tool.bin_name,
            &RepoStatus::Skipped(format!(
                "v{installed} (up-to-date by version; pass --force to rebuild from source)"
            )),
        );
        return Ok(InstallOutcome::Skipped { version: installed });
    }

    reporter.on_start(tool.bin_name, &format!("compiling v{target_version}"));

    let target_dir = workspace_root.join("target");
    let mut cmd = Command::new("cargo");
    cmd.arg("install")
        .arg("--path")
        .arg(&crate_path)
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--force");

    if repo_dir.join("Cargo.lock").exists() {
        cmd.arg("--locked");
    }

    if opts.debug {
        cmd.arg("--debug");
    }

    let output = run_captured(&mut cmd)?;
    if !output.success {
        let err_text = output.combined_output();
        reporter.on_complete(tool.bin_name, &RepoStatus::Failed(err_text.clone()));
        return Err(OutfeedError::Install(format!(
            "cargo install for {} failed:\n{}",
            tool.bin_name, err_text
        )));
    }

    let verified_ver = get_installed_version(tool.bin_name).ok_or_else(|| {
        OutfeedError::Install(format!(
            "Installed binary {} failed --version check",
            tool.bin_name
        ))
    })?;

    reporter.on_complete(
        tool.bin_name,
        &RepoStatus::Success(format!("v{verified_ver}")),
    );

    Ok(InstallOutcome::Installed {
        version: verified_ver,
        output: output.combined_output(),
    })
}

/// True when the installed binary is older than the repository's sources (`GB-264`, `GB-266`).
///
/// Scans the whole repository, not just the leaf crate. A leaf-only check cannot see a change in a
/// dependency crate, which is precisely how a "fixed" install once shipped the pre-fix binary: the edit
/// was in `benchwork-core` while the binary is built from `benchwork-cli`. Generated directories are
/// skipped, because `target/` alone is newer than every binary and would force a rebuild every time.
///
/// Only consulted for `--debug` installs. Returns `true` when anything is unknown - no binary on PATH, no
/// readable mtime - because rebuilding unnecessarily costs a minute while skipping a stale binary costs a
/// confusing debugging session.
/// Executes installation across all tools in the suite with default progress reporter.
pub fn install_all(
    workspace_root: &Path,
    opts: &InstallOpts,
) -> Result<Vec<(String, InstallOutcome)>, OutfeedError> {
    install_all_with_progress(workspace_root, opts, &NoopProgress)
}

/// Executes installation across all tools in the suite with progress reporting.
pub fn install_all_with_progress(
    workspace_root: &Path,
    opts: &InstallOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<(String, InstallOutcome)>, OutfeedError> {
    let mut results = Vec::new();
    for tool in SUITE_TOOLS {
        let outcome = install_tool_with_progress(workspace_root, tool, opts, reporter)?;
        results.push((tool.bin_name.to_string(), outcome));
    }
    Ok(results)
}

/// Finds the suite root by walking up from `start` until every suite repository is present (`GB-271`).
///
/// `outfeed install` resolved crate paths against the current directory, so running it from inside a member
/// repository produced `./benchwork/crates/benchwork-cli` and failed with "Crate path does not exist" -
/// while the previously installed binary stayed in place, which is the same silent-staleness class as
/// `GB-264`. The root is the outermost directory that contains all of `SUITE_TOOLS`.
#[must_use]
pub fn discover_suite_root(start: &Path) -> Option<std::path::PathBuf> {
    for candidate in start.ancestors() {
        if SUITE_TOOLS
            .iter()
            .all(|tool| candidate.join(tool.name).join("Cargo.toml").is_file())
        {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

/// Queries the installed version of a binary from PATH.
pub fn get_installed_version(bin_name: &str) -> Option<String> {
    let output = Command::new(bin_name).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split_whitespace().collect();
    if parts.len() >= 2 {
        Some(parts[1].to_string())
    } else {
        None
    }
}
