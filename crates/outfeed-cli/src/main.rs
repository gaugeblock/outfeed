//! Main CLI entrypoint for outfeed.

mod args;
mod commands;

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

    match cli.command {
        Commands::Plan(args) => handle_plan(&root_dir, args).await?,
        Commands::Apply(args) => handle_apply(&root_dir, args).await?,
        Commands::Status(args) => handle_status(&root_dir, args)?,
        Commands::Install(args) => handle_install(&root_dir, args)?,
        Commands::Push(args) => handle_push(&root_dir, args)?,
        Commands::Publish(args) => handle_publish(&root_dir, args)?,
        Commands::Release(args) => handle_release(&root_dir, args).await?,
        Commands::Clean(args) => handle_clean(&args, &root_dir)?,
        Commands::Prune(args) => handle_prune(&args, &root_dir)?,
        Commands::Mcp(_) => {
            outfeed_mcp::run_stdio_server(&root_dir)
                .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
        }
    }

    Ok(())
}
