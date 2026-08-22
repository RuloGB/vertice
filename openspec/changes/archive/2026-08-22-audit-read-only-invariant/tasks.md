# Tasks: Audit Read-Only Invariant

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | 420-560 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (`size:exception`) |
| Delivery strategy | exception-ok |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|---|---|---|---|
| 1 | Full T14 evidence slice | Single PR | Maintainer-approved `size:exception`; keep tests/docs together |

## Phase 1: Foundation
- [x] 1.1 RED — extend `crates/vertice-core/src/scan.rs` test helpers to expect full-tree snapshots (files, dirs, permission metadata, and modified timestamps; symlink entries only if present in the fixture). [T14/CA-16][Req: scan fixture proof][Seq]
- [x] 1.2 GREEN — implement snapshot structs/helpers in `crates/vertice-core/src/scan.rs` without changing production scan behavior. [T14/CA-16][Req: scan fixture proof][Seq]
- [x] 1.3 REFACTOR — keep snapshot ordering/hash logic deterministic and test-only in `crates/vertice-core/src/scan.rs`. [T14/CA-16][Req: scan fixture proof][Seq]

## Phase 2: Core audit coverage
- [x] 2.1 RED — create `crates/vertice-core/tests/read_only_audit.rs` with failing audit coverage for write, truncate, create, delete, rename, link, permission, and `Write`-trait mutation APIs across `crates/vertice-core/src/`. [T14/CA-16][Req: mutation classes][Seq]
- [x] 2.2 GREEN — implement sorted deny-list/allowed-exception audit in `crates/vertice-core/tests/read_only_audit.rs`, explicitly documenting static-proof limits. [T14/CA-16][Req: mutation classes][Seq]
- [x] 2.3 Verify fixture-tree proof still enforces in-memory-only reporting in `crates/vertice-core/src/scan.rs`. [T14/CA-16][Req: in-memory result][Par after 2.2]

## Phase 3: Desktop-shell audit
- [x] 3.1 RED — add `crates/vertice-app/tests/read_only_audit.rs` asserting `crates/vertice-app/src/commands.rs`, `crates/vertice-app/src/lib.rs`, and `crates/vertice-app/capabilities/default.json` expose only scan/rescan + `core:default`. [T14/CA-16][Req: minimal capability grant][Seq]
- [x] 3.2 GREEN — implement capability/command audit checks, including absence of fs/shell/dialog and mutation-scope strings. [T14/CA-16][Req: minimal capability grant][Seq]

## Phase 4: Verification evidence
- [x] 4.1 Update `openspec/changes/audit-read-only-invariant/tasks.md` progress notes and prepare `openspec/changes/audit-read-only-invariant/verify-report.md` checklist for automated scope + supplemental manual evidence. [T14/CA-16][Req: manual proof supplemental][Par after 3.2]
- [x] 4.2 Run targeted Rust tests for `scan.rs`, `crates/vertice-core/tests/read_only_audit.rs`, and `crates/vertice-app/tests/read_only_audit.rs`; record follow-up full-matrix commands in verify. [T14/CA-16][Req: verification evidence][Seq]

## Work-unit commits
1. `test(core): add failing full-tree read-only proof and mutation audit`
2. `test(app): audit tauri command and capability read-only surface`
3. `docs(openspec): record T14 verification evidence plan`


## Apply Progress Notes

- Implemented full-tree fixture immutability evidence in `crates/vertice-core/src/scan.rs` with file, directory, length, deterministic hash, platform permission evidence (Unix mode, Windows file attributes, or readonly fallback), and required modified-time snapshots. The reference fixture currently contains no symlink, so runtime symlink-preservation is not claimed; link mutation APIs remain covered by static audit.
- Added `crates/vertice-core/tests/read_only_audit.rs` to audit the scoped core source tree for write (including `std::fs::write`/`fs::write`), truncate, create, delete, rename, link, permission, copy, and `Write`-trait mutation APIs. Static-text audit is recorded as a regression guard, not a full transitive proof.
- Added `crates/vertice-app/tests/read_only_audit.rs` to audit `commands.rs`, `lib.rs`, and `capabilities/default.json` for scan/rescan-only IPC and exact `core:default` permissions with no filesystem/shell/dialog grants or scopes.
- Targeted Rust tests, core/app package tests, formatting, and clippy passed during apply. Full release/build matrix remains for `sdd-verify`.
