# `outfeed` — Neurosymbolic Release Gate, AST Drift Inspector & Delivery Orchestrator

[![Version](https://img.shields.io/badge/version-0.1.0--beta.5-blue.svg)](https://gaugeblock.dev)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![gaugeblock](https://img.shields.io/badge/suite-gaugeblock-orange.svg)](https://github.com/gaugeblock)
[![Documentation](https://img.shields.io/badge/docs-9--doc%20topology-green.svg)](ARCHITECTURE.md)

Part of the **[`gaugeblock`](https://gaugeblock.dev)** neurosymbolic code synthesis ecosystem.

> In carpentry, an **outfeed table** sits directly behind a table saw or planer to smoothly receive, support, and guide finished workpieces as they exit the blades, preventing tipping, kickback, and tear-out. In the `gaugeblock` meta-harness, `outfeed` is the packaging, release, and delivery engine: it ensures code exiting development meets rigorous verification gates, calculates deterministic SemVer increments, inspects public AST drift via `miter`, synthesizes architectural changelogs via `sawhorse`, updates manifests via `toml_edit`, and orchestrates suite-wide releases into `suite.lock`.

---

## 1. Architecture Overview

```text
┌────────────────────────────────────────────────────────┐
│ Workspace Checkouts & Git Repositories                 │
│ Conventional Commits  •  Git Tags  •  Cargo.toml       │
└───────────────────────────┬────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────┐
│ outfeed 3-Phase Release Pipeline (Check -> Plan -> Run)│
│ - Preflight Check: Porcelain Git tree, main branch, CI │
│ - Deterministic SemVer: Conventional Commit grouping   │
│ - AST Drift Inspector: miter public signature diffs    │
│ - Changelog Synthesizer: sawhorse AI generation        │
│ - Manifest Synchronizer: toml_edit lossless updates    │
│ - Storypole Closed-Loop: Release chores & verified tags│
└───────────────┬────────────────────────┬───────────────┘
                │                        │
                ▼                        ▼
┌───────────────────────────┐ ┌──────────────────────────┐
│ CLI (`outfeed`)           │ │ MCP Stdio Server         │
│ - outfeed release [--all] │ │ - outfeed_plan           │
│ - outfeed install [--all] │ │ - outfeed_apply          │
│ - outfeed clone / sync    │ │                          │
│ - outfeed clean / prune   │ │                          │
└───────────────────────────┘ └──────────────────────────┘
```

---

## 2. Core Capabilities

* **Check -> Plan -> Execute 3-Phase Pipeline:** Preflights repository invariants, presents a transparent SemVer bump plan with changelog preview, and requires confirmation before modifying disk or cutting tags.
* **Deterministic SemVer Calculation:** Analyzes Conventional Commits (`feat:`, `fix:`, `BREAKING CHANGE:`) between the latest Git tag and `HEAD`.
* **AST Drift & Contract Inspection:** Validates public symbol signatures via `miter` to guarantee breaking API changes receive mandatory major/minor base bumps.
* **Neurosymbolic Changelog Synthesis:** Ingests commits, diffs, and Storypole task plans, leveraging `sawhorse` to author architectural, human-grade release notes.
* **Lossless Manifest Synchronization:** Uses `toml_edit` to bump workspace `Cargo.toml`, member crate dependencies, and web `package.json` without destroying comments or formatting.
* **Packaging & Delivery Suite:** Built-in `outfeed install` for rapid compilation into `~/.cargo/bin`, plus `outfeed clone` and `outfeed sync` for multi-repository bootstrap against `suite.lock`.

---

## 3. Workspace Crate Layout

* **`outfeed`**: Facade crate re-exporting `outfeed-core`.
* **`crates/outfeed-core`**: Core release engine, 3-phase pipeline, SemVer calculator, AST drift inspector, `toml_edit` sync, and changelog synthesizer.
* **`crates/outfeed-cli`**: Clap binary entrypoint (`outfeed`) featuring indicatif progress spinners and concurrency controls.
* **`crates/outfeed-mcp`**: JSON-RPC 2.0 Model Context Protocol stdio server.

---

## 4. Installation & CLI Quickstart

```bash
# Install globally via cargo
cargo install --path crates/outfeed-cli

# 1. Plan and execute release for current repository
outfeed release

# 2. Orchestrate suite-wide release across all 10 repositories
outfeed release --all --yes

# 3. Deterministic release without LLM changelog synthesis
outfeed release --no-llm

# 4. Install all suite binaries to ~/.cargo/bin
outfeed install --all

# 5. Clone all suite member repositories
outfeed clone --all --protocol ssh

# 6. Audit and synchronize local checkouts with suite.lock
outfeed sync --all --locked
```

---

## 5. Model Context Protocol (MCP) Reference

| Tool | Parameters | Description |
|---|---|---|
| `outfeed_plan` | `repo_dir?, target_version?` | Compute next SemVer increment, inspect AST drift, and draft changelog preview. |
| `outfeed_apply` | `plan_id, create_commit?, create_tag?` | Execute manifest updates, commit release changes, and create annotated Git tag. |

---

## 6. Repository Documentation

`outfeed` adheres strictly to the canonical **9-Document Architecture**:
* **[`CONTRIBUTING.md`](./CONTRIBUTING.md):** Local toolchain prerequisites, release testing, and test execution.
* **[`ARCHITECTURE.md`](./ARCHITECTURE.md):** 3-phase release pipeline, AST comparison logic, and manifest editor.
* **[`SPEC.md`](./SPEC.md):** SemVer calculation invariants, release receipt schemas, and error contracts.
* **[`AGENTS.md`](./AGENTS.md):** Autonomous systems directives, <=250 LOC limits, and zero-unwrap rules.
* **[`CHANGELOG.md`](./CHANGELOG.md):** Chronological historical release entries and unreleased changes.

---

## 7. Verification & Development Directives

* **File Size Ceiling:** All `.rs` source files must strictly remain under 250 LOC.
* **Tiered Verification Matrix:**
  * Tier 1: `cargo check -p outfeed-core` (<2s).
  * Tier 2: `cargo fmt --check -p outfeed-core && cargo clippy -p outfeed-core && cargo test -p outfeed-core`.
  * Tier 3: `chalkline docs check -d .` before task verification.

---

## 8. License

Licensed under the [Apache License, Version 2.0](LICENSE).
Part of the [`gaugeblock`](https://gaugeblock.dev) suite.
