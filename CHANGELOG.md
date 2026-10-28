# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Documentation
- Elevate all 9 canonical documents to gold standard matching suite specification (`GB-118`).
- Codify 3-phase release pipeline, topological DAG publishing, and lossless manifest invariants in `SPEC.md`.
- Document complete CLI commands and MCP tool matrix in `README.md`.
- Expand release pipeline architecture diagrams and 5 concrete ADRs in `ARCHITECTURE.md` and `DESIGN.md`.

## [v0.1.0-beta.6] - 2026-10-27

### Added
- `outfeed-core`: add crates.io registry sync and topological publish --all (GB-101)
- `outfeed-core`: add disk space pre-flight check and inter-repo cache hygiene (GB-100)
- `outfeed-cli`: live progress spinners, immediate header presentation, and fast suite planning (GB-85)
- `outfeed-cli`: add --yes flag and --jobs concurrency controls to outfeed release (GB-83)
- `outfeed-core`: implement Check -> Plan -> Execute 3-phase suite release pipeline (GB-82)
- `outfeed-core`: expand synthesis max_tokens to 4096 and timeout to 30s for reasoning models (GB-80)
- `outfeed-cli`: enhance outfeed release with progress reporting, --no-llm deterministic flag, and graceful error handling (GB-77)
- `outfeed-cli`: implement outfeed sync to audit and align local checkouts with remotes or suite.lock (GB-75)
- `outfeed-cli`: implement outfeed clone command for suite and selective setup (GB-74)
- `outfeed-core`: orchestrate meta-harness suite release and suite.lock generation (GB-73)
- `outfeed-cli`: add --all suite-wide orchestration flag to outfeed release (GB-72)

### Fixed
- `outfeed-core`: roll back Cargo.lock on release verification failure
- `outfeed-core`: capture verification output, fix misleading step labels, and add --no-verify to release (GB-78)
- `outfeed-core`: treat absence of unreleased commits as no-op in release orchestration

### Maintenance
- `outfeed`: standardize README and establish 9-doc architecture
- `deps`: update Cargo.lock patch tables
- Merge branch 'feat/gb-72-outfeed-release-all' (GB-72)

## [v0.1.0-beta.5] - 2026-10-26

### Fixed
- `outfeed-core`: canonicalize repo_dir when asserting parent workspace branch
- `outfeed-core`: enforce main branch and porcelain tree in release pipeline (GB-69, GB-70)

### Maintenance
- `storypole`: record OF-1 release ticket

## [v0.1.0-beta.4] - 2026-10-26

### Added
- `outfeed`: add indicatif progress spinners, buffer child output, and standard verbosity controls (GB-68)

### Maintenance
- `storypole`: attach GB-55 design plan
- `storypole`: migrate scoped tasks and design plans
- `storypole`: initialize local backlog store with prefix OF

## [v0.1.0-beta.3] - 2026-10-26

### Added
- `outfeed`: add workspace clean and prune hygiene commands (GB-55)
- `outfeed-core`: enforce --locked on cargo install when Cargo.lock is present

## [v0.1.0-beta.2] - 2026-10-26

### Added
- `outfeed-core`: support explicit version override promotions when commits are empty

### Fixed
- `outfeed-core`: import BumpType in engine.rs
- `outfeed-core`: gracefully skip already uploaded crates during publish

### Maintenance
- `deps`: update Cargo.lock for miter 0.1.0-beta.1

## [v0.1.0-beta.1] - 2026-10-26

### Added
- `outfeed`: add packaging and delivery suite — install, push, publish, release
- `outfeed`: initial implementation of neurosymbolic release gate, AST inspector, and changelog synthesizer

