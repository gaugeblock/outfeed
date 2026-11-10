# GB-55: `outfeed clean` & `outfeed prune` Workspace Cache Hygiene

> **Status:** InProgress  
> **Target Scope:** `outfeed` (`outfeed-core`, `outfeed-cli`, `outfeed-mcp`)  
> **Key Invariants:**  
> 1. Hard 250 LOC rule on all `.rs` files.  
> 2. Zero-panic safety: no `.unwrap()` / `.expect()` in production library paths (`thiserror`).  
> 3. Safe-by-default: `outfeed clean` cleans `target/debug` while strictly preserving `target/release/` and release binaries.  
> 4. Accurate disk reporting in `--dry-run` modes.  
> 5. Age-based test executable pruning in `outfeed prune`.  

---

## 1. Executive Summary & Problem Statement

The `gaugeblock` meta-harness configures a shared build directory (`target-dir = "target"` in `.cargo/config.toml`) to prevent 10 separate workspaces from compiling redundant copies of third-party crates.

While this saves significant compile time, full test executions and continuous development rapidly balloon `target/debug` to 15–30+ GB with stale test executables, `.rlib`s, and incremental compilation hashes.

Standard `cargo clean` is too blunt—it wipes `target/` entirely, deleting optimized release binaries and forcing lengthy re-compilation.

`GB-55` adds centralized workspace cache hygiene to `outfeed`:
1. **`outfeed clean`**: Selectively purges `target/debug` build artifacts and test binaries while preserving `target/release`.
2. **`outfeed clean --dry-run`**: Audits and reports exact disk usage broken down by component (`debug`, `release`, `doc`, total reclaimable).
3. **`outfeed clean --all`**: Full target directory clean across meta-harness and member checkouts.
4. **`outfeed clean --doc`**: Purges only documentation artifacts in `target/doc`.
5. **`outfeed prune [--keep-days <N>]`**: Selectively prunes test binaries and artifacts older than N days (default 3) in `target/debug/deps`.

---

## 2. Architecture & File Layout

```text
outfeed/crates/outfeed-core/src/
├── clean.rs          # Core disk calculation, selective clean, and age-based pruning (<200 LOC)
├── model.rs          # CleanOpts, PruneOpts, CleanReport, DiskUsage
└── lib.rs            # Re-exports clean module

outfeed/crates/outfeed-cli/src/
├── commands/
│   ├── clean.rs      # Subcommand handler for outfeed clean and prune (<100 LOC)
│   └── mod.rs        # Re-export clean handler
└── args.rs           # CleanArgs and PruneArgs Clap definitions

outfeed/crates/outfeed-mcp/src/
├── schema.rs         # outfeed_clean schema definition
└── dispatch.rs       # MCP tool dispatcher

outfeed/crates/outfeed-core/tests/
└── clean_tests.rs    # Unit tests for size calculation, dry-run, and selective pruning
```

---

## 3. Data Contracts

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskUsage {
    pub debug_bytes: u64,
    pub release_bytes: u64,
    pub doc_bytes: u64,
    pub other_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanReport {
    pub initial_usage: DiskUsage,
    pub reclaimed_bytes: u64,
    pub paths_removed: Vec<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOpts {
    pub clean_all: bool,
    pub clean_doc: bool,
    pub clean_release: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneOpts {
    pub keep_days: u32,
    pub dry_run: bool,
}
```

---

## 4. Verification & Dogfooding Gate

1. **Tier 1:** `cargo check -p outfeed-core`
2. **Tier 2:** `cargo fmt --check -p outfeed-core`, `cargo clippy -p outfeed-core --all-targets -- -D warnings`, `cargo test -p outfeed-core`
3. **Tier 3:** `cargo test --workspace --manifest-path outfeed/Cargo.toml`
4. **Tier 4 / Dogfooding:** Run `outfeed clean --dry-run` and `outfeed clean` to reclaim disk space safely.
