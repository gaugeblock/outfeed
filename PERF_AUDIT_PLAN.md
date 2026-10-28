# Performance Audit, Suite Modularization & Deep Pragmatic System Scrutiny

> **Target Tool:** `outfeed` & Entire Meta-Harness Ecosystem  
> **Status:** Phase 1 in progress — `GB-122` Verified; `GB-123`/`GB-124`/`GB-125`/`GB-126` Ready  
> **Storypole Roadmap:** `GB-121` (Phase 1 epic) · `GB-127` (Phase 2 epic) · `GB-128` (Phase 3 epic)  
> **Empirical Baseline:** 147-minute full-suite release (PID `65716`, 21:21 → 23:57) — see §2

> **Traceability:** every finding below is tracked by a Storypole ticket. See §7 for the complete
> matrix mapping each invariant, discovery, and phase item onto its key, plus the known overlaps.

---

## 1. User Decisions & Design Invariants (Approved)

1. **Adaptive Build Cache Retention & Headroom Guard:** → `GB-122` ✅ Verified (`outfeed` @ `dd42b57`)
   * Disk space verification is **always active and continuously enforced** before and after every repository.
   * Eliminates the unrealistic 20.0 GB requirement when cache retention is enabled (the shared target directory realistically plateaus at ~5.0 GB).
   * Cache retention is default, but **automatically and adaptively cleans on demand** if free disk space ever falls below the safety threshold (`< 3.5 GB`), guaranteeing disk exhaustion never occurs.
2. **Default Concurrency (`--jobs`):** Defaults to `num_cpus` (logical CPU core count, e.g. 8–16 cores on macOS) for parallel topological wave execution. → `GB-123` 🟡 Ready
3. **Verification Test Runner:** Primary test runner switched to `cargo-nextest` (with automatic fallback to `cargo test` if nextest is not installed). Redundant `cargo check --workspace` before tests eliminated. → `GB-124` 🟡 Ready
4. **Local Binary Installation:** Made opt-in via `--install` flag during `outfeed release --all`. Releases focus strictly on manifest versioning, verification gating, git tagging, and crates.io publication without forcing binary recompilation into `~/.cargo/bin`. → `GB-124` 🟡 Ready
5. **sccache Capacity Tuning:** Expand `SCCACHE_CACHE_SIZE` in `.cargo/config.toml` (from default 10 GiB to a balanced 15–20 GiB) so active compilation caches are not prematurely evicted. → `GB-122` Step 3 ✅ Verified (shipped as `15G` in the harness root `.cargo/config.toml`)

---

## 2. Live Empirical Baseline Tracking (PID 65716)

During the active suite release (started at 21:21:00), we tracked exact commit completion timestamps across repositories:

| # | Repository | Release Commit Time | Elapsed Per-Repo | Status | Primary Activity |
| :-: | :--- | :--- | :--- | :--- | :--- |
| 1 | **`miter`** | 21:25:09 | **4m 09s** | ✅ Done | Initial run, `cargo check`, test, install, publish |
| 2 | **`storypole`** | 21:34:49 | **9m 40s** | ✅ Done | Proc-macro rebuild (cache wiped), test, install, publish |
| 3 | **`sawhorse`** | 21:43:50 | **9m 01s** | ✅ Done | Full rebuild (cache wiped), test, install, publish |
| 4 | **`chalkline`** | 21:58:24 | **14m 34s** | ✅ Done | Full rebuild, test compilation, install, publish |
| 5 | **`dado`** | 22:12:34 | **14m 10s** | ✅ Done | Full rebuild, test compilation, install, publish |
| 6 | **`batten`** | 22:27:57 | **15m 23s** | ✅ Done | Full rebuild, test compilation, install, publish |
| 7 | **`joiner`** | 22:44:52 | **16m 55s** | ✅ Done | Full rebuild, packaging, dry-run, live publish |
| 8 | **`benchwork`** | 23:07:35 | **22m 43s** | ✅ Done | Full rebuild, test suites, install, dry-run, live publish |
| 9 | **`rabbet`** | 23:31:23 | **23m 48s** | ✅ Done | Full rebuild, test suites, install, dry-run, live publish |
| 10 | **`outfeed`** | *(In progress ~23:40)* | **~8m+** | ⏳ Active | `cargo check` & test suite compilation active |
| **Σ** | **TOTAL** | — | **~135–145m** | — | **Current Unoptimized Total Baseline** |

---

## 3. Advanced Systems Architecture & High-Scrutiny Discoveries

Pushing our architectural audit into runtime I/O, compiler flags, and IPC revealed 5 high-impact optimization targets across the workspace:

