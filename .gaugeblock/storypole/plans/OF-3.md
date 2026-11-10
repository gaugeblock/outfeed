# GB-48: Packaging & Delivery Suite: `outfeed` install, push, publish & release orchestration

> **Status:** Ready / Specified  
> **Target Scope:** `outfeed` (`outfeed-core`, `outfeed-cli`, `outfeed-mcp`), `scripts/install-suite.sh`  
> **Key Invariants:**  
> 1. Hard 250 LOC rule on all `.rs` files.  
> 2. Intra-repo and cross-repo topological DAG sorting for publish order.  
> 3. Safe `--dry-run` modes for both push and publish.  
> 4. Verification of installed binaries via `--version` checks.  

---

## 1. Executive Summary

`outfeed` currently handles release planning, changelog synthesis, manifest bumping, and local git commit/tag creation (`outfeed plan`, `outfeed apply`).
GB-48 completes the packaging and delivery pipeline by introducing four core capabilities:
1. **`outfeed install [repo | --all]`**: Compiles and installs CLI binaries to `~/.cargo/bin`, replacing the legacy `scripts/install-suite.sh` with a native Rust runner.
2. **`outfeed push [repo | --all]`**: Pushes release commits and git tags to the configured remote with branch validation and dry-run safety.
3. **`outfeed publish [repo | --all]`**: Dynamically computes the workspace crate DAG, runs `cargo publish --dry-run`, and uploads crates to crates.io in strict topological dependency order (e.g., `*-core` before `*-cli`/`*-mcp` before root facade), with index propagation backoff.
4. **`outfeed release [repo | --all]`**: An orchestrated macro command executing the full delivery sequence: `plan` -> `apply` -> `install` -> `push` -> `publish`.

---

## 2. Architecture & File Layout

```text
outfeed/crates/outfeed-core/src/
├── install.rs      # Discovers CLI binary crates, runs cargo install, verifies --version
├── push.rs         # Validates git remote, pushes branch and tags with --dry-run
├── publish.rs      # Computes workspace crate DAG, sorts topologically, runs cargo publish
├── release.rs      # Macro orchestrator uniting apply, install, push, publish
├── model.rs        # Adds InstallOpts, PushOpts, PublishOpts, ReleaseOpts
└── lib.rs          # Re-exports new modules

outfeed/crates/outfeed-cli/src/
├── args.rs         # Adds Subcommands::Install, Push, Publish, Release
└── main.rs         # Dispatches new commands

outfeed/crates/outfeed-mcp/src/
├── schema.rs       # Adds schemas for outfeed_install, outfeed_push, outfeed_publish
└── dispatch.rs     # Dispatches tool execution to outfeed-core
```

---

## 3. Implementation Milestones

### Phase 1: `outfeed-core` Install Engine (`install.rs`)
* [x] Discover member CLI crates in repository workspaces (crates with `src/main.rs` or `[[bin]]`).
* [x] Invoke `cargo install --path <path> [--release] [--force]`.
* [x] Verify installed binary executes `--version` successfully.
* [x] Support `--all` to build and install all 9 suite CLI binaries in order.

### Phase 2: `outfeed-core` Git Push Engine (`push.rs`)
* [x] Validate git remote tracking and assert clean working tree.
* [x] Execute `git push <remote> <branch> --tags` (or specific release tag).
* [x] Support `--dry-run` to preview remote push refspecs without network mutation.

### Phase 3: `outfeed-core` Topological Publish Engine (`publish.rs`)
* [x] Parse workspace members and intra-workspace dependencies via `toml_edit` / `cargo_metadata`.
* [x] Build DAG and compute topological ordering (leaf crates like `*-core` first, facade crates last).
* [x] Execute `cargo publish --dry-run` gate for each member crate.
* [x] Execute real `cargo publish` in topological order with configurable delay/polling for index propagation.

### Phase 4: `outfeed-core` Release Macro Orchestrator (`release.rs`)
* [x] Sequentially run: `apply` -> `install` -> `push` -> `publish`.
* [x] Support `--dry-run` and `--skip-publish` / `--skip-push` flags.

### Phase 5: `outfeed-cli` & `outfeed-mcp` Command Exposure
* [x] Update `args.rs` and `main.rs` in `outfeed-cli`.
* [x] Update `schema.rs` and `dispatch.rs` in `outfeed-mcp`.
* [x] Update `scripts/install-suite.sh` to delegate to `outfeed install --all "$@"`.

