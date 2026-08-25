# Proposal: Application Logging (`add-application-logging`)

Phase: sdd-propose · Artifact store: openspec · Status: done
Depends on: `openspec/changes/add-application-logging/exploration.md`

## 1. Intent

### Problem

When a user reports that Vertice behaved unexpectedly — a client was not detected, freshness
could not be determined, a scan found nothing — there is no artifact the maintainer can ask for.
The application writes no log at all: `crates/vertice-app/src/lib.rs` `run()` contains no logging
call, and no logging sink, trait, or module exists anywhere in the workspace. Diagnosis today
depends entirely on the user's ability to describe a transient UI state.

This is not a gap in domain signal. Exploration §5 establishes, and this proposal re-verified, that
every event worth logging is already a fully-formed typed value by the time it reaches
`vertice-app`: `ScanReport.duration_ms`, `ScanReport.issues`, `SearchRootStatus::NotFound` inside
`ScanReport.roots_scanned`, `ClientPresenceStatus::NotDetected` inside `ScanReport.client_presence`,
and `Freshness::Unknown { reason }` inside `FreshnessReport.checks`. The values exist and are
discarded. The missing piece is a sink and an observation point.

### A second, independent defect this cycle repairs

While answering "can the user actually open the folder the log would live in", the exploration
found a pre-existing bug: nothing in production code creates the application data directory.
`tauri::Manager::path().app_data_dir()` resolves a path but does not create it; every
`create_dir_all` under `crates/vertice-app/src/` sits below a `#[cfg(test)]` marker. Consequently
`freshness::cache::save`'s `fs::write` (`crates/vertice-app/src/freshness/cache.rs:83`) fails with
`NotFound` on a machine where the directory was never created, and both call sites discard the
error with `let _ =` (`crates/vertice-app/src/commands.rs:145`, `crates/vertice-app/src/freshness/mod.rs`).
The freshness response cache has therefore never persisted, and `set_freshness_settings` silently
reverts on every restart.

The best-effort tolerance is sound in itself; what was not anticipated is a failure that is
*permanent and total* rather than occasional. This defect is repaired inside this cycle (user
decision, 2026-08-24) as a distinct, independently verifiable work item with its own spec coverage
and its own regression test — not as an incidental side effect of the logging work — because it
predates logging and repairs an already-shipped user-visible behaviour.

### Why now

Vertice has left the PoC stage. The next users are people the maintainer cannot sit next to. A
support channel with no evidence artifact does not scale past that point, and the logging sink is
cheapest to build now, while every signal it needs is already typed and centralised in one crate.

### Success

- A maintainer can say "send me your log", and the user can find, open, and attach one file.
- That file answers, without a reproduction: did the app start, did a scan run and how long did it
  take, which search roots were missing, which clients were undetected, and why a freshness verdict
  could not be determined.
- Toggling the freshness check off survives an application restart.
- `cargo test --workspace` still proves CA-16: the workspace writes nothing outside the application
  data directory.

## 2. Resolved constraints inherited from exploration

These are settled. This proposal does not reopen them.

| Decision | Resolution |
|---|---|
| Log location | Inside `app_data_dir()` (Tauri `Manager::path().app_data_dir()`, bundle identifier `com.vertice.app`). **Not** `~/.vertice/`. |
| CA-16 | **Not amended.** The read-only invariant stands verbatim in `AGENTS.md`, `openspec/config.yaml` design principle 6, and the `desktop-shell` spec. |
| Audit exception | `crates/vertice-app/tests/read_only_audit.rs` gains a *second* named exception module beside `CACHE_MODULE_EXCEPTION` (`freshness/cache.rs`), subject to the identical proof obligations already applied there: the module must reference `app_data_dir`, must contain no literal absolute path, and must not read `std::env::`. |
| PII | Absolute paths containing the OS username are logged deliberately. No redaction, relativisation, or anonymisation work is in scope. A support log without real paths diagnoses nothing. |
| App-data-directory defect | Fixed in this cycle, as its own slice, with its own spec requirement and regression test, sequenced first; the logging work depends on it. |

