# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `outfeed-core`: the gate's `contracts` step builds the index it reads, into a temporary database, instead of failing when a member repository has none. Verifying any member-scoped ticket ran a chain whose contracts step refused from a clean checkout until somebody ran `miter index` there by hand - the one situation a gate exists for. The step also passes the suite's contracts allowlist, because the non-mirror declarations live at the suite root and a member checking without them reported the suite's own decision as drift (`role_override_config.api_key` failed a clean `benchwork` gate). The temporary database is removed afterwards, so a gate leaves no artefact and a stale index can never decide a result (OF-39).
- `outfeed-cli`: `reserve scaffold` no longer stamps the suite's identity into a stranger's placeholder. `--homepage` and `--license` are exposed (neither had a flag before, while both were defaults), `--repository` loses its `github.com/gaugeblock/{name}` default, and all three are required - supplied by flag, or derived from the caller's `origin` remote, or the command fails naming what to pass. A mono repository that publishes several crates omits `{name}` and every placeholder links to the one repository. The SPDX expression is carried verbatim, including `OR` expressions, because a placeholder claiming one half of a dual-licensed project is a licensing statement nobody made, and published metadata is permanent (GB-364).
- `outfeed-core`: `rate_limit::parse_cooldown` now reads the RFC 2822 deadline that crates.io embeds in a 429 message body (`Please try again after Wed, 28 Oct 2026 15:30:25 GMT`) instead of falling back to 30 s, and it no longer mistakes a day-of-month for a second count; explicit `Retry-After` values still win (GB-140).

### Added
- `outfeed-core`: crates.io rate-limit resilience — `rate_limit` parses `429`/`Retry-After` (integer seconds or RFC 2822 dates), reports a 30-second countdown and retries a publication up to three times; `index_poll` queries the crates.io sparse index (shard-aware) to wait for propagation instead of sleeping blindly (GB-125).

### Changed
- `outfeed-core`: registry lookups prefer one sparse-index request over a `cargo search` process, falling back to the CLI parser when the index is unreachable (GB-125).

### Fixed
- `outfeed-core`: `outfeed prune` no longer deletes compiled dependencies, proc-macro libraries or dep-info files; pruning is now limited to extension-less executables and directories via `clean::is_prunable_artifact`, so a prune can never trigger a rebuild wave (GB-135).

### Changed
- `outfeed-core`: `outfeed release --all` now releases from the Phase 2 plan instead of re-planning every repository, eliminating a second `git log` walk and changelog synthesis per release; `plan_suite_releases` honours its `no_llm` argument (GB-126).
- `outfeed-core`: the 99.2% clone between single-repository and suite commit/tag flows is unified into `git::stage_commit_and_tag` (GB-126).
- `outfeed-core`: `clean.rs` and `semver.rs` are split into modules (`clean/{mod,size}.rs`, `semver/{mod,commit}.rs`) and the wave worker pool moved to `suite_waves/pool.rs`, keeping every source file inside the 250-line law (GB-126).

### Added
- `outfeed-core`: `outfeed_core::verify_runner` selects `cargo-nextest` when it is installed (falling back to `cargo test`) and runs doctests separately, so Tier 4 verification stops paying for two full compiles (GB-124).

### Changed
- `outfeed-core`: `Cargo.lock` is synchronized with `cargo update --workspace --offline` instead of a full redundant `cargo check --workspace` (GB-124).
- `outfeed-core` / `outfeed-cli`: binary installation is now opt-in — `ReleaseOpts::skip_install` defaults to `true` and `outfeed release` installs only with the new `--install` flag, replacing `--skip-install` (GB-124).

### Documentation
- `outfeed`: refresh the README version badge to `0.1.0-beta.7` and correct the SPEC symbol table (`ReleasePlan` module path, `RepoStatus` is an enum, drop the removed `DeliveryOpts`); all 36 documented symbols now resolve to matching AST declarations (GB-134).

### Fixed
- `outfeed-core`: `clean_workspace_with_progress` emits a single completion event instead of reporting both `reclaimed` and `pruned` for the same pass (GB-133).

### Added
- `outfeed-core`: adaptive disk space headroom guard with continuous pre/post-repository monitoring, surgical test-binary pruning below 3.5 GB, and emergency target clean below 2.0 GB (GB-122).
- `outfeed-core`: `outfeed_core::headroom` module exposing `enforce_disk_headroom`, `prune_test_binaries`, and `clean_target_package` (GB-122).
- `outfeed-core`: `outfeed_core::suite_waves` partitions the suite plan into five leaf-first dependency waves and runs each wave through a bounded worker pool, with per-worker Tokio runtimes and mutex-serialized disk-guard passes (GB-123).
- `outfeed-cli`: `outfeed release` defaults `--jobs` to the logical CPU count instead of `1` (GB-123).

### Changed
- `outfeed-core`: cache retention between suite repositories is now the default (`ReleaseOpts::clean_between_repos = false`), reusing optimised dependency and proc-macro artifacts across all 10 releases (GB-122).
- `outfeed-core`: suite pre-flight disk headroom is a single realistic 4.0 GB threshold; the 20.0 GB no-clean reservation is gone (GB-122).
- `outfeed-core`: `execute_suite_plan` releases repositories wave-by-wave instead of one serial loop; `jobs = 1` keeps the previous strictly serial path (GB-123).
- `outfeed-cli`: `outfeed release --no-clean-between-repos` replaced by opt-in `--clean-between-repos` (GB-122).

## [v0.1.0-beta.7] - 2026-05-09

### Maintenance
- Elevated the 9-document architecture to gold standard for `outfeed` (GB-118).

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

