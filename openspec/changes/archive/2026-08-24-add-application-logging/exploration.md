# Exploration: Application Logging (`add-application-logging`)

Phase: sdd-explore · Artifact store: openspec · Status: done

## 1. Context & user request summary

The user wants a persistent application log, written to a file under the user's home directory inside a `.vertice/` folder, covering:

- App startup.
- Scan start/end, including which search roots were not found and which AI clients were not detected.
- Freshness-verification failures ("cannot determine if a client is outdated"), including the machine-readable **reason**.
- Every log line prefixed with the emitting **source file** plus **date and time**.

This directly collides with the project's central, heavily-audited invariant: **CA-16 — read-only outside the app data directory** (`~/.vertice/` is not the platform app-data directory). That collision is the central finding of this exploration.

## 2. Current state of the codebase

**CA-16 enforcement — verified, multi-layered:**

- `AGENTS.md`: "no `File::create`, `OpenOptions::write`, or equivalent outside the app data directory."
- `openspec/config.yaml` line 18: `design_principles` item 6, "Read-only in PoC (CA-16: no writes outside app data directory)".
- `crates/vertice-app/capabilities/default.json`: `"permissions": ["core:default"]` only — no fs/shell/dialog plugin, so the webview has zero filesystem capability by construction.
- `crates/vertice-app/tests/read_only_audit.rs`: a mechanical audit, not just a doc. It:
  - Scans **every** `.rs` file under `crates/vertice-app/src/**` (stripping `#[cfg(test)]` bodies) for 16 forbidden mutation patterns (`File::create`, `OpenOptions`, `.write(`, `fs::write`, `create_dir`, `remove_file`, …).
  - Hard-codes exactly **one** exception: `const CACHE_MODULE_EXCEPTION: &str = "freshness/cache.rs"` (`read_only_audit.rs:16`).
  - Additionally asserts that exception module contains `app_data_dir`, **no** literal absolute path (`C:\`, `/home/`, `/Users/`, `/etc/`) and **no** `std::env::` read (`read_only_audit.rs:132-148`).
  - Asserts the Tauri command surface is exactly `scan`, `rescan`, `freshness`, `freshness_settings`, `set_freshness_settings`, and the capability grant is exactly `core:default`.
  - A parallel `crates/vertice-core/tests/read_only_audit.rs` audits core the same way.
- `openspec/specs/desktop-shell/spec.md:11`: "The settings-write command SHALL be the only command in the shell permitted to cause a write, and only to the sanctioned settings/cache location inside the app data directory." Normative RFC-2119 language, not historical PoC scaffolding.
- `crates/vertice-app/src/freshness/cache.rs:1-8` (module doc): "This is the only module in the whole workspace that writes a file (CA-16), and its path is derived exclusively from `tauri::Manager::path().app_data_dir()` — never a literal path, never an env read."
- `openspec/changes/archive/2026-08-22-audit-read-only-invariant/` is a whole completed change dedicated to proving this invariant.

`~/.vertice/` (a literal, hand-built path under the OS home directory, not `app_data_dir()`) is exactly the shape of write this machinery was built to catch and currently *does* catch: it would fail `read_only_audit.rs` as written today, and it contradicts the desktop-shell spec.

**Layering / core purity — verified:**

- `crates/vertice-core/src/model/mod.rs:8-16`: import allow-list `std::path`, `std::time::Duration`, `serde`, `ts_rs`, `thiserror`, `unicode_normalization`; explicitly forbids `std::fs`, `std::io`, `std::env`, `SystemTime`/`Instant`. `ScanReport::duration_ms` is "a value passed in by the caller, never measured here."
- `crates/vertice-core/src/scan.rs:5,15-19,25,49` — `pub fn scan()` resolves `home_dir()`, then `scan_for(&home, HostPlatform::current())` measures `Instant::now()` / `started.elapsed()` and populates `duration_ms`. The clock read lives in `scan.rs`, not in `model/`.
- `deny.toml` bans: `tauri`/`tauri-build` with `vertice-app` as sole allowed direct parent; `reqwest` similarly fenced.
- `openspec/specs/workspace-architecture/spec.md:140-154` "The Reference-Version Seam Is Owned By vertice-app" is the existing precedent for a trait-in-core / impl-in-app seam (`vertice_core::freshness::ReferenceLookup`, implemented by `vertice-app`'s `freshness/fetch.rs`).

There is currently **no** logging seam, trait, or module anywhere in the workspace.

## 3. Constraints and invariants in play

| Constraint | Where enforced | Risk to this feature |
|---|---|---|
| CA-16 read-only outside app data dir | `read_only_audit.rs` (both crates), `desktop-shell` spec, `capabilities/default.json` | Directly blocks `~/.vertice/` as specified |
| Core purity (no `tauri`) | `deny.toml` bans, `workspace-architecture` spec | A logger in core must not pull tauri in, directly or transitively |
| `model/` I/O-free | `model/mod.rs` doc comment | A file-writing logger cannot live in `model/`; likely not in `vertice-core` at all without a trait seam |
| Dependency containment precedent | `workspace-architecture` spec, `deny.toml` | A new logging crate in `vertice-core` gets the same scrutiny `reqwest` got |
| MSRV floor 1.88, toolchain pinned 1.97.1 | `Cargo.toml` `rust-version`, `rust-toolchain.toml` | Any new crate's declared MSRV must not exceed 1.88 |
| License allow-list | `deny.toml` `[licenses].allow` (MIT, Apache-2.0, BSD-2/3, ISC, Unicode-3.0, Zlib, CC0-1.0, MPL-2.0) | New crate's license must fall in this list |
| Strict TDD | `openspec/config.yaml` `strict_tdd: true` | RED-GREEN cycle required for the writer/seam |
| No telemetry by default | `openspec/config.yaml` design principle 8 | A local log file is not telemetry; the proposal should say so explicitly |

## 4. Dependency verification — no logging crate exists in the graph today

Checked `Cargo.toml` (root, `vertice-core`, `vertice-app`) and `Cargo.lock`:

- **CORRECTION (verified against `Cargo.lock`, 2026-08-24).** This section originally claimed no logging crate existed in the lockfile. That is wrong, and the `sdd-propose` phase caught it. `Cargo.lock` already resolves `log 0.4.33` (line 1804), `tracing 0.1.44` (3774), `tracing-core 0.1.36` (3784), `chrono 0.4.45` (313), `jiff 0.2.35` (1578) and `time 0.3.55` (3517), pulled in transitively by `tauri`/`reqwest`. **Absent** from the lockfile: `tracing-subscriber`, `tracing-appender`, `fern`, `env_logger`.

  This materially changes the §6.B tradeoff: promoting an already-locked crate to a direct dependency adds no new licence surface (it has already passed `cargo deny check licenses`) and no MSRV risk (the `msrv` CI job already runs `cargo check` at the 1.88 floor against this lockfile). Only the genuinely absent crates carry vetting cost. The proposal's library decision is based on the corrected facts.
- `vertice-app` deps: `vertice-core` (path), `tauri = "2"`, `reqwest = "0.13"` (native-tls, json), `serde`, `serde_json`, `semver`.
- `vertice-core` deps: `jsonc-parser`, `semver`, `serde`, `serde_norway`, `thiserror`, `toml_seam` (renamed `toml`), `ts-rs`, `unicode-normalization`, `walkdir`.

Library options (**not** vetted against `cargo deny` — `cargo` was not run in this session):

- **`tracing` + `tracing-subscriber` (`with_file(true)`, `with_line_number(true)`) + `tracing-appender`** — idiomatic match for "source file + timestamp per line" (`file!()`/`line!()` captured at the macro call site). Typically MIT/Apache-2.0. Transitive graph impact unverified.
- **`log` + `env_logger`/`fern`** — smaller footprint; `Record::file()`/`line()` are populated by the macros; needs a custom formatter, and rotation is not built in.
- **Hand-rolled writer** — `writeln!` to a `Mutex<File>`/`BufWriter` using `file!()`/`line!()` and `SystemTime` formatting. Zero new dependencies, but reimplements rotation/buffering/thread-safety.

MSRV 1.88 is recent enough that modern `tracing`/`log` majors are unlikely to be blocked, but each crate's declared MSRV was **not** verified — that check belongs to the design phase.

## 5. Signal inventory — what already exists vs. what's missing

| Required log event | Data source already in code | What's missing |
|---|---|---|
| App startup | None — `crates/vertice-app/src/lib.rs:10-21` `run()` has no logging call | A logging call at the top of `run()`, after logger init |
| Scan start/end | `crates/vertice-core/src/scan.rs:24-59` `scan_for()` already computes `started`/`duration_ms` | No emission point; core `scan()`/`scan_for()` have no I/O side channel. Logging happens either in `vertice-app`'s `commands::run_scan()` (`commands.rs:16-20`) or via an injected port |
| Search roots not found | `model/location.rs` `SearchRootStatus::NotFound`, consumed at `scan.rs:61-72` `append_missing_root_issues()`, producing `ScanIssue { severity: Warning, path: None, reason: "search root {id} was not found" }` | Nothing — already present in `ScanReport.issues` / `ScanReport.roots_scanned` |
| Clients not detected | `model/presence.rs` `ClientPresenceStatus::NotDetected`, produced in `installations.rs:79-109`, surfaced in `ScanReport.client_presence` | Nothing — iterate `ScanReport.client_presence` |
| Freshness "cannot verify" + reason | `model/freshness.rs:19-23` `Freshness::Unknown { reason: String }`. Populated in `vertice-app/src/freshness/mod.rs`: `ReferenceLookup::Unavailable{reason}` (network/parse/join failures, `mod.rs:144-146`; HTTP-client-build failure `mod.rs:163-166`), `ReferenceLookup::NoUpstream{reason}` (`upstream.rs`, e.g. `ClaudeCodeBundled`, reason built at `mod.rs:177`), fallback `"reference lookup was not resolved"` at `mod.rs:183` | Nothing — observe `FreshnessReport.checks` after `build_report()` returns in `freshness()` (`commands.rs:65-72`) |

**Conclusion: no new domain signal needs to be invented.** Every event the user wants logged is already a fully-formed, typed value by the time it reaches `vertice-app`. The design problem narrows to "where does an observer sit and how does it write", not "what do we need to track".

## 6. Approach options

### A. Where does the log live? (the CA-16 question)

| Option | Description | Tradeoff |
|---|---|---|
| **A1. App-data directory** | `tauri::Manager::path().app_data_dir()`, e.g. a `logs/` subfolder next to `freshness-cache.json` | Smallest blast radius: reuses the seam CA-16 already carves out; `read_only_audit.rs`'s `CACHE_MODULE_EXCEPTION` pattern extends to a second named exception with the same `app_data_dir`-derivation checks. But it is not the literal path requested |
| **A2. Amend CA-16 to allow `~/.vertice/`** | Add one additional declared directory for logs only | Matches the request exactly. Requires rewriting CA-16 in `AGENTS.md`, `openspec/config.yaml`, the `desktop-shell` spec, and `read_only_audit.rs`'s exception logic. Weakens a deliberately tight, heavily-tested invariant; needs explicit sign-off |
| **A3. Opt-in logging inside app-data dir** | Logging disabled by default; when enabled, still writes only inside `app_data_dir()` | Simplest to implement/test, no CA-16 scope change. Still not the requested path |

**RESOLVED (user decision, 2026-08-24): option A1.** The literal `~/.vertice/` path is *not* a hard requirement. The stated need is "a log I can find and open" — specifically for support: when a user reports a problem, the maintainer asks them to send their log. The log therefore lives inside `app_data_dir()` and **CA-16 is not amended**. `read_only_audit.rs`'s `CACHE_MODULE_EXCEPTION` pattern is extended to a second named exception module subject to the same `app_data_dir`-derivation checks (no literal path, no `std::env::` read).

**Is the resulting file reachable by the user? Yes.** The bundle identifier is `com.vertice.app` (`crates/vertice-app/tauri.conf.json:5`), so `app_data_dir()` resolves by Tauri 2 convention to `%APPDATA%\com.vertice.app\` on Windows, `~/Library/Application Support/com.vertice.app/` on macOS, and `~/.local/share/com.vertice.app/` on Linux. These are ordinary user-owned directories: no elevation, no special ACL, and a plain-text log opens in any text editor. (The exact Windows mapping is the Tauri/`dirs` convention — roaming rather than local app data — and was not verified against a running build in this session.)

### BLOCKING PRE-EXISTING DEFECT: the app-data directory is never created

Discovered while answering "can the user open that folder?" — on the maintainer's own machine, `%APPDATA%\com.vertice.app\` **does not exist**, despite the app having been run via `npx --prefix frontend tauri dev`. This is not a dev-vs-release artifact (`app_data_dir()` resolves identically in both; the identifier is the same). It is a defect:

- `tauri::Manager::path().app_data_dir()` *resolves* a path; it does **not** create the directory, and no Tauri plugin that would create it is installed (the capability grant is `core:default` only).
- No production code creates it either: every `create_dir_all` in `crates/vertice-app/src/` sits below a `#[cfg(test)]` marker (`commands.rs:155` guarding `commands.rs:238,293`; `freshness/mod.rs:203` guarding `mod.rs:219`; `freshness/cache.rs:106` guarding `cache.rs:122`).
- Therefore `cache::save`'s `fs::write` (`cache.rs:84`) fails with `NotFound` on every call, and both call sites discard the error: `freshness/mod.rs:195` and `commands.rs:145`, each `let _ = …`.

The best-effort `let _ =` tolerance is itself sound and well-reasoned in its comments. What was not anticipated is a failure that is **permanent and total** rather than occasional: on a machine where the directory never existed, the freshness cache has never persisted and never will, and `set_freshness_settings` silently reverts on every restart (same write path). User-visible prediction that confirms it without code changes: toggle the freshness check off, restart the app, observe it back on.

**Consequence for this change:** a logger targeting `app_data_dir()` cannot simply open a file there — it must `create_dir_all` the directory first. `create_dir_all`/`create_dir` is one of the 16 mutation patterns `tests/read_only_audit.rs` denies outside the single exception module, so the audit test's exception list must accommodate the logging module's directory creation under the same proof obligations already applied to `freshness/cache.rs` (path derived from `app_data_dir`, no literal absolute path, no `std::env::` read).

**Sequencing — RESOLVED (user decision, 2026-08-24): the fix ships inside this SDD cycle.** It is not deferred to a separate change. It must still carry its own spec coverage and its own regression test rather than riding along as an incidental side effect of the logging work: the defect predates logging and repairs an already-shipped behaviour (freshness-cache and settings persistence), so `sdd-tasks` should slice it as the first, independently verifiable work item that the logging tasks then depend on.

**Design lesson carried forward from this defect.** The `let _ =` best-effort tolerance turned a permanent, total failure into an invisible one. The logger must not repeat that shape. It has to distinguish two failure classes:

- *Per-line write failure* — best-effort and silent, exactly like the cache. A scan must never fail because a log line could not be written.
- *Logger initialisation failure* (directory not creatable, file not openable) — must be reported once at startup on stderr, not swallowed. Otherwise the maintainer asks a user for a log that was never going to exist, which is precisely the situation this exploration uncovered.

**Derived requirement from the support use case: discoverability, not accessibility.** Reachable is not the same as findable: `AppData` is hidden by default in Explorer, `~/Library` in Finder, and `~/.local` is a dotfile. A maintainer can type the path; the user being asked to email their log generally cannot find it unaided. The proposal must decide how the path is surfaced:

- **D1. Display the absolute path as selectable text in the UI.** No new capability, no new IPC command, no widening of the ACL that `tests/read_only_audit.rs` asserts is exactly `core:default`. Support instruction becomes "paste this into your file manager's address bar", which resolves the hidden-folder problem without any code reaching outside the app.
- **D2. A "reveal in file manager" action.** More convenient for a non-technical user, but costs a new IPC command plus a shell/opener capability, and every one of those additions is asserted against by the read-only audit test and by the `desktop-shell` spec's "Minimal Scan Command Surface" and "Minimal Capability Grant" requirements.

D1 is the cheaper option and appears sufficient for the stated support workflow; the proposal decides.

### B. Logging library

| Option | Pros | Cons |
|---|---|---|
| **B1. `tracing` + `tracing-subscriber` + `tracing-appender`** | Idiomatic; `with_file(true)`/`with_line_number(true)` gives exactly the requested per-line metadata; rolling/non-blocking writers for free | Heaviest dependency addition; several new transitive crates; needs a fresh `cargo deny check bans licenses` run |
| **B2. `log` + `fern` (or a minimal `log::Log` impl)** | Smaller footprint; macros already capture `file!()`/`line!()`/`module_path!()` | Rotation not built in; weaker structured-context story if the feature grows |
| **B3. Hand-rolled writer** | Zero new third-party surface to vet; fully controlled format | Reimplements buffering/thread-safety/rotation; more code to maintain under strict TDD |

### C. Where does the seam live?

- **C1. No logging code in `vertice-core`.** `vertice-app`'s `commands.rs` observes the already-returned `ScanReport`/`FreshnessReport` and logs from there. Core stays unaware. Simplest; but a future CLI binary would reimplement its own logging.
- **C2. Trait/port in `vertice-core`** implemented by `vertice-app`, mirroring `ReferenceLookup`. Only worth it if core must *emit* during scanning (e.g. per-adapter timing). Section 5 shows every needed value is already in the returned report, so a live callback trait may be unnecessary for v1.

Direction (not a decision): **C1** looks sufficient and sidesteps the core-purity/dependency-containment risk entirely for the first slice.

## 7. Risks

- **CA-16 collision is not cosmetic** — `read_only_audit.rs` will fail CI the moment a literal `~/.vertice/` write appears, plus normative `desktop-shell` spec language. Shipping requires either a deliberate, documented invariant change or relocating the log.
- **Audit surface growth** — the audit currently proves *zero* writes outside one exception. A second write path must be designed as carefully as the freshness cache's exception was.
- **PII/path leakage** — any log line naming scanned roots or client paths contains the OS username (`C:\Users\<name>\.claude\skills`). Unaddressed privacy question.
- **Never block or crash a scan** — precedent exists for best-effort degradation (`freshness/mod.rs:195` `let _ = cache::save(...)`, `commands.rs:141-145`). A failed log write must never fail `scan`/`rescan`/`freshness`.
- **Dependency vetting unverified** — `cargo` was not confirmed runnable in this session. `cargo deny check bans licenses` must actually be run against the chosen crate before it is treated as settled.

## 8. Open questions for the user

1. ~~**Location**~~ — **ANSWERED**: not a hard requirement; log goes inside `app_data_dir()`, CA-16 stands unchanged. See §6.A.
2. **Rotation/retention**: max file size, number of rotated files, or time-based rotation? No policy exists today. Now higher priority: a log meant to be *emailed to support* must stay small enough to attach, and old data must not accumulate indefinitely.
3. **Log level / verbosity**: is there a level knob, what is the default, and is it runtime-configurable? Support use case pushes toward a verbose-but-bounded default, since the user cannot be asked to reproduce with a flag.
4. ~~**PII**~~ — **ANSWERED**: absolute paths containing the OS username are accepted, deliberately and without redaction. Rationale (user, 2026-08-24): a support log without real paths cannot diagnose anything, and every desktop application writes readable logs containing them. No redaction or path-relativization work is in scope. Recorded as a conscious decision so a later reviewer does not reopen it as an oversight.
5. **Discoverability / UI visibility**: display the absolute path as selectable text (no new capability) vs. a "reveal in file manager" action (new IPC surface + wider ACL). See the derived requirement in §6.A.
6. **Format**: plain text lines vs. JSON Lines (both can carry file + timestamp). Plain text favours the support workflow — a human reads it directly.
7. **Failure behavior**: confirm logging failures are silent/best-effort, mirroring the freshness-cache write tolerance.

## 9. Spec surface

- **`openspec/specs/desktop-shell/spec.md`** — "Minimal Capability Grant" and "Minimal Scan Command Surface" need amendment if the log write path or a "reveal log" IPC command is added.
- **`openspec/specs/scan-orchestration/spec.md`** — "Visible and Isolated Diagnostics" already covers *what* gets recorded; logging is an orthogonal sink, so likely only a cross-reference.
- **`openspec/specs/component-freshness/spec.md`** — cross-reference for "the `Unknown.reason` value is also written to the application log".
- **`openspec/specs/workspace-architecture/spec.md`** — the place to document a C2-style trait seam if chosen.
- A **new capability spec `application-logging`** is warranted regardless of the location decision: log format, event coverage, rotation, and failure tolerance are independently testable and do not nest naturally inside any existing spec.

## 10. Recommended next phase and scope boundary

**Next phase: `sdd-propose`.** The CA-16 location fork is already resolved (A1 — inside `app_data_dir()`, invariant unchanged), so the proposal inherits it as a settled constraint rather than reopening it.

Open decisions the proposal must still make:

1. **Logging library** (§6.B: `tracing` stack vs. `log`+`fern` vs. hand-rolled), pending a real `cargo deny check bans licenses` run.
2. **Discoverability mechanism** (§6.A derived requirement): display the path vs. reveal-in-file-manager, weighed against the minimal-capability ACL.
3. **Rotation/retention policy**, now load-bearing because the log is meant to be attached to a support request.

Suggested first-slice scope: cover **all four required events** (startup; scan start/end with root and client diagnostics; freshness-unknown with reason), since §5 shows all their data already exists. Treat configurable verbosity as out-of-scope for the first slice unless the user decides otherwise.

## Files verified in this exploration

`AGENTS.md`, `openspec/config.yaml`, `deny.toml`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
`crates/vertice-app/capabilities/default.json`, `crates/vertice-app/tests/read_only_audit.rs`,
`crates/vertice-app/src/{lib.rs,commands.rs}`, `crates/vertice-app/src/freshness/{mod.rs,cache.rs,upstream.rs}`,
`crates/vertice-app/Cargo.toml`, `crates/vertice-core/src/{scan.rs,installations.rs}`,
`crates/vertice-core/src/model/{mod.rs,report.rs,error.rs,presence.rs,location.rs,freshness.rs}`,
`crates/vertice-core/Cargo.toml`,
`openspec/specs/{desktop-shell,scan-orchestration,workspace-architecture}/spec.md`,
`openspec/changes/archive/2026-08-22-audit-read-only-invariant/design.md`.