## 3. Decisions this proposal makes

### D1. Logging library — `log` facade plus a single-owner sink module

**A correction to exploration §4 first.** The exploration states that "no `tracing`, `tracing-subscriber`,
`tracing-appender`, `log`, or `fern` entry exists anywhere in `Cargo.lock` today". That is not
accurate. `Cargo.lock` already resolves:

- `log` 0.4.33
- `tracing` 0.1.44 and `tracing-core` 0.1.36
- `chrono` 0.4.45, `jiff` 0.2.35, `time` 0.3.55 (plus `time-core`, `time-macros`)

Absent from the lockfile: `tracing-subscriber`, `tracing-appender`, `fern`, `env_logger`.

This changes the calculus. A crate already resolved in the lockfile has already passed
`cargo deny check licenses` (the gate is deterministic over the locked graph) and already compiles
under the MSRV floor, because the `msrv` CI job runs `cargo check` at 1.88 against this same
lockfile. Promoting such a crate from transitive to direct adds no license risk and no MSRV risk.

**Decision:** take `log` as a direct dependency of `vertice-app` and write the sink ourselves in one
module, exactly as `crates/vertice-core/src/yaml.rs` owns the YAML crate and `freshness/cache.rs`
owns the only file write. `log`'s macros populate `Record::file()` and `Record::line()` at the call
site, which is precisely the per-line metadata requirement, at the cost of one tiny, already-present
facade crate. Timestamp formatting uses a date crate already in the lockfile; the design phase picks
between `jiff`, `time`, and `chrono` (see the local-offset risk in §9) and runs the dependency gate.

**Rejected — `tracing` + `tracing-subscriber` + `tracing-appender` (exploration B1).** `tracing` and
`tracing-core` are already present, but the two crates that do the actual work are not, and they
bring a substantial new subtree that must be vetted from scratch. Independently of graph size,
`tracing-appender` rotates by wall-clock interval, not by size; the size bound in D3 is the
load-bearing requirement here, because the file has to be small enough to email. Adopting the
facade `log` keeps the door open: `tracing` can consume `log` records if the project later grows
structured spans.

**Rejected — `log` + `fern`.** `fern` is not in the lockfile, and it does not implement rotation
either, so it would buy formatting only — the part that is trivial to own.

**Rejected — fully hand-rolled with zero new direct dependencies (exploration B3).** Owning the
writer, the size check and the format is cheap and desirable. Owning civil-calendar conversion from
`SystemTime` to a human-readable local date and time is not; that is thirty lines of date arithmetic
we would have to test and maintain for no benefit, when three suitable crates are already resolved.

**Gate, explicitly not yet run:** `cargo deny check bans licenses` must actually pass with the new
direct dependencies before this is treated as settled. `cargo` may not resolve on PATH in the agent
environment; if the command cannot be run, that must be reported as unverified rather than assumed
green.

### D2. Discoverability — display the absolute path as selectable text (exploration D1)

The support workflow is: the maintainer tells the user to paste a path into their file manager's
address bar and email the file back. That requires the user to *see* the path, because `%APPDATA%`
is hidden in Explorer, `~/Library` is hidden in Finder, and `~/.local` is a dotfile.

**Decision:** the frontend renders the absolute log-file path as selectable text on the existing
`scan` route (`inventory-ui` spec, "Full Scan Report Route"), with a localised label. The path
reaches the frontend through one new read-only IPC command that returns a `String` derived from
`app_data_dir()`.

Be precise about what this costs. It is *not* free: it adds a sixth command, which widens the
surface that `crates/vertice-app/tests/read_only_audit.rs` asserts is exactly
`["scan", "rescan", "freshness", "freshness_settings", "set_freshness_settings"]` (both the
`exported_tauri_commands` matcher and the `assert_eq!`), and which the `desktop-shell` spec's
"Minimal Scan Command Surface" requirement fixes at five. What it does *not* cost is a capability:
the grant stays exactly `core:default`, so "Minimal Capability Grant" is reaffirmed rather than
weakened. The command returns a plain `String` (or `Result<String, ScanError>`, reusing the existing
`ScanError::Internal` mapping already used by `resolve_app_data_dir`), so no new `ts_rs` type and no
new generated binding is introduced.

