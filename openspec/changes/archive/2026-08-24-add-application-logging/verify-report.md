# Verification Report

**Change**: `add-application-logging`
**Version**: N/A (unreleased)
**Mode**: Strict TDD

## Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 49 (46 original + 3 follow-up, see "Follow-Up: Closing WARNING 1" below) |
| Tasks complete | 49 |
| Tasks incomplete | 0 |

Spot-checked ticks against real artifacts (not taken on the apply agent's word): Phase 1 (Slice A)
tests exist and pass; Phase 3/4 (`logging.rs`, `SANCTIONED_WRITERS`) exist and pass; Phase 5
(`log_scan_report_with`/`log_freshness_report_with`) exist and pass; Phase 6 (`log_file_path`,
`appLog.ts`, catalog keys, `ScanPage.svelte` element) exist and pass. All ticks are honest.

## Build and Tests Execution (re-run independently, not trusted from apply-progress.md)

Rust gates (export PATH="$PATH:/c/Users/Raul/.cargo/bin", toolchain 1.97.1 per rust-toolchain.toml):

```
cargo fmt --all --check                                       -> clean, no output
cargo clippy --workspace --all-targets -- -D warnings          -> clean, no warnings/errors
cargo test --workspace --locked                                -> all green (see below)
cargo deny check bans licenses                                 -> bans ok, licenses ok
                                                                    (only pre-existing unused-allowlist
                                                                    warnings, not new)
```

cargo test --workspace --locked summary by binary (all 0 failed):
- vertice_app_lib unittests: 55 passed, 1 ignored (pre-existing network-dependent test),
  including all 8 logging::tests::*, both Slice-A regression tests
  (freshness::cache::tests::save_creates_the_app_data_directory_when_it_does_not_yet_exist,
  commands::tests::writing_settings_survives_a_never_created_app_data_directory), and all three
  Slice-C tests (scan_result_is_byte_identical_whether_or_not_a_working_sink_observed_it,
  scan_report_with_not_found_root_and_not_detected_client_emits_one_warn_line_each,
  freshness_report_with_an_unknown_verdict_emits_a_warn_line_carrying_the_reason_verbatim).
- vertice-app integration tests/read_only_audit.rs: 4 passed (the main audit test plus the
  three new classification unit tests).
- vertice_core unittests: 116 passed. All other vertice-core integration suites (agent_scanner,
  client_installations, codex_agent_scanner, consolidation, freshness_compare, freshness_evaluate,
  frontmatter_reader, jsonc_behavior, model_contract, opencode_agent_scanner, skill_scanner,
  toml/yaml behavior+seam invariants): all passed, unaffected by this change (as expected, no core
  changes).

Frontend gates, run from frontend/ (never a subdirectory):

```
npm run lint    -> clean, 0 problems
npm run check   -> svelte-check: 220 FILES, 0 ERRORS, 0 WARNINGS
npm run test    -> 12 files, 112/112 passed
npm run build   -> succeeds, ~113 KB JS / ~39 KB CSS
```

All gates the orchestrator reported as passing are confirmed independently, including the
npm run check type-check that npm run test alone does not cover.

## Working-tree diff sanity

- crates/vertice-core: zero diff (git status --short crates/vertice-core -- no output). Confirms
  workspace-architecture "vertice-core dependency graph gains no logging crate" and the module
  claim that crates/vertice-core/** is unchanged.
- deny.toml, crates/vertice-app/capabilities/default.json: zero diff.
- crates/vertice-core/Cargo.toml: contains no log/chrono reference (grepped directly).
- frontend/src/bindings/**: git diff --ignore-space-at-eol on all 24 files is zero content
  diff -- the git status --short "modified" markers observed are CRLF-normalization noise from
  the verification agent own cargo test -p vertice-core re-run in this session (Windows checkout,
  LF-committed files), the exact phenomenon apply-progress.md already documented and had restored
  with git checkout --. The verification agent was not able to restore them (the shell classifier
  blocked that specific git checkout call), so the working tree currently shows this cosmetic diff
  again; it is not a content change and does not affect the design section 2 byte-identical claim,
  but the orchestrator should re-run git checkout -- frontend/src/bindings/ before commit to keep
  the diff clean.
- Actual changed-file set matches apply-progress.md table exactly: git diff --stat on the 10
  modified core/app/frontend files (excluding bindings) shows 550 insertions(+), 62 deletions(-),
  matching the reported ~550 line delta on modified files, plus the three new files
  (logging.rs, appLog.ts, appLog.test.ts) -- consistent with the ~933-line total claim.

## Spec Compliance Matrix

### application-logging (new capability)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| Log Sink Location Inside The App Data Directory | Sink creates its own directory on first use | logging::tests::fresh_install_has_exactly_one_log_file (calls FileSink::open against a never-created temp dir) | COMPLIANT |
| (same requirement) | Sink module derives its path exclusively from app_data_dir | read_only_audit.rs::assert_write_path_is_derived_from_app_data_dir run against real logging.rs source in the audit test | COMPLIANT |
| Fixed-Column Plain-Text Line Format | A logged line carries source file, timestamp, offset | logging::tests::format_line_has_the_fixed_column_shape_with_one_trailing_newline, format_line_left_pads_every_level_to_five_characters | COMPLIANT |
| The Four Required Event Classes: startup | Startup is logged once | lib.rs's `.setup` decision logic is now factored into a pure, testable `startup_sequence` function (an injectable-closure seam mirroring `log_scan_report_with`, chosen over a second global logger); `lib.rs::tests::startup_sequence_logs_exactly_one_info_line_on_successful_init` asserts exactly one INFO line, carrying the version, and no stderr output on a successful init | COMPLIANT |
| (same requirement) scan start/end + duration | A scan logs its start, end, and duration | `run_scan` is now backed by a testable `run_scan_with(label, emit)` seam; `commands::tests::run_scan_emits_one_info_start_line_and_one_info_finish_line_carrying_the_duration` asserts exactly two INFO lines -- `"scan started"` and `"scan finished in {duration_ms} ms"` carrying the real measured duration | COMPLIANT |
| (same requirement) missing root / undetected client | Both are logged | commands::tests::scan_report_with_not_found_root_and_not_detected_client_emits_one_warn_line_each -- exact count (2), level (WARN), content (claude-skills, Codex) asserted | COMPLIANT |
| (same requirement) freshness-unknown | Logged with reason verbatim | commands::tests::freshness_report_with_an_unknown_verdict_emits_a_warn_line_carrying_the_reason_verbatim -- reason string asserted verbatim | COMPLIANT |
| Size-Bounded Rotation | Writing at/above threshold triggers rotation | logging::tests::writing_past_max_bytes_rotates_leaving_exactly_two_files_with_no_line_duplicated_or_torn | COMPLIANT |
| (same requirement) | Fresh install has no predecessor | logging::tests::fresh_install_has_exactly_one_log_file | COMPLIANT |
| A Per-Line Write Failure Is Silent And Never Fails A Scan | Failure does not fail/alter scan result | commands::tests::scan_result_is_byte_identical_whether_or_not_a_working_sink_observed_it + logging::tests::write_line_keeps_returning_even_after_the_underlying_file_is_removed | COMPLIANT |
| Sink Initialisation Failure Is Reported Once, On Stderr | Reported exactly once | `logging::tests::init_against_an_uncreatable_directory_returns_err_and_does_not_panic` proves `init` returns `Err` without panicking; `lib.rs::tests::startup_sequence_reports_stderr_once_and_never_logs_on_init_failure` now drives that real, failing `logging::init` call through `startup_sequence` and asserts exactly one stderr line, and zero INFO lines -- a genuine `logging::init` failure, not a stand-in. A third test, `startup_sequence_reports_stderr_once_when_app_data_dir_is_unresolvable`, covers the sibling branch (an unresolvable `app_data_dir`) with the same exactly-once assertion | COMPLIANT |

### desktop-shell (delta)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| Minimal Scan Command Surface (now 6) | log-path command returns path without touching file | read_only_audit.rs::desktop_shell_exposes_only_scan_commands_and_core_default_capability (asserts 6-command list) + commands::log_file_path performs only a path join, no I/O | COMPLIANT |
| Minimal Capability Grant | log-path command adds no new capability | same test asserts permissions == ["core:default"]; git status --short on capabilities/default.json is empty | COMPLIANT |
| The Read-Only Audit Recognizes A Second Write Exception | Audit proves the logging sink exception on its own merits | assert_write_path_is_derived_from_app_data_dir looped over SANCTIONED_WRITERS (2 entries), plus 3 dedicated classification unit tests | COMPLIANT |

### component-freshness (delta)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| App Data Directory Created Before Sanctioned Write | Cache write succeeds when dir does not exist | freshness::cache::tests::save_creates_the_app_data_directory_when_it_does_not_yet_exist | COMPLIANT |
| (same requirement) | Disabling freshness survives restart (regression) | commands::tests::writing_settings_survives_a_never_created_app_data_directory | COMPLIANT |
| Freshness-Unknown Verdicts Are Also Recorded In The Application Log | Verdict appears in both report and log | commands::tests::freshness_report_with_an_unknown_verdict_emits_a_warn_line_carrying_the_reason_verbatim | COMPLIANT |

### scan-orchestration (delta)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| Visible and Isolated Diagnostics | Logging a report does not mutate ScanReport or ScanIssue | commands::tests::scan_result_is_byte_identical_whether_or_not_a_working_sink_observed_it (equality assert before/after log_scan_report_with) | COMPLIANT |

### workspace-architecture (delta)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| The Logging Sink Is A Single-Owner Seam Owned By vertice-app | vertice-core gains no logging crate | git status --short crates/vertice-core empty; crates/vertice-core/Cargo.toml has no log/chrono | COMPLIANT (build/manifest evidence, appropriate for this structural claim) |
| (same requirement) | Exactly one module owns the log file | read_only_audit.rs FORBIDDEN_MUTATION_PATTERNS scan over every .rs file under src/, denying all patterns outside SANCTIONED_WRITERS two entries | COMPLIANT |
| (same requirement) | Dependency gate passes with promoted direct dependency | cargo deny check bans licenses -- re-run independently, bans ok, licenses ok | COMPLIANT |

### frontend-i18n (delta)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| Log-Path Label Is Fully Localized | Spanish catalog stays complete | locale.test.ts asserts scan.logPathLabel/scan.logPathHint non-blank in both en and es | COMPLIANT |
| (same requirement) | Path value itself never translated | App.test.ts log-path test asserts the identical raw path string renders in both en and es mounts | COMPLIANT |

### inventory-ui (delta)

| Requirement | Scenario | Test | Result |
|---|---|---|---|
| The Log File Path Is Displayed As Selectable Text On The Scan Route | Path visible/selectable on scan route, no reveal action | App.test.ts log-path test asserts a code element with data-testid log-path, exact text match, and no button matching a reveal/open/show-in pattern | COMPLIANT |
| (same requirement) | Rendered path matches command return | Same test -- exact string equality against the mocked fetchLogFilePath resolution | COMPLIANT |

Compliance summary (updated in this follow-up pass, see "Follow-Up: Closing WARNING 1" below):
25/25 scenario-level rows COMPLIANT. Zero PARTIAL, zero UNTESTED, zero FAILING.

## Specific Scrutiny Points

1. Requirement coverage -- see matrix above. No requirement is entirely untested; the three
PARTIAL rows are real gaps in test precision (the behavior is implemented and confirmed by direct
source reading, but no test pins the exact log content/count for startup, scan start/end/duration
lines, or the no-repeat property of the stderr message), not gaps in implementation. See WARNING
findings.

2. The two log events not observed in the wild ("search root not found", "AI client not
detected") -- genuinely implemented and genuinely tested, not merely specified. Confirmed by direct
reading of commands.rs::log_scan_report_with (lines ~48-71): it iterates report.roots_scanned
for SearchRootStatus::NotFound and report.client_presence for ClientPresenceStatus::NotDetected
and emits one Level::Warn line each via the injected emit closure -- real production code, not a
stub. The covering test scan_report_with_not_found_root_and_not_detected_client_emits_one_warn_line_each
builds a real ScanReport/ClientPresence fixture with those exact statuses, captures emitted
(Level, String) pairs through the closure, and asserts count == 2, both WARN, and that the messages
contain "claude-skills" and "Codex" respectively -- a real, non-vacuous assertion exercising the
production closure, not a mock of it. Not a concern.

3. CA-16 -- the SANCTIONED_WRITERS rework genuinely tightens the audit rather than weakening
it, confirmed by direct reading of tests/read_only_audit.rs:
- The blanket continue for the cache module exception is gone; every file is now scanned
  unconditionally, and only patterns present in that specific module allowed list are excused
  (is_pattern_permitted).
- remove_file, remove_dir, .set_len(, set_permissions, hard_link, symlink_file, symlink_dir never
  appear in either module allowed list (cache.rs: fs::write plus create_dir; logging.rs:
  OpenOptions, .write(, .write_all(, File::create, create_dir, fs::rename, std::fs::rename), so
  they remain denied everywhere, including inside both sanctioned modules -- confirmed both by
  reading the const table and by two dedicated unit tests that assert this directly against the
  real table.
- assert_write_path_is_derived_from_app_data_dir (contains app_data_dir, no std::env:: or plain
  env::, no literal-path marker) now runs in a loop over both SANCTIONED_WRITERS entries against
  each module real, cfg(test)-stripped source -- not accepted merely by list membership. Confirmed
  logging.rs doc comment and body reference app_data_dir as a parameter, contain no std::env::, and
  contain none of the five literal-path markers.
- assert_eq!(SANCTIONED_WRITERS.len(), 2) pins growth as a reviewed event.
- Capability grant: confirmed core:default only, capabilities/default.json has zero diff from
  pre-change, and deny.toml has zero diff. Both re-confirmed via git status --short, not taken from
  the report.
- No weakening found.

4. Strict TDD honesty -- spot-checked against real test files, all consistent with
apply-progress.md TDD Cycle Evidence table:
- 1.1/1.2 (Slice A): both tests use a helper that deliberately does not pre-create the temp
  directory (temp_app_data_dir_not_created), so before fs::create_dir_all(parent) existed in
  cache.rs::save, these would genuinely fail with a NotFound-class IO error -- the claimed RED is
  plausible, not vacuous.
- 3.1-3.4 (rotation/fresh-install/resilience/init-failure): compile-time RED is legitimate under
  strict TDD when the module/function is new -- confirmed FileSink, format_line, init did not exist
  before this change (new file). Reasonable.
- 4.1 (audit widened): plausible -- the pre-4.1 audit had only one hard-coded exception so it would
  legitimately flag logging.rs writes as forbidden once that file existed.
- 6.4 (six-command assertion): plausible -- an assert_eq! against a literal 6-element vec would
  fail against a 5-command lib.rs before the command was wired.
- No tautologies, no ghost loops, no assertion-without-production-code-call patterns found across
  logging.rs, commands.rs new tests, read_only_audit.rs new tests, appLog.test.ts, or App.test.ts
  new log-path scenario. Assertions consistently check exact values (message content, counts,
  levels, tag names, exact string equality), not just is-defined or is-truthy checks.
- The one genuine gap: no test asserts the exact count of startup/init-failure log or stderr lines
  (see PARTIAL rows above) -- apply-progress.md own TDD Cycle Evidence table does not claim a
  separate RED cycle for that assertion either (folded into 5.1 wire-it-up step with no RED entry),
  which is consistent -- this was never claimed as a separate TDD cycle, so it is not a dishonesty
  finding, just a coverage gap.

5. Slice A independence -- confirmed genuinely independent. freshness/cache.rs save function
create_dir_all(parent) is the only change Slice A makes. logging.rs FileSink::open (Slice B) calls
its own fs::create_dir_all(app_data_dir) -- the two do not share a helper (design section 9
explicitly states this was a deliberate choice to avoid a third audit exception). Reverting Slice A
one-line change in cache.rs would restore the pre-existing "settings write silently reverts on a
machine that never had the app-data dir" bug without touching logging.rs independent directory
creation, and nothing in Slices B-E reads or depends on cache.rs directory-creation behavior.
Confirmed independent.

6. Deviation 3 (MSRV, open) -- it was possible to partially close this further without installing
an MSRV toolchain, using cargo info against the resolved lockfile versions (crates.io-declared
rust-version metadata, not an empirical compile):
- cargo tree -e no-dev -p vertice-app --target x86_64-pc-windows-msvc (re-run independently):
  confirms the Windows-target chrono clock-feature subtree is exactly chrono 0.4.45, num-traits
  0.2.19 (plus build-dep autocfg 1.5.1), windows-link 0.2.1 -- matches apply-progress.md own
  reported output, no iana-time-zone on this target.
- cargo info on each: chrono 0.4.45 reports rust-version 1.62.0; num-traits 0.2.19 reports
  rust-version 1.60; autocfg 1.5.1 reports rust-version 1.0; windows-link 0.2.1 reports
  rust-version 1.71. All four are comfortably under the workspace 1.88 floor (Cargo.toml line 8)
  by their own declared metadata.
- This is secondary evidence, not proof: rust-version is author-declared metadata and can be wrong
  or stale; it is not the same as an empirical cargo +1.88 check on this target. It does
  meaningfully narrow the residual risk beyond unverified -- every crate newly pulled onto the
  direct-dependency Windows build path declares a floor 17-28 minor versions below 1.88 -- but it
  is not authoritative.
- Verdict on this point: the residual risk is now Low, not Unknown, but CI msrv job
  (.github/workflows/ci.yml, installs the pinned 1.88 toolchain and runs cargo check --workspace
  --locked --all-targets under RUSTUP_TOOLCHAIN 1.88) remains the only authoritative check, and it
  has not run in this environment. This is consistent with apply-progress.md own framing of
  Deviation 3 -- this is additional narrowing evidence, not an override.

7. Failure semantics -- both halves confirmed tested and confirmed that a scan cannot fail because
of logging:
- Per-line write failure silent, never fails a scan:
  logging::tests::write_line_keeps_returning_even_after_the_underlying_file_is_removed (no panic,
  no propagated Err from write_line itself -- its signature returns unit) plus
  commands::tests::scan_result_is_byte_identical_whether_or_not_a_working_sink_observed_it (the
  ScanReport returned by run_scan is asserted equal before and after being observed by a no-op
  sink). Source-confirmed: run_scan Result type is produced entirely by
  spawn_blocking(vertice_core::scan::scan) before any logging call runs; log_scan_report(report)
  is called only as a side effect on the already-Ok result and its return value (unit) is
  discarded, so there is no code path by which a log failure can become a scan failure.
- Sink init failure reported once on stderr:
  logging::tests::init_against_an_uncreatable_directory_returns_err_and_does_not_panic proves init
  returns Err without panicking or aborting. The exactly-once and app-continues-and-starts-normally
  properties are enforced by lib.rs structure (a single eprintln! in the Err arm of a match, never
  called again) rather than a dedicated test that captures stderr output -- flagged as a WARNING
  below, not a defect: the code is provably correct by inspection (only one eprintln! call site
  exists in the whole init path, and none exists inside write_line/rotate), but the property is not
  pinned by an automated regression test.

## TDD Compliance

| Check | Result | Details |
|-------|--------|---------|
| TDD Evidence reported | Yes | Found in apply-progress.md TDD Cycle Evidence table, 12 rows |
| All tasks have tests | Yes | 46/46 tasks map to a real test file or an explicit non-test task (spec grep, gate re-run) |
| RED confirmed (tests exist) | Yes | 12/12 listed test files/functions verified to exist and to be plausible RED cycles |
| GREEN confirmed (tests pass) | Yes | 12/12 corresponding tests re-run independently in this session and pass |
| Triangulation adequate | Yes | Event-coverage requirement triangulated by 3 distinct scenarios across 2 test functions with different fixtures |
| Safety Net for modified files | Yes | Full cargo test --workspace --locked re-run confirms no regression in any pre-existing test |

TDD Compliance: 6/6 checks passed

---

### Test Layer Distribution

| Layer | Tests | Files | Tools |
|-------|-------|-------|-------|
| Unit (Rust) | 19 new/changed | 4 | cargo test |
| Unit (frontend) | 1 | 1 (appLog.test.ts) | Vitest |
| Integration (frontend, component + i18n) | 2 new scenarios | 2 (App.test.ts, locale.test.ts) | Vitest + DOM query |
| E2E | 0 | 0 | not installed |
| Total | ~22 new/changed test cases | 7 | |

---

### Assertion Quality

No tautologies, no ghost loops, no assertion-without-production-code-call patterns found across
logging.rs, commands.rs new tests, read_only_audit.rs new tests, appLog.test.ts, or App.test.ts
new log-path scenario. All reviewed assertions check concrete values (exact string equality, exact
counts, exact log levels) rather than type-only or smoke-test-only checks.

Assertion quality: All assertions verify real behavior

---

### Quality Metrics

Linter (ESLint): No errors, re-run independently
Type Checker (svelte-check): No errors, 220 files, re-run independently
Rust (clippy -D warnings): No errors, re-run independently

## Correctness (Static Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| Single-owner seam for the log file | Implemented | logging.rs is the only module naming log/chrono; enforced by the audit |
| No literal path / no env read in logging.rs | Implemented | Confirmed by direct read: parameter-only app_data_dir, no std::env::, no literal path string |
| Rotation never tears/duplicates a line | Implemented | rotate() flushes, drops, renames, reopens under the same lock as write_line single write_all |
| Six-command IPC surface, core:default only | Implemented | lib.rs generate_handler! plus capability file both confirmed |

## Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| chrono 0.4.45 clock-only, default-features=false (design section 5) | Yes | Cargo.toml diff matches exactly |
| One file, not a folder (design section 6) | Yes | logging.rs only |
| Rotate-before-write, evaluated under one lock (design section 7) | Yes | Confirmed in write_line/rotate |
| SANCTIONED_WRITERS two-entry table (design section 10) | Yes | Confirmed in read_only_audit.rs |
| Injectable-closure test seam for C1 (design section 14 C1, one of two named alternatives) | Yes | log_scan_report_with/log_freshness_report_with |
| Slice A independently revertible (design section 9) | Yes | Confirmed no shared helper with logging.rs |
| log needs features std (deviation 1) | Yes, justified | set_boxed_logger is gated behind std; confirmed real, not invented |
| log_file_path positioned after set_freshness_settings (deviation 2) | Yes, justified | The audit exported_tauri_commands matcher is order-sensitive; confirmed by reading both files |
| MSRV closed against 1.97.1, not 1.88 (deviation 3) | Yes, disclosed as open | See scrutiny point 6 -- narrowed further in this report, still not CI-authoritative |
| Injectable-closure shape chosen over a second global logger (deviation 4) | Yes, justified | log set_boxed_logger can only succeed once per process, so a second real logger per test would be flaky under parallel cargo test |

## Follow-Up: Closing WARNING 1

Applied after the initial verify pass, in a dedicated `sdd-apply` follow-up batch. All three
PARTIAL scenarios from the original WARNING 1 are now closed with dedicated, non-vacuous tests,
reusing the design's established injectable-closure test seam (design §14 C1) rather than
installing a second process-global logger (a deliberately rejected alternative, per design
deviation 4 and the earlier "Injectable-closure shape chosen over a second global logger" row):

- `crates/vertice-app/src/lib.rs`: the `.setup` closure's decision logic is factored into a small,
  pure `startup_sequence` function taking the `app_data_dir` result, the `logging::init` call, the
  version string, and two injectable emit closures (`log_info`, `report_stderr`). `.setup` itself
  is unchanged in behavior -- it calls `startup_sequence` with the real `log::info!`/`eprintln!`
  sinks, so the bytes written to the log file and to stderr are identical to before this change;
  only the source line number of the macro call site moved (from inside the closure to `lib.rs`
  top level), which the spec does not pin. Three new tests in `lib.rs::tests`:
  - `startup_sequence_logs_exactly_one_info_line_on_successful_init` -- closes "Startup is logged
    once".
  - `startup_sequence_reports_stderr_once_and_never_logs_on_init_failure` -- drives a real, failing
    `logging::init` call (reusing `logging.rs`'s own "unwritable directory" fixture shape) through
    `startup_sequence` and asserts exactly one stderr line and zero INFO lines. Closes "Sink
    Initialisation Failure Is Reported Once, On Stderr".
  - `startup_sequence_reports_stderr_once_when_app_data_dir_is_unresolvable` -- the sibling branch,
    same exactly-once assertion (not part of the original WARNING, added for parity).
- `crates/vertice-app/src/commands.rs`: `run_scan(label)` is now a thin wrapper over a new
  `run_scan_with(label, emit)`, mirroring `log_scan_report_with`'s existing shape. The start/end
  lines that were previously direct `log::info!` calls are now emitted through the injected
  closure; `run_scan`'s production behavior (what it logs, in what order, on the real global
  logger) is unchanged. The new test
  `commands::tests::run_scan_emits_one_info_start_line_and_one_info_finish_line_carrying_the_duration`
  asserts exactly two INFO lines -- `"scan started"` and `"scan finished in {duration_ms} ms"`
  carrying the real measured duration from a genuine scan. Closes "A scan logs its start, end, and
  duration".

Non-vacuousness was reasoned about, not merely assumed: each new assertion counts emitted
(level, message) pairs and asserts their exact content, so removing the corresponding production
`emit(...)` call would make the assertion fail (an empty or short vector, or a mismatched string),
not pass by construction.

All three scenarios move from PARTIAL to COMPLIANT in the Spec Compliance Matrix above. The full
gate matrix was re-run after this change and all gates pass (see below); `crates/vertice-core`
remains a zero diff and `frontend/src/bindings/**` was regenerated and confirmed byte-identical,
then restored with `git checkout -- frontend/src/bindings/` to clear the cosmetic CRLF-only
`git status` noise (WARNING 2 below), successfully this time.

Re-run gate matrix (`export PATH="$PATH:/c/Users/Raul/.cargo/bin"`):

```
cargo fmt --all --check                                       -> clean, no output
cargo clippy --workspace --all-targets -- -D warnings          -> clean, no warnings/errors
cargo test --workspace --locked                                -> all green
cargo deny check bans licenses                                 -> bans ok, licenses ok
                                                                    (only pre-existing unused-allowlist
                                                                    warnings, not new)
```

`vertice_app_lib` unit tests: 59 passed, 1 ignored (pre-existing network-dependent test) -- includes
all previously-listed tests plus the four new ones above.

Frontend gates, run from `frontend/`:

```
npm run lint    -> clean, 0 problems
npm run check   -> svelte-check: 220 FILES, 0 ERRORS, 0 WARNINGS
npm run test    -> 12 files, 112/112 passed (unchanged -- this follow-up touched no frontend files)
npm run build   -> succeeds
```

## Issues Found

CRITICAL: None.

WARNING:
1. **CLOSED** (see "Follow-Up: Closing WARNING 1" above). All three spec scenarios previously
   marked PARTIAL now have dedicated, non-vacuous automated tests pinning their exact behavior.
2. frontend/src/bindings/** showed a cosmetic CRLF-only git status diff (zero content diff under
   --ignore-space-at-eol) earlier in this project's history, caused by a `cargo test -p
   vertice-core` re-run on a Windows checkout of LF-committed files. Re-confirmed zero content diff
   and restored with `git checkout -- frontend/src/bindings/` in this follow-up session; the
   working tree is currently clean.
3. Deviation 3 (MSRV) remains open pending CI msrv job. The residual risk was narrowed with cargo
   info crates.io-declared rust-version metadata for every crate the chrono clock feature pulls
   onto the Windows build (chrono 1.62.0, num-traits 1.60, autocfg 1.0, windows-link 1.71, all
   comfortably under the 1.88 floor), but this is declared metadata, not an empirical
   cargo +1.88 check. CI remains authoritative and has not run.

SUGGESTION:
1. iana-time-zone (the Linux-only branch of the chrono clock subtree) was not on the Windows tree
   and so was not directly exercised in this session cargo tree output; its declared MSRV (0.1.65
   maps to 1.62.0) was checked and is comfortably under 1.88, but if CI Linux/macOS msrv runners
   ever diverge from the Windows result it is worth a targeted re-check.

## Verdict

PASS WITH WARNINGS

All 49 tasks (46 original + 3 follow-up) are honestly complete, all four Rust gates (cargo fmt,
cargo clippy -D warnings, cargo test --workspace --locked, cargo deny check bans licenses) and all
four frontend gates (lint, check, test, build) pass on the current working tree. CA-16 read-only
audit rework is a genuine tightening, not a weakening, of the invariant. Slice A is genuinely
independently revertible. WARNING 1 (test-coverage precision gaps on the Four Required Event
Classes / Sink Initialisation area) is now closed with three non-vacuous tests reusing the
design's established injectable-closure seam. Only two items remain open, neither new to this
pass: WARNING 3 (MSRV, CI-authoritative, already narrowed) and SUGGESTION 1 (a targeted iana-
time-zone re-check if CI ever diverges by platform). Neither blocks archive.
