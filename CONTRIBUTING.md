# Contributing to `outfeed`

Thank you for contributing to `outfeed`! We welcome contributions to our release engine, SemVer calculation algorithms, manifest synchronizers, and delivery orchestration pipelines.

---

## 1. Prerequisites

* **Rust Toolchain:** Rust 2024 edition (stable 1.85+ / nightly for bleeding-edge checks).
* **Git Version:** Git 2.30+ with porcelain status support.
* **Testing Runners:** `cargo-nextest` (recommended) or native `cargo test`.
* **Suite Tooling:** Installed `storypole`, `chalkline`, and `miter` on `PATH`.

---

## 2. Development Workflow

1. **Claim a Task:**
   ```bash
   storypole claim <KEY> <YOUR_ID> --branch feat/<slug>
   ```
2. **Dedicated Branch:**
   Always develop non-trivial tickets on dedicated feature branches (`feat/<slug>` or `fix/<slug>`).
3. **Inner-Loop Iteration (Tier 1):**
   ```bash
   cargo check -p outfeed-core -p outfeed-cli
   cargo nextest run -p outfeed-core
   ```
4. **Pre-Commit Verification Gate (Tier 2):**
   Before committing, verify code formatting, clippy lints, tests, and documentation drift:
   ```bash
   cargo fmt --check -p outfeed-core -p outfeed-cli -p outfeed-mcp
   cargo clippy -p outfeed-core -p outfeed-cli -p outfeed-mcp --all-targets -- -D warnings
   cargo test -p outfeed-core -p outfeed-cli
   chalkline docs check -d .
   ```
5. **Commit with Conventional Commits:**
   ```bash
   git commit -m "feat(outfeed-core): support sparse index polling for dry-run publish"
   ```

---

## 3. Engineering Invariants & Coding Standards

* **Hard 250-Line File Limit:** No `.rs` source file may exceed 250 lines. Modularize submodules cleanly.
* **Strict Error Handling:** Library crates (`outfeed-core`) must use strongly typed `OutfeedError` via `thiserror`. Never use `.unwrap()` or `.expect()` in library code.
* **Lossless Manifest Invariant:** Never modify TOML manifests using regex find-replace; use `toml_edit` AST methods exclusively.
* **9-Document Compliance:** Maintain 100% compliance across all 9 canonical documents with zero drift. Run `chalkline docs check -d .` to certify.
