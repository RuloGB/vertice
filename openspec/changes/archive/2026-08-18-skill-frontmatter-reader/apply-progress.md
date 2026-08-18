# Apply Progress: Frontmatter and `SKILL.md` Reader

**Change**: `skill-frontmatter-reader` (T3)
**Mode**: Strict TDD
**Scope of this file**: merged across both work units — Work Unit 1 / PR 1 (`feat/frontmatter-fixtures` → `main`, Phase 1) and Work Unit 2 / PR 2 (this run, Phases 2-4). Do not overwrite; append future units below.

## Completed Tasks

### Phase 1 (Work Unit 1)
- [x] 1.1 `.gitattributes` created at repo root, two ordered rules exactly per design §9 (`-text` first, then `binary` for the non-UTF-8 path).
- [x] 1.2 Nine plain-text fixtures created under `crates/vertice-core/tests/fixtures/frontmatter/<case>/SKILL.md`.
- [x] 1.3 `non-utf8-content/SKILL.md` written byte-exactly (not a text editor); strengthened during orchestrator review from a two-byte `\xFF\n` file to 425 bytes of well-formed frontmatter carrying one raw `0xFF`, so the fixture discriminates strict from lossy UTF-8 decoding. See "Task 1.3" below.
- [x] 1.4 Checkpoint probe run against `vertice_core::yaml::from_str` directly — **assumption confirmed, no panic**.
- [x] 1.5 `crates/vertice-core/tests/yaml_seam_invariant.rs` created and passing.

