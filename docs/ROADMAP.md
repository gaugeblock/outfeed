# `outfeed` — Roadmap & Strategic Horizons

> **Current Lifecycle:** `0.1.0-beta.5` | **Suite:** `gaugeblock`

This document tracks active milestones, near-term enhancements, and long-term horizons for `outfeed`.

---

## 1. Active Milestone: `v0.1.0-beta.5` (Current)

- [x] **3-Phase Release Pipeline:** Transparent Preflight $\to$ Plan $\to$ Apply lifecycle with interactive review gates.
- [x] **Deterministic SemVer Calculator:** Conventional Commit grouping with `miter` AST drift floor verification.
- [x] **Lossless Manifest Synchronizer:** `toml_edit` AST bumping preserving all comments and formatting.
- [x] **Topological DAG Registry Publisher:** Leaf-first crates.io publishing with sparse-index propagation polling.
- [x] **Suite Delivery Orchestrator:** Multi-repository releases and `suite.lock` ecosystem synchronization.
- [x] **Packaging Suite:** Built-in `install`, `clone`, `sync`, `clean`, and `prune` commands.
- [x] **Model Context Protocol (MCP) Server:** `outfeed-mcp` exposing stdio release planning and application tools.

---

## 2. Near-Term Horizon: `v0.2.0-beta.1`

- [ ] **GitHub Releases Integration:** Automated upload of compiled binary artifacts and changelog release notes to GitHub Releases.
- [ ] **Homebrew Formula Synthesizer:** Automated formula generation and tap repository updates upon successful release.
- [ ] **Release Rollback Engine:** Automated git tag deletion and commit revert in the event of post-tag delivery failures.
- [ ] **Multi-Platform Cross-Compilation:** Built-in `cross` and `cargo-zigbuild` integration for universal multi-arch release builds.

---

## 3. Long-Term Horizon: `v0.3.0+`

- [ ] **Autonomous Release Swarm:** Completely autonomous release certification where agent swarms verify, bump, tag, and publish without human intervention.
- [ ] **Binary Provenance & Cosign Attestation:** Cryptographic SLSA Level 3 provenance signing for all published suite binaries.
- [ ] **Real-Time Release Telemetry:** Streaming delivery metrics into Rabbet for tracking deployment latencies and crate adoption.
