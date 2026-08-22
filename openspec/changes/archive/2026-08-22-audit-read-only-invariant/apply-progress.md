# Apply Progress: Audit Read-Only Invariant

## Status

Done — 10/10 tasks completed for `audit-read-only-invariant` under Strict TDD with maintainer-approved `size:exception`.

## Completed Tasks

- [x] 1.1 RED — extended `scan.rs` expectations to require full-tree snapshots.
- [x] 1.2 GREEN — implemented test-only full-tree snapshot structs/helpers.
- [x] 1.3 REFACTOR — kept ordering/hash logic deterministic and test-local.
- [x] 2.1 RED — added failing core mutation-surface audit coverage.
- [x] 2.2 GREEN — implemented sorted core deny-list audit and static-proof limit evidence.
- [x] 2.3 Verified fixture-tree proof still enforces in-memory-only reporting.
- [x] 3.1 RED — added failing desktop-shell capability/command audit.
- [x] 3.2 GREEN — implemented scan/rescan-only and `core:default` audit checks.
- [x] 4.1 Prepared verify checklist for automated and supplemental manual CA-16 evidence.
- [x] 4.2 Ran targeted Rust tests and recorded full verify follow-up commands.

## TDD Cycle Evidence

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| 1.1 | `crates/vertice-core/src/scan.rs` | Unit | PASS `reference_fixture_is_fast_and_read_only` 1/1 before edit | PASS: compile failed on missing `fixture_tree_snapshot`/`FixtureEntryKind` | PASS: snapshot tests passed | PASS: existing read-only test + metadata coverage test | PASS: `cargo fmt`, targeted tests, clippy green |
| 1.2 | `crates/vertice-core/src/scan.rs` | Unit | PASS same scan safety net | PASS: covered by 1.1 RED | PASS: implemented full-tree helper; scan tests passed | PASS: file and directory/metadata paths exercised | PASS: deterministic FNV hash and sorted walk kept test-local |
| 1.3 | `crates/vertice-core/src/scan.rs` | Unit | PASS same scan safety net | PASS: covered by 1.1 RED | PASS: `reference_fixture_is_fast_and_read_only` passed | PASS: full-tree equality plus metadata test | PASS: `cargo fmt --all --check` and package tests passed |
| 2.1 | `crates/vertice-core/tests/read_only_audit.rs` | Integration | N/A (new file) | PASS: compile failed on missing `audit_core_mutation_surface` | PASS: core audit test passed | PASS: covered write/truncate/create/delete/rename/link/permissions/write_trait classes and explicit `std::fs::write`/`fs::write` patterns | PASS: sorted class list and scoped source walk |
| 2.2 | `crates/vertice-core/tests/read_only_audit.rs` | Integration | N/A (new file) | PASS: covered by 2.1 RED | PASS: core audit found no forbidden mutation APIs | PASS: added copy/create, `std::fs::write`/`fs::write`, covered-pattern evidence, and comment-stripping behavior | PASS: static-proof limit kept explicit |
| 2.3 | `crates/vertice-core/src/scan.rs` | Unit | PASS scan package baseline passed | PASS: covered by 1.1 RED | PASS: in-memory report assertions remained green | PASS: non-empty report plus unchanged full-tree snapshot | PASS: full `vertice-core` package tests passed |
| 3.1 | `crates/vertice-app/tests/read_only_audit.rs` | Integration | PASS `commands::tests` 3/3 before edit | PASS: compile failed on missing `audit_desktop_shell_read_only_surface` | PASS: app audit test passed | PASS: commands + permissions + forbidden surface checks | PASS: capability text audit scoped to permissions/scope keys |
| 3.2 | `crates/vertice-app/tests/read_only_audit.rs` | Integration | PASS app command tests remained green | PASS: covered by 3.1 RED | PASS: app package tests passed | PASS: IPC handler and `#[tauri::command]` extraction both checked | PASS: static-proof limit kept explicit |
| 4.1 | `openspec/changes/audit-read-only-invariant/verify-report.md` | Documentation | N/A (artifact update) | PASS: checklist absent before apply | PASS: verify checklist created | Skipped: structural evidence artifact | PASS: tasks progress notes updated |
| 4.2 | test commands | Verification | PASS baselines captured before edits | PASS: failing compile evidence captured after RED tests | PASS: targeted/package/workspace tests passed | PASS: targeted + package + workspace scopes run | PASS: formatter and clippy passed |