**Rejected — "reveal in file manager" (exploration D2).** More convenient, but it costs the same new
command *plus* a shell/opener plugin permission. That permission is asserted against by name in the
read-only audit (`"shell:"`, `"tauri-plugin-shell"` are in its forbidden list) and by the
"Minimal Capability Grant" requirement. Trading a deliberately minimal, mechanically-enforced ACL
for the convenience of not pasting a path is a bad trade at this stage. It remains available later
if the paste instruction proves too much for real users.

### D3. Rotation and retention — 1 MiB per file, one rotated predecessor

**Decision:** the sink writes to a single current file. Before writing, if the current file is at or
above **1 MiB**, it is renamed over the single predecessor slot and a new empty current file is
started. Maximum on-disk footprint: **two files, ~2 MiB**.

Rationale for the numbers. The file is meant to be attached to an email, so the bound that matters
is per-file, not aggregate: 1 MiB is comfortably inside every mainstream provider's attachment limit
even before compression. At roughly 100 bytes per line, 1 MiB is on the order of ten thousand lines,
while the four event classes in this slice emit on the order of tens of lines per scan — so a
1 MiB current file holds far more history than any support conversation needs, and rotation will be
rare in practice. One predecessor rather than none exists so that a rotation occurring between the
incident and the maintainer's request does not destroy the evidence; more than one predecessor is
not justified when the request is always "send me the log", singular, and every extra file is
another thing the user has to be told to find.

**Rejected — time-based rotation (daily/hourly).** It bounds nothing that matters: an unlucky day
can exceed the attachment limit while a quiet week produces seven near-empty files the user must
choose between.

**Rejected — unbounded with manual cleanup.** A read-only-by-reputation desktop app must not grow a
file in the user's profile without a ceiling.

### D4. Format — plain text, one line per event, fixed leading columns

**Decision:** each line is
`<timestamp>  <LEVEL>  <source file>:<line>  <message>`, where the timestamp carries the date, the
time to at least second precision, and an explicit UTC offset. The column layout is fixed and
asserted by tests, so it cannot drift silently.

Rationale: the sole consumer is a human maintainer reading an emailed file in a text editor. Plain
text greps and eyeballs well, and costs roughly half the bytes of the equivalent JSON Lines against
the size bound in D3. Local time with an explicit offset — rather than bare UTC — matches how a user
reports an incident ("it happened around three") without leaving the reader guessing which zone the
line was written in.

**Rejected — JSON Lines.** It buys machine parseability for a pipeline that does not exist and is
not planned (design principle 8: no telemetry by default; nothing ingests these files). It can be
adopted later without changing the sink's location, rotation, or event coverage.

### D5. Failure classes — silent per line, loud once at startup

This codifies the design lesson the app-data-directory defect taught: a `let _ =` tolerance turned a
permanent, total failure into an invisible one.

**Decision — two distinct classes, specified separately and tested separately:**

1. **Per-line write failure is best-effort and silent.** A scan, rescan, freshness lookup or
   settings write MUST NOT fail, slow down, or change its result because a log line could not be
   written. This matches the existing, well-reasoned tolerance at `commands.rs:145` and in
   `freshness/mod.rs`.
2. **Sink initialisation failure is reported once, at startup, on stderr.** If the directory cannot
   be created or the file cannot be opened, the application still starts and still works, but it
   says so exactly once. Silence here is what produces the situation this exploration uncovered: a
   maintainer asking for a log that was never going to exist.

The asymmetry is the point. Per-line failures are transient and self-describing by absence;
initialisation failure is permanent and total, and must therefore be loud.

### D6. Architecture — confirm C1, no logging in `vertice-core`

