//! Command handler for `outfeed install`.

use anyhow::{Context, Result};
use outfeed_core::{
    InstallOpts, InstallOutcome, SUITE_TOOLS, install_all_with_progress, install_tool_with_progress,
};
use std::path::Path;

use crate::args::InstallArgs;
use crate::progress::create_reporter;

pub fn handle_install(root: &Path, args: InstallArgs, verbose: u8, quiet: bool) -> Result<()> {
    let opts = InstallOpts {
        debug: args.debug,
        force: args.force,
    };
    let reporter = create_reporter(quiet, args.json);

    if args.all || args.tool == "all" {
        let results = install_all_with_progress(root, &opts, reporter.as_ref())
            .context("Failed to install suite tools")?;

        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &results
                        .iter()
                        .map(|(n, o)| {
                            serde_json::json!({
                                "name": n,
                                "outcome": match o {
                                    InstallOutcome::Installed { version, .. } => {
                                        format!("installed {version}")
                                    }
                                    InstallOutcome::Skipped { version } => {
                                        format!("up-to-date {version}")
                                    }
                                }
                            })
                        })
                        .collect::<Vec<_>>()
                )?
            );
        } else if verbose > 0 {
            for (name, outcome) in &results {
                if let InstallOutcome::Installed { output, .. } = outcome {
                    if !output.is_empty() {
                        let formatted = output
                            .lines()
                            .map(|l| format!("  [{name}] {l}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        println!("{formatted}");
                    }
                }
            }
        }
        return Ok(());
    }

    let tool = SUITE_TOOLS
        .iter()
        .find(|t| t.name == args.tool || t.bin_name == args.tool)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown tool '{}'. Available: miter, storypole, sawhorse, chalkline, dado, batten, joiner, benchwork, rabbet, outfeed",
                args.tool
            )
        })?;

    let outcome = install_tool_with_progress(root, tool, &opts, reporter.as_ref())
        .with_context(|| format!("Failed to install {}", tool.bin_name))?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "tool": tool.bin_name,
                "outcome": match &outcome {
                    InstallOutcome::Installed { version, .. } => format!("installed {version}"),
                    InstallOutcome::Skipped { version } => format!("up-to-date {version}"),
                }
            })
        );
    } else if verbose > 0 {
        if let InstallOutcome::Installed { output, .. } = &outcome {
            if !output.is_empty() {
                let formatted = output
                    .lines()
                    .map(|l| format!("  [{}] {l}", tool.bin_name))
                    .collect::<Vec<_>>()
                    .join("\n");
                println!("{formatted}");
            }
        }
    }

    Ok(())
}
