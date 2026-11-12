//! Main CLI entrypoint for outfeed.

mod args;
mod commands;
mod progress;

use anyhow::Result;
use args::{Cli, Commands};
use clap::Parser;
use commands::apply::handle_apply;
use commands::clean::{handle_clean, handle_prune};
use commands::clone::handle_clone;
use commands::install::handle_install;
use commands::plan::handle_plan;
use commands::publish::handle_publish;
use commands::push::handle_push;
use commands::release::handle_release;
use commands::status::handle_status;
use commands::sync::handle_sync;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // One span per invocation (`GB-339`). Opened before any work and closed with the outcome.
    // The subcommand is the operation the panel groups by: `miter audit` and `miter verify` share an
    // executable but not a job. Skipped when the first argument is a flag, which is not a command.
    let command = std::env::args().nth(1).filter(|arg| !arg.starts_with('-'));
    let invocation = gaugeblock_workspace::InvocationSpan::start("outfeed", command.as_deref());
    // Resolve the suite root by walking up, not by trusting the current directory (`GB-271`): running
    // `outfeed install` from inside a member repository used to resolve every crate path relative to that
    // repository and fail, leaving the previous binary silently in place.
    let root_dir = match cli.dir {
        Some(dir) => dir,
        None => std::env::current_dir()
            .ok()
            .and_then(|cwd| outfeed_core::install::discover_suite_root(&cwd))
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    let log_level = if cli.quiet {
        tracing::Level::ERROR
    } else {
        match cli.verbose {
            0 => tracing::Level::WARN,
            1 => tracing::Level::INFO,
            _ => tracing::Level::DEBUG,
        }
    };

    let _ = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .try_init();

    let result = async {
        match cli.command {
            Commands::Plan(args) => handle_plan(&root_dir, args).await?,
            Commands::Apply(args) => handle_apply(&root_dir, args).await?,
            Commands::Status(args) => handle_status(&root_dir, args)?,
            Commands::Install(args) => handle_install(&root_dir, args, cli.verbose, cli.quiet)?,
            Commands::Push(args) => handle_push(&root_dir, args, cli.verbose, cli.quiet)?,
            Commands::Publish(args) => handle_publish(&root_dir, args, cli.verbose, cli.quiet)?,
            Commands::Release(args) => handle_release(&root_dir, args).await?,
            Commands::Clone(args) => handle_clone(&root_dir, args, cli.quiet)?,
            Commands::Sync(args) => handle_sync(&root_dir, args, cli.quiet)?,
            Commands::Clean(args) => handle_clean(&args, &root_dir, cli.verbose, cli.quiet)?,
            Commands::Prune(args) => handle_prune(&args, &root_dir, cli.verbose, cli.quiet)?,
            Commands::Gate(args) => commands::gate::handle_gate(&args, &root_dir)?,
            Commands::Lockfiles(args) => commands::lockfiles::handle_lockfiles(&args, &root_dir)?,
            Commands::Consumers(args) => commands::consumers::handle_consumers(&args, &root_dir)?,
            Commands::Reserve(args) => {
                commands::reserve::handle_reserve(&args, &root_dir, cli.quiet)?
            }
            Commands::Mcp(_) => {
                outfeed_mcp::run_stdio_server(&root_dir)
                    .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
            }
        }

        Ok(())
    }
    .await;

    invocation.finish(result.is_ok());
    result
}
