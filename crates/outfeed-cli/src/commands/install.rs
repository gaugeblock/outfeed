//! Command handler for `outfeed install`.

use std::path::Path;
use anyhow::{Context, Result};
use outfeed_core::{install_all, install_tool, InstallOpts, InstallOutcome, SUITE_TOOLS};

use crate::args::InstallArgs;

pub fn handle_install(root: &Path, args: InstallArgs) -> Result<()> {
    let opts = InstallOpts {
        debug: args.debug,
        force: args.force,
    };

    if args.all || args.tool == "all" {
        println!("================================================================================");
        println!(" [outfeed] Installing all 10 suite CLI tools (profile: {})", if opts.debug { "debug" } else { "release" });
        println!("================================================================================");

        let results = install_all(root, &opts).context("Failed to install suite tools")?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&results.iter().map(|(n, o)| {
                serde_json::json!({
                    "name": n,
                    "outcome": match o {
                        InstallOutcome::Installed { version } => format!("installed {version}"),
                        InstallOutcome::Skipped { version } => format!("up-to-date {version}"),
                    }
                })
            }).collect::<Vec<_>>())?);
        } else {
            for (name, outcome) in results {
                match outcome {
                    InstallOutcome::Installed { version } => println!("✅ {name}: installed v{version}"),
                    InstallOutcome::Skipped { version } => println!("⏩ {name}: v{version} already up-to-date (use --force to rebuild)"),
                }
            }
        }
        return Ok(());
    }

    let tool = SUITE_TOOLS
        .iter()
        .find(|t| t.name == args.tool || t.bin_name == args.tool)
        .ok_or_else(|| anyhow::anyhow!("Unknown tool '{}'. Available: miter, storypole, sawhorse, chalkline, dado, batten, joiner, benchwork, rabbet, outfeed", args.tool))?;

    let outcome = install_tool(root, tool, &opts)
        .with_context(|| format!("Failed to install {}", tool.bin_name))?;

    if args.json {
        println!("{}", serde_json::json!({
            "tool": tool.bin_name,
            "outcome": match &outcome {
                InstallOutcome::Installed { version } => format!("installed {version}"),
                InstallOutcome::Skipped { version } => format!("up-to-date {version}"),
            }
        }));
    } else {
        match outcome {
            InstallOutcome::Installed { version } => println!("✅ {}: installed v{}", tool.bin_name, version),
            InstallOutcome::Skipped { version } => println!("⏩ {}: v{} already up-to-date (use --force to rebuild)", tool.bin_name, version),
        }
    }

    Ok(())
}

