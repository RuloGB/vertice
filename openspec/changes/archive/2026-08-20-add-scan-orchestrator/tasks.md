# Tasks: Add Scan Orchestrator

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 450–650 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 → PR 2 (historical technical recommendation; approved delivery is one PR) |
| Delivery strategy | exception-ok |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: size-exception
400-line budget risk: High

### Historical Technical Split Recommendation

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Core facade and complete composition | PR 1 | Base: main; tests and combined fixture included. |
| 2 | Diagnostics, CA-15, and read-only proof | PR 2 | Base: PR 1 branch; independently verifiable. |

## Phase 0: T9 Test Fixtures (CA-12, CA-15)

- [x] 0.1 RED — Create versioned combined homes under `crates/vertice-core/tests/fixtures/scan-orchestrator/` for complete, corrupt-skill, missing-root/client, and reference-volume cases; add failing `scan.rs` tests against each fixture.
- [x] 0.2 RED — In `crates/vertice-core/src/scan.rs` tests, assert six roots, all installations/issues, T8 identity consolidation, corrupt `SKILL.md` path isolation (CA-12), and `duration_ms < 2_000` (CA-15).

## Phase 1: T9 Core Orchestration (CA-12, CA-15)

- [x] 1.1 GREEN — Add `crates/vertice-core/src/scan.rs` with public `scan() -> Result<ScanReport, ScanError>` resolving home once, plus private `scan_for(&Path, HostPlatform)` for deterministic fixture tests.
- [x] 1.2 GREEN — In `scan.rs`, invoke `skills::scan`, `agents::scan`, `opencode_agents::scan`, and `installations::scan_for`; move all outputs into one in-memory `ScanReport` and apply `consolidate`.
- [x] 1.3 GREEN — In `scan.rs`, append warning `ScanIssue`s for unique `SearchRootStatus::NotFound` roots while preserving adapter parse and not-detected-client issues; measure with `Instant` and saturate milliseconds to `u32`.
- [x] 1.4 GREEN — Modify `crates/vertice-core/src/installations.rs` to expose the current-platform selector crate-wide; modify `crates/vertice-core/src/lib.rs` to export `scan` without adding Tauri, IPC, SQLite, or persistence.

## Phase 2: T9 Isolation and Read-Only Verification (CA-12, CA-15)

- [x] 2.1 GREEN — Complete `scan.rs` fixture tests proving unavailable roots and clients remain visible, valid sibling adapter output survives corrupt input, and every report collection is in memory.
- [x] 2.2 GREEN — Add a fixture-tree byte snapshot assertion in `scan.rs` tests before/after `scan_for`; prove CA-16 read-only behavior without real-machine inputs.
- [x] 2.3 REFACTOR — Keep orchestration outside `src/model/`, remove duplication without changing adapter parsing, root/probe tables, T8 semantics, diagnostics, or generated bindings.

## Phase 3: T9 Quality Gate (CA-12, CA-15)

- [x] 3.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test -p vertice-core --locked`; fix only failures caused by T9.
- [x] 3.2 Inspect the T9 diff for `File::create`, `OpenOptions::write`, SQLite/persistence, `tauri` imports, IPC, and frontend changes; reject all as out of scope.
