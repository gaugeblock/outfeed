# `outfeed` — Product Vision & Strategy

> **Domain:** Neurosymbolic Release Gate & Delivery Engine | **Suite:** `gaugeblock`

`outfeed` is the packaging, release, and delivery engine for the `gaugeblock` neurosymbolic suite, transforming commits and verified code into deterministic SemVer releases, human-grade changelogs, and synchronized packages.

---

## 1. Personas & Problems

| Persona | Pain Points | How `outfeed` Solves It |
|:---|:---|:---|
| **Release Engineer** | Publishing multi-crate workspaces is stressful, error-prone, and full of dependency race conditions on crates.io. | Automates topological leaf-first publishing with automated sparse-index polling and dry-run validation. |
| **Meta-Harness Maintainer** | Coordinating version alignment across 10 independent Git repositories causes configuration drift and broken lockfiles. | Provides `suite.lock` ecosystem synchronization, multi-repo status audits, and one-command suite releases (`outfeed release --all`). |
| **Open-Source Contributor** | Unclear release rules, manual changelog writing, and accidentally broken manifest formatting during version bumps. | Offers automated conventional commit parsing, `sawhorse` AI changelog synthesis, and lossless `toml_edit` AST bumping. |
| **Autonomous Systems Agent** | Shelling out to raw Git and Cargo commands lacks typed receipts, structured errors, and interactive safeguards. | Exposes clean, structured Model Context Protocol (MCP) tools (`outfeed_plan`, `outfeed_apply`) with staged approvals. |

---

## 2. Jobs to Be Done (JTBD)

1. **Deterministic SemVer Calculation:** When cutting a new release, determine the exact SemVer increment based on Conventional Commits and AST signature diffs without human guesswork.
2. **Lossless Manifest Synchronization:** When bumping workspace versions, update root and member `Cargo.toml` files and dependency constraints without corrupting human comments or whitespace.
3. **Architectural Changelog Authoring:** When preparing release announcements, synthesize human-grade, architectural release notes from commits, PR descriptions, and Storypole task plans.
4. **Multi-Repo Ecosystem Delivery:** When publishing a milestone release, orchestrate the verification, tagging, binary installation, git push, and crates.io publishing across all 10 suite repositories.

---

## 3. Product Success Metrics

* **Zero Failed Releases:** 0% failure rate during crates.io publishing due to dependency ordering or index lag.
* **Manifest Comment Fidelity:** 100% of comments, formatting, and table ordering preserved in updated `Cargo.toml` manifests.
* **Planning Velocity:** $\le 5$ seconds to compute a full single-repo release plan; $\le 30$ seconds across the full 10-repo suite.
* **Reproducibility:** 100% deterministic alignment of suite repository checkouts when synchronizing with `suite.lock`.