## Test Results

- Corrective RED evidence: `cargo test -p vertice-core reference_fixture_snapshot_tracks_files_directories_and_metadata --locked; cargo test -p vertice-core --test read_only_audit --locked` failed as expected on missing `permission_evidence` and `covered_patterns`, proving the prior implementation did not yet expose full permission evidence or explicit `std::fs::write`/`fs::write` pattern coverage.
- RED evidence: `cargo test -p vertice-core reference_fixture_snapshot_tracks_files_directories_and_metadata --locked; cargo test -p vertice-core --test read_only_audit --locked; cargo test -p vertice-app --test read_only_audit --locked` failed as expected on missing `permission_evidence`, `covered_patterns`, `fixture_tree_snapshot`, `FixtureEntryKind`, `audit_core_mutation_surface`, and `audit_desktop_shell_read_only_surface`.
- Safety net before edits: `cargo test -p vertice-core reference_fixture_is_fast_and_read_only --locked` — PASS, 1/1; `cargo test -p vertice-app commands::tests --locked` — PASS, 3/3.
- Targeted GREEN: `cargo test -p vertice-core reference_fixture_snapshot_tracks_files_directories_and_metadata --locked` — PASS; `cargo test -p vertice-core reference_fixture_is_fast_and_read_only --locked` — PASS; `cargo test -p vertice-core --test read_only_audit --locked` — PASS; `cargo test -p vertice-app --test read_only_audit --locked` — PASS.
- Package/quality gates: `cargo fmt --all --check` — PASS; `cargo test -p vertice-core --locked` — PASS (90 unit + integration/doc tests); `cargo test -p vertice-app --locked` — PASS; `cargo clippy --workspace --all-targets -- -D warnings` — PASS.
- Workspace gate: `cargo test --workspace --locked` — PASS.

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `crates/vertice-core/src/scan.rs` | Modified | Replaced file-byte fixture helper with test-only full-tree snapshot covering files, directories, length, deterministic file hash, platform permission evidence (Unix mode, Windows file attributes, or readonly fallback), and required modified timestamps; symlink entries are preserved if present, but the current reference fixture contains none, so runtime symlink coverage is not claimed. |
| `crates/vertice-core/tests/read_only_audit.rs` | Created | Added scoped core source audit for filesystem mutation classes and explicit static-proof limitation. |
| `crates/vertice-app/tests/read_only_audit.rs` | Created | Added desktop shell audit for scan/rescan command surface and exact `core:default` capability boundary. |
| `openspec/changes/audit-read-only-invariant/tasks.md` | Updated | Marked 10/10 tasks complete and added apply progress notes. |
| `openspec/changes/audit-read-only-invariant/verify-report.md` | Created | Added verify checklist and CA-16 evidence scope for automated plus supplemental manual verification. |

## Deviations from Design

None — implementation matches the corrected design. The app capability audit intentionally checks capability `permissions` and explicit `scope` keys rather than banning explanatory description text, preserving the existing audited capability file content.

## Static-Proof Limit

The mutation-surface audits are regression guards, not mathematical proof of every transitive or macro-hidden write. The runtime full-tree fixture assertion remains the automated proof layer, with manual/system-level evidence reserved as supplemental verify/archive evidence.

## Workload / PR Boundary

- Mode: single PR with maintainer-approved `size:exception`.
- Current work unit: full T14 evidence slice.
- Boundary: automated read-only proof and verification checklist only; no production behavior, IPC, UI, persistence, or generated binding contract changes.
- Estimated review budget impact: expected over 400 changed lines by approved exception.

## Remaining Tasks

None. Ready for `sdd-verify`.
