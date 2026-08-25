# Tasks: Application Logging

> Trace: new capability `application-logging`; deltas `desktop-shell`, `component-freshness`,
> `workspace-architecture`, `inventory-ui`, `frontend-i18n`; cross-reference only `scan-orchestration`.
> Bounded by CA-16.
> Authority for every decision is `design.md`; do not reopen its §0-§16. Slice order follows
> `proposal.md` §10: A (directory creation) → B (sink) → C (events) → D (IPC + UI + i18n) →
> E (audit + spec maintenance, interleaved with B and D because RED must precede the module it
> sanctions).

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | ~550-750 (one new Rust module + rotation/format logic + tests, one audit rewrite, three call-site edits, two Cargo.toml lines, one new frontend module + test + i18n keys + one Svelte edit) |
| 400-line budget risk | Moderate-to-high — a single PR is plausible but close to budget; the audit rewrite (Slice B/E) and the sink module (Slice B) are individually large |
| Chained PRs recommended | Not required, but a two-PR split (Slice A alone, then B+C+D+E together) is the cleanest boundary if the budget is exceeded — Slice A is explicitly required to be independently revertible (proposal §8) regardless of PR shape |
| Delivery strategy | ask-on-risk (as instructed) |
| Chain strategy | **not chosen here** — if the actual diff exceeds 400 lines, report the forecast and let the orchestrator ask the user before `sdd-apply` starts, per the delivery strategy |
| Decision needed before apply | **RESOLVED (user, 2026-08-24): single PR with `size:exception`.** No chain strategy is needed and none is chosen. Slice A still lands as its own commit inside that PR and must remain independently revertible (proposal §8) — the size exception changes the PR shape, not the commit boundary |

### Suggested Work Units

| Unit | Goal | Phases | Notes |
|---|---|---|---|
| 1 | Slice A: directory-creation fix, independently revertible | 1 | Zero dependency on logging; its own commit regardless of final PR shape (proposal §8) |
| 2 | Slice B+E(partial): sink module, dependency gate, first audit rewrite pass | 2-4 | Carries the MSRV/dependency risk (U1) and the audit rework; largest single unit |
| 3 | Slice C: event coverage at the four observation points | 5 | Depends on Unit 2's sink existing |
| 4 | Slice D+E(remainder): sixth IPC command, frontend surface, i18n, final audit six-command assertion | 6-7 | Depends on Units 2 and 3 |
| 5 | Cross-slice gates, bindings check, spec-text correction | 0, 8 | Spec-text correction (Phase 0) has no code dependency and can run first |

## Phase 0: Pre-Flight — Spec/Design Naming Agreement and MSRV Gate (blocks all of Slice B)

- [x] 0.1 Verify the sixth command's name across all seven spec deltas under
      `openspec/changes/add-application-logging/specs/`. **Finding as of this writing:** the delta
      spec text (`desktop-shell/spec.md`, `inventory-ui/spec.md`, `frontend-i18n/spec.md`) already
      refers to it generically as "the log-path command" and never hardcodes a literal identifier —
      so there is no live textual conflict with `design.md`'s `log_file_path` today. Re-run this grep
      immediately before implementing Phase 6 (`rg -n "log_path|log_file_path" openspec/changes/add-application-logging/specs`)
      and, if any spec file has since been edited to hardcode `log_path` as the command name, correct
      it to `log_file_path` in that same commit — the design is authoritative (design.md §8). Do not
      defer this discovery into `sdd-apply`. [Req: application-logging / desktop-shell IPC naming][Seq]
- [x] 0.2 Run the MSRV/dependency measurement commands prescribed by design §5, narrowed to the
      `chrono` `clock`-feature subtree, **before writing any code that names `chrono`**:
      ```
      cargo tree -e no-dev -p vertice-app --target x86_64-pc-windows-msvc
      cargo deny check bans licenses
      cargo check --workspace --locked --all-targets
      ```
      Record the actual output. If any command fails, follow the fallback ladder in design §5
      (`jiff` next, then UTC-only via `time` + `formatting`) before proceeding to Phase 3.
      [Req: workspace-architecture "dependency gate passes with the promoted direct dependency"][Seq]

## Phase 1: Slice A — Application Data Directory Creation (independently revertible, `component-freshness` delta)

