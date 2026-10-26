//! Binary compilation and installation into global PATH (`~/.cargo/bin`).

use std::path::Path;
use std::process::Command;

use crate::cmd::run_captured;
use crate::error::OutfeedError;
use crate::manifest::get_repo_version;
use crate::model::InstallOpts;
use crate::progress::{NoopProgress, ProgressReporter, RepoStatus};

/// Metadata describing a tool in the gaugeblock suite.
#[derive(Debug, Clone, Copy)]
pub struct SuiteTool {
    /// Canonical repository / tool name.
    pub name: &'static str,
    /// Path relative to suite workspace root.
    pub rel_path: &'static str,
    /// Name of the installed executable binary.
    pub bin_name: &'static str,
}

/// The 10 tools comprising the gaugeblock meta-harness.
pub const SUITE_TOOLS: &[SuiteTool] = &[
    SuiteTool {
        name: "miter",
        rel_path: "miter",
        bin_name: "miter",
    },
    SuiteTool {
        name: "storypole",
        rel_path: "storypole/crates/storypole-cli",
        bin_name: "storypole",
    },
    SuiteTool {
        name: "sawhorse",
        rel_path: "sawhorse/crates/sawhorse-cli",
        bin_name: "sawhorse",
    },
    SuiteTool {
        name: "chalkline",
        rel_path: "chalkline/crates/chalkline-cli",
        bin_name: "chalkline",
    },
    SuiteTool {
        name: "dado",
        rel_path: "dado/crates/dado-cli",
        bin_name: "dado",
    },
    SuiteTool {
        name: "batten",
        rel_path: "batten/crates/batten-cli",
        bin_name: "batten",
    },
    SuiteTool {
        name: "joiner",
        rel_path: "joiner/crates/joiner-cli",
        bin_name: "joiner",
    },
    SuiteTool {
        name: "benchwork",
        rel_path: "benchwork/crates/benchwork-cli",
        bin_name: "benchwork",
    },
    SuiteTool {
        name: "rabbet",
        rel_path: "rabbet/crates/rabbet-cli",
        bin_name: "rabbet",
    },
    SuiteTool {
        name: "outfeed",
        rel_path: "outfeed/crates/outfeed-cli",
        bin_name: "outfeed",
    },
];

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
    let crate_path = workspace_root.join(tool.rel_path);
    if !crate_path.exists() {
        return Err(OutfeedError::Install(format!(
            "Crate path does not exist: {}",
            crate_path.display()
        )));
    }

    let repo_dir = workspace_root.join(tool.name);
    let target_version = get_repo_version(&repo_dir).unwrap_or_else(|_| "unknown".to_string());

    if !opts.force
        && let Some(installed) = get_installed_version(tool.bin_name)
        && installed == target_version
    {
        reporter.on_complete(
            tool.bin_name,
            &RepoStatus::Skipped(format!("v{installed} (up-to-date)")),
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
