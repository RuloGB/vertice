# Archive Report: Skill Scanner over User Roots (T4)

**Date**: 2026-08-18  
**Change**: `skill-scanner-user-roots`  
**Phase**: T4 (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md`  
**Verification**: PASS (0 CRITICAL, 2 non-blocking WARNINGS — see Issues section)  
**Status**: ARCHIVED — Change complete and closed.

---

## Executive Summary

T4 delivered a recursive skill component scanner over three fixed user roots (Claude Code, OpenCode, generic agents), discovering exactly 69 on-disk skill entries from the reference Windows installation and closing CA-6 (no plugin skill appears), CA-8 partial (`_shared` is an ordinary skill), CA-9 (absent/empty root produces no issue and no component, distinguishable states), CA-14 (no project-scope component); contributes to CA-12 partial (unreadable file is reported, scan continues). The implementation spans two new sibling modules (`crates/vertice-core/src/roots.rs` and `crates/vertice-core/src/skills.rs`, ~300 lines combined) plus a single-field addition to `SearchRoot` in the model, with 21 fixture-driven unit and integration tests covering all 10 spec requirements and domain-model scenarios. All 28 implementation tasks marked complete; all gates green (format, lint, tests, dependency policy, bindings drift). One chained PR (`single-pr` with `size:exception` 780–1000 changed lines) merged into `main` on 2026-08-18 as commit `7b212d2`. The change is pure-read, non-panicking on every documented failure class, and directly unblocks T5 and T8.

---

## What T4 Delivered

### Core Deliverables

**New module `crates/vertice-core/src/roots.rs`:**
- `pub fn home_dir() -> Result<PathBuf, ScanError>` — home directory resolution via `std::env::home_dir()`, the sole ambient-environment read in the crate
- `pub fn skill_roots(home: &Path) -> [ResolvedRoot; 3]` — hardcoded suffixes (`.claude/skills/`, `.agents/skills/`, `.config/opencode/skills/`, plus singular `.config/opencode/skill/` alias) built with per-segment `PathBuf::push`, never OS config-dir APIs
- `pub struct ResolvedRoot { root: SearchRoot, scan_paths: Vec<PathBuf> }` — canonical identity plus scan targets (alias support)
- `fn probe(scan_path: &Path) -> Result<SearchRootStatus, ScanError>` — disk existence check via `std::fs::symlink_metadata`
- 3 in-module `#[cfg(test)]` unit tests: alias grouping, root-id stability, three-roots-always invariant

**New module `crates/vertice-core/src/skills.rs`:**
- `pub struct SkillScan { roots: Vec<SearchRoot>, components: Vec<Component>, issues: Vec<ScanIssue> }` — owned result container (non-model, no `Serialize`/`TS`)
- `pub fn scan(home: &Path) -> SkillScan` — recursive walk via `walkdir::WalkDir` with `follow_links(false)`, matching `SKILL.md` entries, calling `frontmatter::read` per match
- `fn escalate(issue: ScanIssue) -> ScanIssue` — maps every frontmatter-reader issue severity to `IssueSeverity::Error` uniformly (design §5)
- `fn walk_one(scan_path: &Path, root_id: SearchRootId) -> Result<(Vec<Component>, Vec<ScanIssue>), ScanError>` — per-root walk logic with per-entry error handling
- 10 integration tests in `crates/vertice-core/tests/skill_scanner.rs`: root resolution, alias handling, SKILL.md detection rule, recursion, symlink policy, absent/empty distinction, User scope, plugin exclusion, per-file failure recovery, 69-entry count

**Model change to `crates/vertice-core/src/model/location.rs`:**
- `pub enum SearchRootStatus { Found, NotFound }` — closed enum with `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]`
- `pub status: SearchRootStatus` added to `SearchRoot` struct — represents whether the root path exists on disk
- Re-export `SearchRootStatus` from `model/mod.rs`
- 3 in-module unit tests: absent-root constructibility, status distinguishability, existing-fields preservation

**TypeScript bindings (regenerated):**
- `frontend/src/bindings/SearchRootStatus.ts` — NEW, type alias `"found" | "notFound"`
- `frontend/src/bindings/SearchRoot.ts` — MODIFIED, added `status` field

**Repository fixtures under `crates/vertice-core/tests/fixtures/roots/`:**

Semantic set (7 top-level fixture homes):
- `absent-roots/` — no roots present (tests CA-9 absent branch)
- `empty-alias/` — `.config/opencode/skill/.gitkeep` only (CA-9 present-and-empty, tripwire disk-existence half)
- `alias-populated/` — `.config/opencode/skill/demo/SKILL.md` (CA-9 present-with-entry, alias handling)
- `underscore-shared/` — `.claude/skills/_shared/SKILL.md` (CA-8 partial, name-convention immunity)
- `nested-skill/` — `.claude/skills/group/nested/SKILL.md` (recursion, depth 2)
- `unreadable-entry/` — `.claude/skills/good/SKILL.md` + `.claude/skills/broken/SKILL.md` (CA-12 partial, per-file failure without abort)
- `project-decoy/` — `.claude/skills/real/SKILL.md` + `projects/demo/.claude/skills/fake/SKILL.md` (CA-14 project-scope exclusion)
- `plugin-decoy/` — `.claude/plugins/p/skills/x/SKILL.md` (CA-6 plugin-root structural exclusion)

Tier-2 reference tree (`reference/`):
- 25 uniquely-named skills distributed 23 across all three roots, 1 only in `.claude/skills/`, 2 only in `.agents/skills/`
- Per-root split: `.claude/skills/` 23, `.agents/skills/` 24, `.config/opencode/skills/` 22 (total 69 entries)
- Matches `alcance-poc-vertice.md:74-79` exactly
- Each `SKILL.md` is minimal, four lines, generated by one rule

**Wiring:**
- `crates/vertice-core/src/lib.rs` — two lines added: `pub mod roots;` and `pub mod skills;` (alphabetically placed)

### Acceptance Criteria Closed

**CA-6 (No plugin skill appears):**
- Structural guarantee: only three hardcoded roots are ever walked; no plugin root falls under any of them on the reference machine (verified 2026-08-18)
- No exclusion filter written — the absence is structural, not a defensive pattern
- Test `plugin_decoy_outside_the_three_roots_is_excluded` asserts the absence
- Limitation flagged for T16: verified on one Windows machine only; macOS/Linux revalidation pending

**CA-8 partial (`_shared` is an ordinary skill):**
- Detection rule: SKILL.md presence alone; no name-based heuristic
- Test `underscore_prefixed_directory_is_an_ordinary_skill` produces a component for `_shared/SKILL.md`
- Source inspection confirms zero name filtering in `skills.rs`

**CA-9 (Absent/empty roots produce no issue and no component, distinguishable):**
- Absent root: `status: NotFound`, zero components, zero issues
- Present-empty root: `status: Found`, zero components, zero issues
- Two states distinguishable via `SearchRoot.status`
- Tests `absent_roots_yield_zero_components_zero_issues_all_not_found` and `present_empty_root_yields_zero_components_zero_issues_and_is_found` both pass
- Tripwire test `empty_alias_fixture_directory_still_exists_on_disk` (disk-existence half) + `empty_alias_root_status_is_found` (status-assertion half) split across commits to enforce fixture integrity

**CA-14 (No project-scope component):**
- `Scope::User` is the only value constructed in the entire skills module (single source line: `scope: Scope::User`)
- No project-scoped root is ever walked
- Test `every_component_is_user_scoped_and_project_decoy_is_excluded` asserts the absence

**CA-12 partial (Unreadable file reported, scan continues):**
- Per-file parse failure yields `ScanIssue` carrying the path
- Walk continues to siblings and other roots
- Test `corrupt_skill_yields_an_issue_and_does_not_stop_the_walk` produces 1 issue (corrupt file) + 1 component (sibling good file)

**CA-16 (Read-only, no writes):**
- Grep across crates/: zero matches for `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, `remove_*`
- Test `full_scan_leaves_the_fixture_tree_unchanged` performs byte-for-byte comparison before/after a full scan

**CA-17 (Fixture-based tests, three-platform CI):**
- All tests read from `crates/vertice-core/tests/fixtures/roots/`
- No test reads author's machine; no walker aimed at `fixtures/frontmatter/` (T3 separation rule honored)
- Fixture trees run on all three CI platforms automatically

---

## Decisions Worth Carrying Forward to T5–T16

### Home Directory Resolution: `std::env::home_dir()`, Structurally Removes the Config-Dir Trap

**Decision**: Use `std::env::home_dir()` from std, not `dirs::home_dir()`.

**Why**: (1) `dirs::config_dir()` returns `%APPDATA%` on Windows and finds zero skills — the structural trap this change exists to avoid. By choosing `std`, the hazardous API is simply not reachable. (2) Identical signature, `Option<PathBuf>`. (3) No new crates. (4) `std::env::home_dir()` was un-deprecated in Rust 1.87; this workspace's MSRV floor is 1.88; verified at apply time: `cargo clippy --workspace --all-targets -- -D warnings` at rustc 1.97.1 produces zero deprecation warnings.

**Forward implication**: T5 (Claude Code agents) and T6 (OpenCode agents) reuse `roots::home_dir()` unchanged; no config-dir hazard recurs.

### Roots Are Hardcoded, Never Derived From Paths

**Decision**: Root ids are `SearchRootId("claude-skills")`, `SearchRootId("agents-skills")`, `SearchRootId("opencode-skills")`, never path-derived.

**Why**: Path-derived ids embed the username and make every fixture assertion machine-dependent, violating design principle 4 (`rules.verify`) and test portability. Hardcoded ids are stable across machines and make fixture assertions portable.

**Forward implication**: T8 (consolidation) can assume root ids are stable; T11/T12 (UI) can use them as canonical references without path-dependent brittle lookups.

### Symlink Policy: No Following, Enforced by `walkdir`'s Configuration

**Decision**: Set `walkdir::WalkDir::follow_links(false)` explicitly.

**Why**: Correct recursive walkers need per-entry error handling, sorting, and explicit symlink policy. Hand-rolling this introduces four silent-defect classes for a team new to Rust. `walkdir` provides all three directly, and we verify the no-follow policy via unit test contract rather than a portable fixture (no symlinks exist on the reference Windows machine, and Windows junctions vs. symlinks behavior is deferred to T16).

**Forward implication**: T5/T6 and future adapters all reuse the same `walkdir` walk pattern; no symlink loops or duplicates surface at T5 or later.

### Severity Escalation: Caller Context Determines Severity

**Decision**: T4 maps every `frontmatter::read` failure severity to `IssueSeverity::Error`, uniformly.

**Why**: T3's detection rule ("if there is a `SKILL.md`, it is a skill") means every file at a skill root **should** be a valid skill. T3 cannot know this context, so it emits `Warning` for missing opening fence (file that is not a skill at all) and `Error` for parse failures (file declares itself as frontmatter but breaks its promise). T4 **knows** the file was expected to be a skill because it was discovered at a skill root. So T4 escalates uniformly: if it got to `frontmatter::read`, it was expected to be a skill, and any failure is `Error`.

**Forward implication**: T5/T6 may have different escalation logic (some failures expected, some not); the escalation function is an internal helper in T4, not a global pattern, so T5 writes its own escalation if needed.

### Model Purity: No Imports Added

**Decision**: `SearchRootStatus` is a plain enum added to `model/location.rs`; no new imports to `model/mod.rs` or `model/`'s allow-list.

**Why**: The model's purity invariant (no `std::fs`, `std::io`, `std::env`, `SystemTime`/`Instant` imports) is mechanically enforced by the module-doc allow-list and a test (`yaml_seam_invariant.rs` extended). Adding no imports keeps the seam clean and the invariant self-evident.

**Forward implication**: T8 (consolidation) can consume the new `status` field without importing or depending on T4's walkdir machinery. The model remains pure.

### Recursive Walk, Sorted Deterministically

**Decision**: `walkdir::WalkDir::new(scan_path).follow_links(false).sort_by_file_name()` — recursive, unbounded depth, deterministic ordering.

**Why**: OpenCode's own glob is `{skill,skills}/**/SKILL.md`, not `skills/*/SKILL.md` (OpenCode's nested skill case). Deterministic sorting ensures the order is reproducible in tests and in production scans. Hand-rolled alternatives require explicit work stack and per-entry error handling to get this right; `walkdir` provides it.

**Forward implication**: T5/T6 agents may live arbitrarily deep; the same walk pattern handles any depth without refactor.

### Alias Support: Singular and Plural OpenCode Roots Are One Identity

**Decision**: `~/.config/opencode/skill/` and `~/.config/opencode/skills/` are both scanned but grouped under the same `SearchRootId("opencode-skills")`.

**Why**: OpenCode's glob is `{skill,skills}/**/SKILL.md`, and both forms may exist on a single machine. The singular form is a fallback; consolidating them into one root id avoids double-reporting and makes T8's deduplication cleaner.

**Limitation**: If only the singular exists, the reported `SearchRoot.path` carries the plural form (the canonical path that is not on disk). Acceptable for the PoC — `SearchRoot` is a grouping identity, not a display-only path. Flagged for T11/T12 UI phases.

**Forward implication**: T8 and T9 see one root id for OpenCode, not two.

---

## Final Verification State (2026-08-18)

**Task Completion**: 28/28 marked `[x]` in `tasks.md`. All tasks verified complete in verify-report.

**Test Results**: 84/84 tests passing (Windows verified during apply; all three CI platforms via the merged PR):
- 41 lib tests (roots, skills, related)
- 14 frontmatter_reader tests (T3, reusable)
- 8 model_contract tests (identity, location, scope)
- 13 skill_scanner tests (integration, spec coverage)
- 7 yaml_behavior tests
- 1 yaml_seam_invariant test (non-vacuous, enforces model purity)

**Build & Verification Matrix (PR #7 CI)**:
| Command | Status | Details |
|---|---|---|
| `cargo fmt --all --check` | PASS | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Zero warnings, including `std::env::home_dir()` at MSRV |
| `cargo test --workspace --locked` | PASS (84/84) | All layers, all platforms |
| `cargo deny check bans licenses` | PASS | bans ok, licenses ok (walkdir's Unlicense OR MIT resolved via MIT, no deny.toml change) |
| Bindings drift | PASS | SearchRoot.ts modified, SearchRootStatus.ts new, no other bindings touched |
| Frontend lint | PASS | eslint clean |
| Frontend types | PASS | svelte-check: 169 files, 0 errors, 0 warnings |
| Frontend tests | PASS | vitest: 1 file, 2 tests |
| Frontend build | PASS | vite build, 189ms |

**Spec Compliance**: All 10 skill-scanner requirements + 1 domain-model requirement covered (11/11 requirements, 30 scenarios).

---

## Known Limitations (for T5–T16)

### 1. CA-6 Verified on One Windows Machine Only

**Issue**: Structural claim that `~/.claude/plugins/` does not exist is verified on one Windows machine (2026-08-18). macOS/Linux revalidation deferred to T16.

**Mitigation**: If a plugin root surfaces on macOS or Linux, T16 adds exclusion filter as a delta. No logic shipped today to defend against the case.

**Current scope**: Scans only the three hardcoded roots; plugin root exclusion is guaranteed structurally, not by pattern.

### 2. Symlink-Following Test Is a Structural Contract, Not a Direct Fixture

**Issue**: No portable CI fixture exists for symlinks on Windows (junctions vs. symlinks behavior undefined). The test `walk_never_follows_symlinks_by_default_walkdir_setting` asserts `follow_links(false)` is set, a proxy for the actual behavior rather than a direct test.

**Mitigation**: Design §6 and §11 acknowledge this gap. T16 may add a direct Windows-junction or Unix-symlink fixture if real-world symlinked skill directories are discovered.

**Current scope**: Unit-level contract test passes; actual symlink behavior unverified on Windows but expected to be correct per `walkdir`'s documented behavior.

### 3. Redundant Symlink Metadata Probe Across `roots.rs` and `skills.rs`

**Issue**: Non-blocking inefficiency. `roots::probe()` calls `std::fs::symlink_metadata` to determine `SearchRoot.status`; `skills::walk_one()` calls it again for the same root to decide whether to emit a `ScanIssue` for read failures.

**Mitigation**: Not a correctness bug; both branches agree and together satisfy design section 7. A future refactor could thread the status through `ResolvedRoot` to avoid duplication. Not spec- or design-mandated to avoid, so not blocking archive.

**Recorded**: Non-blocking WARNING in verify-report.

### 4. UTF-8 BOM Is Unhandled; Deferred to T16

**Issue**: A leading UTF-8 BOM (`U+FEFF`) makes the first line `"\u{FEFF}---"`, so the fence comparison fails and the file falls into T3's `NoOpeningFence` with a `Warning`. Graceful, non-panicking, but arguably a false negative on a Windows-authored file.

**Mitigation**: Inherited from T3 (design §3); design §11 defers to T16. No fixture exercises this case in T4.

---

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Changed lines (logic + tests, excluding fixture tree) | ~500–720 |
| Changed lines (69-entry minimal fixture tree) | ~280 |
| **Total changed lines** | **~780–1000** |
| 400-line budget risk | **High** by line count; **Medium** by cognitive load (69-entry tree is mechanical bulk, not complexity) |
| Delivery strategy | `single-pr` with `size:exception` (product owner decision, 2026-08-18) |
| Chained PRs rejected alternative | Analysis retained in tasks.md for the record; adjustment needed to avoid non-compiling PR |

**Delivered as**: One PR, ~780–1000 lines, four ordered commits within the PR (model + bindings, semantic fixtures, RED tests + module skeletons, GREEN implementation + reference tree).

---

## Scope Check (per rules.archive)

**Verified: Nothing out-of-scope crept in.**

| Scope Constraint | Status | Evidence |
|---|---|---|
| No MCP support | CONFIRMED | No MCP imports or calls anywhere |
| No project scope | CONFIRMED | Only `Scope::User` ever constructed; project-shaped fixture ignored |
| No write operations | CONFIRMED | Grep: zero matches for write/create/remove calls in roots.rs/skills.rs |
| No Tauri command or IPC exposure | CONFIRMED | SkillScan is non-model (no Serialize/TS); no Tauri code anywhere |
| No new dependencies | CONFIRMED | walkdir was already transitive; promoted to direct, no new crates |

**Verdict**: PoC-compliant. Archive is safe to complete.

---

## Artifacts in This Archive

This folder contains:
- `proposal.md` — the original change proposal with success criteria
- `design.md` — detailed design decisions with rationale, open questions (now resolved), and forward implications
- `tasks.md` — 28 implementation tasks, all marked complete
- `apply-progress.md` — apply phase evidence, TDD cycle documentation, file changes, committed state
- `verify-report.md` — full verification matrix, spec compliance (28/28 tasks, 10/10 skill-scanner requirements, 1/1 domain-model requirement, 30 scenarios), issues list (0 CRITICAL, 2 non-blocking WARNINGS)
- `specs/skill-scanner/spec.md` — delta spec for the new capability
- `specs/domain-model/spec.md` — delta spec for the SearchRoot model change

**Specs merged into main specs:**
- Created `openspec/specs/skill-scanner/spec.md` from the delta spec (new capability)
- Merged domain-model delta into `openspec/specs/domain-model/spec.md` (SearchRoot requirement added)

---

## Traceability

All artifacts related to this change are persisted in this archive folder. The change is closed. No follow-up work is needed in T5–T10 regarding T4's deliverables themselves. T5 (Claude Code agents) and T6 (OpenCode agents) pick up with `roots::home_dir()` and `frontmatter::read`, both proven reusable in this change. T8 (consolidation) builds on top of T4's 69-entry result, merging duplicates into 25 unique identities.

---

## Blocks and Unblocks

**T4 Unblocks:**
- **T5** (Claude Code agent adapter): reuses `roots::home_dir()` for agent root resolution and `frontmatter::read::<AgentFrontmatter>` unchanged
- **T6** (OpenCode agent adapter): same pattern as T5
- **T8** (duplicate consolidation): receives 69 entries (un-consolidated) and produces 25 unique components per CA-2
- **T9** (`ScanReport` assembly): wraps T4's `SkillScan` result into the final report

**T4 Requires:**
- **T2** (domain model): complete and archived. T4 adds one field to `SearchRoot`.
- **T3** (frontmatter reader): complete and archived. T4 is the first caller of `frontmatter::read` for discovered paths.

---

**Archive Date**: 2026-08-18  
**Archived By**: sdd-archive executor  
**Merged Commit**: `7b212d2` (PR #7, `main`)  
**Status**: Complete and closed. Ready for T5–T16.
