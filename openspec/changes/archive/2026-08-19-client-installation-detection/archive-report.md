# Archive Report: Client Installation Detection (T7)

**Date**: 2026-08-19  
**Change**: `client-installation-detection`  
**Phase**: T7 (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md:171-187`  
**Verification**: PASS WITH WARNINGS: 0 CRITICAL, 2 WARNING, 5 SUGGESTION, no blockers  
**Status**: ARCHIVED — Change complete and closed.

---

## Executive Summary

T7 delivered client installation detection for Windows (Claude Code npm, Claude Code desktop, OpenCode npm), closing CA-7 (two Claude Code installations detected separately, each with its version) and CA-11 (absent client reported explicitly as "not detected", never as a parse error or silent omission). The implementation adds one new module (`installations.rs`), a platform-dispatch seam prepared for T16 macOS/Linux addition, 12 synthetic fixture homes, and 28 new tests (10 unit + 18 integration) exercising the Windows probe table, version extraction, per-slot isolation, and read-only invariant (CA-16/CA-17). All 39 implementation tasks complete and verified independently against the actual code; 179 tests green across all suites; all four gates (fmt, clippy, test, deny) verified green; no CRITICAL verification issues found. Delivery: a single PR with `size:exception`, chosen by the user after the chained split was offered and declined. The tasks phase forecast ~445–685 changed lines; the real figure is ~965 lines of code and tests plus 63 across 32 fixture files, so the forecast under-estimated an adapter of this shape by roughly half. The PR (#15) is **open for review, not merged**, and CI has not yet exercised the Linux and macOS legs. The change is pure-read, deterministic, machine-independent, and directly unblocks T9 and T10.

---

## What T7 Delivered

### Core Deliverables

**New module `crates/vertice-core/src/installations.rs`:**
- `pub struct InstallationScan { pub installations: Vec<ClientInstallation>, pub issues: Vec<ScanIssue> }` — owned result, distinct from `SkillScan`/`AgentScan`/`OpenCodeAgentScan` (no `roots` field per design §3)
- `pub fn scan(home: &Path) -> InstallationScan` — dispatch point delegating to `scan_for` with `HostPlatform::current()`
- `pub fn scan_for(home: &Path, platform: HostPlatform) -> InstallationScan` — per-platform scanner, testable with explicit platform parameter
- `pub enum HostPlatform { Windows, Unsupported }` — platform discriminator (non-model, no `Serialize`/`TS`)
- Private `struct InstallProbe`, `enum InstallKind { Npm, Desktop }`, `enum VersionSource { PackageJson, DirectoryName }`
- Private `fn windows_install_probes(home: &Path) -> [InstallProbe; 3]` — fixed-size probe table, three slots in order: Claude Code npm, Claude Code desktop, OpenCode npm
- Private resolution functions: `resolve(probe, issues) -> Option<ClientInstallation>` for npm slots; `resolve_desktop(probe, issues) -> Vec<ClientInstallation>` for desktop (N installations per candidate subdirectory, per design §6)
- Private `fn exists(path: &Path) -> bool` — local check for path existence (three-line `symlink_metadata` helper, design §5.3)
- Private `fn extract_package_json_version(value: &JsonValue) -> Option<String>` — value-level extraction of `"version"`, `None` for absent/non-string/empty (design §5.4)
- 10 unit tests: probe table structure, `HostPlatform` dispatch, version extraction from JSON values

**Full `ScanIssue` taxonomy per design §8:**
- Absent npm/desktop dir → `Warning`, `{Client} ({kind}) not detected`, `path: Some(<probe path>)` (CA-11)
- npm dir present, `package.json` absent/unreadable/unparseable/non-object/missing "version"/wrong type/"version" empty → `Error`, `path: Some(<package.json>)`, reason specific to the failure (read vs parse vs shape)
- desktop: 0 candidates → `Error`, reason and path per design §6; N ≥ 1 candidates → N installations, zero issues; candidate with non-UTF-8 name → `Error`, `path: None`, name rendered lossily in reason
- `HostPlatform::Unsupported` → `Warning`, `"client installation detection is not implemented on this platform"`, `path: None`
- No `escalate` function; every `ScanIssue` constructed at point of caller context (design §8/T6D §5.6 precedent)

**Integration tests (`crates/vertice-core/tests/client_installations.rs`):**
- 18 integration tests, one per spec requirement or CA pin, exercising all 12 fixtures
- `two-claude` pin: exactly 2 `ClaudeCode` installations, different versions, distinct paths, 0 issues (CA-7)
- `nothing` pin: 0 installations, exactly 3 `Warning` issues, one per probe path, no `Error` (CA-11)
- `isolation` pin: 1 malformed slot yields 1 `Error`; other two installations still present (per-slot independence)
- `npm-dir-no-package-json` pin: 1 `Error`, zero `Warning` (broken ≠ absent, CA-11 contrapositive)
- `desktop-two-versions` pin: exactly 2 `ClaudeCode` desktop installations, different versions, distinct paths, 0 issues, never merged (design §6, CA-7 extended)
- `reference` fixture: 4 installations (npm Claude Code + 2 desktop Claude Code + npm OpenCode), 0 issues
- Platform seam: `scan_for(home, Unsupported)` → 0 installations, 1 `Warning`, `path: None`
- Entry point dispatch: `scan(home)` matches `scan_for(home, HostPlatform::current())` with `cfg!(target_os = "windows")` expression (testable on all CI legs per design §5.2)
- Determinism: byte-identical results across two runs on same fixture
- Read-only: full scan leaves fixture tree unchanged (CA-16)
- Contract: no `ClientInstallation` carries empty `version`

**Modified `crates/vertice-core/src/lib.rs`:**
- One line: `pub mod installations;` (no crate-root re-export, matching existing style)

**New dependency file entries:** None — `Cargo.toml`, `Cargo.lock`, `deny.toml` byte-identical to pre-change state

**Modified `.gitattributes`:**
- Added binary registration line for `crates/vertice-core/tests/fixtures/installations/package-json-unreadable/AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/package.json` (non-UTF-8 content fixture)

**Fixture tree under `crates/vertice-core/tests/fixtures/installations/`:**
- 12 synthetic homes (30 files on disk), new tree, never reused from T4/T5/T6:
  - `nothing/` — `.gitkeep` only; 0 installations, 3 Warnings (CA-11 pin)
  - `two-claude/` — npm Claude Code + 2 desktop Claude Code versions; 2 installations, 0 issues (CA-7 pin)
  - `opencode-npm/` — npm OpenCode; 1 installation
  - `isolation/` — malformed npm Claude Code + healthy desktop + healthy OpenCode; 1 Error + 2 installations (per-slot isolation pin)
  - `no-version-key/`, `version-not-a-string/`, `package-json-empty/` — npm package.json edge cases; each 1 Error (collapsed reasons per design §8/V5)
  - `package-json-unreadable/` — non-UTF-8 content in npm package.json; 1 Error (`could not read package.json: {io}`), distinct from parse-failure reason
  - `npm-dir-no-package-json/` — npm dir present, no `package.json`; 1 Error, 0 Warnings (CA-11 contrapositive pin)
  - `desktop-empty/` — Claude/claude-code/ present, no versioned subdirs; 0 installations, 1 Error
  - `desktop-two-versions/` — Claude/claude-code/{1.0.0,2.0.0}/; 2 installations, 0 issues (design §6, CA-7 extended)
  - `reference/` — realistic fixture mirroring verified reference machine: npm Claude Code (2.1.140) + 2 desktop Claude Code versions + npm OpenCode (1.17.20); 4 installations, 0 issues

**Model and bindings (unchanged):**
- `git diff --exit-code -- crates/vertice-core/src/model frontend/src/bindings` verified clean (zero lines changed)
- `ClientInstallation`, `ClientKind`, `ScanIssue`, `IssueSeverity::Warning/Error` all pre-existing from T2
- No new type derives `Serialize` or `TS`; `InstallationScan`, `InstallProbe`, `InstallKind`, `VersionSource`, `HostPlatform` are non-model

**`roots.rs` invariant (unchanged):**
- `git diff --exit-code -- crates/vertice-core/src/roots.rs` verified clean (zero lines changed, per design §5.3)
- Private `exists` helper in `installations.rs` is a local 3-line `symlink_metadata` check, not a reuse of `roots::probe`

---

## Verification Outcome

**Verdict from verify-report.md:** PASS WITH WARNINGS — 0 CRITICAL, 2 WARNING (both documented as mitigated, not blocking archive), 5 SUGGESTION

**Completeness:**
- All 39 tasks marked complete with `[x]`; verify-report independently re-ran every gate task (1.14, 3.1–3.11) rather than accepting marks
- 39/39 tasks verified against the actual code: fixtures exist on disk, types/tests present in source, gates re-executed in this session

**Gates (actually re-run in verify session):**
| Gate | Result |
|---|---|
| `cargo fmt --all --check` | PASS (clean, re-run in verify) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (0 warnings, re-run in verify) |
| `cargo test --workspace --locked` | PASS — 179 tests green (73 lib unit + 18 client_installations + 14 frontmatter + 9 jsonc + 8 model + 24 opencode_agent + 13 skill + 7 yaml + 1 yaml_seam + vertice-app unit) |
| `cargo deny check bans licenses` | PASS — `bans ok, licenses ok` (two pre-existing license-not-encountered warnings, unrelated to T7) |
| Model diff | PASS — `git diff --exit-code -- crates/vertice-core/src/model` clean |
| Bindings diff | PASS — `git diff --exit-code -- frontend/src/bindings` clean (pre-existing LF/CRLF warnings unrelated to T7) |
| `roots.rs` diff | PASS — `git diff --exit-code -- crates/vertice-core/src/roots.rs` clean |
| Dependency files diff | PASS — `Cargo.toml`, `Cargo.lock`, `deny.toml` byte-identical |
| Read-only (CA-16) | PASS — grep for `File::create`, `OpenOptions`, `fs::write`, `create_dir*`, `remove_*` yields zero matches |
| Seam invariant | PASS — no `dirs`, no `directories`, no `std::env` read (outside `env!` macro), no `regex`, no second JSON crate |
| Privacy | PASS — only `InstallationScan`, `HostPlatform`, `scan`, `scan_for` are `pub` |
| Frontend regression | PASS — `npm run lint && npm run check && npm run test && npm run build` all green (regression only; no new consumer) |

**Spec compliance:** 14/15 requirement rows COMPLIANT, 1 PARTIAL (static-inspection only, not a functional gap — "Only Path Resolution Is Platform-Specific" scenario was achieved via code inspection rather than a runtime test assertion; spec's own wording is a static-inspection scenario, not runtime)

**Platform coverage caveat:** Only the Windows CI leg was exercised in this apply/verify session (Windows host, no cross-compilation available). Linux/macOS CI legs are unverified until PR runs through GitHub Actions. Design §5.2's reasoning for why Windows-only local run is sufficient (all fixture calls use `scan_for(home, HostPlatform::Windows)` uniformly; synthetic `home` with `AppData/Roaming/...` is just directory names to any filesystem) is sound and was independently confirmed by source inspection, but actual CI execution on those two legs remains outstanding.

**Two WARNINGs flagged in verify-report (both mitigated, not blocking):**
1. **RED-evidence gap for unit tests 1.1–1.4, 2.1–2.2:** Tests and implementation authored in one write with no captured compile-fail transcript. Mitigated by: the load-bearing CA-7 checkpoint (tasks 2.3/2.4) was genuinely executed RED-then-GREEN with captured stub-and-revert transcript (visible in apply-progress.md), and every affected test passes on independent re-execution. Recommendation for future changes: capture compile-fail transcript for all unit tests, since `strict_tdd: true` is meant to be auditable.

2. **Fixture completeness deviation:** `two-claude/`, `desktop-two-versions/`, and several single-edge-case fixtures built with all three slots populated (rather than single isolated slot as literal design §10 bullets describe), to satisfy those same bullets' explicit "0 issues" assertion. Session independently verified every fixture's actual content to confirm the extra populated slots are genuine and the target broken/edge slot is the only source of the asserted issue — so the "0 issues" assertion is not vacuous, it exercises real multi-slot resolution. Legitimate superset coverage, but flagged WARNING rather than silent because it diverges from literal design bullet text.

**5 SUGGESTIONs (open, not blocking):**
1. No runtime test directly asserts "scanner's extraction/assembly code has no `cfg(target_os)` branch" — spec's own scenario is static-inspection, and that was done by verify session, not automated. Grep-based CI check would self-verify on future changes.
2. Two mildly self-referential loop-shaped assertions noted in verify-report's Assertion Quality table — not defects, but worth strengthening if file is touched again.
3. `.gitattributes` binary registration and non-UTF-8 fixture bytes are correct today; no automated check pins this (bit-rot risk). Precedent (`frontmatter/non-utf8-content/SKILL.md`) has same unpinned risk — consistent with existing project practice.
4. Pre-existing `frontend/src/bindings/*.ts` LF/CRLF working-tree noise predates T7; confirmed by `git diff --stat` showing only line-ending normalization warnings and zero content changes.
5. Linux/macOS CI legs unverified in this session (no cross-compilation). Design reasoning is sound and code is symmetric; CI execution remains outstanding.

---

## Acceptance Criteria Closed

**CA-7 (two Claude Code installations detected separately, each with its version):**
- Implementation produces N `ClientInstallation` per slot, no merging by `ClientKind`
- Fixtures `two-claude/` and `desktop-two-versions/` prove two distinct installations with different versions, distinct paths, never merged
- Integration tests: `two_claude_fixture_yields_two_never_merged_claude_installations`, `desktop_two_versions_fixture_yields_two_installations_never_merged`

**CA-11 (absent client reported as explicit "not detected", never silent):**
- Absence signalled via `ScanIssue(Warning, "{Client} ({kind}) not detected", path: Some(<probe>))`, not as silent omission
- `nothing/` fixture produces exactly 3 Warnings, one per slot, each naming the client and path
- Distinguishable from parse-failure: `nothing/` vs `isolation/` have different severities and reason formats
- Integration tests: `nothing_fixture_yields_zero_installations_and_three_warnings_never_an_error`

**CA-16 (read-only, no writes):**
- Complete disk surface: `std::fs::symlink_metadata`, `std::fs::read_to_string`, `std::fs::read_dir` only
- No `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, `remove_*` anywhere in module or tests
- Fixture-based tests never materialize temp trees, only read committed fixtures
- Integration test: `full_scan_leaves_the_reference_fixture_tree_unchanged` performs byte-comparison before/after

**CA-17 (fixture-based, machine-independent, three-platform):**
- All tests read from `crates/vertice-core/tests/fixtures/installations/`, a new tree never reused from T4/T5/T6
- No test reads author's machine, sets environment variable, or invokes `claude`/`opencode`
- Fixture paths built via `env!("CARGO_MANIFEST_DIR")` + per-segment `push`; synthetic `home` contains `AppData/Roaming/...`
- Windows probe table exercised on all three CI legs via `scan_for(home, HostPlatform::Windows)` (per design §5.2)

---

## Known Limitations (for T8–T16)

### 1. Linux/macOS CI legs not exercised locally in this apply/verify session

Windows host only; no cross-compilation available. Design §5.2's structural reasoning (Windows probe table is just directory names to any filesystem, testable identically on all legs) is sound and code inspection confirms it, but actual CI execution on Linux/macOS remains outstanding until PR runs through GitHub Actions.

### 2. RED-evidence audit trail gap for unit tests 1.1–1.4, 2.1–2.2

Tests and implementation written in single edit session with no separate compile-fail transcript. The load-bearing CA-7 checkpoint (2.3/2.4) was genuinely RED-then-GREEN with captured evidence; all tests pass on re-execution; no regression risk. Recommend: capture compile-fail transcript for all RED phases in future strict_tdd changes, not just checkpoints.

### 3. Fixture completeness deviation from literal design §10 bullet description

Fixtures built as "all three slots populated, target slot broken/edge-cased" rather than single-slot homes, to satisfy those same bullets' explicit "0 issues" assertion. Session independently verified actual fixture content; superset coverage, not weakened. Recorded in apply-progress.md as Deviation 1; not a defect, noted for awareness.

### 4. Desktop directory N ≥ 1 allows phantom installations from stray directories

Under design §6's corrected N-installations rule (refuting U2's "exactly one" premise), a stray leftover directory (partial download, `.tmp` staging dir) becomes a phantom installation indistinguishable from a real version directory. No independent oracle to validate the directory name (all predicate approaches rejected per design §6). Mitigated by: T16 manual oracle (`claude --version` contrast), which now carries more detection burden than before. Accepted risk per design §6.

### 5. Platform seam unverified against real macOS/Linux installations

The path tables for macOS/Linux are unknown (T16 scope, `plan-desarrollo-poc.md:187`). `Unsupported` is the honest placeholder. T7 ships Windows with structure prepared for additive path tables; no trait, no registry, one function per OS.

---

## Decisions Worth Carrying Forward to T8–T16

### Client installations reported independently, never merged

`for probe in probes.iter()` loop pushing `Option<ClientInstallation>` (npm) or `Vec<ClientInstallation>` (desktop) per probe, never a map keyed by `ClientKind`. Two Claude Code installations are two rows, always.

### Per-slot independence, not early return

Every probe runs even after a failure; one slot's parse error does not skip the other two. All issues are collected and emitted together (design §8/T6D isolation discipline).

### Absence is `Warning`, not `Error`

CA-11's own words; painting every single-client machine red would train users to ignore the issue list (T6D §8 generalized).

### Version extracted verbatim, never validated

Directory name accepted as-is without semver/plausibility predicate; no heuristic, no name validation. The PoC reports, it does not interpret (design §6/precedent from T6D §6.1 — presence is the detection rule).

### Platform seam is one function per OS, no trait or registry

`windows_install_probes(home: &Path) -> [InstallProbe; 3]`, plus `HostPlatform::current()` with `cfg!` (expression, not attribute) as the single dispatch point. T16 adds `MacOs` variant, `macos_install_probes`, `linux_install_probes`; extraction code untouched.

### `roots.rs` and `jsonc.rs` seams untouched; local helpers where needed

`roots::probe` stays private (imports search-root vocabulary inappropriate to this module). `jsonc.rs` is the sealed JSON/JSONC seam; second caller, first new one since T6. Local 3-line `exists` helper avoids importing model types (design §5.3).

### Value-level extraction, never DTO-based

`entry.get("version")` matched against `JsonValue::String` only; anything else (number, object, array, null, bool) yields `None` + `Warning`. No `#[derive(Deserialize)]` struct for agent entry. Mirrors T6D §5.4 — consumer schema fragility (T5's scalar-tools finding) makes DTO-based extraction unsafe for an inventory tool.

---

## Scope Check (per rules.archive)

**Verified: Nothing out-of-scope crept in.**

| Scope Constraint | Status | Evidence |
|---|---|---|
| No MCP support | CONFIRMED | No MCP imports or calls in `installations.rs` or tests |
| No project scope | CONFIRMED | Only `Scope::User` path via fixture `home` parameter; no scope enum variant added |
| No write operations | CONFIRMED | Grep and byte-comparison test both pass; fixtures never modified |
| No Tauri command or IPC exposure | CONFIRMED | `InstallationScan` is non-model type; no command registered; `capabilities/default.json` untouched |
| No new dependencies | CONFIRMED | `Cargo.toml`, `Cargo.lock`, `deny.toml` byte-identical; `jsonc.rs` reused, no second JSON crate |
| No model changes | CONFIRMED | `git diff --exit-code -- crates/vertice-core/src/model` clean |
| No bindings regeneration | CONFIRMED | `git diff --exit-code -- frontend/src/bindings` clean |

**Verdict**: PoC-compliant. Archive is safe.

---

## Artifacts in This Archive

This folder contains:
- `proposal.md` — original change proposal with success criteria and risks (all met)
- `explore.md` — exploration phase findings and approach comparison
- `design.md` — detailed design decisions, platform seam, error taxonomy, fixture architecture (all approved decisions with evidence)
- `tasks.md` — 39 implementation tasks; 39/39 marked complete and independently verified
- `apply-progress.md` — apply phase evidence, TDD cycle documentation, gate results, deviations recorded
- `verify-report.md` — full verification matrix, spec compliance (14/15 compliant + 1 partial), gate re-execution, TWO WARNINGs closed, FIVE SUGGESTIONs open
- `specs/client-installation-detector/spec.md` — new capability spec
- `state.yaml` — DAG state (if present from orchestration)

**Specs created and merged into main specs:**
- Created `openspec/specs/client-installation-detector/spec.md` from the delta spec (new capability, no existing main spec to modify)

**No existing capabilities modified.**

---

## Traceability

All artifacts related to this change are persisted in this archive folder. Per-file line-count preservation confirmed:

| Artifact | Lines | Status |
|---|---|---|
| proposal.md | 194 | Preserved verbatim |
| explore.md | 96 | Preserved verbatim |
| spec.md | 148 | Copied to main specs, delta preserved in archive |
| design.md | 400 | Preserved verbatim |
| tasks.md | 103 | Preserved verbatim |
| apply-progress.md | 107 | Preserved verbatim |
| verify-report.md | 162 | Preserved verbatim |

The change is closed. No follow-up work on T7's deliverables is needed in T8–T15. T8 (consolidation) and T9 (`ScanReport` assembly) pick up unchanged from this point. T16 has explicit open questions (Linux/macOS paths, stray-directory phantom installations, oracle contrast) deferred with reasons recorded above.

---

## Blocks and Unblocks

**T7 Unblocks:**
- **T8** (duplicate consolidation): receives client installations from three independent probes, un-consolidated, to be merged by client if the UI presents them that way
- **T9** (`ScanReport` assembly): receives `InstallationScan` struct (parallel to `SkillScan`, `AgentScan`, `OpenCodeAgentScan`), with explicit determinism guarantees and error paths documented

**T7 Requires:**
- **T2** (domain model): complete. T7 constructs `ClientInstallation`, `ClientKind`, `ScanIssue`, `IssueSeverity::Warning/Error`, `Scope::User` — all pre-existing
- **T4** (skill scanner): complete. T7 reuses `roots::home_dir()`, the absence-checking pattern
- **T6** (OpenCode agent adapter + jsonc seam): complete. T7 reuses `jsonc.rs` for `package.json` parsing

**Parallel work:**
- **T8** (consolidation), **T9** (`ScanReport` assembly) may run in parallel; T7 does not block either

---

## CA & T8–T16 Handoff Notes

**Delivered to T8:**
- Three independent `ClientInstallation` values (two Claude Code + one OpenCode, or subsets thereof), sorted by probe order, never pre-merged
- All `ScanIssue` instances already constructed with full severity and path context
- Fixture set demonstrating every behavior case, ready for consolidation oracle contrast

**CA coverage:**
- **CA-7**: two Claude Code installations detected separately (CLOSED by T7)
- **CA-11**: absent client named in `ScanIssue`, not silent (CLOSED by T7)
- **CA-16**: read-only, no writes (CLOSED by T7)
- **CA-17**: fixture-based, machine-independent, three-platform (Windows verified; Linux/macOS unverified until CI)

---

## Status Summary

**Change**: T7 — Client Installation Detection  
**Archived to**: `openspec/changes/archive/2026-08-19-client-installation-detection/`  
**Verification Verdict**: PASS WITH WARNINGS (0 CRITICAL, 2 WARNING closed, 5 SUGGESTION open)  
**Archive Date**: 2026-08-19  
**Status**: Complete and closed. No further work on T7 itself is required.  
**Ready for**: T8, T9, with open questions deferred to T16 recorded in this archive for visibility.

---

**Archive Date**: 2026-08-19  
**Archived By**: sdd-archive executor  
**Status**: Complete and closed. Ready for T8–T16.
