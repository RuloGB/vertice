# Apply Progress: Application Logging

**Change**: `add-application-logging`
**Mode**: Strict TDD
**Delivery**: single PR with `size:exception` (resolved 2026-08-24). Slice A (Phase 1) is committed
separately from the rest and remains independently revertible.

## Status

46/46 tasks complete (`tasks.md`). All phases done. Ready for `sdd-verify`.

## Phase 0 — Pre-Flight

- 0.1: Grepped all seven spec deltas for `log_path|log_file_path` — no hardcoded literal command
  name exists anywhere in `specs/`; no drift to correct. Re-ran again before Phase 6, still clean.
- 0.2: Measured the `chrono` `clock`-feature subtree before writing any `chrono` code.
  - `cargo tree -e no-dev -p vertice-app --target x86_64-pc-windows-msvc`: on this target the delta
    is `chrono 0.4.45`, `num-traits 0.2.19` (+`autocfg` build-dep), `windows-link 0.2.1` — no
    `iana-time-zone` on Windows (chrono uses `windows-link` there instead).
  - `cargo deny check bans licenses`: `bans ok, licenses ok`, no new allow-list entry needed.
  - `cargo check --workspace --locked --all-targets`: passes.
  - **U1 residual gap**: the check ran against the pinned toolchain (1.97.1, `rust-toolchain.toml`),
    not literally the 1.88 MSRV floor (`Cargo.toml:8`) — no MSRV-pinned toolchain is installed in
    this environment. The CI `msrv` job is the authoritative check for the exact floor.

## Phase 1 — Slice A (independently revertible)

RED confirmed by running the two new tests before the fix (`Os { code: 3, kind: NotFound }` panics),
then GREEN after adding `fs::create_dir_all(parent)` to `freshness/cache.rs::save`.

- `freshness::cache::tests::save_creates_the_app_data_directory_when_it_does_not_yet_exist`
- `commands::tests::writing_settings_survives_a_never_created_app_data_directory`

`read_only_audit.rs` proof obligations against `cache.rs` pass unmodified (1.4) — this slice touched
zero audit lines. `cargo fmt`, `clippy -D warnings`, `cargo test --workspace --locked` all pass with
only Slice A's changes present (1.5).

**Commit boundary** (not created — git policy forbids committing): `crates/vertice-app/src/freshness/cache.rs` (the `create_dir_all` line + doc) and its two new tests in `cache.rs`/`commands.rs`.

## Phase 2-3 — Slice B (sink module)

Created `crates/vertice-app/src/logging.rs`. `log = "0.4"` needed `features = ["std"]` beyond the
design's snippet — `set_boxed_logger` is gated behind the `std` feature and is not on by default;
this is a deviation from the design's exact `log = "0.4"` line, noted below.

RED evidence:
- `format_line`/rotation/resilience/init-failure tests were authored before the module existed
  (compile-time RED, per design's own framing).
- `current_timestamp`'s RFC 3339 test was independently re-verified RED by temporarily stubbing
  `current_timestamp` to `unimplemented!()` and observing the test panic, then restoring the real
  `chrono::Local::now().to_rfc3339_opts(...)` implementation (GREEN).

All 8 `logging::tests::*` pass: format shape, level padding, embedded-newline flattening, RFC 3339
timestamp, fresh-install single-file, N-lines-past-`MAX_BYTES` rotation (two files, no line
duplicated/torn), write-after-file-removed resilience, and init-against-uncreatable-directory.

## Phase 4 — Slice E first pass (audit widened to two exceptions)

RED confirmed by running the *unmodified* audit against the newly-created `logging.rs`: it failed
listing `logging.rs`'s writes as forbidden (`OpenOptions`, `.write(`, `.write_all(`, `fs::rename`,
`create_dir`) because the blanket exception only recognized `cache.rs`.

GREEN: replaced the blanket `continue` with `SANCTIONED_WRITERS: [SanctionedWriter; 2]`
(`freshness/cache.rs` + `logging.rs`, each with its own `allowed` pattern list), a per-module
`is_pattern_permitted` classification, and a loop calling
`assert_write_path_is_derived_from_app_data_dir` over both entries. `assert_eq!(SANCTIONED_WRITERS.len(), 2)` pins list growth as a reviewed event.

