# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
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

