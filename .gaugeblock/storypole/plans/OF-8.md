# GB-68: Enhance outfeed CLI with indicatif Spinners, Captured Subprocess Execution, and Standard Verbosity Controls

> **Status:** Ready  
> **Target Scope:** `outfeed` (`outfeed-core`, `outfeed-cli`)  
> **Key Invariants:**  
> 1. Hard 250 LOC rule on all `.rs` files.  
> 2. Zero-panic safety: no `.unwrap()` / `.expect()` in production library paths (`thiserror`).  
> 3. Clean terminal hygiene: child processes (`git`, `cargo`) never dump raw stdout/stderr directly to console handles; output is captured, structured, and only surfaced on failure or `--verbose`.  
> 4. Decoupled domain engine: `outfeed-core` defines a lightweight `ProgressReporter` trait / observer so CLI rendering (`indicatif`) does not leak into library/MCP consumers.  
> 5. Non-TTY resilience: dynamic spinners gracefully degrade to clean milestone lines when not on an interactive TTY, and are completely suppressed in `--json` and `-q`.  

---

## 1. Executive Summary & Problem Statement

Currently, when executing batch commands across the meta-harness such as `outfeed push --all` or `outfeed install --all`, child processes (`git push`, `cargo install`) run directly via `.status()`, inheriting stdout and stderr from the parent terminal.

This causes raw subprocess messages (such as 20 consecutive un-prefixed lines of `Everything up-to-date`) to stream into the terminal with no repository indication, before outfeed suddenly dumps its completion emojis at the very end:
```text
➜  neurosymbolic-suite git:(main) outfeed push --all
Everything up-to-date
Everything up-to-date
... (20 lines) ...
🚀 miter: pushed branch 'main' and tag 'v0.1.0-beta.1' to origin
🚀 storypole: pushed branch 'main' and tag 'v0.1.0-beta.2' to origin
```

Furthermore, outfeed lacks unified verbosity controls (`-v`, `-vv`, `-q`), making it impossible to quiet down progress during automation or inspect child command details without hacking debug prints.

`GB-68` resolves this across all multi-repo commands (`push`, `install`, `release`, `clean`, `publish`):
1. **Dynamic Progress & Spinners**: Uses `indicatif` (v0.18, matching `miter`) with `MultiProgress` and dynamic per-repo spinners (`⠋`) transitioning into clean colored status icons (`✔`, `✖`, `⊘`).
2. **Subprocess Capturing**: Standardizes child process execution to buffer stdout and stderr. On success in normal mode, the terminal remains clean. On error, or with `-v` (verbose), the buffered output is printed indented and prefixed with the repo name (e.g. `  [miter] Everything up-to-date`).
3. **Standard Verbosity Flags**:
   - Default: clean interactive spinners, warning-level logs.
   - `-v, --verbose` (counted): `-v` enables info logs + repo-prefixed child output; `-vv` enables debug tracing.
   - `-q, --quiet`: suppresses all non-essential output, spinners, and progress bars; emits only errors.
   - `--json`: machine-readable output with progress bars automatically hidden.
4. **TTY / CI Auto-Detection**: Detects `is_terminal()`; renders dynamic spinners on interactive terminals, linear milestone lines in non-TTY/CI, and hides spinners entirely in quiet/JSON modes.

---

## 2. Resolved Architectural Decisions (from /grill-me)

| Decision ID | Question | Chosen Option | Rationale |
|---|---|---|---|
| `2f8dd71d` | Visual progress style and engine | `indicatif MultiProgress / Spinners` | Dynamic spinners for active repos showing current action, settling into clean status lines; aligns with `indicatif 0.18` already in workspace (`miter`). |
| `95d45ab7` | Subprocess stdout/stderr handling | `Capture by default; display on error or verbose` | Keeps terminal clean during normal execution; displays full diagnostics indented by repo on failure or `-v`. |
| `d51a03c0` | CLI verbosity flag convention | `Multi-level -v/-vv count + -q/--quiet` | Standard Rust CLI pattern (`-q` quiet, default clean spinners, `-v` verbose with child output, `-vv` debug tracing). |
| `5ba1a0b9` | Non-interactive / CI / pipes behavior | `Auto-detect TTY: dynamic on TTY, linear on non-TTY, hidden in quiet/json` | Prevents ANSI redraw corruption in CI logs or pipes while retaining interactive richness on real terminals. |
| `86aa8bf2` | Rollout scope across subcommands | `Full suite rollout (push, install, release, clean, publish)` | Delivers consistent developer experience across all multi-repo operations. |
| `23502b4c` | Terminal iconography & styling aesthetic | `Modern CLI styling (⠋, ✔, ✖, ⊘)` | Clean, subtle modern CLI aesthetic matching modern developer tools like cargo, gh, and docker. |

