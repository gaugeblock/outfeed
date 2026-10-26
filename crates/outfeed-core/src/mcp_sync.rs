//! MCP configuration synchronizer for Antigravity, Cline, and workspace agents.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::OutfeedError;

const MCP_TOOLS: &[&str] = &[
    "miter",
    "storypole",
    "chalkline",
    "dado",
    "batten",
    "joiner",
    "benchwork",
    "outfeed",
];

/// Synchronizes MCP client configuration files to register installed suite binaries.
pub fn sync_antigravity_mcp(workspace_root: &Path) -> Result<Vec<PathBuf>, OutfeedError> {
    let mut configured = Vec::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

    let candidates = [
        PathBuf::from(&home).join(".gemini/antigravity/mcp_config.json"),
        PathBuf::from(&home).join(".gemini/config/mcp_config.json"),
        workspace_root.join(".agents/mcp_config.json"),
    ];

    let mcp_json = generate_mcp_config_json();

    for target in &candidates {
        if let Some(parent) = target.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(target, &mcp_json).is_ok() {
            configured.push(target.clone());
        }
    }

    Ok(configured)
}

fn generate_mcp_config_json() -> String {
    let mut servers = serde_json::Map::new();

    for tool in MCP_TOOLS {
        if which_binary(tool) {
            let mut server = serde_json::Map::new();
            server.insert(
                "command".to_string(),
                serde_json::Value::String(tool.to_string()),
            );
            server.insert(
                "args".to_string(),
                serde_json::Value::Array(vec![serde_json::Value::String("mcp".to_string())]),
            );
            servers.insert(tool.to_string(), serde_json::Value::Object(server));
        }
    }

    let mut root = serde_json::Map::new();
    root.insert("mcpServers".to_string(), serde_json::Value::Object(servers));

    serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".to_string())
}

fn which_binary(name: &str) -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    let cargo_bin = PathBuf::from(home).join(".cargo/bin").join(name);
    if cargo_bin.exists() {
        return true;
    }
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
