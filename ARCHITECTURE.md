# `outfeed` — System Architecture

> **Domain:** Neurosymbolic Release Gate & Delivery Engine | **Suite:** `gaugeblock`

This document details the software architecture, subsystem components, dataflow, and registry publishing pipelines of `outfeed`.

---

## 1. System Decomposition

```mermaid
graph TD
    subgraph GitEngine["Git & History Analysis"]
        GitRepo[Git Repository Interface]
        CommitParser[Conventional Commit Classifier]
        TagResolver[Tag & Base SHA Resolver]
    end

    subgraph Verification["Inspection & Drift Gates"]
        PreflightGate[Preflight Invariant Checker]
        AstInspector[miter AST Drift Verifier]
        Tier4Runner[Tier 4 Verification Gate]
    end

    subgraph SynthesisEngine["Synthesis & Manifest Engine"]
        SemVerCalc[SemVer Bump Calculator]
        ChangelogGen[sawhorse AI Changelog Synthesizer]
        TomlEditSync[toml_edit Lossless Manifest Sync]
    end

    subgraph DeliveryEngine["Packaging & Delivery"]
        Installer[Global Binary Installer: ~/.cargo/bin]
        Publisher[Topological crates.io Publisher]
        SuiteLockMgr[suite.lock State Synchronizer]
    end

    subgraph Interfaces["User & Agent Boundaries"]
        CLI[outfeed Clap CLI + Progress Spinners]
        MCP[outfeed-mcp JSON-RPC Stdio Server]
    end

    CLI & MCP --> PreflightGate
    PreflightGate --> GitRepo
    GitRepo --> CommitParser & TagResolver
    CommitParser & TagResolver --> SemVerCalc
    SemVerCalc --> AstInspector
    AstInspector --> ChangelogGen
    ChangelogGen --> TomlEditSync
    TomlEditSync --> Tier4Runner
    Tier4Runner --> Installer & Publisher & SuiteLockMgr
```

---

## 2. Release & Publication Sequence

```mermaid
sequenceDiagram
    autonumber
    actor Engineer as Engineer / Agent
    participant Outfeed as outfeed Engine
    participant Git as Local Git Checkout
    participant Miter as miter AST Engine
    participant Toml as toml_edit Synchronizer
    participant Cargo as crates.io Registry
    participant Storypole as Storypole Ledger

    Engineer->>Outfeed: outfeed release --yes
    Outfeed->>Git: Check Clean Tree & Branch (main)
    Outfeed->>Git: Collect Commits since Last Tag
    Outfeed->>Miter: Compare Public AST Signatures
    Miter-->>Outfeed: Verified AST Drift Delta
    Outfeed->>Outfeed: Calculate SemVer Increment (BumpType)
    Outfeed->>Outfeed: Synthesize Changelog (Keep a Changelog)
    Outfeed->>Toml: Update Cargo.toml (Lossless AST Bump)
    Outfeed->>Git: Stage Changes & Run Tier 4 Verification Gate
    Outfeed->>Git: Commit Release & Tag: git tag -a v<version>
    Outfeed->>Storypole: Mark Release Task Verified
    
    opt Publishing to Registry
        Outfeed->>Cargo: Verify Leaf-First Dependency Order
        Outfeed->>Cargo: cargo publish --dry-run
        Outfeed->>Cargo: cargo publish
        Outfeed->>Cargo: Await Sparse Index Propagation
    end
    Outfeed-->>Engineer: Release Summary (Tag SHA, Crates Published)
```

---

## 3. Manifest Synchronization & Safety Architecture

* **Lossless AST Modification:** Built on `toml_edit`, parsing `Cargo.toml` into a concrete syntax tree where comments, spacing, and table order are preserved.
* **Topological Dependency Resolution:** Analyzes intra-workspace member dependencies to construct a Directed Acyclic Graph (DAG), ensuring crates with zero internal dependencies publish first, followed by intermediate crates, and finally top-level CLI entrypoints.
* **Network Fault Resilience:** Interrogates crates.io sparse index between publish steps to guarantee package availability before launching dependent crate publications, preventing registry build failures.

---

## 4. Crate & Package Topology

| Crate | Path | Responsibility |
|:---|:---|:---|
| [`outfeed`](./src/lib.rs) | `.` | Root facade re-exporting `outfeed_core` domain APIs |
| [`outfeed-core`](./crates/outfeed-core) | `crates/outfeed-core` | Core release engine, SemVer calculator, AST drift inspector, `toml_edit` sync |
| [`outfeed-cli`](./crates/outfeed-cli) | `crates/outfeed-cli` | Clap CLI binary, interactive terminal UI with `indicatif` progress spinners |
| [`outfeed-mcp`](./crates/outfeed-mcp) | `crates/outfeed-mcp` | Stdio Model Context Protocol (MCP) JSON-RPC 2.0 server |
