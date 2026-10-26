# GB-47: Phase 9: `outfeed` — Neurosymbolic Release Gate, AST Drift Inspector & Changelog Synthesizer

> **Status:** Ready / Specified  
> **Target Scope:** `outfeed` (`outfeed-core`, `outfeed-cli`, `outfeed-mcp`), `workspace`, `storypole-core`  
> **Key Decisions:** D1 (Trunk-Based Release Orchestration), D2 (Closed-Loop Storypole Chore Ticket Generation), D3 (Dual-Mode Sawhorse LLM Release Synthesis & AST Verification), D4 (Transactional Manifest Rollback on Tier 4 Failure), D5 (Dedicated MCP Server for Antigravity / Cline)

---

## 1. Executive Summary & Vision

As the `gaugeblock` suite expanded across 9 standalone repositories, release management faced a critical tooling void:
1. **Per-Task vs. Release Tension:** Agents attempting to bump versions at `storypole verify` created churn or confusion, while batching releases left unreleased commits uncataloged across multiple repos.
2. **Multi-Repo Manifest Drift:** Bumping a repository requires synchronized updates across root `Cargo.toml`, member crate path requirements, `web/package.json`, and lockfiles.
3. **Mechanical vs. Meaningful Changelogs:** Standard changelog generators dump raw commit strings. High-quality releases require synthesizing architectural narratives from git commits, Storypole task plans, and `miter` AST impact diffs.

`outfeed` (Phase 9) serves as the dedicated release gate and delivery engine for `gaugeblock`. Named after the woodworking outfeed table—which receives, supports, and guides finished stock as it leaves the machine—`outfeed` provides precision release planning, neurosymbolic changelog synthesis, manifest bumping, and closed-loop Storypole synchronization.

---

## 2. Architecture & Subsystem Taxonomy

```text
outfeed/
├── Cargo.toml                 # Workspace root & facade crate
├── README.md                  # Usage, CLI verbs, and MCP protocol specs
├── src/lib.rs                 # Re-exports outfeed-core
└── crates/
    ├── outfeed-core/          # Pure domain: Git inspection, SemVer rules, toml_edit, sawhorse synthesis
    │   ├── Cargo.toml
    │   └── src/
    │       ├── git.rs         # Commit range traversal, tag discovery, clean tree assertions
    │       ├── semver.rs      # Conventional Commit classifier & SemVer calculator
    │       ├── manifest.rs    # toml_edit AST workspace & member dependency synchronizer
    │       ├── changelog.rs   # Keep-a-Changelog parser, formatter, and merge engine
    │       ├── synthesis.rs   # sawhorse-core prompt executor for narrative changelog generation
    │       ├── storypole.rs   # Ledger reader & release chore ticket creator
    │       └── error.rs       # Strongly-typed thiserror enum
    ├── outfeed-cli/           # Clap CLI entrypoint
    │   ├── Cargo.toml
    │   └── src/
    │       ├── main.rs        # CLI dispatcher (plan, apply, rollback, status)
    │       └── commands/      # Command handlers with --dry-run and --json support
    └── outfeed-mcp/           # Dedicated MCP stdio server
        ├── Cargo.toml
        └── src/
            ├── lib.rs         # MCP JSON-RPC protocol server
            └── tools.rs       # outfeed_plan and outfeed_apply tool handlers
```

---

## 3. Implementation Phases & Milestones

### Phase 0: Project Naming & Namespace Verification
* [x] Confirm availability of `outfeed`, `outfeed-core`, `outfeed-cli`, `outfeed-mcp` on crates.io (verified clean).
* [x] Add `outfeed` to `scripts/reserve-namespaces.sh` and execute dry-run check.
* [x] Scaffold `gaugeblock/outfeed` workspace with Rust 2024 root `Cargo.toml`.
* [x] Map sibling checkout into root `.cargo/config.toml` `[patch.crates-io]`.

### Phase 1: `outfeed-core` Deterministic Engine
* [x] `git.rs`: Assert clean git state on `main`; resolve `latest_tag..HEAD` commit lineage.
* [x] `semver.rs`: Parse Conventional Commits (`feat`, `fix`, `refactor`, `BREAKING CHANGE:`) and compute pre-release vs minor base bump.
* [x] `manifest.rs`: Use `toml_edit` to bump `[workspace.package] version` and intra-repo sibling constraints in `Cargo.toml` without dropping comments.
* [x] `manifest.rs`: Update `benchwork/web/package.json` version in tandem if present.
* [x] `rollback.rs`: Transactional rollback mechanism restoring files if downstream checks fail.

### Phase 2: Neurosymbolic Changelog Synthesis (`sawhorse` + `storypole`)
* [x] `storypole.rs`: Parse referenced `GB-XX` ticket keys from commits and extract task blueprints and verification receipts from `.gaugeblock/storypole`.
* [x] `synthesis.rs`: Construct structured prompt containing commit summaries, AST diffs, and Storypole plans.
* [x] `synthesis.rs`: Invoke `sawhorse-core` via configured provider from `.gaugeblock/config.toml` to draft Keep a Changelog markdown sections with SemVer justification.

### Phase 3: `outfeed-cli` Command Surface
* [x] Implement `outfeed plan <repo>` (`--dry-run`, `--json`): outputs proposed SemVer bump and full changelog diff.
* [x] Implement `outfeed apply <repo>`: writes manifests, runs Tier 4 verification, commits `chore(release): release vX.Y.Z`, and creates annotated git tag.
* [x] Implement `outfeed rollup`: evaluates all 9 member repositories and synthesizes root meta-harness `CHANGELOG.md` for milestone `suite-vX.Y.Z`.

### Phase 4: Closed-Loop Storypole Integration
* [x] On `outfeed plan`, optionally scaffold a Storypole chore ticket: `chore(release): release <repo> vX.Y.Z`.
* [x] On `outfeed apply`, record a `VerificationReceipt` certifying the release ticket and stamp verified member tasks with release metadata.
* [x] Ensure release tickets render rich changelog previews inside Benchwork UI (Panels 1 & 2).

### Phase 5: `outfeed-mcp` Agent Protocol Server
* [x] Implement stdio MCP server exposing `outfeed_plan` and `outfeed_apply`.
* [x] Enable Antigravity and Cline to inspect release readiness, preview release notes in chat, and trigger releases interactively.

---

## 4. Verification & Dogfooding Criteria
* **Dogfooding Target:** Immediately use `outfeed` to evaluate and cut the pending releases for `sawhorse` and `benchwork`.
* **Zero 250+ LOC Files:** Enforced across all `.rs` files.
* **Tier 3 Verification:** Full clean pass across `cargo fmt`, `cargo clippy`, `cargo nextest`, and `cargo test --doc`.

