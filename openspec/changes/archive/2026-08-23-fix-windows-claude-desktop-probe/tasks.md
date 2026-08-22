# Tasks: Fix The Windows Claude Desktop Probe Path And Slot Vocabulary

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~450-650 (installations.rs ~150-200, fixture tree delete+recreate ~150-250, client_installations.rs ~150-200, scanDiagnostics.ts/.test.ts ~20) |
| 400-line budget risk | High |
| Chained PRs recommended | No (delivery strategy already resolved) |
| Suggested split | Single PR, `size:exception` |
| Delivery strategy | exception-ok |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Entire change | PR 1 (`size:exception`) | User pre-accepted the exception; fixture tree replacement is the bulk of the diff and is not independently shippable (old tree must go in the same commit the new one lands, per CA-17). |

## Phase 1: Fixture Foundation

- [x] 1.1 Create `crates/vertice-core/tests/fixtures/client-installations/` with cases: `packaged`, `legacy`, `packaged-and-legacy`, `two-packages`, `packaged-empty`, `non-claude-packages`, `nothing`, plus recreated npm-slot cases (error/isolation/determinism/read-only, malformed `package.json`).
- [x] 1.2 Delete `crates/vertice-core/tests/fixtures/installations/` entirely (CA-17 non-reuse).

## Phase 2: RED — Failing Tests (CA-7 pin, then CA-11 pin)

- [x] 2.1 In `crates/vertice-core/tests/client_installations.rs`, write failing test `packaged_and_legacy_yields_four_never_merged_claude_installs` against fixture `packaged-and-legacy` (spec scenario "One MSIX package and the legacy path both present, both counted" + npm/bundled non-merge).
- [x] 2.2 Write failing test `nothing_yields_zero_installs_three_warnings_zero_errors` against fixture `nothing`.
- [x] 2.3 Run `cargo test -p vertice-core --locked`, confirm both fail (compile error or assertion failure — no resolver exists yet).

## Phase 3: GREEN — Core Implementation

- [x] 3.1 In `crates/vertice-core/src/installations.rs`, replace `InstallKind` with private `InstallSlot { ClaudeCodeNpm, ClaudeCodeBundled, OpenCodeNpm }` exposing `client()`, `label()`, `version_source()`.
- [x] 3.2 Shrink `InstallProbe` to `{ slot, path }`.
- [x] 3.3 Implement `windows_install_probes(home) -> Vec<InstallProbe>`: enumerate `home/AppData/Local/Packages` filtering `Claude_*` via `OsStr::as_encoded_bytes().starts_with(b"Claude_")`, sort byte-wise, map to `<pkg>/LocalCache/Roaming/Claude/claude-code`, then unconditionally append the legacy path.
- [x] 3.4 Implement `resolve_slot(slot, candidates)` producing per-slot verdicts per the design's Error-Paths table (absent Packages dir = no event; unreadable Packages dir = `Error`, legacy still evaluated; per-`DirEntry` error = `Error`, continue; existing-but-empty candidate root = `Error`; no candidate at all = one `Warning`).
- [x] 3.5 Update reason-string construction to `format!("{} not detected", slot.label())` using labels `Claude Code CLI (npm)`, `Claude Code (bundled in Claude Desktop)`, `OpenCode (npm)`.
- [x] 3.6 Run `cargo test -p vertice-core --locked`; confirm both Phase 2 pins pass.

## Phase 4: Additional Test Coverage

- [x] 4.1 Add fixture-based cases to `client_installations.rs`: `two-packages`, `packaged-empty` (asserts `Error`, not `Warning`), `non-claude-packages` (contributes nothing, no issue), unreadable-`Packages` (Error + legacy still resolves).
- [x] 4.2 Add in-module unit tests in `installations.rs` for slot labels, candidate ordering, the byte-prefix filter, and `HostPlatform::current()` staying a `cfg!` expression (table compiled/testable on all OS legs).
- [x] 4.3 Remove the obsolete `desktop-empty` tripwire test; confirm its coverage is replaced by `packaged-empty` and `nothing`.
- [x] 4.4 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`.

## Phase 5: Frontend Sync

- [x] 5.1 Update `frontend/src/lib/scanDiagnostics.ts` `MISSING_CLIENT_REASONS` to the three settled strings, removing `"Claude Code (desktop) not detected"`.
- [x] 5.2 Update `frontend/src/lib/scanDiagnostics.test.ts` to the new strings, adding the scenario "a bundled-slot not-detected issue is classified as missing-client".
- [x] 5.3 From `frontend/`, run `npm run lint && npm run check && npm run test && npm run build` (never from `frontend/src/`).

## Phase 6: Cleanup & Verification

- [x] 6.1 Update the module doc comment in `installations.rs`: remove §-references to the superseded fixed-table design, describe the slot-grouped resolver.
- [x] 6.2 After `cargo test -p vertice-core --locked`, run `git diff --stat frontend/src/bindings/` and confirm it is empty (no exported type changed).
- [x] 6.3 Re-run the full gate: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, and from `frontend/`: `npm run lint && npm run check && npm run test && npm run build`.

## Phase 7: Adversarial Review Follow-Up (test-coverage gaps, no behaviour change)

- [x] 7.1 Extract the `file_name.to_str()` conversion inside `resolve_bundled_slot`'s per-version-directory loop into a pure helper `install_from_version_dir(slot, file_name, path) -> Result<ClientInstallation, ScanIssue>`; unit-test it directly with a synthetic non-UTF-8 `OsString` (`#[cfg(unix)]` via `OsStringExt::from_vec`, precedent `roots.rs::resolve_home_fails_on_non_utf8_path`) and, where constructible, an unpaired-surrogate case on `#[cfg(windows)]` via `OsStringExt::from_wide`. Assert `severity == Error` AND `path == None`.
- [x] 7.2 In `client_installations.rs`, add a raw (unsorted) order assertion on `scan.installations` for the `two-packages` fixture pinning that pkg1's install is enumerated before pkg2's through the real `read_dir` path, not only via the `filter_and_sort_claude_packages` unit test.
- [x] 7.3 Re-run the full gate (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, bindings diff check, and from `frontend/`: `npm run lint && npm run check && npm run test && npm run build`) and confirm all pass.