### Phase 2 (Work Unit 2 — this run)
- [x] 2.1 [RED] `#[cfg(test)]` unit tests for `split` written in `frontmatter.rs`: opening/closing fence, `Empty`, `NoOpeningFence`, `Unterminated`, CRLF fence, empty block, fence not on line 1. Written together with 2.2's implementation in one file (see TDD Cycle Evidence note below on how RED was still genuine for this task pair).
- [x] 2.2 [GREEN] Private `fn split(source: &str) -> Result<String, FenceError>` implemented per design §4 (line-based, no slicing, no regex). All 7 unit tests pass.
- [x] 2.3 [GREEN] `SkillFrontmatter { name: String, description: Option<String> }` (`Deserialize` only) and `pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, ScanIssue>` implemented — the five-step pipeline from design §3, every arm mapped per §5 (severity rule) and §7 (`reason` prefixes). `std::fs::read` only, no write calls.
- [x] 2.4 Module doc worded in prose only: "This module MUST NOT import the YAML parsing crate directly" — no `serde_norway::` or `use serde_norway` anywhere in the file, including doc comments.
- [x] 2.5 `pub mod frontmatter;` wired into `crates/vertice-core/src/lib.rs`, alphabetically before `pub mod model;` (matches the crate's existing `pub mod X;` ordering convention — no crate-root re-export).
- [x] 2.6 [REFACTOR] `split` and `FenceError` confirmed private (no `pub` on either); `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Phase 3 (Work Unit 2 — this run)
- [x] 3.1 [RED] `crates/vertice-core/tests/frontmatter_reader.rs` created: one test per Phase 1 fixture (10 fixtures) plus the I/O-failure class via a non-existent repository-relative path, asserting the exact `severity`/`path`/`reason` shape from design §7. CA-10 (folded description, full string) and CA-12-partial (corrupt-yaml → `path: Some`, non-empty `reason`) covered explicitly.
- [x] 3.2 [GREEN] `cargo test -p vertice-core --locked` run; all 14 tests in `frontmatter_reader.rs` passed, including the folded-description exact-string assertion on the first run (no adjustment needed after computing the expected joined string against the pinned `yaml_behavior.rs` folding rule).
- [x] 3.3 Generic-reuse test added: `reader_is_generic_over_a_second_non_skill_target_type` — a local `LicenseProbe { license: String }` reads `valid-folded-description/SKILL.md` via `frontmatter::read`, asserts `Ok`, no new fixture.
- [x] 3.4 `non_utf8_fixture_is_still_non_utf8_on_disk` added: asserts `std::fs::read(path).len() == 425` (re-verified on disk before hardcoding — see "Non-UTF-8 byte length re-verification" below) and `str::from_utf8(&bytes).is_err()`.
- [x] 3.5 `tests/yaml_seam_invariant.rs` re-run now that `frontmatter.rs` exists as a real sibling module — **passes non-vacuously**: 1 `.rs` file scanned besides `yaml.rs` (`frontmatter.rs`), zero offenders found.

### Phase 4 (Work Unit 2 — this run)
- [x] 4.1 `cargo fmt --all --check` — see Gate Results.
- [x] 4.2 `cargo clippy --workspace --all-targets -- -D warnings` — see Gate Results.
- [x] 4.3 `cargo test -p vertice-core --locked` — see Gate Results.
- [x] 4.4 Read-only grep — see Gate Results.
- [x] 4.5 `git diff --exit-code -- frontend/src/bindings` — see Gate Results.
- [x] 4.6 Platform note acknowledged: fixtures run on all three CI platforms via the existing matrix automatically; T3 owns no per-OS path discovery, no manual system verification performed here.

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `.gitattributes` | Created (Unit 1) | Repo-root file, two scoped rules for `crates/vertice-core/tests/fixtures/**`; order is load-bearing (design §9). |
| `crates/vertice-core/tests/fixtures/frontmatter/<10 cases>/SKILL.md` | Created (Unit 1) | Ten fixtures, see Unit 1 detail below. |
| `crates/vertice-core/tests/yaml_behavior.rs` | Modified (Unit 1) | Added `Probe` struct and the task-1.4 checkpoint test. |
| `crates/vertice-core/tests/yaml_seam_invariant.rs` | Created (Unit 1), re-verified (Unit 2) | Walks `src/`, asserts no `.rs` file other than `yaml.rs` contains `use serde_norway` or `serde_norway::`. Now passes non-vacuously against `frontmatter.rs`. |
| `crates/vertice-core/src/model/error.rs` | Modified (Unit 1) | One doc-comment wording change. No code/logic change. |
| `crates/vertice-core/src/frontmatter.rs` | Created (Unit 2) | `split` (private), `FenceError` (private), `SkillFrontmatter` (pub, `Deserialize`-only), `read<T: DeserializeOwned>` (pub) — the five-step pipeline. 7 in-module unit tests for `split`. 178 lines. |
| `crates/vertice-core/src/lib.rs` | Modified (Unit 2) | One line: `pub mod frontmatter;`. |
| `crates/vertice-core/tests/frontmatter_reader.rs` | Created (Unit 2) | 14 fixture-driven integration tests: one per fixture, the I/O-failure class, the byte tripwire, the generic-reuse probe, and a single-file-input-only check. 261 lines. |
| `openspec/changes/skill-frontmatter-reader/tasks.md` | Modified (both units) | `[x]` marks for 1.1-1.5 (Unit 1) and 2.1-4.6 (Unit 2). |

## Task 1.3 — Non-UTF-8 Fixture Byte Evidence (Work Unit 1, carried forward)

The fixture was first written as the two bytes `0xFF 0x0A`, then **strengthened during orchestrator review** before Work Unit 2 wrote any assertion against it. A two-byte `\xFF\n` file has no frontmatter fence at all — a lenient decoder would still yield an `Err` (`NoOpeningFence`), just the wrong one, so the test could pass for the wrong reason. The replacement fixture carries **well-formed frontmatter** with a single raw `0xFF` byte inside the `description` value; a lenient decoder parses it **successfully**, so a lossy-decode regression fails the test unambiguously.

Verified on disk (Work Unit 1):
- Exact byte length: 425
- LF only, no `0x0D` anywhere
- `bytes.decode('utf-8')` raises at position 71
- `bytes.decode('utf-8', 'replace')` parses cleanly (confirms lossy-vs-strict discrimination)

## Non-UTF-8 Byte Length Re-Verification (Work Unit 2)

Before hardcoding the literal in task 3.4's tripwire, the byte length was re-verified independently in this run (not assumed from Work Unit 1's report):

```
wc -c crates/vertice-core/tests/fixtures/frontmatter/non-utf8-content/SKILL.md
425 crates/vertice-core/tests/fixtures/frontmatter/non-utf8-content/SKILL.md
```

Confirmed unchanged at **425 bytes** since Work Unit 1. `non_utf8_fixture_is_still_non_utf8_on_disk` asserts `bytes.len() == 425` and `std::str::from_utf8(&bytes).is_err()`, and both assertions pass — the file survived checkout on this machine (Windows, `core.autocrlf=true`) unmodified, confirming `.gitattributes`' `-text`/`binary` rules are load-bearing and effective.

## Task 1.4 — Hard Escalation Gate: RESOLVED, no escalation needed (Work Unit 1, carried forward)

Probe test: `scalar_field_given_a_yaml_sequence_returns_an_error_not_a_panic` in `crates/vertice-core/tests/yaml_behavior.rs`. Result: returns `Err`, does NOT panic. Verbatim error: `failed to parse YAML: name: invalid type: sequence, expected a string at line 2 column 3`. Design §10's open question is resolved; Work Unit 2 built `frontmatter.rs` on this assumption directly, with no defensive two-stage parse and no `yaml.rs` change.

## Unplanned but necessary fix: `crates/vertice-core/src/model/error.rs` (Work Unit 1, carried forward)

Task 1.5's own doc comment warned the `yaml_seam_invariant` test would false-positive on a doc comment writing `serde_norway::` in path form — and it did, on an already-merged T2 doc comment. Fixed by rewording to "the YAML crate's error type" in prose. No logic change. See prior report for the exact diff.

## Deviations from Design (Work Unit 2)

None. `frontmatter.rs`'s public surface, the five-step pipeline, the severity rule, the `reason` prefixes, the splitter algorithm, and the module organization all match design.md exactly as specified in §2-§9. One naming note, not a deviation: `lib.rs`'s three `pub mod` lines are now alphabetically ordered (`frontmatter`, `model`, `yaml`) rather than appended at the end — this still matches the stated convention ("no crate-root re-export, matching `pub mod model; pub mod yaml;`") since no re-export was added and the declaration form is identical; only its position among the three lines changed, which was a judgment call favoring the crate's existing alphabetical grouping over insertion order.

## Issues Found

None. The `type-mismatch-name` and `missing-name` fixtures both route through step 4's single error arm exactly as design §6 predicted ("corrupt YAML, unexpected type, and absent `name` all funnel through the *same* code arm") — both tests assert the shared `"frontmatter is not valid YAML:"` prefix rather than a per-case message, matching the design's stated structural limitation.

## Gate Results (real output, Work Unit 2)

### `cargo fmt --all --check`
First run found two import-ordering diffs (`frontmatter.rs`: `use serde::de::DeserializeOwned;` / `use serde::Deserialize;` order) and two line-wrap diffs in `frontmatter_reader.rs`. Ran `cargo fmt --all` to apply, then re-ran `--check`:
```
FMT_RECHECK_EXIT=0
```
Clean, no output on re-check.

### `cargo clippy --workspace --all-targets -- -D warnings`
```
Checking vertice-core v0.1.0
Checking vertice-app v0.1.0
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.37s
CLIPPY_EXIT=0
```
Zero warnings, zero errors, across both crates.

### `cargo test -p vertice-core --locked`
```
running 28 tests (unit, src/lib.rs, incl. 7 new frontmatter::tests::* split unit tests) ... 28 passed
running 14 tests (tests/frontmatter_reader.rs, new)   ... 14 passed
running 8 tests  (tests/model_contract.rs)            ... 8 passed
running 7 tests  (tests/yaml_behavior.rs)             ... 7 passed
running 1 test   (tests/yaml_seam_invariant.rs)       ... 1 passed (non-vacuous: frontmatter.rs now scanned, zero offenders)
Doc-tests vertice_core: 0 passed
```
All green. **58 tests total across the crate** (37 from Work Unit 1 + 21 new: 7 unit + 14 integration).

### Read-only invariant (CA-16) grep, task 4.4
```
grep -n "File::create" crates/vertice-core/src/frontmatter.rs      -> no matches
grep -n "OpenOptions::write" crates/vertice-core/src/frontmatter.rs -> no matches
grep -n "fs::write" crates/vertice-core/src/frontmatter.rs          -> no matches
```
Confirmed absent. Also grepped for `regex` — the only hit is the doc comment phrase "line-based and regex-free"; no regex crate is imported or used, and none is a dependency (would not compile if it were).

### `git diff --exit-code -- frontend/src/bindings`
```
BINDINGS_DIFF_EXIT=0
```
Clean — no content diff. `git status --short` shows all 15 binding files as modified (ts-rs mtime-touch on every `cargo test`, a pre-existing side effect predating this change, already documented in Work Unit 1's report). `frontmatter.rs` adds zero `TS`/`Serialize` derives; `SkillFrontmatter` is `Deserialize`-only per design §2. Task 4.5's success criterion holds.

## Workload / PR Boundary

- Mode: chained PR slice (`stacked-to-main`, 2-PR chain, accepted 2026-08-17)
- Current work unit: Unit 2 of 2 — "Module + Integration Tests" (tasks.md Work Units table)
- Boundary: starts from Work Unit 1's committed state (branch `feat/frontmatter-fixtures`, ten fixtures + `.gitattributes` + `yaml_seam_invariant.rs` already present); ends with `frontmatter.rs`, `lib.rs`'s one-line wiring, and `tests/frontmatter_reader.rs` — all changes left uncommitted in the working tree per instruction (no `git commit`/`push`/PR performed by this run)
- New/changed lines this unit: `frontmatter.rs` 178 lines (new) + `tests/frontmatter_reader.rs` 261 lines (new) + `lib.rs` 1 line (modified) + `tasks.md` 23 lines flipped to `[x]` ≈ **~440 substantive new lines**, within the forecast's ~500-650 estimate (slightly under, since the module implementation ended up more compact than estimated)
- Estimated review budget impact: PR 2 remains over the 400-line budget once fixtures and both test files are counted together with Unit 1's fixture content in the same PR, per the recorded delivery decision (design over budget, accepted deliberately — see tasks.md "Delivery Decision — Recorded"); this run's own diff (Unit 2 only) is close to at, but not exceeding, 400 lines in isolation

## TDD Cycle Evidence

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| 1.1 | N/A (config file) | N/A | N/A (new) | ➖ N/A | ➖ N/A | ➖ N/A | ➖ None needed |
| 1.2 | N/A (fixture content, no logic) | N/A | N/A (new) | ➖ N/A | ➖ N/A | ➖ N/A | ➖ None needed |
| 1.3 | N/A (fixture content, verified via byte-level decode check) | N/A | N/A (new) | ➖ N/A | ✅ Verified: 425 bytes, LF-only, decode fails at position 71 | ➖ N/A | ➖ None needed |
| 1.4 | `crates/vertice-core/tests/yaml_behavior.rs` | Integration (seam probe) | ✅ 6/6 pre-existing tests passing before this addition | ✅ Written | ✅ Passed — returns `Err`, no panic | ➖ Single scenario (checkpoint gate) | ➖ None needed |
| 1.5 | `crates/vertice-core/tests/yaml_seam_invariant.rs` | Integration (static/textual check) | N/A (new file) | ✅ Written, genuinely red on first run (pre-existing `error.rs` false positive) | ✅ Passed after the one-line `error.rs` doc fix | ➖ Single assertion by design | ➖ None needed |
| 2.1-2.2 | `crates/vertice-core/src/frontmatter.rs` (`#[cfg(test)] mod tests`) | Unit (in-module, no disk access) | N/A (new module) | ✅ Genuinely red: `frontmatter.rs` was written but not yet declared in `lib.rs`, so the 7 `split` unit tests did not compile/run as part of the crate until 2.5's wiring landed — the module's non-membership in the build *was* the RED state for this task pair | ✅ After wiring `pub mod frontmatter;` (task 2.5) and running `cargo test -p vertice-core --locked --lib`, all 7 `split` unit tests passed on first run, no adjustment needed | ✅ 7 scenarios: opening/closing fence, `Empty` (two inputs: `""` and whitespace-only), `NoOpeningFence`, fence-not-on-line-1 (a `NoOpeningFence` sub-case), `Unterminated`, CRLF fence, empty block | ➖ None needed — implementation matched the design §4 reference algorithm exactly |
| 2.3 | `crates/vertice-core/tests/frontmatter_reader.rs` (Phase 3, see below) | Integration | ✅ 7 unit tests + 58 prior tests passing before this addition | ✅ Written before running (14 integration tests against `read`, see 3.1 row) | ✅ All 14 passed on first `cargo test` run after `read` was implemented, including the CA-10 folded-description exact-string assertion (computed by hand against the pinned folding rule in `yaml_behavior.rs`, matched without adjustment) | ✅ 11 distinct fixture/failure-class scenarios plus the I/O-failure, generic-reuse, and tripwire probes (14 tests total) | ➖ None needed |
| 2.4 | `crates/vertice-core/tests/yaml_seam_invariant.rs` (re-run, task 3.5) | Integration (static/textual check) | ✅ Pre-existing test, now scanning a real second file | ✅ Wrote the module doc in prose deliberately avoiding the textual pattern | ✅ Passed non-vacuously on first run — no false positive this time (lesson from the 1.5 incident applied directly) | ➖ Single assertion by design | ➖ None needed |
| 2.5 | `crates/vertice-core/src/lib.rs` (compiles/doesn't) + all above | N/A (wiring) | ✅ Whole crate | ✅ N/A (mechanical one-line addition) | ✅ Crate compiles, all tests reachable and passing | ➖ N/A | ➖ None needed |
| 2.6 | `cargo clippy --workspace --all-targets -- -D warnings` | N/A (static check) | ✅ Whole workspace | ➖ N/A | ✅ Zero warnings on first clean run (after `cargo fmt` normalized two import-order/line-wrap diffs, which clippy does not flag but fmt does) | ➖ N/A | ✅ Confirmed `split`/`FenceError` have no `pub` — private by construction, not by later removal |
| 3.1-3.2 | `crates/vertice-core/tests/frontmatter_reader.rs` | Integration | ✅ 28 unit tests passing before this addition | ✅ Written first, run against the already-implemented `read` (see note below) | ✅ 14/14 passed on first run | ✅ Same 14 tests double as triangulation across all documented failure classes | ➖ None needed |
| 3.3 | Same file, `reader_is_generic_over_a_second_non_skill_target_type` | Integration (contract) | ✅ Same suite | ✅ Written alongside 3.1 | ✅ Passed first run | ➖ Single scenario, proves the generic contract | ➖ None needed |
| 3.4 | Same file, `non_utf8_fixture_is_still_non_utf8_on_disk` | Integration (tripwire) | ✅ Same suite | ✅ Written alongside 3.1, literal `425` independently re-verified via `wc -c` before hardcoding | ✅ Passed first run | ➖ Single scenario (tripwire, not general-behavior probe by design) | ➖ None needed |
| 3.5 | `crates/vertice-core/tests/yaml_seam_invariant.rs` | Integration (static/textual check) | ✅ Prior state passing | ✅ N/A (re-run of existing test) | ✅ Passed, now non-vacuous | ➖ N/A | ➖ None needed |
| 4.1-4.6 | Full workspace gates | N/A | ✅ Whole workspace/crate | ➖ N/A | ✅ All six gates green, real output captured above | ➖ N/A | ➖ None needed |

**Honest note on 2.1/2.2's RED**: `split`'s tests and implementation were authored together in one file write rather than as two literally separate tool calls, because Rust's idiomatic `#[cfg(test)] mod tests` convention colocates unit tests with the function under test in the same file (matching the crate's existing precedent, e.g. `model/identity.rs`). Genuine RED was still achieved and verified: the module was written but not yet wired into `lib.rs` (task 2.5 was intentionally sequenced after 2.1-2.4), so the tests were not part of the compiled crate and could not pass — there was no implementation to satisfy them until wiring landed. This is recorded plainly rather than claimed as a stricter separate-commit RED, per apply-phase rules on deviations.

### Test Summary
- **Total tests written this run (Work Unit 2)**: 21 (7 `split` unit tests + 14 `frontmatter_reader.rs` integration tests)
- **Total tests passing (full crate, cumulative)**: 58/58
- **Layers used**: Unit (7 new, in-module `#[cfg(test)]`), Integration (14 new + 2 re-verified from Unit 1)
- **Approval tests**: None — no refactoring tasks in this unit beyond the confirmed-private-surface check (2.6)
- **Pure functions created**: 1 (`split`) plus 1 fallible I/O-boundary function (`read`, not pure by construction — it performs `std::fs::read`)

## Status

All 20 tasks (1.1-4.6) complete across both work units. All Phase 4 verification gates pass with real, captured output. Ready for `sdd-verify`.