**Decision: confirmed.** `vertice-core` gains no logging dependency, no logging module, and no
emission port. `vertice-app` observes the already-returned `ScanReport` and `FreshnessReport` and
logs from there; app startup is logged in `lib.rs` `run()` after the sink is initialised.

Rationale: §5's inventory shows every required value is already present in the returned reports, so
a live callback trait would buy nothing today. Introducing one would push an I/O-shaped concept into
a crate whose purity is mechanically enforced (`deny.toml` bans `tauri`/`tauri-build`/`reqwest`
outside `vertice-app`) and whose `model/` module declares an import allow-list that explicitly
forbids `std::fs`, `std::io`, `std::env`, `SystemTime` and `Instant`.

**On the future-CLI rationale, which is the strongest argument for C2.** A future `vertice-cli`
would reuse core's scan and get the same typed reports; what it would want to share is the *sink*
and the *line format*, not an inversion of control inside core. The correct move at that point is to
extract the sink module into a small third crate that depends on neither `tauri` nor `vertice-core`,
and have both binaries own their own observation points. That is a strictly cheaper refactor than
unwinding a trait seam from core, and it keeps the option open at zero cost today. If a later need
appears for core to emit *during* a scan — per-adapter timings, per-component progress — that is the
moment to introduce a `ReferenceLookup`-shaped port (the existing precedent in the
`workspace-architecture` spec, "The Reference-Version Seam Is Owned By vertice-app"), because that
need genuinely cannot be served from a returned value.

## 4. Scope

### In scope

**Slice A — application data directory creation (independent, sequenced first).**
The sanctioned settings/cache write path creates its parent directory before writing, so the
freshness cache and the freshness settings actually persist. The directory path stays derived
exclusively from `app_data_dir()`. Because `crates/vertice-app/src/freshness/cache.rs` is already the
single sanctioned exception module, this fix requires **no** change to the audit's exception list —
only that `create_dir` remains confined to that module. Own spec requirement, own regression test.

**Slice B — the logging sink.**
One module in `vertice-app` owns the file, the format, the size check and the rotation. Its path is
derived from `app_data_dir()`; it becomes the second named exception in
`crates/vertice-app/tests/read_only_audit.rs`, carrying the same three proof obligations as
`freshness/cache.rs`. Initialisation happens once at startup; the two failure classes of D5 are
implemented and tested separately.

**Slice C — event coverage.** All four required event classes, in this slice:

| Event | Observation point | Data (already exists) |
|---|---|---|
| App startup | `crates/vertice-app/src/lib.rs` `run()`, after sink init | — |
| Scan start and end, with duration | `commands.rs` around `run_scan()` | `ScanReport.duration_ms` |
| Search roots not found | `commands.rs`, over the returned report | `SearchRootStatus::NotFound` in `roots_scanned`; corresponding `ScanIssue` |
| AI clients not detected | `commands.rs`, over the returned report | `ClientPresenceStatus::NotDetected` in `client_presence` |
| Freshness undetermined, **with reason** | `commands.rs` `freshness`, over the returned report | `Freshness::Unknown { reason }` in `FreshnessReport.checks` |

**Slice D — discoverability.** One read-only IPC command returning the absolute log path; the `scan`
route renders it as selectable text with localised English and Spanish labels (design principle 7:
i18n from first commit).

**Slice E — invariant maintenance.** Audit test extended to two exception modules and six commands,
with per-exception proofs preserved; `desktop-shell` and `workspace-architecture` specs amended to
match; `cargo deny check bans licenses` run against the new direct dependencies.

### Out of scope

- **Configurable verbosity or a runtime log-level knob.** Justified out (exploration §10): the
  support workflow cannot ask a user to reproduce an incident with a flag, so the default must be
  useful as shipped. A single fixed level set, bounded by D3's size policy, serves that. A knob is
  additive later and costs nothing to defer.
- **Redaction, path relativisation, or PII scrubbing.** Resolved as a deliberate non-goal.
- **"Reveal in file manager" / any opener or shell capability.** Rejected in D2.
- **Any transmission of the log by the application** — no upload, no email, no crash reporter. The
  user attaches the file themselves. Design principle 8 stands: no telemetry by default. A local
  file the user chooses to send is not telemetry, and the specs should say so explicitly.
