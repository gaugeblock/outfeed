//! MCP Tool and resource schemas for `outfeed`.

use serde_json::{json, Value};

/// Returns the list of tools provided by the `outfeed` MCP server.
pub fn list_tools() -> Value {
    json!([
        {
            "name": "outfeed_plan",
            "description": "Evaluate unreleased commits and generate a proposed SemVer release plan and changelog",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Target repository path or name (defaults to current dir)"
                    },
                    "override_version": {
                        "type": "string",
                        "description": "Optional explicit version string to use instead of computed bump"
                    }
                }
            }
        },
        {
            "name": "outfeed_apply",
            "description": "Execute a release plan: bump manifests, update changelog, run verification, commit, tag, and update Storypole",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Target repository path or name (defaults to current dir)"
                    },
                    "override_version": {
                        "type": "string",
                        "description": "Optional explicit version string to release"
                    },
                    "verify_tier4": {
                        "type": "boolean",
                        "description": "Whether to run full Tier 4 cargo verification before committing (default true)"
                    }
                }
            }
        }
    ])
}

