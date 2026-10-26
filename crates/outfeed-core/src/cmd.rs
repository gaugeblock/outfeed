//! Child process execution and output capture utilities.

use crate::error::OutfeedError;
use std::process::{Command, Stdio};

/// Captured output from a child process execution.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandOutput {
    /// Whether process exited with status code 0.
    pub success: bool,
    /// Process exit code if available.
    pub code: Option<i32>,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

impl CommandOutput {
    /// Combined output lines (stderr and stdout).
    pub fn combined_output(&self) -> String {
        let mut out = String::new();
        let s = self.stdout.trim();
        if !s.is_empty() {
            out.push_str(s);
        }
        let e = self.stderr.trim();
        if !e.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(e);
        }
        out
    }

    /// Formats the output with indentation and a repository prefix for verbose logging.
    pub fn format_prefixed(&self, prefix: &str) -> String {
        let combined = self.combined_output();
        if combined.is_empty() {
            return String::new();
        }
        combined
            .lines()
            .map(|l| format!("  [{prefix}] {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Executes a configured `Command`, capturing stdout and stderr.
pub fn run_captured(cmd: &mut Command) -> Result<CommandOutput, OutfeedError> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd.output().map_err(OutfeedError::Io)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CommandOutput {
        success: output.status.success(),
        code: output.status.code(),
        stdout,
        stderr,
    })
}