- **Logging inside `vertice-core`; per-adapter, per-component, or per-file trace events.**
- **JSON Lines output, an in-app log viewer, log search, or a "copy log" button.**
- **Amending CA-16, or adding any write path outside `app_data_dir()`.**
- **Logging the frontend's own console output.** Webview diagnostics stay in the webview.

## 5. Behaviour added, modified, removed — and for whom

| Change | Kind | For whom |
|---|---|---|
| A persistent, rotating, human-readable log file inside the app data directory | Added | Maintainer (diagnosis), user (has something to send) |
| Startup, scan lifecycle with duration, missing roots, undetected clients, and freshness-unknown reasons recorded | Added | Maintainer |
| The absolute log path visible and selectable on the `scan` route, in English and Spanish | Added | End user |
| One read-only IPC command returning the log path | Added | Frontend |
| Freshness opt-out and disclosure-seen state now survive a restart; the response cache now actually caches | **Modified (bug fix, user-visible)** | End user — a freshness check switched off will now stay off |
| A single stderr message when the log sink cannot be initialised | Added | User running from a terminal; maintainer |
| — | Removed | Nothing is removed |

Regression note for the modified row: the freshness cache becoming real means live reference lookups
stop happening on every launch. That is the intended, already-specified behaviour
(`component-freshness`: TTL and stale ceiling in `crates/vertice-app/src/freshness/cache.rs:17,21`),
but it will change observed network behaviour on machines where the cache has never worked, which is
all of them. Verification should expect it rather than treat it as a surprise.

## 6. Spec surface — traced to living capability specs

Per `openspec/config.yaml` proposal rule 1, this change touches:

1. **NEW `openspec/specs/application-logging/spec.md`.** Warranted on its own: sink location derived
   from the app data directory, line format, event coverage, rotation and retention bounds, the two
   failure classes, and the discoverability contract are independently testable and nest naturally
   inside no existing spec.
2. **`openspec/specs/desktop-shell/spec.md` — MODIFIED.** "Minimal Scan Command Surface" moves from
   five commands to six, naming the new read-only path command and restating that it causes no
   write. "Minimal Capability Grant" is amended to state explicitly that logging adds no capability
   and the grant remains exactly `core:default` — the same shape the freshness command's scenario
   already uses.
3. **`openspec/specs/component-freshness/spec.md` — MODIFIED.** Adds the requirement behind Slice A:
   the sanctioned settings/cache location MUST be created before it is written, so persistence
   survives a restart on a machine where the directory never existed. Plus a cross-reference that
   `Freshness::Unknown.reason` is also written to the application log.
4. **`openspec/specs/workspace-architecture/spec.md` — MODIFIED.** Records that logging is owned
   exclusively by `vertice-app`, that `vertice-core` acquires no logging dependency, and that the
   sink is a single-owner seam module in the same family as the YAML seam — swappable by touching
   one file.
5. **`openspec/specs/scan-orchestration/spec.md` — CROSS-REFERENCE ONLY.** "Visible and Isolated
   Diagnostics" already governs what is recorded; the log is an orthogonal sink. `ScanReport` and
   `ScanIssue` semantics are unchanged, and the spec should say so rather than absorb logging rules.
6. **`openspec/specs/inventory-ui/spec.md` — MODIFIED.** "Full Scan Report Route" gains the
   selectable log-path element.
7. **`openspec/specs/frontend-i18n/spec.md` — MODIFIED.** English and Spanish keys for the log-path
   label.
8. **`openspec/specs/ci-quality-gates/spec.md` — UNCHANGED, but exercised.** The existing
   `cargo deny check bans licenses` gate is the mechanism that validates the new direct dependencies;
   no new gate is proposed.

## 7. Structural invariants — verification

Per `openspec/config.yaml` proposal rule 3:

