# GB-70: Enforce main branch, porcelain tree checks, and clean Storypole release commit

> **Status:** InProgress  
> **Target Scope:** `outfeed-core`, `outfeed-cli`  
> **Key Invariants:**  
> 1. Hard 250 LOC rule on all `.rs` files.  
> 2. Zero-panic safety: no `.unwrap()` / `.expect()` in production library paths (`thiserror`).  
> 3. Releases MUST occur strictly on branch `main` with a clean working tree across both target repo and meta-harness root.  
> 4. `outfeed apply` MUST NOT leave a dirty `.storypole/ledger.jsonl`: the release ticket must be created and certified before the release commit is made.  

---

## 1. Executive Summary & Root Cause

1. **Branch Enforcement Gap**:
   `OutfeedError::InvalidBranch(String)` was defined in `error.rs`, but `assert_main_branch` was never implemented or invoked in `plan_release` and `apply_release`. Furthermore, when targeting a member repository within a meta-harness, only the child directory was inspected, ignoring the parent workspace's active branch.

2. **Dirty Ledger on Release**:
   `apply_release` performed git commit and tag *before* creating and certifying the Storypole release ticket. As a result, the release ticket appended to `.storypole/ledger.jsonl` was uncommitted, leaving the working tree dirty immediately after release.

---

## 2. Implementation Steps

1. **Step 1: Implement `assert_main_branch` in `outfeed-core::git`**
   - Check `get_current_branch(repo_dir)? == "main"`.
   - Traverse upward to detect any enclosing parent Git repository (the meta-harness workspace root) and ensure it is also on `main` and has a clean working tree.
   - *Verification Gate:* `cargo check -p outfeed-core`.

2. **Step 2: Enforce checks in `plan_release` and `apply_release`**
   - Call `assert_main_branch(repo_dir)?` at the entry of both `plan_release` and `apply_release`.
   - *Verification Gate:* `cargo test -p outfeed-core`.

3. **Step 3: Reorder Storypole Ticket Creation before Release Commit**
   - In `apply_release`, create and certify the Storypole release ticket *before* calling `create_release_commit_and_tag`.
   - `git add -A` within `create_release_commit_and_tag` will stage the updated `.storypole/ledger.jsonl` into the atomic release commit.
   - *Verification Gate:* `cargo test -p outfeed-core`.

4. **Step 4: Dogfooding & Binary Update**
   - Test `outfeed apply` and ensure working tree remains 100% clean (`git status` is empty).
   - Recompile and reinstall binary into `~/.cargo/bin`.

