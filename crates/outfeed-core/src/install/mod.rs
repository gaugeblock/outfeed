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
    let source_is_newer = opts.debug && binary_is_stale(&crate_path, tool.bin_name);

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

/// True when the installed binary is older than the crate's sources (`GB-264`).
///
/// Only consulted for `--debug` installs. Returns `true` when anything is unknown - no binary on PATH, no
/// readable mtime - because rebuilding unnecessarily costs a minute while skipping a stale binary costs a
/// confusing debugging session, which is the asymmetry that produced this function.
fn binary_is_stale(crate_path: &std::path::Path, bin_name: &str) -> bool {
    let Some(home) = std::env::var_os("HOME") else {
        return true;
    };
    let binary = std::path::PathBuf::from(home)
        .join(".cargo")
        .join("bin")
        .join(bin_name);
    let Ok(binary_mtime) = std::fs::metadata(&binary).and_then(|meta| meta.modified()) else {
        return true;
    };

    newest_mtime(&crate_path.join("src"))
        .map(|source| source > binary_mtime)
        .unwrap_or(true)
}

/// Newest modification time anywhere under `dir`, including the manifest beside it.
fn newest_mtime(dir: &std::path::Path) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut consider = |candidate: std::time::SystemTime| {
        newest = Some(newest.map_or(candidate, |current| current.max(candidate)));
    };

    if let Some(manifest) = dir.parent().map(|parent| parent.join("Cargo.toml"))
        && let Ok(meta) = std::fs::metadata(manifest)
        && let Ok(mtime) = meta.modified()
    {
        consider(mtime);
    }

    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(inner) = newest_mtime(&path) {
                consider(inner);
            }
        } else if let Ok(meta) = entry.metadata()
            && let Ok(mtime) = meta.modified()
        {
            consider(mtime);
        }
    }
    newest
}

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