- **`vertice-core` stays Tauri-free.** Core is untouched by this change: no new dependency, no new
  module, no signature change. `deny.toml`'s `tauri`/`tauri-build`/`reqwest` bans are unaffected.
- **`vertice-core/src/model/` stays I/O-free.** Nothing in this change goes near `model/`. Its
  declared import allow-list — and its explicit prohibition of `std::fs`, `std::io`, `std::env`,
  `SystemTime`, `Instant` — is not relaxed. `ScanReport::duration_ms` continues to be a value passed
  in by the caller.
- **Single-owner seams.** The sink module becomes the sole owner of the logging crate and the sole
  owner of the log file, mirroring `crates/vertice-core/src/yaml.rs` (sole owner of `serde_norway`)
  and `crates/vertice-app/src/freshness/cache.rs` (sole owner of the settings write). No other module
  may open, write, or rotate the log file.
- **CA-16 read-only.** No write occurs outside `app_data_dir()`. Both exception modules derive their
  paths from `app_data_dir()`, contain no literal absolute path, and read no environment variable —
  each proved individually by `crates/vertice-app/tests/read_only_audit.rs`, not merely by the fact
  of being named in an exception list. The distinction matters: the exception list going from one
  entry to two makes a third easier to argue for, so the per-module proofs are what actually hold
  the line.
- **Type contract.** No new `ts_rs`-exported type is introduced; the new command returns a `String`.
  `frontend/src/bindings/` should regenerate byte-identical, and CI's bindings-in-sync check should
  stay green without a bindings commit.
- **Capability ACL.** `crates/vertice-app/capabilities/default.json` remains `["core:default"]`, with
  no scope block.

## 8. Rollback impact across the three layers

Per `openspec/config.yaml` proposal rule 4:

- **`vertice-core`** — zero delta, therefore zero rollback surface. Reverting this change cannot
  regress the domain library or the generated TypeScript bindings. This is the main structural
  benefit of D6.
- **`vertice-app`** — reverting Slices B–E removes the sink module, the startup and command-site
  observation points, the new IPC command, the two direct dependencies, and the second audit
  exception. The audit test returns to one exception and five commands with no residue. The revert
  is mechanically self-checking: if the exception entry were left behind while the module went away,
  the audit's `fs::read_to_string` of the exception module fails the test.
- **`frontend/`** — reverting Slice D removes the log-path element and its i18n keys from the `scan`
  route; nothing else on that route depends on them, and no binding changes, so the frontend gate
  (`npm run lint && npm run check && npm run test && npm run build`) is unaffected either way.
- **Critical rollback constraint.** **Slice A must not be reverted with the logging work.** The
  directory-creation fix repairs an already-shipped behaviour that predates logging, and reverting it
  silently re-breaks freshness-settings persistence. This is exactly why it is sliced first, with its
  own spec requirement and its own regression test: it must be revertible independently, and the
  logging revert must not touch it. `sdd-tasks` should keep it in its own commit.

## 9. Risks carried forward

1. ~~**Dependency gate unrun.**~~ **CLOSED (orchestrator, 2026-08-24).** `cargo deny check bans licenses`
   was run against the current tree and reports `bans ok, licenses ok` (only informational
   `license-not-encountered` and `unused-wrapper` warnings). The `[bans].deny` list in `deny.toml`
   contains exactly `tauri`, `tauri-build` and `reqwest` — no logging or date crate is banned, so
   promoting `log` (already resolved at 0.4.33) to a direct dependency of `vertice-app` does not
   trip the gate. This confirms the D1 baseline; the gate must still be re-run after the dependency
   is actually added, since it validates the resolved graph, not a hypothetical one.
2. **Lockfile presence is not the same as being compiled.** A crate can be resolved in `Cargo.lock`
   yet feature-gated off in the current build. Promoting `log` or a date crate to a direct dependency
   may still enable features or pull a subtree that is not compiled today. Not verified without
   running cargo.
