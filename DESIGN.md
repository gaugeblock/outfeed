# `outfeed` — Architectural Design & Decisions

> **Domain:** Neurosymbolic Release Gate & Delivery Engine | **Suite:** `gaugeblock`

This document captures the key Architectural Decision Records (ADRs) governing `outfeed`.

---

## Architectural Decision Records (ADRs)

### ADR-001: Lossless Manifest Modification via `toml_edit`
* **Status:** Accepted
* **Context:** Automated release tools frequently mangle `Cargo.toml` files by using regex patterns that miss edge cases or using `toml::to_string` which strips human comments, reorders keys, and obliterates custom whitespace.
* **Decision:** Perform all manifest version increments, workspace package edits, and dependency constraints using the `toml_edit` concrete syntax tree library.
* **Consequences:** Guarantees 100% comment and formatting retention across manifests while providing provably safe automated version bumping.

### ADR-002: Topological Leaf-First Registry Publishing with Index Retries
* **Status:** Accepted
* **Context:** Multi-crate workspaces fail during crates.io publishing if child dependencies are published concurrently or before upstream crates are indexed in crates.io.
* **Decision:** Build a topological DAG of workspace crate dependencies and publish member crates strictly leaf-first. After each publish step, poll the crates.io sparse index with exponential backoff until the crate is verifiably downloadable before initiating dependent crate publish.
* **Consequences:** Eliminates registry build failures and broken partial releases, ensuring smooth automated publishing of complex multi-crate workspaces.

### ADR-003: Conventional Commits with AST Drift Floor Verification
* **Status:** Accepted
* **Context:** Developers may label a commit as `feat:` when it actually altered or removed existing public traits and structs, introducing unexpected breaking changes.
* **Decision:** Combine conventional commit parsing with `miter` AST inspection. If `miter impact` or signature diffs reveal breaking signature removals in public API exports, enforce an AST drift floor that elevates the bump to `Minor` (during active beta) or `Major` regardless of commit messages.
* **Consequences:** Guarantees absolute Semantic Versioning fidelity and protects downstream consumers from unnoticed breaking changes.

### ADR-004: Ecosystem Snapshot Synchronization via `suite.lock`
* **Status:** Accepted
* **Context:** The `gaugeblock` suite consists of 10 independent Git repositories that evolve at different rates. Developers and automated harnesses need a deterministic, reproducible snapshot of verified cross-repo versions.
* **Decision:** Maintain a machine-readable `suite.lock` file in the meta-harness root recording exact Git commit SHAs, crate versions, and wire contract compatibility hashes. `outfeed sync` and `outfeed clone` align local checkouts against this lockfile.
* **Consequences:** Eliminates version drift across multi-repo environments and enables reproducible testing and release certification across all 10 repositories.

### ADR-005: 3-Phase Interactive Safety Gate (Check $\to$ Plan $\to$ Execute)
* **Status:** Accepted
* **Context:** Irreversible operations like tagging, committing, and publishing to crates.io must never happen accidentally from a misfired command or unreviewed agent action.
* **Decision:** Structure release commands into three distinct phases: Check (preflight verification) $\to$ Plan (deterministic proposal generation) $\to$ Execute (user confirmation). The execution phase requires either an interactive prompt confirmation or an explicit `--yes` flag.
* **Consequences:** Prevents accidental releases, provides full transparency into planned version increments and changelog previews, and keeps humans in control of production delivery.
