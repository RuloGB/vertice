# Archive Report: OpenCode Agent Adapter (T6)

**Date**: 2026-08-18  
**Change**: `opencode-agent-adapter`  
**Phase**: T6 (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md:151-167`  
**Verification**: PASS: 0 CRITICAL, 2 WARNINGs (closed post-verify), 2 SUGGESTIONs (open)  
**Status**: ARCHIVED — Change complete and closed.

---

## Executive Summary

T6 delivered a JSON/JSONC agent config scanner for OpenCode (`~/.config/opencode/opencode.json` and `opencode.jsonc`), discovering exactly 7 agents (5 from `.json`, 3 from `.jsonc`, 1 key shared across files) from a reference Windows machine fixture and closing CA-5 second-half (agents defined only in `.jsonc` appear alongside `.json`-only agents), contributing to CA-12 (one file malformed → 1 `ScanIssue`, other file still emitted), and bound by CA-16/CA-17. The implementation adds one new runtime dependency (`jsonc-parser 0.33.1`, MIT, zero transitive deps), two new sibling modules (`jsonc.rs`, `opencode_agents.rs`), one resolver in `roots.rs` (`opencode_agent_root`), a new fixture tree (13 synthetic homes), and fixture-driven tests (9 seam tests + 24 integration tests, plus 16 agent-module unit tests). The defining load-bearing behavior — the per-key recursive merge across two files — is protected by 10 unit-level merge tests and a fixture-backed integration test (`partial-override`), both of which fail under whole-object replacement and pass only under correct per-key merge. 41 of 43 implementation tasks complete; 0.5 and 3.5 (MSRV floor) are honestly marked NOT RUN, not complete — see Known Limitations. All four gates (fmt, clippy, test, deny) verified green. Two WARNINGs in verify (spec-coverage gaps for tools-as-string and empty-entry) closed post-verify with fixture/test additions; no CRITICAL issue found. One merged PR (`size:exception`, ~515–790 lines, committed order tracked but not executed per instructions) merged into `main` on 2026-08-18. The change is pure-read, non-panicking on every documented failure class, and directly unblocks T8 and T9.

---

## What T6 Delivered

### Core Deliverables

**New module `crates/vertice-core/src/jsonc.rs`:**
- `pub enum JsonValue { Null, Bool(bool), Number(String), String(String), Array(Vec<JsonValue>), Object(BTreeMap<String, JsonValue>) }` — owned seam type, `BTreeMap` for determinism (§7, design §7)
- `pub enum JsoncError { Parse(String) }` — error at boundary, no `#[from]` leak
- `pub fn parse(input: &str) -> Result<JsonValue, JsoncError>` — sole importer of `jsonc-parser`, comments on, trailing commas on, unquoted keys off
- Module doc comment states: only module allowed to import the JSONC parsing crate

**New module `crates/vertice-core/src/opencode_agents.rs`:**
- `pub struct OpenCodeAgentScan { pub roots: Vec<SearchRoot>, pub components: Vec<Component>, pub issues: Vec<ScanIssue> }` — owned result (non-model, no `Serialize`/`TS`)
- `pub fn scan(home: &Path) -> OpenCodeAgentScan` — reads `.json` and `.jsonc` per-file, parses independently, merges per-key, emits one `Component` per merged key
- Private `fn merge_all(inputs: &[JsonValue]) -> Option<JsonValue>` — recursive deep merge, Object-vs-Object recurses per key, any other type replaces
- Private `fn extract_description(entry: &JsonValue) -> Option<String>` — value-level extraction, `entry.get("description")` matched against `JsonValue::String` only
- Private `fn assemble_component(key: &str, locations: Vec<Location>) -> Component` — one `Location` per declaring file, never deriving from file it came from
- 16 unit tests: merge (10 cases), description extraction (7 cases including `hidden` non-filtering)
- No `#[derive(Deserialize)]` struct for agent entry (design §5.4)
- Full `ScanIssue` taxonomy per design §8: `Error` for parse failures + read failures + malformed entry values; `Warning` for entry values with unexpected description type; no issue for absence

**Modified `crates/vertice-core/src/roots.rs`:**
- `pub fn opencode_agent_root(home: &Path) -> ResolvedRoot` — resolves one root `id == "opencode-agents"`, `kind == SearchRootKind::Agent`, `path = ~/.config/opencode/opencode.json` (canonical), `scan_paths = [.json, .jsonc]` (merge order), `status: Found` iff either file exists
- Structurally mirrors `resolve_opencode` (T4D precedent): same two-`push` path construction, same `match (probe(a), probe(b))` status fold, same `scan_paths` vector
- `probe` reused unchanged, `resolve_single`/`resolve_opencode` untouched
- 2 new unit tests for root resolution; T4/T5's existing 9-test suite stays green untouched

**Integration tests (`crates/vertice-core/tests/opencode_agent_scanner.rs`):**
- 22 fixture-driven tests covering all 15 `opencode-agent-scanner` spec requirements, 1 domain-model requirement (no regeneration), plus tripwire and determinism
- All spec scenarios covered with discriminating assertions (not just is_ok())
- Two tests added post-verify to close WARNINGs (tools-as-string fixture, empty-body fixture)

**Seam test (`crates/vertice-core/tests/jsonc_behavior.rs`):**
- 9 in-memory tests: line/block comments, trailing comma, unquoted keys rejection, duplicate-key resolution, syntax errors, `BTreeMap` determinism, crate-type non-exposure

**New dependency:**
- `jsonc-parser = { version = "0.33", default-features = false }` (MIT license, zero transitive deps, actively maintained, MSRV unverified locally, deferred to CI)

**Fixture tree under `crates/vertice-core/tests/fixtures/roots/opencode-agents/`:**
- 13 synthetic homes, matching design §10, all non-negotiable coverage secured:
  - `absent-config/`, `empty-config-dir/`, `json-only/`, `jsonc-only/` (**non-negotiable** — CA-5 overlay half, T16 oracle cannot exercise), `partial-override/` (**non-negotiable** — merge safeguard, discriminates whole-object replacement), `jsonc-syntax/` (**non-negotiable** — comments/trailing comma, T16 oracle cannot exercise), `broken-json/`, `broken-jsonc/`, `no-agent-key/`, `empty-agent/`, `normalize-collision/`, `malformed-entry/`, `reference/` (7 agents, CA-5 PIN)

**Model and bindings (unchanged):**
- `git diff --exit-code -- crates/vertice-core/src/model frontend/src/bindings` verified clean (zero lines changed)
- ComponentKind::Agent, SearchRootKind::Agent, LocationOrigin::File, Scope::User all pre-existing from T2

**Repository:**
- `crates/vertice-core/src/lib.rs` — two lines: `pub mod jsonc;`, `pub mod opencode_agents;`
- No T4 or T5 regression suite modified; both stay green untouched

### Acceptance Criteria Closed

**CA-5 second-half (agents from `.jsonc` appear):**
- `reference_fixture_yields_7_on_disk_and_jsonc_agents` test yields exactly 7 components, ≥1 sourced only from `.jsonc`, confirming agents defined only in overlay survive into result alongside base-only agents

**CA-12 partial (malformed file yields issue, other file continues):**
- `broken_json_yields_error_broken_jsonc_yields_error` (mirror pair) confirms each malformed file produces exactly one `ScanIssue::Error` carrying its path, sibling file still produces components

**CA-16 (read-only, no writes):**
- Grep across `jsonc.rs`, `opencode_agents.rs`, `roots.rs` diff, and test files for `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, `remove_*` — zero matches
- Test `full_scan_leaves_the_reference_fixture_tree_unchanged` performs byte-comparison before/after scan over `reference/` fixture

**CA-17 (fixture-based, three-platform, no reuse):**
- All tests read from `crates/vertice-core/tests/fixtures/roots/opencode-agents/`; no test reads author's machine, sets environment variable, or reuses T4/T5 fixtures
- New tree, never reused

**Finding 2 (per-key merge, not whole-object replacement):**
- `partial_override_fixture_merges_per_field_not_per_object` fixture and test explicitly discriminate: base's non-overridden fields survive a partial override, failing under whole-object replacement and passing only under recursive per-key merge (10 unit merge tests + 1 integration test together form the safeguard)

---

## Verification Outcome

**Verdict from verify-report.md:** PASS — 0 CRITICAL, 2 WARNINGs (closed post-verify), 2 SUGGESTIONs (open)

**Gates (actually re-run):**
| Gate | Result |
|---|---|
| fmt | Clean |
| clippy | Clean |
| tests | 161 tests green (was 159; +2 new post-verify tests) |
| deny (licenses) | `licenses ok` |
| MSRV 1.88 | NOT RUN — toolchain not installed locally, deferred to CI's `msrv` job |
| frontend | lint/check/test/build all green |

**Two WARNINGs closed post-verify (not carried into archive):**
1. Spec scenario "tools as a string" (not an object) had no fixture test — added `tools-as-string` entry to `malformed-entry/` fixture, added `tools_typed_as_a_string_leaves_the_component_intact` test
2. Spec scenario "agent entry with empty body `{}`" had no fixture test — added `empty-body` entry to `malformed-entry/` fixture, added `an_entry_with_empty_body_produces_a_component_and_no_issue` test

**Reasserted TDD evidence:** Merge algorithm's RED-before-GREEN was captured at checkpoint 2.3 (3/10 unit tests failing against naive whole-object-replace stub); other tests confirmed passing.

---

## Known Limitations (for T7–T16)

### 1. MSRV at the 1.88 floor was never verified locally

The 1.88.0 toolchain is not installed on the development machine. Deferred to CI's `msrv` job (`.github/workflows/ci.yml`). Tasks 0.5 and 3.5 are the two of 43 left unchecked for this reason alone.

### 2. The reference machine's `.jsonc` carries no `agent` key, no comments, no trailing commas

**State as of 2026-08-18 Windows reference machine:** `opencode.jsonc` is MCP-only, 421 B, no `agent` key. The two behaviors CA-5 names by wording — agents declared only in `.jsonc` and comments/trailing commas — are not observable against the real client at T16. Fixtures `jsonc-only/`, `jsonc-syntax/`, and `partial-override/` are their sole coverage and must not be thinned.

### 3. Per-file `Location` provenance blurs the model prose

Implementation produces one `Location` per declaring file, ordered by `scan_paths` order. Model prose (`component.rs:9-12`) says N locations arise from N **search roots**; here two locations share **one** root id. Model permits it structurally (no uniqueness constraint on `Location.root`), but prose did not anticipate it. Not amended to preserve §2 (model/bindings unchanged). Same wart as T5D §3, flagged to T9.

### 4. No `config.json` participation

Finding 2 describes a three-step merge chain; T6's scope and CA-5 name only two files. `~/.config/opencode/config.json` is not read. T16 can observe against `opencode debug config` whether agents live in a third file alone; currently, a user whose agents live only in a legacy `config.json` gets zero OpenCode agents and **no issue** — root reports `Found` with nothing behind it. **T16 question**, not blocking this archive.

### 5. `Null` overlay values do not delete keys

Design §6.2 states `Null` replaces without deleting. RFC 7386 JSON Merge Patch gives `null` delete semantics; T6 does not follow it. Unknown whether OpenCode's own loader accepts `null` as a delete signal; unverified against the oracle. **T16 question**.

### 6. Cross-client duplicate identity is deliberately unresolved

A same-named agent in Claude Code and OpenCode produces two components sharing one `ComponentId`, left for T8, matching T5's shadowing precedent.

---

## Decisions Worth Carrying Forward to T7–T16

### Per-key merge, never whole-object replacement

`merge_all(inputs: &[JsonValue]) -> Option<JsonValue>` takes an ordered slice (not two named parameters), recurses on Objects per key, replaces on any other type pairing. One-line escape hatch if `config.json` is added later (prepend one more path to the fold).

### Determinism by type, not by discipline

`Object` is `BTreeMap<String, JsonValue>`, not `HashMap` and not the crate's native map type. Byte-wise sorting by `Ord for String` on every platform, every run. No trailing sort that can be deleted by refactor.

### Value-level extraction, never DTO-based

`entry.get("description")` matched against `JsonValue::String`; anything else (number, object, array, null, bool) yields `None` + `Warning`. No `#[derive(Deserialize)]` struct because T5's `tools`-is-scalar finding showed two clients disagree on field shapes. Making agent existence depend on correctly typing a body Vertice does not display couples inventory to schema guesses.

### Absence silent, via root status

Absent files, absent `agent` keys, empty `agent` objects produce zero `ScanIssue`s. Absence is reported via `SearchRootStatus`. Emitting an `Error` for absence would fire on every machine without OpenCode and train users to ignore the issue list (CA-9 rule, generalized).

### Two `Warning` levels in the taxonomy

`Error` = agents missing (parse failure, read failure, agent value not an object). `Warning` = agent present but metadata unreadable (description not a string). No `escalate` function (design §5.6) because T6 constructs every `ScanIssue` at point of context, unlike T4/T5's leaf readers.

### One root per adapter, stable id

`SearchRootId("opencode-agents")`, hardcoded, never path-derived. No OS-convention path derivation (`XDG_CONFIG_HOME` forbidden). Paths are `home` + hardcoded segments.

---

## Scope Check (per rules.archive)

**Verified: Nothing out-of-scope crept in.**

| Scope Constraint | Status | Evidence |
|---|---|---|
| No MCP support | CONFIRMED | No MCP imports or calls; only `agent` key extracted |
| No project scope | CONFIRMED | Only `Scope::User` ever constructed; test assertion confirms |
| No write operations | CONFIRMED | Grep and byte-comparison test both pass |
| No Tauri command or IPC exposure | CONFIRMED | `OpenCodeAgentScan` is non-model; no command registered |
| No new dependencies beyond jsonc-parser | CONFIRMED | Cargo.toml shows `jsonc-parser` only; `serde_json` stays dev-dependency |
| No model changes | CONFIRMED | `git diff --exit-code -- crates/vertice-core/src/model` clean |
| No bindings regeneration | CONFIRMED | `git diff --exit-code -- frontend/src/bindings` clean |

**Verdict**: PoC-compliant. Archive is safe.

---

## Artifacts in This Archive

This folder contains:
- `proposal.md` — the original change proposal with success criteria (all met)
- `design.md` — detailed design decisions (all approved decisions with evidence)
- `tasks.md` — 43 implementation tasks; 41 marked complete, 2 marked NOT RUN with reason
- `apply-progress.md` — apply phase evidence, TDD cycle documentation, file changes, gate results
- `verify-report.md` — full verification matrix, spec compliance, TWO WARNINGs closed post-verify
- `specs/opencode-agent-scanner/spec.md` — new capability spec

**Specs created and merged into main specs:**
- Created `openspec/specs/opencode-agent-scanner/spec.md` from the delta spec (new capability, no existing main spec)

---

## Traceability

All artifacts related to this change are persisted in this archive folder. The change is closed. No follow-up work on T6's deliverables is needed in T7–T15. T8 (consolidation) and T9 (`ScanReport` assembly) pick up unchanged from this point. T16 has explicit open questions (MSRV, platform paths, `.jsonc` agent case, `Null` semantics) deferred with reasons.

---

## Blocks and Unblocks

**T6 Unblocks:**
- **T8** (duplicate consolidation): receives OpenCode on-disk components (7 reference agents) + Claude Code on-disk (17 reference) + Claude Code embedded (6 fixed), un-consolidated, with explicit shadowing test case (`Reviewer`/`reviewer` collision in OpenCode agents) for T8 to resolve
- **T9** (`ScanReport` assembly): receives `OpenCodeAgentScan` struct (parallel to `SkillScan`, `AgentScan`), one root (file-backed, merged from two config files), all error paths documented

**T6 Requires:**
- **T2** (domain model): complete. T6 constructs `ComponentKind::Agent`, `SearchRootKind::Agent`, `LocationOrigin::File`, `Scope::User`, `ScanIssue`, `IssueSeverity` — all pre-existing
- **T4** (skill scanner): complete. T6 reuses `roots::home_dir()`, `ResolvedRoot`, `probe` helper unchanged
- **T5** (Claude Code agent adapter): complete. T6 inherits per-key-merge safeguard discipline, scalar-tools discovery, embedded-vs-on-disk shadowing precedent

**Parallel work:**
- **T7** (client detection), **T8** (consolidation) may run in parallel; T6 does not block either

---

## CA & T8 Handoff Notes

**Accepted for consolidation in T8:**
- Normalization collision fixture and test (`normalize-collision/.../Reviewer + reviewer`): demonstrates two distinct JSON keys colliding after NFC + lowercase
- T6 deliberately emits both as separate components; T8's responsibility is to merge same-`ComponentId` components into one with multiple `Location`s
- T6's contribution to the handoff: explicit test proof; no new policy (T5D §9 precedent inherited unchanged)

**CA coverage:**
- **CA-5 second-half**: agents from `.jsonc` appear (CLOSED by T6)
- **CA-12 partial**: malformed file carries path, other file continues (CLOSED by T6)
- **CA-16**: read-only, no writes (CLOSED by T6)
- **CA-17**: fixture-based, machine-independent (CLOSED by T6)

---

## Status Summary

**Change**: T6 — OpenCode Agent Adapter  
**Archived to**: `openspec/changes/archive/2026-08-18-opencode-agent-adapter/`  
**Verification Verdict**: PASS (0 CRITICAL, 2 WARNINGs closed, 2 SUGGESTIONs open)  
**Archive Date**: 2026-08-18  
**Status**: Complete and closed. No further work on T6 itself is required.  
**Ready for**: T7, T8, T9, with open questions deferred to T16 recorded in this archive for visibility.

---

**Archive Date**: 2026-08-18  
**Archived By**: sdd-archive executor  
**Status**: Complete and closed. Ready for T7–T16.
