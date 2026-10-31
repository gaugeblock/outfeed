//! Model Context Protocol (MCP) stdio server for `outfeed`.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod dispatch;
pub mod schema;

use gaugeblock_mcp::{AsyncMcpHandler, ToolOutcome};
use serde_json::Value;
use std::io::{self};
use std::path::Path;

/// MCP server version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `outfeed`'s tool surface, bound to the checkout it was started in.
#[derive(Debug)]
pub struct OutfeedHandler<'a> {
    root_dir: &'a Path,
}

impl<'a> OutfeedHandler<'a> {
    /// Builds a handler for a root directory.
    #[must_use]
    pub fn new(root_dir: &'a Path) -> Self {
        Self { root_dir }
    }
}

impl AsyncMcpHandler for OutfeedHandler<'_> {
    fn server_name(&self) -> &str {
        "outfeed-mcp"
    }

    fn list_tools(&self) -> Value {
        schema::list_tools()
    }

    async fn call_tool(&self, name: &str, arguments: &Value) -> ToolOutcome {
        match dispatch::dispatch_tool(name, arguments, self.root_dir).await {
            Ok(content) => ToolOutcome::Json(content),
            Err(message) => ToolOutcome::Failed(message),
        }
    }
}

/// Runs the stdio JSON-RPC 2.0 MCP server loop over stdin and stdout.
///
/// # Errors
///
/// Returns the I/O failure when stdin cannot be read or stdout cannot be written, or the failure to
/// start the runtime the async dispatch needs.
pub fn run_stdio_server(root_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    tracing::info!("Starting outfeed-mcp stdio server loop");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let handler = OutfeedHandler::new(root_dir);
    runtime.block_on(gaugeblock_mcp::serve_async(
        stdin.lock(),
        &mut stdout,
        &handler,
    ))?;
    Ok(())
}
