//! The suite tool registry: every repository, its binaries, and its crate list.
//!
//! Split out of `install::mod` (GB-139); the registry is data, the installation is behaviour.

/// Metadata describing a tool in the gaugeblock suite.
#[derive(Debug, Clone, Copy)]
pub struct SuiteTool {
    /// Canonical repository / tool name.
    pub name: &'static str,
    /// Path relative to suite workspace root.
    pub rel_path: &'static str,
    /// Name of the installed executable binary.
    pub bin_name: &'static str,
}

/// The 11 repositories comprising the gaugeblock meta-harness.
///
/// `common` is listed first because the contracts crate every member depends on must publish before
/// them; it ships no binary, so its `bin_name` is empty and installation skips it.
pub const SUITE_TOOLS: &[SuiteTool] = &[
    SuiteTool {
        name: "common",
        rel_path: "common",
        // Library-only: no binary to install, so `bin_name` is empty.
        bin_name: "",
    },
    SuiteTool {
        name: "miter",
        rel_path: "miter",
        bin_name: "miter",
    },
    SuiteTool {
        name: "storypole",
        rel_path: "storypole/crates/storypole-cli",
        bin_name: "storypole",
    },
    SuiteTool {
        name: "sawhorse",
        rel_path: "sawhorse/crates/sawhorse-cli",
        bin_name: "sawhorse",
    },
    SuiteTool {
        name: "chalkline",
        rel_path: "chalkline/crates/chalkline-cli",
        bin_name: "chalkline",
    },
    SuiteTool {
        name: "dado",
        rel_path: "dado/crates/dado-cli",
        bin_name: "dado",
    },
    SuiteTool {
        name: "batten",
        rel_path: "batten/crates/batten-cli",
        bin_name: "batten",
    },
    SuiteTool {
        name: "joiner",
        rel_path: "joiner/crates/joiner-cli",
        bin_name: "joiner",
    },
    SuiteTool {
        name: "benchwork",
        rel_path: "benchwork/crates/benchwork-cli",
        bin_name: "benchwork",
    },
    SuiteTool {
        name: "rabbet",
        rel_path: "rabbet/crates/rabbet-cli",
        bin_name: "rabbet",
    },
    SuiteTool {
        name: "outfeed",
        rel_path: "outfeed/crates/outfeed-cli",
        bin_name: "outfeed",
    },
];
