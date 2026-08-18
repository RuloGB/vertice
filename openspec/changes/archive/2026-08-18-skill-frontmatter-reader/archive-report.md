# Archive Report: Frontmatter and SKILL.md Reader (T3)

**Date**: 2026-08-18  
**Change**: `skill-frontmatter-reader`  
**Phase**: T3 (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md`  
**Verification**: PASS WITH WARNINGS (0 CRITICAL, 1 WARNING — bookkeeping gap, not functional defect)  
**Status**: ARCHIVED — Change complete and closed.

---

## Executive Summary

T3 delivered a generic, single-file frontmatter reader turning `&Path` into typed values or `ScanIssue`, closing CA-10 (folded multi-line description complete) and CA-12 (partial: corrupt file carries its path). The implementation (`crates/vertice-core/src/frontmatter.rs`, 178 lines) ships with 14 fixture-driven integration tests plus 9 in-module unit tests, ten repository fixtures, and mechanical enforcement of the `yaml.rs` single-import invariant. All 60 tests pass on all three CI platforms (Windows, macOS, Linux) via PR #5 after a delivery incident with PR #4 was discovered and remediated. The change is pure-read, non-panicking on every documented failure class, and directly reusable by T5 and T4 without modification.

---

## What T3 Delivered

### Core Deliverables

**New module `crates/vertice-core/src/frontmatter.rs`:**
- `pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, ScanIssue>` — the five-step pipeline (read bytes, validate UTF-8, split fence, deserialize, map error)
- Private `fn split(source: &str) -> Result<String, FenceError>` — line-based, regex-free fence splitter
- Private `enum FenceError { Empty, NoOpeningFence, Unterminated }` — internal error type
- `pub struct SkillFrontmatter { name: String, description: Option<String> }` — reader DTO with `Deserialize` only, no `Serialize`/`TS`
- 9 in-module `#[cfg(test)]` unit tests covering `split` edge cases (opening/closing fence, empty file, unterminated, CRLF, fence not on line 1, indented fence rejected, trailing whitespace after the fence tolerated)

**Integration tests (`crates/vertice-core/tests/frontmatter_reader.rs`):**
- 14 fixture-driven tests: one per documented failure class plus success cases
- CA-10 assertion: folded description (`description: >`) returned in full, not a prefix — tested against exact joined string
- CA-12 assertion: corrupt YAML returns `Err(ScanIssue)` with `path: Some(fixture_path)` and non-empty `reason`
- Generic-reuse probe: a local `LicenseProbe { license: String }` struct reads `valid-folded-description/SKILL.md` via the same `frontmatter::read`, proving T5's reuse path without refactor
- Non-UTF-8 tripwire: asserts exact byte length (425) and decode failure, catching `.gitattributes` corruption

**Invariant enforcement (`crates/vertice-core/tests/yaml_seam_invariant.rs`):**
- Walks `src/` and asserts no `.rs` file besides `yaml.rs` contains `use serde_norway` or `serde_norway::`
- Now passes non-vacuously: `frontmatter.rs` scanned, zero offenders found
- Runs locally on all three CI platforms without workflow change — consistent with proposal's locked "no CI-workflow change" criterion

**Repository fixtures (10 cases) under `crates/vertice-core/tests/fixtures/frontmatter/`:**
- `valid-minimal/SKILL.md` — basic `name` and `description`
- `valid-folded-description/SKILL.md` — multi-line `description: >` plus extra keys (`license`, `disable-model-invocation`, `metadata` map) proving unknown-field tolerance
- `valid-no-description/SKILL.md` — `name` only, no description key; returns `Ok` with `None`
- `no-frontmatter/SKILL.md` — Markdown body with no `---` fence
- `empty/SKILL.md` — zero-byte file
- `corrupt-yaml/SKILL.md` — malformed YAML inside the fence
- `missing-name/SKILL.md` — `description` present, `name` absent
- `type-mismatch-name/SKILL.md` — `name` as a YAML list instead of string
- `unterminated-fence/SKILL.md` — opening `---` with no closing fence before EOF
- `non-utf8-content/SKILL.md` — 425 bytes of well-formed frontmatter with one raw `0xFF` byte, triggering strict UTF-8 decode failure

**Configuration and invariant:**
- `.gitattributes` (repository-wide) — two ordered rules: `crates/vertice-core/tests/fixtures/** -text` then `...non-utf8-content/SKILL.md binary` (order load-bearing)
- One doc-comment wording fix in `crates/vertice-core/src/model/error.rs` (no logic change) to avoid false-positive on `yaml_seam_invariant.rs`'s textual check

**Wiring:**
- `crates/vertice-core/src/lib.rs` — one line added: `pub mod frontmatter;` (alphabetically placed)

### Acceptance Criteria Closed

**CA-10 (folded description complete and correct):**
- `valid_folded_description_is_complete_and_correct` test asserts the exact joined string from the multi-line `description: >` scalar
- Manually recomputed against the YAML folding rule (lines joined with single space, one trailing newline per clip chomping) and verified independently in verify-report
- Passed on first run, no adjustment needed

**CA-12 (partial — corrupt file carries its path):**
- `corrupt_yaml_carries_its_path_and_a_parse_reason` test asserts `Err(ScanIssue)` with `path: Some(fixture_path)` and a `reason` string describing the YAML parse failure
- Non-empty `reason` carrying `serde_norway::Error`'s message verbatim
- Verified independently in verify-report

**Indirectly supports CA-2 (consolidation count):**
- `valid-no-description` succeeds with `description == None`, so a description-less skill does not vanish from the inventory
- This allows the consolidated count to reach exactly 25 skills as required by CA-2

---

## Final Verification State (2026-08-18)

**Task Completion**: 20/20 marked `[x]` in `tasks.md`. All tasks confirmed complete in apply-progress and re-verified in verify-report.

**Test Results**: 60/60 tests passing (all three CI platforms via PR #5)
- 28 unit tests (src/lib.rs, incl. 7 new `frontmatter::tests::split_*`)
- 14 integration tests (`tests/frontmatter_reader.rs`)
- 8 model contract tests (`tests/model_contract.rs`)
- 7 YAML behavior tests (`tests/yaml_behavior.rs`)
- 1 invariant test (`tests/yaml_seam_invariant.rs`, now non-vacuous)

**Build & Verification Matrix (PR #5 CI)**:
| Command | Status | Details |
|---|---|---|
| `cargo fmt --all --check` | PASS | Clean after one formatting pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Zero warnings on all three OSes |
| `cargo test -p vertice-core --locked` | PASS (60/60) | All layers, all fixtures |
| `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --locked --all-targets` | PASS | MSRV floor (1.88) confirmed |
| `cargo deny check bans` | PASS | Core Purity Invariant holds; zero `tauri`/`tauri-*` edges in `vertice-core` |
| `cargo deny check licenses` | PASS | Zero delta; `SkillFrontmatter` adds no new type crossing IPC |
| `git diff --exit-code -- frontend/src/bindings` | PASS | No regeneration; `SkillFrontmatter` is `Deserialize`-only, not `TS`-derived |

**Spec Compliance**: All 14 requirements (from frontmatter-reader/spec.md) covered; 19/19 scenarios compliant per verify-report matrix.

---

## Decisions Worth Carrying Forward to T4–T10

### Reader Is Generic Over Deserialization Target, Built on Day One

- T5 supplies `AgentFrontmatter { name, description, model, tools }` and calls `frontmatter::read::<AgentFrontmatter>(path)` unchanged
- No refactor needed; the generic signature and the five-step pipeline do not depend on the target type
- **Forward implication for T4–T5**: call `frontmatter::read` with your own `T: DeserializeOwned` struct; the reader is the leaf primitive, not the adapter

### Non-UTF-8 Content Carries Its Path; Non-UTF-8 Path Does Not

- **T2's contract (archive/2026-08-17)**: `path: None` for a path that cannot be represented as UTF-8 — a T4 concern
- **T3's rule**: When file *bytes* fail UTF-8 decode, path is perfectly valid and MUST be carried: `path: Some(path)`
- Non-UTF-8 *path* is structurally unreachable in T3 — receives an already-valid `&Path` from caller
- **Implication**: T4's walker is first phase that can meet an unrepresentable path; it MUST emit `ScanIssue` with `path: None` for that case, not T3

### Severity Rule: Error iff Opening Fence Found, Then Failed

- `Warning` = "file does not declare itself a frontmatter document or was skipped"
- `Error` = "file announced a frontmatter block (opening `---` found) and then broke its own promise"
- One rule, one predicate, directly testable — not eight separate verdicts
- **Implication for T4/T5**: May escalate a returned `Warning` to `Error` using caller context T3 lacks (e.g., "this file was expected to be a SKILL.md")

### `ScanIssue.reason` Is a Developer Diagnostic, Not Localized Copy

- Raw `serde_norway::Error` text embedded verbatim; not stable across versions, not user-facing
- Stable English prefix per failure class for human readability (logs, verbatim display)
- **Implication for T11/T12**: Render `severity` + `path` as user-facing copy; show `reason` as verbatim technical detail (monospace, collapsible). Zero T3-authored strings need i18n.

### Fixture Layout Separates Addressed Files from Walked Trees

- `tests/fixtures/frontmatter/` — T3 only, files addressed directly by path, **NEVER walked**
- `tests/fixtures/roots/` — RESERVED for T4+, whole trees walked from a root
- **Implication for T4+**: Add walked trees under `fixtures/roots/<client>/skills/...`. Do **NOT** aim a walker at `fixtures/frontmatter/`, or test assertions on "found N skills" will couple to T3's fixture count
- Fixture layout can be locked now because the separation is permanent

### The `yaml.rs` Single-Import Invariant Is Now Mechanically Enforced

- `tests/yaml_seam_invariant.rs` enforces it via a Rust test, not CI grep
- Runs locally on all three legs without workflow change — consistent with "no CI-workflow change" criterion
- Textual check; can be fooled by re-exports or certain macros, but acceptable for catching accidental breakage
- **Module doc caveat**: Write constraints in prose without path-form `serde_norway::` or `use serde_norway`, or the textual check false-positives

### `.gitattributes` Exists Now

- Two ordered rules scoped to the fixture tree
- Later rules win; `binary` macro line must follow the `-text` line
- **Implication for T4+**: Any future byte-exact fixture must be added under the existing `-text` rule. A binary fixture needs its own `binary` line after it.

### Delivery Incident: PR Chain Targeting Misalignment

**What happened**: T3 shipped as a 2-PR chain (`PR #3` infrastructure, `PR #4` module + tests), per the locked delivery decision. PR #4 was merged into its own base branch **26 seconds after PR #3 reached `main`**, before GitHub re-targeted PR #4's base to `main`. Result: `main` sat with fixtures and no reader; tests on `main` stayed green because fixtures are inert data. Discovered during verify phase by checking merge state, not by trusting the "merged" label.

**Remediation**: PR #5 cut from current `main`, so its content reached `main` cleanly. Also preserved `CLAUDE.md`, which the stale PR #4 base would have deleted.

**Lesson for future multi-PR changes in this repo**: Merge the infrastructure/fixture PR first, wait for GitHub to re-target the module PR's base, or stagger them with an explicit delay. The default GitHub behavior is not safe for chained PRs without explicit re-targeting coordination.

---

## Known Limitations (for T4 and T16)

### 1. UTF-8 BOM Is Unhandled

**Issue**: A leading UTF-8 BOM (`U+FEFF`) makes the first line `"\u{FEFF}---"`, so the fence comparison fails and the file falls into `NoOpeningFence` with a `Warning`. Graceful, non-panicking, but arguably a false negative on a Windows-authored file.

**Mitigation**: Deliberately not handled in T3 under strict TDD — the fixture set is locked at ten, and shipping an untested branch violates discipline.

**Recommendation**: Flagged for T4 (which sees real trees) and T16 (real-machine validation). If it occurs in practice, add a fixture and fix in the next phase.

### 2. T3's Non-Panic Guarantee Is Leaf-Level Only

- T3 guarantees: every input to `frontmatter::read` yields a value, never a panic
- T3 does **NOT** guarantee: if T4 finds 100 files and calls `read` on each, and 3 fail, the scan continues
- The "one failing adapter does not abort the rest of the scan" guarantee is **T9's** responsibility
- T3 supplies the leaf-level floor; T9 orchestrates recovery

### 3. Splitter Treats Any Line That Trims to `---` as Closing Fence

- No fixture exercises a `---` line inside a block scalar (e.g., `description: >` containing a line that is just `---`)
- If T4 or T5 meets one in a real file, that is where it will surface
- Strict TDD discipline: not shipping an untested branch, so this case is deliberately unfixed

---

## Scope Check (per rules.archive)

**Verified: Nothing out-of-scope crept in.**

| Scope Constraint | Status | Evidence |
|---|---|---|
| No directory walking, root discovery, plugin/project exclusion | CONFIRMED | `read` accepts `&Path`, performs zero path discovery or filtering |
| No duplicate consolidation | CONFIRMED | T3 is single-file, per-file logic; consolidation is T8's responsibility |
| No "one failing file aborts the scan" guarantee | CONFIRMED | T3 is leaf-level, per-file only; T9 owns scan-level recovery |
| No agent-specific fields (`model`, `tools`) | CONFIRMED | `SkillFrontmatter` holds only `name` and `description` |
| No IPC exposure or Tauri command | CONFIRMED | `SkillFrontmatter` has no `Serialize`/`TS` derives; no Tauri code anywhere in `frontmatter.rs` |
| No new dependencies | CONFIRMED | `Cargo.toml`, `deny.toml`, `.github/workflows/ci.yml` untouched |
| No write operations | CONFIRMED | `std::fs::read` only; no `File::create`, `OpenOptions::write`, `fs::write` anywhere in the module |

**Verdict**: PoC-compliant. Archive is safe to complete.

---

## Artifacts in This Archive

This folder contains:
- `proposal.md` — the original change proposal, now with the last Success Criterion ticked
- `explore.md` — exploration phase findings
- `design.md` — detailed design decisions with rationale and open questions
- `tasks.md` — 20 implementation tasks, all marked complete
- `apply-progress.md` — apply phase evidence, TDD cycle documentation, file changes, committed state
- `verify-report.md` — full verification matrix, spec compliance (19/19 scenarios compliant), issues list (0 CRITICAL, 1 WARNING W3 — bookkeeping gap)
- `specs/frontmatter-reader/spec.md` — delta spec for the new capability

**Spec merged into main specs:**
- Created `openspec/specs/frontmatter-reader/spec.md` from the delta spec above
- No modifications to existing specs (`domain-model`, `ci-quality-gates`)

---

## Traceability

All artifacts related to this change are persisted in this archive folder. The change is closed. No follow-up work is needed in T4–T10 regarding T3's deliverables themselves. T4 picks up with skill scanner implementation, building on top of T3's leaf-level reader and invariant.

---

## Blocks and Unblocks

**T3 Unblocks:**
- **T4** (skill scanner): walks search roots and calls `frontmatter::read::<SkillFrontmatter>(path)` for each discovered `SKILL.md`
- **T5** (Claude Code agent adapter): reuses `frontmatter::read::<AgentFrontmatter>` unchanged, proving the generic contract

**T3 Requires:**
- **T2** (domain model and type contract): complete and archived. T3 consumes `ScanIssue`, `IssueSeverity`, `Component.description: Option<String>` as merged.

---

**Archive Date**: 2026-08-18  
**Archived By**: sdd-archive executor  
**Status**: Complete and closed. Ready for release.
