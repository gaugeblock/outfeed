//! Main CLI entrypoint for outfeed.

mod args;
mod commands;
mod progress;

use anyhow::Result;
use args::{Cli, Commands};
use clap::Parser;
use commands::apply::handle_apply;
use commands::clean::{handle_clean, handle_prune};
use commands::install::handle_install;
use commands::plan::handle_plan;
use commands::publish::handle_publish;
use commands::push::handle_push;
use commands::release::handle_release;
use commands::status::handle_status;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let root_dir = cli.dir.unwrap_or_else(|| PathBuf::from("."));

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

    match cli.command {
        Commands::Plan(args) => handle_plan(&root_dir, args).await?,
        Commands::Apply(args) => handle_apply(&root_dir, args).await?,
        Commands::Status(args) => handle_status(&root_dir, args)?,
        Commands::Install(args) => handle_install(&root_dir, args, cli.verbose, cli.quiet)?,
        Commands::Push(args) => handle_push(&root_dir, args, cli.verbose, cli.quiet)?,
        Commands::Publish(args) => handle_publish(&root_dir, args, cli.verbose, cli.quiet)?,
        Commands::Release(args) => handle_release(&root_dir, args).await?,
        Commands::Clean(args) => handle_clean(&args, &root_dir, cli.verbose, cli.quiet)?,
        Commands::Prune(args) => handle_prune(&args, &root_dir, cli.verbose, cli.quiet)?,
        Commands::Mcp(_) => {
            outfeed_mcp::run_stdio_server(&root_dir)
                .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
        }
    }

    Ok(())
}