- [x] 1.1 (RED) Add a `freshness/cache.rs` unit test asserting `save()` against a path whose parent
      does not exist returns `Ok` and the written file is readable back (design §14 A1) — build the
      temp path but deliberately do **not** create it, unlike the existing `temp_app_data_dir` helper.
      [Req: component-freshness "The Application Data Directory Is Created Before The Sanctioned Path
      Writes To It" — first scenario][Seq]
- [x] 1.2 (RED) Add a `commands.rs` unit test on `write_/read_freshness_settings` (or the equivalent
      settings round-trip) that never pre-creates the app-data directory, asserting the written
      settings values read back unchanged — the "toggle survives restart" regression (design §14 A2).
      [Req: component-freshness "The Application Data Directory Is Created Before The Sanctioned Path
      Writes To It" — restart scenario][Seq]
- [x] 1.3 (GREEN) Implement `fs::create_dir_all(parent)` in `crates/vertice-app/src/freshness/cache.rs`
      `save()`, guarded by `path.parent()`, exactly as design §9's snippet — no other file changes.
      Confirm 1.1-1.2 now pass. [Req: component-freshness "The Application Data Directory Is Created
      Before The Sanctioned Path Writes To It"][Seq]
- [x] 1.4 Confirm `read_only_audit.rs`'s existing `CACHE_MODULE_EXCEPTION` proofs (`app_data_dir`
      reference, no `std::env::`, no literal-path marker) still pass unmodified against the changed
      `cache.rs` — design §9 states this slice needs **zero** audit change. Do not touch
      `read_only_audit.rs` in this phase. [Req: desktop-shell "Minimal Capability Grant" (unchanged
      surface)][Seq]
- [x] 1.5 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace --locked` — end of Slice A. This is its own commit, independently
      revertible from everything below (proposal §8, "Slice A must not be reverted with the logging
      work"). [Seq]

## Phase 2: Slice B — Sink Module Skeleton and Format (depends on Phase 0.2's MSRV gate passing)

- [x] 2.1 Add `log = "0.4"` and
      `chrono = { version = "0.4", default-features = false, features = ["clock"] }` to
      `crates/vertice-app/Cargo.toml`, with the same comment discipline already used for the
      `reqwest`/TLS entry (design §13). [Req: workspace-architecture "The Logging Sink Is A
      Single-Owner Seam Owned By vertice-app"][Seq, after 0.2]
- [x] 2.2 (RED) Write `format_line`'s test in a new `crates/vertice-app/src/logging.rs`: exact column
      shape `ts␣␣LEVEL␣␣file:line␣␣msg\n`, `LEVEL` left-padded to 5, exactly one trailing `\n`, no
      interior newline even when the message itself contains one (design §14 B1) — fails to compile
      until `format_line` exists. [Req: application-logging "Fixed-Column Plain-Text Line Format"][Seq]
- [x] 2.3 (RED) Write the timestamp test: the token from `chrono::Local::now().to_rfc3339_opts(...)`
      parses as RFC 3339 with a non-empty offset and contains no space — asserted with a parser, not a
      golden string (design §14 B2). [Req: application-logging "Fixed-Column Plain-Text Line Format" —
      "A logged line carries source file, timestamp, and offset"][Seq]
- [x] 2.4 (GREEN) Implement `format_line(timestamp: &str, level: log::Level, file: &str, line: u32,
      message: &str) -> String` — pure, no I/O, no clock (design §6) — and the `Local::now()` call site
      producing the timestamp token. Confirm 2.2-2.3 pass. [Req: application-logging "Fixed-Column
      Plain-Text Line Format"][Seq]

## Phase 3: Slice B — FileSink, Rotation, Init (depends on Phase 2)

- [x] 3.1 (RED) Write `FileSink` rotation tests in `logging.rs`: writing N lines past `MAX_BYTES`
      leaves exactly two files; the predecessor holds the earlier lines whole; the current file holds
      the newest line whole; no line appears twice or truncated (design §14 B3, using a test-visible
      smaller size limit or synthetic long lines — design's open question in §16 permits a
      `pub(crate)` constructor taking the limit if that proves cleaner). [Req: application-logging
      "Size-Bounded Rotation With One Retained Predecessor"][Seq]
- [x] 3.2 (RED) Write the fresh-install test: no rotation has occurred, exactly one log file exists.
      [Req: application-logging "Size-Bounded Rotation With One Retained Predecessor" — "A fresh
      install has no predecessor file"][Seq]
- [x] 3.3 (RED) Write the resilience test: a sink whose underlying file was removed mid-test keeps
      returning from `write_line` — no panic, no `Err` propagated (design §14 B4, D5 class 1).
      [Req: application-logging "A Per-Line Write Failure Is Silent And Never Fails A Scan"][Seq]
- [x] 3.4 (RED) Write the init-failure test: `logging::init` against a path that cannot be created
      returns `Err` and the calling test process does not panic or abort (design §14 B5, D5 class 2).
      [Req: application-logging "Sink Initialisation Failure Is Reported Once, On Stderr"][Seq]
- [x] 3.5 (GREEN) Implement `FileSink { path, rotated, state: Mutex<LogFile> }`, `LogFile { file,
      written }`, `FileSink::open`, `FileSink::write_line`, the rotate-before-write sequence from
      design §7 (compute line + length outside the lock; acquire lock; rotate if
      `written > 0 && written + n > MAX_BYTES` via flush-drop-rename-recreate; `write_all`; advance
      `written` only on success), poison absorption via
      `state.lock().unwrap_or_else(|e| e.into_inner())`, and `impl log::Log for FileSink`. Confirm
      3.1-3.4 pass. [Req: application-logging "Size-Bounded Rotation With One Retained Predecessor",
      "A Per-Line Write Failure Is Silent And Never Fails A Scan"][Seq]
- [x] 3.6 (GREEN) Implement `log_path(app_data_dir: &Path) -> PathBuf` and
      `init(app_data_dir: &Path) -> Result<(), InitError>` (creates the directory via
      `create_dir_all`, opens/creates the file, calls `set_boxed_logger` + `LevelFilter::Info`, fixed).
      Confirm 3.4 fully covers the `Err` path once directory creation is wired. [Req:
      application-logging "Log Sink Location Inside The App Data Directory"][Seq]
- [x] 3.7 Add the module doc comment naming `logging.rs` as the sole owner of `log`/`chrono` and the
      second CA-16 write exception (mirrors design §6's opening comment). [Req: workspace-architecture
      "Exactly one module owns the log file"][Seq]

## Phase 4: Slice E (first pass) — Widen the Read-Only Audit to Two Exceptions (RED before Slice B lands in `lib.rs`)

- [x] 4.1 (RED) Extend `crates/vertice-app/tests/read_only_audit.rs`: replace the blanket
      `if relative_path == CACHE_MODULE_EXCEPTION { continue; }` (currently line 109) with a
      `SANCTIONED_WRITERS: [SanctionedWriter; 2]` table (design §10) naming `freshness/cache.rs` with
      `allowed: &["fs::write", "create_dir"]` and `logging.rs` with
      `allowed: &["OpenOptions", ".write(", ".write_all(", "File::create", "create_dir", "fs::rename",
      "std::fs::rename"]`. Assert the audit fails while `logging.rs` does not yet exist (design §14
      E2, first half). [Req: desktop-shell "The Read-Only Audit Recognizes A Second Write
      Exception"][Seq, after 3.7 exists on disk]
- [x] 4.2 (RED) Assert the audit fails if `logging.rs` (once it exists from Phase 3) contained
      `remove_file` or a literal `C:\` marker — confirm by temporarily reasoning over the module's
      actual content, not by injecting a synthetic violation into production code (design §14 E2,
      second half). [Req: desktop-shell "The Read-Only Audit Recognizes A Second Write Exception"][Seq]
- [x] 4.3 (RED) Assert the audit fails if `cache.rs` gained a pattern outside its own two-entry
      allow-list (design §14 E3) — the per-module `allowed` list, not the blanket continue, is what
      must now fail. [Req: desktop-shell "The Read-Only Audit Recognizes A Second Write Exception"][Seq]
- [x] 4.4 (GREEN) Implement the loop over `SANCTIONED_WRITERS`: no `continue`, per-module `allowed`
      lookup, `remove_file`/`remove_dir`/`.set_len(`/`set_permissions`/`hard_link`/`symlink_file`/
      `symlink_dir` still denied unconditionally in every module including both exceptions (design
      §10 table). Re-run the three path-derivation proofs
      (`assert_write_path_is_derived_from_app_data_dir`) in a loop over the table instead of once
      against `cache.rs` alone. Add `assert_eq!(SANCTIONED_WRITERS.len(), 2)`. Confirm 4.1-4.3 pass.
      [Req: desktop-shell "The Read-Only Audit Recognizes A Second Write Exception"][Seq]

## Phase 5: Slice C — Event Coverage at the Four Observation Points (depends on Phase 3)

- [x] 5.1 `lib.rs`: add `mod logging;`, call `logging::init(&app_data_dir)` inside `.setup(app)` (Err
      → `eprintln!` once, app continues per D5 class 2 — design §12 degradation table), and
      `log::info!("vertice {version} starting")` using `env!("CARGO_PKG_VERSION")` after successful
      init. [Req: application-logging "Startup is logged once", "Sink Initialisation Failure Is
      Reported Once, On Stderr"][Seq, after 3.6]
- [x] 5.2 (RED) Add a `commands.rs` test asserting `run_scan()`'s returned `ScanReport` is
      byte-identical with and without a working sink (design §14 C2, D5 class 1). [Req:
      scan-orchestration "Logging a report does not mutate ScanReport or ScanIssue"][Seq]
- [x] 5.3 Give `run_scan` a `&'static str` label parameter (`"scan"` / `"rescan"`); update its three
      existing test-module callers in the same commit (design §11). [Req: application-logging "A scan
      logs its start, end, and duration"][Seq]
- [x] 5.4 (RED) Add `commands.rs` tests factoring observation into `log_scan_report(&ScanReport)` /
      `log_freshness_report(&FreshnessReport)`: a `ScanReport` carrying a `SearchRootStatus::NotFound`
      root, a `ClientPresenceStatus::NotDetected` client, and a `FreshnessReport` carrying
      `Freshness::Unknown { reason }` each produce one WARN line carrying the value/reason verbatim
      (design §14 C1). [Req: application-logging "A missing root and an undetected client are both
      logged", "A freshness-unknown verdict is logged with its reason"; component-freshness
      "Freshness-Unknown Verdicts Are Also Recorded In The Application Log"][Seq]
- [x] 5.5 (GREEN) Wire `run_scan(label)` to emit an INFO start line, an INFO end line carrying
      `ScanReport.duration_ms`, and one WARN line per `NotFound`/`NotDetected` entry in the returned
      report; wire the freshness command to emit one WARN line per `Freshness::Unknown` check via
      `log_freshness_report`. Confirm 5.2 and 5.4 pass. [Req: application-logging "The Four Required
      Event Classes Are Recorded" (all four scenarios)][Seq]
- [x] 5.6 Convert the two discarded freshness-store write results to logged warnings:
      `commands.rs:145` and `freshness/mod.rs:195`, each becoming
      `if let Err(err) = … { log::warn!("could not persist freshness store: {err}") }` — the returned
      result is unaffected; only the silence becomes evidence (design §9 table, "Does `let _ =` stay
      at the call sites?"). [Req: application-logging "A Per-Line Write Failure Is Silent And Never
      Fails A Scan"][Seq]

## Phase 6: Slice D — Sixth IPC Command, Frontend Surface, i18n (depends on Phases 3-5; naming per Phase 0.1)

- [x] 6.1 Re-run the Phase 0.1 grep immediately before this phase (`log_path|log_file_path` across
      `specs/`); correct any drift in the same commit. [Req: desktop-shell IPC naming
      agreement][Seq]
- [x] 6.2 Add `crates/vertice-app/src/commands.rs::log_file_path(app: tauri::AppHandle) ->
      Result<String, ScanError>` exactly as design §8's snippet: `resolve_app_data_dir(&app)?` then
      `crate::logging::log_path(&app_data_dir).to_string_lossy().into_owned()`, `async` for the
      audit's matcher, no I/O beyond the path join. [Req: desktop-shell "Minimal Scan Command Surface"
      — "The log-path command returns the path without touching the file"][Seq]
- [x] 6.3 `lib.rs`: add `commands::log_file_path` to `generate_handler!`. [Req: desktop-shell "Minimal
      Scan Command Surface"][Seq]
- [x] 6.4 (RED) `read_only_audit.rs`: update the `assert_eq!` command list (currently five, at line
      ~22-31) to the six names including `log_file_path`; extend `exported_tauri_commands`'s matcher
      (currently five `starts_with` checks at line ~289-300) with a `pub async fn log_file_path("`
      branch; extend the `lib_source.contains` checks and the handler description string (line
      ~89-99) to require `commands::log_file_path`. Assert this fails against pre-6.2/6.3 source
      (design §14 E1). [Req: desktop-shell "The Read-Only Audit Recognizes A Second Write Exception"
      cross-cutting with "Minimal Scan Command Surface"][Seq]
- [x] 6.5 (GREEN) Confirm 6.4 passes once 6.2-6.3 land; confirm `capabilities/default.json` stays
      byte-identical (`["core:default"]`, no `"scope"` block). [Req: desktop-shell "Minimal Capability
      Grant" — "The log-path command adds no new capability grant"][Seq]
- [x] 6.6 (RED) Add `frontend/src/lib/appLog.test.ts`: `fetchLogFilePath()` invokes `"log_file_path"`
      via a mocked `@tauri-apps/api/core`, and returns the string unmodified — same shape as
      `scan.test.ts` (design §14 F1). [Req: desktop-shell IPC contract (frontend consumer)][Par with
      6.7, after 6.5]
- [x] 6.7 (GREEN) Create `frontend/src/lib/appLog.ts`: `invoke<string>("log_file_path")`, mirroring
      `frontend/src/lib/scan.ts`. Confirm 6.6 passes. [Req: desktop-shell IPC contract (frontend
      consumer)][Seq, after 6.6]
- [x] 6.8 (RED) Extend `frontend/src/lib/i18n/locale.test.ts` (or equivalent catalog-completeness
      test): assert `scan.logPathLabel` and `scan.logPathHint` exist and are non-empty in both `en`
      and `es`. [Req: frontend-i18n "Log-Path Label Is Fully Localized" — Spanish-catalog-stays-complete
      scenario][Seq]
- [x] 6.9 (GREEN) Add `scan.logPathLabel` / `scan.logPathHint` to `frontend/src/lib/i18n/catalogs.ts`
      — interface (~line 92), `en` (~line 267), `es` (~line 439), per design §8's table. Confirm 6.8
      passes. [Req: frontend-i18n "Log-Path Label Is Fully Localized"][Seq]
- [x] 6.10 (RED) Extend `frontend/src/App.test.ts` (or a `ScanPage`-specific test): the scan route
      renders `[data-testid="log-path"]` with the value returned by the mocked `log_file_path`
      invocation, in both `en` and `es`; assert no reveal-in-file-manager or file-opening control is
      present (design §14 F2). [Req: inventory-ui "The Log File Path Is Displayed As Selectable Text
      On The Scan Route"][Seq]
- [x] 6.11 (GREEN) Modify `frontend/src/lib/pages/ScanPage.svelte`: below `ScanIssueList`, a labelled
      `<code data-testid="log-path">` rendering `appLog.ts`'s fetched value verbatim, with the
      localized label from 6.9. Confirm 6.10 passes. [Req: inventory-ui "The Log File Path Is
      Displayed As Selectable Text On The Scan Route"][Seq]

## Phase 7: Slice E (remainder) — `cargo deny` Re-Verification

- [x] 7.1 Re-run `cargo deny check bans licenses` now that `log` and `chrono` are actual direct
      dependencies of `vertice-app` (Phase 2.1) — the orchestrator's earlier run validated only the
      pre-change graph. Record the actual `bans`/`licenses` verdict; a new allow-list entry would
      contradict design §5's premise and must be treated as a stop, not a routine addition. [Req:
      workspace-architecture "The dependency gate passes with the promoted direct dependency"][Seq]

## Phase 8: Cross-Slice Gates and Verification

- [x] 8.1 Regenerate `frontend/src/bindings/` via `cargo test -p vertice-core` and confirm a
      byte-identical result (`git status --short frontend/src/bindings/` shows no diff) — design §2
      states no new `ts_rs` type is introduced, so any diff here is a design violation to stop on, not
      a file to commit. [Req: workspace-architecture / core data model "none" (design §2)][Seq]
- [x] 8.2 Confirm `crates/vertice-core/**` has zero diff (`git status --short crates/vertice-core`) —
      stated explicitly in design §13 as a design violation if not true. [Req: workspace-architecture
      "vertice-core's dependency graph gains no logging crate"][Seq]
- [x] 8.3 Confirm `deny.toml` and `crates/vertice-app/capabilities/default.json` are byte-identical to
      pre-change (design §13: "Unchanged"). [Req: desktop-shell "Minimal Capability Grant"][Seq]
- [x] 8.4 Full Rust gate: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
      warnings`, `cargo test --workspace --locked`. Report each gate's actual pass/fail; if `cargo` is
      not resolvable on PATH, say so rather than reporting the gate as passing. [Seq]
- [x] 8.5 Full frontend gate, from `frontend/`: `npm run lint && npm run check && npm run test && npm
      run build`. Run explicitly `npm run check` in addition to `npm run test` — `npm run test`
      (Vitest) does not typecheck, so a binding or prop-type mismatch would otherwise pass silently.
      [Seq]
- [x] 8.6 Confirm the freshness-cache regression note (proposal §5, design §15): after Slice A, the
      response cache persists for the first time on a machine where the app-data directory never
      existed, so live reference lookups should stop happening on every launch within the existing
      6h TTL / 7-day stale ceiling (`cache.rs:17,21`). Verify this as the expected behaviour, not a
      regression, if it is observable in the test run. [Req: component-freshness restart-survival
      scenario][Seq]

## Phase 9: Follow-Up — Close `verify-report.md` WARNING 1 (test-coverage precision gaps)

Added after `sdd-verify`'s initial PASS WITH WARNINGS pass. Closes the three PARTIAL spec
scenarios by reusing the design's established injectable-closure test seam (design §14 C1)
instead of installing a second process-global logger (design deviation 4).

- [x] 9.1 Factor `lib.rs`'s `.setup` decision logic into a pure, testable `startup_sequence`
      function (app_data_dir result, `logging::init`, version, `log_info`/`report_stderr`
      closures); `.setup` calls it with the real `log::info!`/`eprintln!` sinks so runtime bytes
      written are unchanged. Add `lib.rs::tests::startup_sequence_logs_exactly_one_info_line_on_successful_init`.
      [Req: application-logging "Startup is logged once"]
- [x] 9.2 Add `lib.rs::tests::startup_sequence_reports_stderr_once_and_never_logs_on_init_failure`,
      driving a real, failing `logging::init` call through `startup_sequence` and asserting
      exactly one stderr line and zero INFO lines (plus a sibling test for the unresolvable-
      `app_data_dir` branch). [Req: application-logging "Sink Initialisation Failure Is Reported
      Once, On Stderr"]
- [x] 9.3 Factor `commands.rs`'s `run_scan(label)` into a thin wrapper over a new
      `run_scan_with(label, emit)`, mirroring `log_scan_report_with`'s shape. Add
      `commands::tests::run_scan_emits_one_info_start_line_and_one_info_finish_line_carrying_the_duration`,
      asserting exactly two INFO lines (`"{label} started"`, `"{label} finished in {duration_ms}
      ms"`) carrying the real measured duration. [Req: application-logging "A scan logs its start,
      end, and duration"]
- [x] 9.4 Re-run the full gate matrix (`cargo fmt`, `cargo clippy -D warnings`, `cargo test
      --workspace --locked`, `cargo deny check bans licenses`, and from `frontend/`: `npm run lint
      && npm run check && npm run test && npm run build`), confirm `crates/vertice-core` stays a
      zero diff, and restore `frontend/src/bindings/` after the `cargo test -p vertice-core`
      regeneration if it shows only a cosmetic CRLF diff. Update `verify-report.md`'s Spec
      Compliance Matrix (three PARTIAL rows -> COMPLIANT) and findings section. [Seq]

## Work-unit commits

1. `fix(app): create app data directory before the sanctioned settings/cache write` (Slice A, Phase 1)
2. `feat(app): add file-based logging sink with rotation` (Slice B + Slice E first pass, Phases 2-4)
3. `feat(app): log startup, scan, and freshness diagnostic events` (Slice C, Phase 5)
4. `feat(app,frontend): expose log file path via IPC and display it on the scan route` (Slice D + Slice E remainder, Phases 6-7)
5. `chore: cross-slice gate verification and bindings check` (Phase 8, may land folded into commit 4)