---

## 3. Architecture & File Layout

```text
outfeed/crates/outfeed-core/src/
├── progress.rs        # ProgressReporter trait, NoopProgress, and RepoProgressEvent (<150 LOC)
├── cmd.rs             # Subprocess runner capturing stdout/stderr into CommandOutput (<150 LOC)
├── push.rs            # Updated push_repo/push_all to use runner and ProgressReporter (<200 LOC)
├── install.rs         # Updated install_repo/install_all to use runner and reporter (<200 LOC)
├── publish.rs         # Updated publish_repo to use runner and reporter (<200 LOC)
├── clean.rs           # Updated clean_workspace to report progress (<200 LOC)
└── lib.rs             # Re-exports progress types and CommandOutput

outfeed/crates/outfeed-cli/src/
├── args/
│   └── mod.rs         # Add global -v/--verbose (count) and -q/--quiet flags (<150 LOC)
├── progress/
│   ├── mod.rs         # Progress factory returning appropriate reporter based on TTY/flags (<100 LOC)
│   ├── multi.rs       # indicatif MultiProgress reporter for interactive terminals (<200 LOC)
│   └── text.rs        # Plain milestone-line reporter for non-TTY/CI (<120 LOC)
├── commands/
│   ├── push.rs        # Wire progress reporter and verbosity (<120 LOC)
│   ├── install.rs     # Wire progress reporter and verbosity (<120 LOC)
│   ├── clean.rs       # Wire progress reporter and verbosity (<120 LOC)
│   ├── publish.rs     # Wire progress reporter and verbosity (<120 LOC)
│   └── release.rs     # Wire progress reporter and verbosity (<150 LOC)
└── main.rs            # Initialize tracing-subscriber dynamically from verbosity flags (<100 LOC)
```

---

## 4. Execution Steps & Verification Gates

1. **Step 1: Core Progress Observer & Subprocess Runner (`outfeed-core`)**
   - Define `ProgressReporter` trait and event models in `outfeed-core/src/progress.rs`.
   - Implement `run_command` in `outfeed-core/src/cmd.rs` capturing stdout/stderr into `CommandOutput`.
   - *Verification Gate:* `cargo check -p outfeed-core` and unit tests in `crates/outfeed-core/tests`.

2. **Step 2: Core Subcommand Adaptation (`outfeed-core`)**
   - Update `push.rs`, `install.rs`, `publish.rs`, and `clean.rs` to execute via `cmd.rs` and notify `ProgressReporter`.
   - *Verification Gate:* `cargo test -p outfeed-core`.

3. **Step 3: CLI Progress Reporter & Verbosity Setup (`outfeed-cli`)**
   - Add `indicatif = "0.18"` to `outfeed-cli/Cargo.toml`.
   - Implement interactive `MultiProgress` and non-TTY fallback in `outfeed-cli/src/progress/`.
   - Add `-v` / `-vv` (count) and `-q` (flag) to `outfeed-cli/src/args/mod.rs` and configure `tracing-subscriber`.
   - *Verification Gate:* `cargo check -p outfeed-cli`.

4. **Step 4: CLI Subcommands Wiring (`outfeed-cli`)**
   - Wire reporter into `push`, `install`, `publish`, `clean`, and `release` CLI handlers.
   - Print repo-prefixed child output when `-v` is active or on error.
   - *Verification Gate:* `cargo clippy -p outfeed-cli --all-targets -- -D warnings` and `cargo test -p outfeed-cli`.

5. **Step 5: End-to-End Suite Dogfooding & Binary Update**
   - Install binary into `~/.cargo/bin` via `cargo install --path outfeed/crates/outfeed-cli --force`.
   - Verify `outfeed push --all` (and `--dry-run`), testing interactive TTY, piped non-TTY, `-v`, and `-q`.
   - *Verification Gate:* Full Tier 3 workspace checks and Storypole verification receipt.
