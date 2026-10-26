# `outfeed`

> **Phase 9:** Neurosymbolic release gate, AST drift inspector, and changelog synthesizer for the `gaugeblock` ecosystem.

`outfeed` is the release and delivery engine of the `gaugeblock` meta-harness. Named after the woodworking outfeed table—which receives, supports, and guides finished workpieces as they exit the machine—`outfeed` manages precision SemVer calculation, Conventional Commit grouping, `toml_edit` manifest synchronization, and LLM-assisted changelog synthesis.

## Features

* **Deterministic SemVer Calculation:** Analyzes Conventional Commits (`feat:`, `fix:`, `BREAKING CHANGE:`) between the latest git tag and `HEAD`.
* **AST Drift & Contract Inspection:** Validates public signatures via `miter` to ensure breaking API changes receive appropriate major/minor base bumps.
* **Neurosymbolic Changelog Synthesis:** Ingests commits, diffs, and Storypole task plans, leveraging `sawhorse` to write architectural, human-grade changelogs.
* **Lossless Manifest Synchronization:** Uses `toml_edit` to bump workspace `Cargo.toml`, member crate dependencies, and web `package.json` without destroying comments or formatting.
* **Closed-Loop Storypole Integration:** Automatically creates release chore tickets, stamps shipped tasks, and surfaces releases in Benchwork.
* **Native MCP Support:** Exposes `outfeed_plan` and `outfeed_apply` for seamless operation inside Antigravity and Cline.

## License

Apache-2.0