3. **Local-offset formatting is a known trap.** The `time` crate refuses to determine the local UTC
   offset in a multi-threaded process, which Tauri's runtime certainly is. If `time` is chosen, D4's
   local-time-with-offset requirement may be unsatisfiable and the design must either pick `jiff` or
   `chrono` or fall back to UTC. This risk is concrete enough that it may decide the crate.
4. **Audit exception plurality.** Going from one sanctioned write module to two makes a third easier
   to justify. Mitigation is stated in §7 and must be enforced in design: the per-module proof
   obligations, not the name list, are the invariant.
5. **Windows path mapping unverified.** `app_data_dir()` mapping to roaming rather than local
   `%APPDATA%` is Tauri/`dirs` convention, asserted from documentation, not confirmed against a
   running build. The displayed path is produced at runtime from `app_data_dir()`, so the feature is
   correct either way — but any spec text or support instruction that names a literal Windows
   directory would be a guess and should be avoided.
6. **Write-path contention.** The sink is called from inside `spawn_blocking` tasks; a mutex-guarded
   writer serialises them. Negligible at the volume of D3, but the design must not hold the lock
   across the size check plus a slow filesystem call any longer than necessary, and must guarantee
   risk 7 below.
7. **The log must never become a scan failure.** D5 class 1 is the rule; the risk is that a future
   contributor "improves" a silent `let _` into a propagated error. The spec requirement and its
   test are the defence.
8. **Behavioural side effect of Slice A.** As noted in §5, the freshness cache starting to work
   changes observed network behaviour across restarts. Verification should assert the intended TTL
   behaviour rather than flag it as a regression.
9. **Command-surface creep.** Six commands is still minimal, but the `desktop-shell` spec's framing
   is "exactly N". Each future addition re-litigates that sentence. Worth watching, not blocking.

## 10. Delivery notes

Strict TDD is enabled (`openspec/config.yaml` `strict_tdd: true`). Every behaviour in §4 lands
test-first: a failing test for the directory-creation regression before the fix; a failing test for
the line format, the rotation threshold, and each failure class before the sink; a failing assertion
for the six-command surface and the second audit exception before the command lands; a failing
frontend test for the rendered path before the UI element. The audit test's own extension is written
before the module it sanctions exists.

Suggested slice order for `sdd-tasks`: **A** (directory creation, independently revertible) → **B**
(sink) → **C** (events) → **D** (IPC + UI + i18n) → **E** (audit and spec maintenance, which in
practice interleaves with B and D because the audit assertions must fail first).

## 11. Proposal question round

Presented here rather than asked interactively, because this phase ran without a direct channel to
the user. None of these block `sdd-spec`; each has a stated default that the proposal already
assumes, so silence is a valid answer.

1. **Rotation numbers (D3).** 1 MiB per file with one predecessor is derived from the email-attachment
   constraint. Is a smaller current file (say 256 KiB) preferable so the user never hesitates to
   attach it, or is more retained history worth more than attachment comfort?
   *Assumed default: 1 MiB, one predecessor.*
2. **Timestamp zone (D4).** Local time with an explicit offset is proposed over UTC because users
   report incidents in their own clock. If logs from several users are ever to be compared
   side-by-side, UTC is better. *Assumed default: local time with explicit offset.*
3. **Startup stderr message (D5 class 2).** In a packaged desktop build, stderr is invisible to a
   normal user — the message only reaches someone launching from a terminal. Is that acceptable, or
   should sink-initialisation failure *also* surface in the UI (which would grow the IPC surface
   again)? *Assumed default: stderr only; a hidden log is the maintainer's problem to notice, not the
   user's.*
4. **Log path placement (D2).** The `scan` route is proposed as the home for the path, since it is
   already the diagnostics route. A settings or about surface would be the conventional home in most
   desktop apps. *Assumed default: the `scan` route.*
5. **Scope check on Slice A.** The directory-creation fix makes the freshness cache real for the
   first time, which changes network behaviour across restarts (§5, risk 8). Confirming this is
   wanted now, rather than deliberately deferred, would be useful before `sdd-spec` writes the
   requirement. *Assumed default: wanted now — it is the already-specified behaviour.*
