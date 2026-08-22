# Verify Report: Fix The Windows Claude Desktop Probe Path And Slot Vocabulary

Change: fix-windows-claude-desktop-probe
Mode: openspec (full artifact set: proposal, delta spec, design, tasks)
Working tree state: uncommitted (verified via git status/git diff)

## Completeness

All 7 phases in tasks.md (25 subtasks) marked [x]. Every task claimed artifact was independently confirmed to exist and do what the task describes, no phantom checkmarks found.

## Gate Evidence (re-run live, not trusted from prior report)

| Gate | Result |
|---|---|
| cargo fmt --all --check | Pass, clean |
| cargo clippy --workspace --all-targets -- -D warnings | Pass, clean |
| cargo test --workspace --locked | Pass, all suites green, including client_installations.rs (20/20) and installations::tests (16/16 in-module) |
| cargo deny check bans licenses | bans ok, licenses ok (2 unrelated license-not-encountered warnings, not failures) |
| npm run lint (frontend) | Pass, no output |
| npm run check (frontend) | 203 FILES 0 ERRORS 0 WARNINGS |
| npm run test (frontend) | 10 files, 94 tests passed |
| npm run build (frontend) | Builds clean |
| git diff --stat -- frontend/src/bindings/ | Empty, confirms design "no exported type changed" invariant |

## Spec Compliance Matrix

| Requirement/Scenario | Covering test | Status |
|---|---|---|
| Windows Probe Paths Are Hardcoded, npm slots no enumeration | windows_install_probes_builds_npm_slots_as_home_plus_hardcoded_segments | PASS |
| Claude Code npm And Bundled Are Never Merged (CA-7), different versions | packaged_and_legacy_yields_four_never_merged_claude_installs | PASS |
| Each Bundled-Slot Version Directory Is Its Own Installation, MSIX + legacy both counted | packaged_and_legacy_yields_four_never_merged_claude_installs | PASS |
| Multiple packages plus one payload-less package isolated | two_packages_fixture_yields_two_installations_third_package_contributes_nothing (incl raw unsorted order pin) | PASS |
| Existing-but-empty candidate root is Error, not absence | packaged_empty_fixture_yields_one_error_never_a_not_detected_warning | PASS |
| An Absent Slot Is An Explicit Not Detected Signal (CA-11), no Packages dir, no legacy | nothing_yields_zero_installs_three_warnings_zero_errors | PASS |
| Unreadable Packages dir errors but legacy fallback still resolves | packages_unreadable_fixture_errors_but_legacy_still_resolves (Packages simulated as a 0-byte file, portable across OS legs) | PASS |
| Payload-less Claude_ package, no legacy, single not-detected warning | non_claude_packages_fixture_contributes_nothing_and_warns_not_detected | PASS |
| Every Case Traceable To A New, Non-Reused Fixture Tree (CA-17) | fixtures/installations/ fully deleted (all entries D in git status); fixtures/client-installations/ is a new, differently-named, differently-shaped tree | PASS |
| Frontend Reason-String Matching Tracks New Vocabulary (TS) | scanDiagnostics.test.ts, exact-string parameterized cases plus new bundled-slot scenario | PASS |
| RENAMED: Each Desktop Version Directory to Each Bundled-Slot Version Directory | Doc comments and identifiers consistently use bundled throughout installations.rs and client_installations.rs; no desktop-star fixture or slot name remains | PASS |

## Exact-String Coupling Check (Rust vs TypeScript)

Verified byte-for-byte, both directions:

| Rust label (installations.rs) | TS constant (scanDiagnostics.ts) | Match |
|---|---|---|
| Claude Code CLI (npm) -> Claude Code CLI (npm) not detected | Claude Code CLI (npm) not detected | Exact |
| Claude Code (bundled in Claude Desktop) -> Claude Code (bundled in Claude Desktop) not detected | Claude Code (bundled in Claude Desktop) not detected | Exact |
| OpenCode (npm) -> OpenCode (npm) not detected | OpenCode (npm) not detected | Exact |

No stray old string, "Claude Code (desktop) not detected", was found anywhere in the working tree.