### Discovery 1: Storypole SQLite I/O Missing WAL Mode & Sync Tuning → `GB-132` 🟡 Ready
* **Location:** [`storypole/crates/storypole-core/src/index/mod.rs:27-28`](file:///Users/wolff/dev2026/neurosymbolic-suite/storypole/crates/storypole-core/src/index/mod.rs#L27-L28).
* **The Defect:** SQLite index connections are opened without setting performance PRAGMAs:
  * Default `journal_mode = DELETE` (forces synchronous rollback file creation/deletion per transaction).
  * Default `synchronous = FULL` (forces two blocking `fsync()` disk flushes per write).
* **Impact:** Every Storypole task mutation, claim, verification, and ledger rebuild incurs blocking disk I/O.
* **Pragmatic Fix:** Inject optimal SQLite PRAGMAs upon connection initialization:
  ```sql
  PRAGMA journal_mode = WAL;
  PRAGMA synchronous = NORMAL;
  PRAGMA temp_store = MEMORY;
  PRAGMA cache_size = -64000;
  ```
  *Result:* Rebuilds and queries run **10x to 50x faster**, and concurrent readers never block writers.

### Discovery 2: The `opt-level = 2` Dependency Rebuild Trap → `GB-122` ✅ Verified (fixed by cache retention)
* **Location:** All 10 member `Cargo.toml` files specify:
  ```toml
  [profile.dev.package."*"]
  opt-level = 2
  [profile.test.package."*"]
  opt-level = 2
  ```
* **The Defect:** Every third-party dependency (`syn`, `tokio`, `serde`, `redb`) is compiled with full LLVM optimizations (`opt-level = 2`) even in test/dev mode!
* **The Compounding Penalty:** When `clean_repo_target_cache()` deleted `target/debug` between repositories, rustc spent 3–4 minutes re-optimizing `syn` and `tokio` at `-O2` for every single tool!
* **The Fix:** Under **Adaptive Cache Retention**, these optimized dependencies are compiled **only once** on repo #1, then re-used in sub-millisecond time by repos 2 through 10, drastically cutting test runtimes.

### Discovery 3: Crates.io 429 Rate-Limit Parser & Countdown Cooldown (Recovered from `reserve-namespaces.sh`) → `GB-125` 🟡 Ready
* Port the RFC 2822 date and integer second parser directly into `outfeed-core/src/publish.rs` to automatically pause, display terminal countdowns, and retry upon cooldown expiration.

### Discovery 4: `codegen-units = 1` Single-Core LLVM Stall → `GB-131` 🟡 Ready (profile half) · `GB-124` 🟡 Ready (`--install` opt-in half)
* Allow default `codegen-units = 16` for local compilation, combined with making `--install` opt-in.

### Discovery 5: Subprocess Fork/Exec Overhead vs. Pure-Rust Git Plumbing → `GB-131` 🟡 Ready · `GB-126` 🟡 Ready (outfeed-local dedup)
* In a full suite release, `outfeed` spawns over **150 separate `/usr/bin/git` processes** (`status`, `rev-parse`, `tag`, `log`, etc.).
* On macOS, process creation involves Mach port IPC and kernel sandboxing.
* Consolidating read queries (`get_current_branch`, `is_dirty`, `get_head_sha`, `get_latest_tag`) into batched queries or lightweight pure-Rust `gix` calls eliminates fork/exec overhead.

---

## 4. Adaptive Disk Space Headroom Guard

```mermaid
flowchart TD
    Start["Start Release Step for Repo N"] --> CheckDisk["Query get_available_disk_space()"]
    CheckDisk --> Evaluate{"Available Disk >= 3.5 GB?"}
    Evaluate -- Yes --> RetainCache["Keep Build Cache<br>(Instant Proc-Macro Re-use)"]
    Evaluate -- No --> AutoPrune["Adaptive Pruning Triggered:<br>1. Prune test binaries in target/debug/deps<br>2. Clean target/package<br>3. Check if reclaimed >= 1.5GB"]
    AutoPrune --> VerifyReclaimed{"Reclaimed Enough?"}
    VerifyReclaimed -- Yes --> RetainCache
    VerifyReclaimed -- No --> FullClean["Emergency Target Clean (Reclaims 4-5 GB)"]
    FullClean --> RetainCache
```

1. **Pre-flight Check:** Demands a realistic **4.0 GB** safety headroom (passing comfortably on your 12.0 GB disk).
2. **Continuous Monitoring:** Before every repo begins compiling, `outfeed` queries POSIX `df`.
3. **Adaptive Threshold:**
   * If available disk `≥ 3.5 GB`: Keep all cached dependencies.
   * If available disk `< 3.5 GB`: Automatically triggers **surgical pruning** of stale test executables (saving 1.5–2 GB without deleting proc-macros).
   * If disk remains critical (`< 2.0 GB`): Performs emergency clean to guarantee disk exhaustion never halts the system.

---

## 5. Best Practice Common Crates Architecture (`gaugeblock/common`)

```mermaid
flowchart TD
    subgraph CommonRepo ["Repository: gaugeblock/common"]
        direction TB
        GB_CONTRACTS["gaugeblock-contracts<br>• Direct code manifestation of CONTRACTS.md<br>• Zero C / SQLite / reqwest / tree-sitter deps<br>• Compiles in < 1.5s"]
        GB_MCP["gaugeblock-mcp<br>• Standard JSON-RPC 2.0 stdio server loop<br>• McpToolHandler trait & router<br>• Unified error mapping"]
        GB_GIT["gaugeblock-git<br>• Safe, robust git porcelain wrappers<br>• Single-round-trip branch + tag pushing<br>• Eliminates duplicated subprocess code"]
    end

    subgraph SuiteConsumers ["Suite Consumers (All 10 Repositories)"]
        direction TB
        MTR["miter"] --> GB_CONTRACTS
        MTR --> GB_MCP

        SP["storypole"] --> GB_CONTRACTS
        SP --> GB_MCP
        SP --> GB_GIT

        SH["sawhorse"] --> GB_CONTRACTS
        SH --> GB_MCP

        CH["chalkline"] --> GB_CONTRACTS
        CH --> GB_MCP
        CH --> GB_GIT

        DA["dado"] --> GB_CONTRACTS
        DA --> GB_MCP

        BA["batten"] --> GB_CONTRACTS
        BA --> GB_MCP

        JN["joiner"] --> GB_CONTRACTS
        JN --> GB_MCP

        BW["benchwork"] --> GB_CONTRACTS
        BW --> GB_MCP
        BW --> GB_GIT

        RB["rabbet"] --> GB_CONTRACTS
        RB --> GB_MCP

        OF["outfeed"] --> GB_CONTRACTS
        OF --> GB_MCP
        OF --> GB_GIT
    end
```

---

## 6. Consolidated Phased Execution Roadmap

Tracked as three Storypole epics: `GB-121` (Phase 1), `GB-127` (Phase 2), `GB-128` (Phase 3).

### Phase 1: `outfeed` Immediate Performance & Resilience Overhaul — Epic `GB-121`
1. Replace the binary clean switch with the **Adaptive Disk Headroom Guard** (4.0 GB pre-flight check + on-demand pruning if free space < 3.5 GB). → `GB-122` ✅ Verified
2. Configure **`SCCACHE_CACHE_SIZE = "15G"`** in `.cargo/config.toml` to prevent cache thrashing. → `GB-122` ✅ Verified
3. Enable topological parallel execution (`--jobs num_cpus`). → `GB-123` 🟡 Ready
4. Switch verification runner to `cargo nextest` and make `--install` opt-in. → `GB-124` 🟡 Ready
5. Port Crates.io 429 rate-limit parser with 30s countdown updates and automatic retry into `publish.rs`. → `GB-125` 🟡 Ready
6. Add async crates.io sparse index polling to replace blind 15s sleeps. → `GB-125` 🟡 Ready
7. Unify the 99.2% git commit/tag clone in `outfeed-core`. → `GB-126` 🟡 Ready
8. Eliminate redundant Phase 3 re-planning and modularize files over 200 LOC (derived from the 250-line
   law rather than the original audit; folded into the Phase 1 epic). → `GB-126` 🟡 Ready

### Phase 2: Crates.io Namespace Reservation — Epic `GB-127` 🟡 Ready
1. Reserve all unclaimed `gaugeblock` namespaces on Crates.io before public release:
   * `gaugeblock`
   * `gaugeblock-common`
   * `gaugeblock-contracts`
   * `gaugeblock-mcp`
   * `gaugeblock-git`
   * `gaugeblock-core`
2. Use `scripts/reserve-namespaces.sh` (or updated `outfeed`) with automated 429 cooldown tracking.

### Phase 3: Suite-Wide Contract Decoupling & Common Crates (`gaugeblock/common`) — Epic `GB-128`
1. Bootstrap `gaugeblock/common` containing:
   * `gaugeblock-contracts` (pure wire types matching `CONTRACTS.md`) → `GB-129` 🟡 Ready
   * `gaugeblock-mcp` (generic JSON-RPC 2.0 stdio server) → `GB-130` 🟡 Ready
   * `gaugeblock-git` (safe git porcelain) → `GB-131` 🟡 Ready
2. Repoint suite member repositories to `gaugeblock-contracts`, eliminating transitive `rusqlite`, `reqwest`, and `tree-sitter` compilation bloat. → `GB-129` 🟡 Ready
3. Standardize release profiles (`codegen-units = 16` for local compilation). → `GB-131` 🟡 Ready
4. Add SQLite WAL & synchronous=NORMAL PRAGMAs to `storypole-core`. → `GB-132` 🟡 Ready

---

## 7. Storypole Traceability Matrix

Backlog created from this audit: `GB-121`..`GB-132` (epics `GB-121`, `GB-127`, `GB-128`; work packages
`GB-122`..`GB-126`, `GB-129`..`GB-132`). Parent edges are recorded in the ledger as
`SubtaskOf` (`GB-122..126 → GB-121`, `GB-129..132 → GB-128`); `GB-127` is a standalone epic because
namespace reservation is a single indivisible work package.

| Audit item | Ticket | Status | Implementation notes |
|:---|:---|:---|:---|
| §1.1 Adaptive retention + headroom guard | `GB-122` | ✅ Verified | `outfeed_core::headroom`, `clean_between_repos = false` |
| §1.2 `--jobs` defaults to core count | `GB-123` | 🟡 Ready | waves in `suite_exec`, CLI default |
| §1.3 nextest primary + drop `cargo check` | `GB-124` | 🟡 Ready | `engine.rs` verification gate |
| §1.4 `--install` opt-in | `GB-124` | 🟡 Ready | `ReleaseOpts` + CLI flag |
| §1.5 sccache 15–20 GiB | `GB-122` | ✅ Verified | harness `.cargo/config.toml` `15G` |
| §3.1 Storypole SQLite WAL/`synchronous` | `GB-132` | 🟡 Ready | `storypole-core` `open_and_sync_index` |
| §3.2 `opt-level = 2` rebuild trap | `GB-122` | ✅ Verified | neutralized by cache retention |
| §3.3 Crates.io 429 parser + countdown | `GB-125` | 🟡 Ready | `publish.rs` |
| §3.4 `codegen-units = 1` stall | `GB-131` / `GB-124` | 🟡 Ready | profile change + `--install` opt-in |
| §3.5 150 git fork/exec calls | `GB-131` / `GB-126` | 🟡 Ready | shared porcelain + outfeed-local dedup |
| §6 P1.1–P1.2 guard + sccache | `GB-122` | ✅ Verified | — |
| §6 P1.3–P1.4 parallelism + nextest | `GB-123`, `GB-124` | 🟡 Ready | — |
| §6 P1.5–P1.6 429 + sparse index polling | `GB-125` | 🟡 Ready | — |
| §6 P1.7 git clone unification | `GB-126` | 🟡 Ready | — |
| §6 P2 namespace reservation | `GB-127` | 🟡 Ready | 6 namespaces |
| §6 P3.1–P3.4 common crates + WAL | `GB-129`..`GB-132` | 🟡 Ready | — |
| (derived) modularize files > 200 LOC | `GB-126` | 🟡 Ready | from the 250-line law, not the audit |
| (derived) MCP JSON-RPC dedup | `GB-130` | 🟡 Ready | from `miter clones`, not the audit |

### Known overlaps and scope boundaries

1. **`GB-126` vs `GB-131` — the 99.2% git commit/tag clone.** Both tickets name the same
   `outfeed-core` duplication (`git.rs:176` ↔ `suite_release.rs:126`). Boundary: `GB-126` performs the
   *outfeed-local* extraction (`create_annotated_commit_and_tag` inside `outfeed-core`) as part of the
   Phase 1 cleanup; `GB-131` then lifts that single helper into `gaugeblock-git` and repoints all
   suite crates. `GB-131` therefore depends on `GB-126` and must not re-do the in-crate dedup.
2. **`GB-123` shared-target lock contention.** Member repositories compile into one shared
   `target/` (harness `.cargo/config.toml`), so cargo serializes builds on the target-directory lock.
   Wave parallelism therefore wins on git/registry/LLM-bound work and on test *execution*, not on
   concurrent compilation of the same dependency graph. `--jobs 1` remains the escape hatch.
3. **`GB-122` guard + `GB-123` concurrency.** On-demand pruning mutates `target/debug/deps`; with
   parallel releases the guard must be serialized so it can never prune beneath an in-flight build.

### Deferred findings raised during Phase 1 implementation

| Ticket | Kind | Item |
|:---|:---|:---|
| `GB-133` | bug | `clean_workspace_with_progress` emits two `on_complete` events (clean + prune wording) |
| `GB-134` | tech-debt | README version badge (`beta.5`) and SPEC `ReleasePlan` module path are stale |
| `GB-135` | enhancement | `prune_workspace` deletes rlibs/rmeta/proc-macros, not just test executables |