4.2/4.3 (audit fails on `remove_file`/literal-path inside `logging.rs`, and on an out-of-allow-list
pattern inside `cache.rs`) are proved directly against the real `SANCTIONED_WRITERS` table via
`is_pattern_permitted`, per design's own suggestion ("reasoning over the module's actual content,
not by injecting a synthetic violation into production code") — three new unit tests in
`read_only_audit.rs` cover this without touching production sources.

## Phase 5 — Slice C (event coverage)

`lib.rs`: added `mod logging;`, a `.setup(app)` hook resolving `app_data_dir()`, calling
`logging::init`, `eprintln!`-ing once on either failure path, and `log::info!("vertice {version}
starting")` on success.

`run_scan` gained a `&'static str` label (`"scan"`/`"rescan"`), emits INFO start/end (with
`duration_ms`) lines, and calls `log_scan_report`. `log_scan_report`/`log_freshness_report` are thin
wrappers around `log_scan_report_with`/`log_freshness_report_with`, which take an injectable
`emit: impl FnMut(log::Level, &str)` closure — this is the design's own explicitly-permitted
alternative to touching the global logger from tests (§14 C1: "taking a `&dyn Fn(&str)`-shaped
sink"). Production wires the closure to `log::log!`; tests capture emitted `(Level, String)` pairs
directly.

New tests: `scan_result_is_byte_identical_whether_or_not_a_working_sink_observed_it` (C2),
`scan_report_with_not_found_root_and_not_detected_client_emits_one_warn_line_each` and
`freshness_report_with_an_unknown_verdict_emits_a_warn_line_carrying_the_reason_verbatim` (C1). The
two discarded `let _ = ...::save(...)` results (`commands.rs`, `freshness/mod.rs`) became
`if let Err(err) = ... { log::warn!(...) }` — the returned result is unaffected in both call sites.

## Phase 6 — Slice D (sixth IPC command, frontend, i18n)

RED confirmed for the audit's six-command assertion by editing the `assert_eq!`/matcher/handler
checks in `read_only_audit.rs` to expect `log_file_path` *before* implementing the command —
observed failure `left: [...5 commands...] right: [...6 commands...]`. GREEN after adding
`commands::log_file_path` (async, no `spawn_blocking`, reuses `resolve_app_data_dir` +
`crate::logging::log_path`) and registering it in `lib.rs`'s `generate_handler!`.
`capabilities/default.json` confirmed byte-identical (`git status --short` shows no diff).

Frontend: RED confirmed for `appLog.test.ts` (module-not-found) before creating `appLog.ts`
(`invoke<string>("log_file_path")`, mirrors `scan.ts`). RED confirmed for
`locale.test.ts`'s new `scan.logPathLabel`/`scan.logPathHint` non-blank assertion before adding
the keys to `catalogs.ts` (interface + `en` + `es`). RED confirmed for a new `App.test.ts` scenario
(`[data-testid="log-path"]` missing) before modifying `ScanPage.svelte` to fetch
(`fetchLogFilePath()`, IIFE pattern mirroring `ClientsPage.svelte`'s freshness fetch) and render a
`<code data-testid="log-path">` element with no reveal-in-file-manager control, verified in both
`en` and `es` via the language selector (the existing `App.test.ts` pattern for locale switching —
not by remounting with a different `navigator.languages`, which does not affect the already-created
`i18n` context).

## Phase 7 — Slice E remainder

`cargo deny check bans licenses` re-run with `log`/`chrono` as actual direct dependencies:
`bans ok, licenses ok`, no new allow-list entry — confirms design §5's premise.

## Phase 8 — Cross-slice gates

- Bindings: `cargo test -p vertice-core` regenerates `frontend/src/bindings/*.ts`. `git status
  --short` initially showed 7 files as modified; `git diff` on each showed **zero content diff** —
  only an LF/CRLF normalization touch from the regenerator rewriting files whose content is
  byte-identical. Restored with `git checkout -- frontend/src/bindings/` after confirming no content
  changed. No new `ts_rs` type introduced, matching design §2.
- `crates/vertice-core`: `git status --short crates/vertice-core` — no output, zero diff.
- `deny.toml` and `capabilities/default.json`: no diff, confirmed via `git status --short`.
- Full Rust gate: `cargo fmt --all --check` (clean), `cargo clippy --workspace --all-targets -- -D
  warnings` (clean), `cargo test --workspace --locked` (all 22 test binaries pass, 0 failures, 1
  pre-existing `#[ignore]`d network test).
- Full frontend gate (run from `frontend/`): `npm run lint` (clean), `npm run check` (0 errors, 0
  warnings across 220 files), `npm run test` (112/112 passed across 12 files), `npm run build`
  (succeeds, ~113 KB JS / ~39 KB CSS bundle).
- Freshness-cache restart-survival regression (proposal §5, design §15): directly covered by
  `save_creates_the_app_data_directory_when_it_does_not_yet_exist` and
  `writing_settings_survives_a_never_created_app_data_directory` (Phase 1) — both pass, confirming
  the cache now persists across a restart on a machine where the app data directory never existed,
  as expected behaviour rather than a regression.

## TDD Cycle Evidence

| Task | RED | GREEN | REFACTOR |
|---|---|---|---|
| 1.1/1.2 | Confirmed: `Os { code: 3, NotFound }` panic before fix | `create_dir_all(parent)` added; both pass | n/a |
| 2.2 | Compile-time RED (module/function did not exist before authoring) | 4 format tests pass | n/a |
| 2.3 | Explicitly re-verified via `unimplemented!()` stub + restore | passes | n/a |
| 3.1-3.4 | Compile-time RED (module did not exist) | all 4 pass (rotation, fresh-install, resilience, init-failure) | n/a |
| 4.1 | Confirmed: unmodified audit failed against `logging.rs`'s writes | `SANCTIONED_WRITERS` loop added; passes | n/a |
| 4.2/4.3 | Direct unit tests against `is_pattern_permitted` (no synthetic prod violation) | pass | n/a |
| 5.2 | Compile-time RED (function did not exist) | `scan_result_is_byte_identical...` passes | n/a |
| 5.4 | Compile-time RED (functions did not exist) | both C1 tests pass | n/a |
| 6.4 | Confirmed: `assert_eq!` mismatch (5 vs 6 commands) before 6.2/6.3 | passes after `log_file_path` wired | n/a |
| 6.6 | Confirmed: `Cannot find module './appLog'` before file created | `appLog.test.ts` passes | n/a |
| 6.8 | Confirmed: `Cannot read properties of undefined` before keys added | `locale.test.ts` passes | n/a |
| 6.10 | Confirmed: `[data-testid="log-path"]` not rendered before `ScanPage.svelte` change | passes in both `en`/`es` | n/a |

No REFACTOR step was needed beyond incidental cleanup (moving `log_file_path` after
`set_freshness_settings` to match the audit's expected command order, and rewording two doc
comments in `logging.rs` that accidentally tripped the audit's `std::env::` literal-substring check
from within comment text, not code).

## Deviations from Design

1. **`log` needs `features = ["std"]`.** The design's Cargo.toml snippet (`log = "0.4"`) does not
   compile: `log::set_boxed_logger` is gated behind the `std` feature, which is not enabled by
   default. Added `features = ["std"]`. This does not change the dependency count or licence
   posture (`log` is already a non-optional direct dependency of `tauri`/`tauri-utils`, V1).
2. **`log_file_path`'s position in `commands.rs`** is after `set_freshness_settings`, not
   immediately "after `resolve_app_data_dir`" as design §8's snippet shows in isolation — the audit's
   `exported_tauri_commands` scanner is order-sensitive and expects the six-command sequence to match
   `lib.rs`'s registration order. Functionally identical; only the file position differs.
3. **U1 (MSRV) is closed against the pinned 1.97.1 toolchain, not the literal 1.88 floor** — no
   MSRV-pinned toolchain is installed in this apply environment. This is the same gap the design's own
   §0 table already flagged as only "half closed" going into apply; the CI `msrv` job remains the
   authoritative confirmation.
4. **`log_scan_report`/`log_freshness_report` use an injectable-closure shape**
   (`log_scan_report_with(report, emit: impl FnMut(log::Level, &str))`), one of the two alternatives
   design §14 C1 explicitly names ("taking a `&dyn Fn(&str)`-shaped sink or asserted through a
   temp-dir `FileSink`"). Chosen over installing a second global logger per test, which would be
   flaky under `cargo test`'s parallel test execution within one process (only one `log` logger can
   ever be installed process-wide).

No other deviations. Line format, rotation mechanics, `SANCTIONED_WRITERS` shape, IPC contract,
i18n key names, and UI element (`<code data-testid="log-path">`, no reveal-in-file-manager control)
all match the design as written.

## Review Workload / PR Boundary

- Mode: single PR, `size:exception` (resolved 2026-08-24).
- Actual diff: 10 modified files (+550/-62) plus 3 new files (`logging.rs` 349 lines, `appLog.ts` 11
  lines, `appLog.test.ts` 23 lines) ≈ **~933 changed lines**, exceeding the forecast's ~550-750
  estimate and its own Moderate-to-high budget risk — consistent with the user's exception decision.
- Slice A (Phase 1) remains its own logically separable, independently revertible unit within this
  PR: only `freshness/cache.rs`'s `create_dir_all` line plus its two regression tests. Nothing in
  Phases 2-8 depends on Slice A being reverted staying broken (Slice A's fix is a pure superset add).

## Issues Found

None beyond the three deviations above, all resolved during apply.

## Files Changed

| File | Action | What Was Done |
|---|---|---|
| `crates/vertice-app/Cargo.toml` | Modified | `log = { version = "0.4", features = ["std"] }`, `chrono = { version = "0.4", default-features = false, features = ["clock"] }` |
| `crates/vertice-app/src/logging.rs` | Created | Sink: `log_path`, `init`, `FileSink`, `format_line`, rotation, `impl log::Log` |
| `crates/vertice-app/src/lib.rs` | Modified | `mod logging;`, `.setup()` init + startup log line, `commands::log_file_path` registered |
| `crates/vertice-app/src/commands.rs` | Modified | `run_scan(label)`, `log_scan_report`/`log_freshness_report` (+ `_with` testable cores), `log_file_path` command, two `let _ =` sites converted to logged warnings |
| `crates/vertice-app/src/freshness/cache.rs` | Modified | `create_dir_all(parent)` in `save()` + regression test |
| `crates/vertice-app/src/freshness/mod.rs` | Modified | `let _ =` → logged warning at the cache-save call site |
| `crates/vertice-app/tests/read_only_audit.rs` | Modified | `SANCTIONED_WRITERS` table, per-module `allowed` lists, looped path-derivation proof, six-command assertion, three new classification unit tests |
| `frontend/src/lib/appLog.ts` | Created | `fetchLogFilePath()` → `invoke<string>("log_file_path")` |
| `frontend/src/lib/appLog.test.ts` | Created | Mocked-`invoke` unit test |
| `frontend/src/lib/pages/ScanPage.svelte` | Modified | Fetches and renders `<code data-testid="log-path">` with localized label/hint |
| `frontend/src/lib/i18n/catalogs.ts` | Modified | `scan.logPathLabel`/`scan.logPathHint` (interface, `en`, `es`) |
| `frontend/src/lib/i18n/locale.test.ts` | Modified | Explicit non-blank assertion for the two new keys |
| `frontend/src/App.test.ts` | Modified | New scenario: log-path element rendered, no reveal action, `en`+`es` |
| `Cargo.lock` | Modified | `log`/`chrono` promoted to direct deps (already-resolved versions, per V1/V2) |

## Not Committed

Per instruction, no `git commit`/`push`/branch was created. All changes are in the working tree,
ready for the orchestrator to review and commit (Slice A first, as its own commit, per proposal §8).