## Design Error-Paths Table vs Implementation

Every row of design.md Error Paths table was matched against bundled_candidates and resolve_bundled_slot line-by-line: absent Packages (no event, CA-11 deferred to resolve_slot), unreadable Packages (Error, legacy still evaluated), per-DirEntry error (Error, loop continues), empty-but-existing candidate root (Error), non-UTF-8 version directory name (Error, path None), no candidate at all (exactly one Warning). All six rows implemented exactly as specified; all six independently exercised by a test that currently passes.

## Prior Adversarial Review Follow-Up (Phase 7), Confirmed Landed

- 7.1 (non-UTF-8 unit pin): install_from_version_dir was extracted as a pure helper and is unit-tested directly: install_from_version_dir_rejects_non_utf8_name_as_error_with_no_path (cfg unix) and install_from_version_dir_rejects_unpaired_surrogate_as_error_with_no_path (cfg windows, confirmed executed and passing on this Windows machine). Both assert severity Error and path None.
- 7.2 (raw/unsorted ordering pin): two_packages_fixture_yields_two_installations_third_package_contributes_nothing includes an explicit unsorted-order assertion (raw_versions equals 10.0.0 then 11.0.0) through the real read_dir path, distinct from the filter_and_sort_claude_packages unit test.

Both gaps closed as claimed.

## CA-16 / CA-17 / Core Purity Checks

- CA-16 (read-only): read_only_audit.rs core_source_audit_covers_all_filesystem_mutation_classes passes; installations.rs only calls read_dir, symlink_metadata, read_to_string, no write/create calls. full_scan_leaves_the_fixture_tree_unchanged independently pins byte-identical fixture state before and after scan.
- CA-17: confirmed via git status --porcelain, the entire old fixtures/installations/ tree is staged as deletions, the new fixtures/client-installations/ tree is untracked/new, structurally distinct (different case names, different directory shapes for the packaged-MSIX cases). No test reads APPDATA/USERPROFILE or any real-machine path, every test goes through fixture_home().
- crates/vertice-core imports no tauri or tauri-star crate (only a doc-comment mention of the rule in lib.rs); deny.toml ban is unaffected. model/ was not touched by this change at all, zero-I/O invariant undisturbed, and the empty frontend/src/bindings/ diff independently confirms no model type changed.

## Assertion Quality Audit (Strict TDD)

Reviewed client_installations.rs (new/modified) and installations.rs in-module tests in full. No tautologies, no assertion-free tests, no ghost loops over possibly-empty collections (the one loop, in nothing_yields_zero_installs_three_warnings_zero_errors, is preceded by assert_eq scan.issues.len() equals 3, so it is guaranteed non-empty). No mocks used, real filesystem fixtures throughout, correct layer. Every test calls production code (scan_for) and asserts on real computed values: counts, sorted/unsorted version lists, exact reason strings, path membership.

Assertion quality: All assertions verify real behavior. 0 CRITICAL, 0 WARNING.

## TDD Compliance

No apply-progress artifact exists under this OpenSpec change folder (OpenSpec mode does not mandate one; tasks.md phases 2-3 explicitly encode RED-then-GREEN ordering, and the CA-7 pin test header comment records it was written and failing first). This could not be independently re-verified post-hoc since the code is now GREEN, but the current test suite is comprehensive, all green, and consistent with the documented TDD sequence. Rated as accepted evidence, not directly re-provable, informational, not a gate failure.

## Issues Found

CRITICAL: None.

WARNING: None.

SUGGESTION:
- No apply-progress.md / TDD-evidence artifact was persisted for this change under OpenSpec mode; future changes could optionally persist one for stronger post-hoc TDD auditability, though this is not part of the OpenSpec artifact contract.

## Verdict

PASS

All spec requirements and scenarios are covered by passing tests, all tasks are genuinely complete, the design Error Paths table is implemented exactly as specified, CA-7, CA-11, CA-16, CA-17 are satisfied, core purity and the zero-diff bindings invariant hold, and the frontend/Rust reason-string coupling matches character-for-character. Both previously-flagged adversarial-review coverage gaps (Phase 7) are confirmed closed. Ready for archive.
