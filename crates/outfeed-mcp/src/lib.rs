//! Model Context Protocol (MCP) stdio server for `outfeed`.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod dispatch;
pub mod schema;

use dispatch::dispatch_tool;
use schema::list_tools;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::Path;

/// MCP server version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runs the stdio JSON-RPC 2.0 MCP server loop over stdin and stdout.
pub fn run_stdio_server(root_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    tracing::info!("Starting outfeed-mcp stdio server loop");

    for line_res in stdin.lock().lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to read stdin line: {e}");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Failed to parse JSON-RPC message: {e}");
                continue;
            }
        };

        if let Some(resp) = rt.block_on(handle_message(&msg, root_dir)) {
            let resp_str = serde_json::to_string(&resp)?;
            writeln!(stdout, "{resp_str}")?;
            stdout.flush()?;
        }
    }

    Ok(())
}

async fn handle_message(msg: &Value, root_dir: &Path) -> Option<Value> {
    let id = msg.get("id");
    let method = msg.get("method").and_then(|m| m.as_str())?;

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "outfeed-mcp", "version": VERSION }
            }
        })),
        "notifications/initialized" => None,
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": list_tools() }
        })),
        "tools/call" => {
            let params = msg.get("params")?;
            let tool_name = params.get("name").and_then(|n| n.as_str())?;
            let empty_args = json!({});
            let args = params.get("arguments").unwrap_or(&empty_args);

            match dispatch_tool(tool_name, args, root_dir).await {
                Ok(content) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": content.to_string() }]
                    }
                })),
                Err(err_msg) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": format!("Error: {err_msg}") }],
                        "isError": true
                    }
                })),
            }
        }
        _ => id.map(|req_id| {
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": { "code": -32601, "message": "Method not found" }
            })
        }),
    }
}

