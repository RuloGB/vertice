# Design: Report Whether A Detected Client Installation Is Out Of Date

> Trace: new capability **`component-freshness`**; deltas on `domain-model`, `workspace-architecture`, `desktop-shell`, `inventory-ui`, `frontend-i18n`, `scan-orchestration`, and — **correcting the proposal** — `client-installation-detector` (§2). Bounded by **CA-15**, **CA-16**, **CA-17**.
> Inherits `archive/2026-08-23-report-client-presence-as-status/design.md` (**P31D**) §2 (`ClientPresence` shape), §3 (single producer), §6 (bindings discipline); reverses P31D §12's "`InstallSlot` stays private" on the retrofit condition that design itself implies — see §2.
> `rules.design` coverage: core data model (§3), core/Tauri isolation (§10), IPC contract surface (§11), error paths (§12), dependency policy (§4, §5).
> **Environment note.** No network request was issued from this phase and no shell was run. Upstream facts in §0 were verified by the orchestrator against the live registries on 2026-08-24 and are recorded as *verified elsewhere*, not re-verified here. Every dependency property in §4 is marked verified-from-`Cargo.lock` or **unverified**, with the exact command that closes it.

## 0. What is verified, and what is not

| # | Statement | Basis |
|---|---|---|
| V1 | `tauri` 2 depends **directly** on `reqwest 0.13.4`, and `hyper 1.11.0`, `tokio 1.53.1`, `serde_json 1.0.151`, `url 2.5.8`, `percent-encoding` are already in the graph | Read `Cargo.lock:1283, 2159, 2472-2503, 2681, 3072-3097, 3394, 3735` |
| V2 | **No TLS backend is in the lock**: no `rustls`, `ring`, `aws-lc-rs`, `native-tls`, `schannel`, `openssl-sys` entry exists | Grepped `Cargo.lock` for each name — zero hits |
| V3 | `deny.toml` bans exactly `tauri` and `tauri-build`, `wrappers` exempts only **direct** dependents, and the licence allow-list is the ten SPDX ids at `deny.toml:55-66` | Read `deny.toml` |
| V4 | MSRV floor `1.88` is declared in `Cargo.toml:8`, `.github/workflows/ci.yml:44`, and floored against `rust-toolchain.toml` channel `1.97.1`; the `msrv` job runs `cargo check --workspace --locked --all-targets` at the floor | Read all three + `ci.yml:70-89, 217-238` |
| V5 | `ClientPresence` publishes `label`, `probed_paths`, `status`, `installations` — **no machine-readable slot identity**; `InstallSlot` is private (`installations.rs:133-139`) | Read `model/presence.rs`, `installations.rs` |
| V6 | `crates/vertice-app/tests/read_only_audit.rs` asserts the command list is **exactly** `["scan", "rescan"]` and greps `commands.rs` + `lib.rs` for 16 mutation patterns including `fs::write` and `create_dir` | Read the test |
| V7 | Commands are `async` and offload via `tauri::async_runtime::spawn_blocking` (`commands.rs:15-19`) | Read `commands.rs` |
| V8 | 18 files exist in `frontend/src/bindings/` | Globbed |
| **U1** | npm `opencode-ai` latest `1.18.21`; npm `@anthropic-ai/claude-code` latest `2.1.241`; `GET /repos/openai/codex/releases/latest` → `tag_name: "rust-v0.149.1"`, `name: "0.149.1"`; GitHub `releases/latest` excludes prereleases | **Verified by the orchestrator against the live registries, 2026-08-24.** Not re-verified in this phase |
| **U2** | Whether the Claude Desktop MSIX bundled runtime tracks the `@anthropic-ai/claude-code` npm release line | **Unverified and unverifiable from evidence available.** Resolved as "no upstream" — §6 |
| **U3** | `reqwest` 0.13.4's MSRV, and the licence set of any TLS backend | **Unverified.** Closing commands in §4 |

## 1. Technical approach

The scan is untouched. A **second, independent IPC command** returns a freshness report. `vertice-app` fetches every reference version first, hands the results to `vertice-core` as plain data, and core performs a **pure, synchronous, total** comparison. Core never holds a socket, never awaits, and never names an HTTP crate.

```
                                    vertice-core                     (no tauri, no http, no async)
 frontend ──IPC scan─────> vertice-app ──> model/  + Freshness, FreshnessSubject,
          ──IPC freshness─>    │                     FreshnessCheck, FreshnessReport,
                               │                     ClientInstallSlot          (plain data, §3)
                               │           freshness ── compare() + trait ReferenceVersions  (§10)
                               │
                               └── freshness/            (the ONLY module that names reqwest)
                                     ├── upstream.rs   slot -> upstream identity      (§6)
                                     ├── fetch.rs      3 concurrent GETs, 5s budget   (§9)
                                     └── cache.rs      app_data_dir(), the only write (§8)

 command freshness(app) -> FreshnessReport
   1. setting off?  -> FreshnessReport { enabled: false, checks: [] }   — no request issued
   2. scan_for(home, platform) -> presence records -> (slot, version, path) subjects
   3. cache hit within TTL? use it.  else fetch. failure -> stale cache (<= 7d) -> else Unavailable
   4. core::freshness::evaluate(&map, &subjects)   <-- pure, sync, total
```

## 2. Decision: the subject key — promote the slot to a public model type

The proposal flagged this as unresolved. It is resolved, and it **corrects the proposal's "Explicitly NOT Modified" list**: `client-installation-detector` and `domain-model` gain deltas.

| Option | Consequence | Decision |
|---|---|---|
| Key on `ClientPresence.label` | Already public and unique per report (V5). But it is **display copy** — `P30D` reworded the slot vocabulary once already, and keying a verdict on a proper-noun string revives exactly the `MISSING_CLIENT_REASONS` string-matching that `P31D` §6 deleted. The fetcher would dispatch upstream identity by matching `"OpenCode (npm)"` | **Rejected** |
| Key on `ClientInstallation.path` alone | Unique by construction and needs no model change, but carries **no slot**, so the fetcher still cannot resolve an upstream without inferring it from the path shape — string matching wearing a `PathBuf` | **Rejected as the sole key** |
| Mint a second public enum mirroring the private `InstallSlot` | Two closed enums that must be kept in lockstep by review. The exact "unrepresentable by construction" failure this codebase avoids | **Rejected** |
| **Promote `InstallSlot` to `model::ClientInstallSlot` and add `pub slot` to `ClientPresence`; key each verdict on `FreshnessSubject::ClientInstallation { slot, path }`** | One closed enum, exhaustively matchable in Rust and in TS. `slot` answers *which upstream*; `path` answers *which of this slot's installations* (CA-7 allows many). Additive to `ClientPresence`; `label()` becomes a method on the public enum, unchanged in output | **Chosen** |

**P31D §12 was right for its evidence.** It kept `InstallSlot` private because nothing outside core needed it and a third exported type would have been speculative. A consumer now exists that must dispatch on slot identity without parsing prose — the same two-part condition P31D §5 set for touching `model/`. The price is one enum, one field, one binding regeneration.

## 3. Decision: core data model

`ClientInstallation`, `ClientPresenceStatus`, `ScanIssue`, `IssueSeverity`, `ScanReport` are **unchanged**. `ClientPresence` gains one field (§2). `model/` gains one file.

```rust
// crates/vertice-core/src/model/freshness.rs — allow-list respected:
// std::path::PathBuf, serde, ts_rs only. No fs, no io, no env, no clock.

/// Three-valued by settled decision. `Unknown` is a first-class outcome,
/// not an error path. There is NO fourth state — see §7.
pub enum Freshness { UpToDate, Outdated { latest: String }, Unknown { reason: String } }

/// The discriminator AND the id in one closed enum. Today one variant;
/// skills and agents arrive later as `Skill { id: ComponentId }` etc.
pub enum FreshnessSubject {
    ClientInstallation { slot: ClientInstallSlot, path: PathBuf },
}

pub struct FreshnessCheck { pub subject: FreshnessSubject, pub installed: String, pub verdict: Freshness }

/// `enabled: false` means the user turned the check off and NO request was
/// issued. It is distinct from every-check-`Unknown`, which means we tried.
pub struct FreshnessReport { pub enabled: bool, pub checks: Vec<FreshnessCheck> }
```

`FreshnessCheck.installed` is denormalised deliberately: the badge renders beside a version, and without it the frontend must re-join on `path` across two payloads returned at different times.

Five new exported types (`Freshness`, `FreshnessSubject`, `FreshnessCheck`, `FreshnessReport`, `ClientInstallSlot`), five new `bindings/*.ts`, plus a modified `ClientPresence.ts`. `domain-model`'s type enumeration grows by five; the resulting count is taken from the spec text at merge time, not asserted here.

`Freshness` and `FreshnessSubject` are the first data-carrying enums in `model/`, so serde's externally-tagged encoding reaches the bindings for the first time. This was the cost P31D §4 declined to pay for a case that did not need it; here the payload *is* a tagged union, and TS receives a discriminated union the badge switches on exhaustively.

## 4. Decision: the HTTP client — `reqwest`, already in the graph

| Option | Consequence | Decision |
|---|---|---|
| **`reqwest`, `default-features = false`, added as a direct dep of `vertice-app`** | V1: `tauri` already depends on it directly at `0.13.4`, and `hyper`/`tokio`/`serde_json`/`url` are already present. Version-unifies — `multiple-versions` is not even exercised. **The only genuinely new crates are the TLS backend** (V2). Async, but that is free: `tauri::async_runtime` *is* tokio and the command is already async (V7) | **Chosen** |
| `ureq` (blocking, small) | Superficially the "dependency-light" answer, and it was the exploration's instinct. It is the opposite: it adds a **second, complete HTTP stack** — its own connection pool, its own `http` glue and its own TLS wiring — beside the hyper stack already compiled into the binary, **and still needs the same TLS backend**. Strictly more crates than the chosen option for the same capability | **Rejected** |
| `tauri-plugin-http` | Exists to expose `fetch` to the **WebView**. Adopting it means a capability grant beyond `core:default` and a CSP conversation. Directly contradicts the settled decision that the WebView never performs the lookup | **Rejected** |
| Platform HTTP APIs (WinHTTP) | Zero crates, but three per-OS implementations and `unsafe` code, which `[workspace.lints.rust] unsafe_code = "deny"` forbids | **Rejected** |

**TLS is the whole risk, and it is unverified (U3).** V2 proves no TLS backend is in the lock today, and both registries are HTTPS-only. Two backends, neither yet evaluated:

- `rustls-tls` — pulls `rustls` plus `ring` or `aws-lc-rs`. `ring`'s licence is a **non-SPDX composite** and is the single most likely thing to fail `cargo deny check licenses` against the ten-id allow-list (V3). Portable; no system libraries.
- `native-tls` — `schannel` on Windows, `security-framework` on macOS, `openssl` on Linux. Avoids `ring` entirely but makes the Linux CI leg depend on system OpenSSL headers.

**This design does not pick one, because picking one without running the gate would be exactly the invented fact the proposal forbids.** The choice is made by running, for each backend, in this order:

```
cargo tree -e no-dev -p vertice-app --target x86_64-pc-windows-msvc   # new-crate delta
cargo deny check bans licenses                                        # allow-list, unchanged
cargo check --workspace --locked --all-targets                        # at toolchain 1.88 (V4)
```

**Ordering rule, binding on `sdd-apply`:** prefer the backend that requires **no new entry in `deny.toml`'s allow-list**. If both require one, adding a licence to the allow-list is a reviewable decision recorded in `deny.toml`'s comment block, never a silent edit. If neither compiles at 1.88, **the dependency changes, not the floor** (V4) — the fallback ladder is: other TLS backend → `ureq` → the pinned-manifest source the proposal rejected, which needs no network at all and would ship a degraded but honest feature.

One inference, flagged as an inference: the `msrv` job compiles the whole workspace at 1.88 today and `reqwest 0.13.4` is in the lock (V1), so **if** it is in the desktop build graph it already builds at the floor. `cargo tree` above confirms or refutes that; do not rely on it before then.

## 5. Decision: `deny.toml` containment

`reqwest` joins the ban list with `vertice-app` **and `tauri`** as allowed direct parents:

```toml
{ name = "reqwest", wrappers = ["vertice-app", "tauri"] },
```

`tauri` must be listed: V1 shows it is a legitimate direct parent, exactly the situation `deny.toml:48-51` already documents for `tauri-build`. Omitting it turns the gate into a false positive on the pre-existing graph.

| Option | Consequence | Decision |
|---|---|---|
| **Ban `reqwest` with two allowed wrappers** | `vertice-core` — or a future `vertice-cli` — acquiring HTTP through a convenience refactor fails CI immediately, the same mechanism that keeps core Tauri-free. Core's HTTP-free property becomes **structural, not reviewed** | **Chosen** |
| Ban the TLS backend and `hyper` too | `wrappers` exempts only direct parents (V3), and neither is a direct dependency of `vertice-app`. Listing them produces false positives on legitimate transitive edges — the mistake `deny.toml:42-45` already records for `wry`/`tao` | **Rejected** |
| No ban; rely on review | The proposal names "core accidentally acquires HTTP" as a live risk. A review-only control is the one the workspace already chose not to accept for `tauri` | **Rejected** |

## 6. Decision: per-slot upstream resolution

| Slot | Upstream | Request | Version field | Basis |
|---|---|---|---|---|
| `ClaudeCodeNpm` | npm `@anthropic-ai/claude-code` | `GET https://registry.npmjs.org/@anthropic-ai%2fclaude-code/latest` | `.version` | Derived from `installations.rs:222-232`; latest confirmed U1 |
| `OpenCodeNpm` | npm `opencode-ai` | `GET https://registry.npmjs.org/opencode-ai/latest` | `.version` | Derived from `installations.rs:245-248`; latest confirmed U1 |
| `CodexStandalone` | GitHub `openai/codex` | `GET https://api.github.com/repos/openai/codex/releases/latest` | see below | U1 |
| `ClaudeCodeBundled` | **none** | **no request issued** | — | U2 |

**Codex version field — neither `name` nor `tag_name` is trusted alone.** U1 shows `tag_name: "rust-v0.149.1"` and `name: "0.149.1"`: the tag carries a release-train prefix that is not part of the version, and both fields are free-form strings an upstream may reshape without notice. The rule is therefore a total, ordered candidate list, not a field pick:

1. `name`, parsed as semver;
2. else `tag_name` with a leading `rust-v` or `v` stripped, parsed as semver;
3. else `Unknown { reason }`.

`releases.openai.com/codex` is OpenAI's documented primary distribution and was considered as the reference source. **Rejected**: its response schema is not verified anywhere in this repository or in U1, whereas the GitHub Releases schema is stable, documented and already confirmed for this exact repository. Choosing an endpoint whose shape we have not seen would be the invented-fact failure the proposal forbids.

**`ClaudeCodeBundled` is permanently `Unknown { reason }`, and issues no request.** Its version is an MSIX package-cache directory name (`installations.rs:482-570`) from a Microsoft Store distribution channel. U2 could not establish that this build tracks the npm release line. Comparing it to `@anthropic-ai/claude-code` would produce a **confidently wrong** verdict — the precise failure mode this feature exists to eliminate — and the proposal's rule is explicit: a slot with no verified upstream MUST NOT report `UpToDate`. Not issuing the request is a bonus: it is one fewer call against the rate limit. **Closing condition:** if Anthropic documents that the bundled runtime tracks a queryable release line, this becomes one row in the table above and one fixture.

**Request content carries no user data**: no query parameters, no identifier, no inventory. A single `User-Agent: vertice/<crate version>` header, required by the GitHub API and otherwise inert. No authentication token — an unauthenticated request is anonymous by construction, and a token would be a credential Vertice has no business storing.

## 7. Decision: prerelease semantics — three states, no fourth

The comparison is `semver::Version` ordering, applied without special-casing. Two cases follow, and the second is the product decision.

| Case | Semver ordering | Verdict | Reasoning |
|---|---|---|---|
| installed `0.150.0-rc.1`, reference `0.150.0` | `<` | **`Outdated { latest: "0.150.0" }`** | The user is on a release candidate of a version that has since shipped. Telling them so is the feature working |
| installed `0.151.0-rc.1`, reference `0.149.1` (GitHub `releases/latest` excludes prereleases, U1) | `>` | **`UpToDate`** | See below |
| either side fails to parse | — | **`Unknown { reason }`** | Total fallback; never a panic, never a guess |

| Option for "installed is ahead of latest stable" | Consequence | Decision |
|---|---|---|
| **`UpToDate`** | The verdict answers exactly one user question: *should I update?* The answer is no. The state is stable — U1 confirms `releases/latest` never returns a prerelease, so a prerelease user would otherwise sit in a permanent non-default state for as long as they stay on that channel | **Chosen** |
| A fourth `Ahead`/`Prerelease` variant | Breaks the settled three-valued decision, adds a binding variant, a badge colour, two i18n keys in two locales — for a state the user **cannot act on** and already knows about, since running a prerelease is a deliberate act. Speculative generality in the one place the proposal did not ask for it | **Rejected** |
| `Unknown` | Dishonest. We *can* tell; we compared successfully. `Unknown` means "cannot tell" and diluting it damages the one state the offline path depends on | **Rejected** |

**Consequence for the enum shape, stated plainly:** `Outdated { latest }` is produced **only** when installed `<` reference strictly. `UpToDate` covers equal-or-greater. The `Freshness` enum keeps exactly three variants, as settled.

## 8. Decision: cache policy

| Property | Decision | Rejected alternative |
|---|---|---|
| Location | `app_data_dir()/freshness-cache.json`, resolved **only** via `tauri::Manager::path().app_data_dir()` — never a literal path, never an env read | A hand-built `%APPDATA%` path: an env read and a per-OS branch, both already forbidden idioms in this workspace |
| Format | JSON via `serde_json` (V1: already in the graph). A map from upstream identity → `{ version, fetched_at_unix_s }` | `toml`/YAML: core's seams are read-only parsers with no serializer, and this file is machine-written machine-read |
| Key | The **upstream identity** (`npm:opencode-ai`, `github:openai/codex`), not the slot | Keying by slot would refetch the same npm package twice if two slots ever shared one |
| TTL | **6 hours** | 1h burns the ~60 req/h unauthenticated GitHub budget for a value that changes weekly; 24h leaves a user who *just updated* staring at `Outdated` for a day — the trust-eroding false positive |
| Force refresh | A user-initiated "check now" bypasses the TTL. One gesture, inherently bounded | An automatic retry loop, which doubles latency and rate-limit consumption invisibly |
| Fetch failure with an expired entry | Serve the stale entry up to a **7-day** ceiling; the reason string records staleness. Beyond the ceiling → `Unknown` | Never serving stale data: turns a brief outage into blank `Unknown` badges when a 3-hour-old answer is almost certainly still correct |
| Corrupt / unreadable / truncated cache | Treat as **empty**. Fetch live; overwrite on next success. Never crash, never surface a `ScanIssue` | Deleting the file, or erroring: both convert a recoverable nuisance into a user-visible failure |
| Write shape | One whole-file `fs::write` of a small document. **No temp-file-plus-rename** | Atomicity buys nothing here: a torn write is indistinguishable from a corrupt cache and the row above already handles it. `fs::rename` would add a second mutation verb to the read-only audit surface for zero benefit |

**CA-16.** This is the only write introduced anywhere by this change, and V6 shows `read_only_audit.rs` currently forbids `fs::write` and `create_dir` in the app's command surface by grep. The write is therefore confined to `freshness/cache.rs` and the audit is extended, not weakened — §14.

## 9. Decision: timeout, retry, and the in-flight state

- **Per-request budget: 3 s connect, 5 s total.** The three requests (§6 — the bundled slot issues none) run **concurrently** on Tauri's existing runtime, so the wall-clock budget for the whole command is ~5 s, not 15 s.
- **Retries within a check: zero.** A retry doubles the worst-case latency and the rate-limit spend to rescue a case the cache and the force-refresh gesture already cover. On HTTP 403/429 the reason names rate limiting and a cooldown timestamp is written to the cache so the next check does not hammer a limiter that is already refusing.
- **CA-15 is untouched by construction**: the scan command is not modified, does not await this, and `ScanReport.duration_ms` never observes it.

**In-flight versus gave-up.** `pending` is a **frontend-only** state — the freshness command has been invoked and has not resolved. It is deliberately **not** a `Freshness` variant: a transient UI condition has no place in a domain enum, and admitting it would reopen §7's three-state decision through a side door. "Gave up" is not a separate state either: **giving up *is* `Unknown { reason }`**, which is exactly what the three-valued model was chosen to represent honestly. The badge therefore has five renderings from two sources: `pending` (no report yet), and `upToDate` / `outdated` / `unknown` / *hidden* (`report.enabled == false`) from the report.

## 10. The core↔app boundary, and the test seam

The seam is deliberately **not** "fetch a version". It is "answer from what has already been fetched":

```rust
// crates/vertice-core/src/freshness.rs  (outside model/, mirroring installations.rs)
pub enum ReferenceLookup { Found(String), NoUpstream { reason: String }, Unavailable { reason: String } }

pub trait ReferenceVersions { fn latest_for(&self, subject: &FreshnessSubject) -> ReferenceLookup; }

/// Ships in core. The test stub AND the app's production adapter.
pub struct MapReferenceVersions(/* subject -> ReferenceLookup */);

pub fn compare(installed: &str, reference: &str) -> Freshness;                 // total, pure
pub fn evaluate(source: &impl ReferenceVersions, subjects: &[(FreshnessSubject, String)])
    -> Vec<FreshnessCheck>;                                                    // total, pure, sync
```

| Option for the seam | Consequence | Decision |
|---|---|---|
| **Synchronous trait; the app fetches first, then calls `evaluate`** | Core has no async, no runtime, no executor concept — it stays usable by a future headless CLI with any runtime or none. The async/sync tension disappears instead of being papered over | **Chosen** |
| `async` trait in core (`async_trait` or AFIT) | Drags a runtime-shaped concept, and probably a crate, into the crate whose whole point is a small auditable dependency footprint. Buys nothing: core has nothing to await | **Rejected** |
| A blocking-fetch trait core calls itself | Core would own the call site of an I/O operation it must never perform, and every core test would need an inversion to stay offline. The current shape needs no inversion — there is nothing to invert | **Rejected** |

**Network-free core testing is structural, not disciplinary.** `vertice-core` has no HTTP dependency at all, so no core test *can* open a socket without adding one — and §5's `deny.toml` entry fails CI the moment anyone tries. This is stronger than the `yaml.rs`/`jsonc.rs` seams, which contain a crate that *is* present; here the crate is absent from the crate graph entirely. CA-17 needs no discipline to hold.

## 11. IPC contract surface

**Three new commands; no event, no capability change, no CSP change.**

> **Corrected 2026-08-24, after Slice 3.** This section originally said "one new command". That was wrong, and the error was structural rather than clerical: the settled default posture requires a visible opt-out and a first-run disclosure, and neither can work without a way to read and persist that state. The gap was found during Slice 2 and closed in Slice 3, but this section was not updated at the time — `sdd-verify` caught the contradiction against `desktop-shell`'s "SHALL expose exactly three commands". Both artifacts now describe the shipped five-command surface.

```rust
#[tauri::command]
pub async fn freshness(app: tauri::AppHandle) -> Result<FreshnessReport, ScanError>

#[tauri::command]
pub async fn freshness_settings(app: tauri::AppHandle) -> Result<FreshnessSettings, ScanError>

#[tauri::command]
pub async fn set_freshness_settings(
    app: tauri::AppHandle,
    enabled: bool,
    disclosure_seen: bool,
) -> Result<FreshnessSettings, ScanError>
```

- `set_freshness_settings` takes the **full desired state**, not a partial patch, so a concurrent caller cannot half-apply an opt-out.
- It is the only command permitted to cause a write, and only inside the app data directory (CA-16, §8).

- Returns the generated model type directly — **no hand-written DTO**, per `desktop-shell`.
- Reuses `ScanError` rather than adding an error type: the only failures that reach here are transport-level, exactly as `map_join_error` already handles (`commands.rs:25-29`). A registry failure is **not** an error — it is `Unknown` inside a successful report.
- The `AppHandle` parameter is new to this crate's command signatures and is what resolves `app_data_dir()` (§8). Path resolution in Rust needs no Tauri permission; `capabilities/default.json` stays `core:default` and its unchanged state is a review check pinned by V6's test.
- `invoke_handler` becomes `generate_handler![commands::scan, commands::rescan, commands::freshness]`.
- The setting (enabled/disabled, and the first-run-disclosure-seen flag) is persisted in the same app-data JSON document as the cache — one file, one write path, one audit surface.

## 12. Error and degradation paths

**No new `ScanIssue`, no new `IssueSeverity`, no `ScanIssue` at all.** A freshness failure never enters the diagnostics channel.

| Condition | Result |
|---|---|
| Setting disabled | `FreshnessReport { enabled: false, checks: [] }`; **no request issued, no cache read** |
| Offline / DNS failure / connection refused | every check `Unknown { reason }` |
| Timeout (§9) | `Unknown { reason }` |
| HTTP 403/429 (rate limited) | `Unknown { reason }` naming rate limiting; cooldown recorded |
| HTTP 4xx/5xx otherwise | `Unknown { reason }` carrying the status code |
| Body unparseable, field missing, field not a string, or response over a **256 KiB** ceiling | `Unknown { reason }`. Responses are untrusted input; the ceiling is enforced before parsing |
| Reference parses but installed version does not (MSIX directory name) | `Unknown { reason }` |
| Slot with no upstream (`ClaudeCodeBundled`) | `Unknown { reason }`, permanently, no request |
| Cache corrupt | treated as empty; live fetch; overwritten on success |
| Any of the above | **zero `ScanIssue`**, `incidentCount` unchanged, Home banner unchanged |

`Outdated` is **information, not a fault**: `frontend/src/lib/scanDiagnostics.ts` is expected **unchanged**, and that expectation is a test (§14), because routing `Outdated` into `incidentCount` is precisely the regression P31D removed.

## 13. File changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/model/freshness.rs` | **Create** | §3 — four exported plain-data types |
| `crates/vertice-core/src/model/installation.rs` or new `slot.rs` | **Create/Modify** | `ClientInstallSlot` public enum (§2) |
| `crates/vertice-core/src/model/presence.rs` | Modify | one field, `pub slot: ClientInstallSlot` |
| `crates/vertice-core/src/model/mod.rs` | Modify | `mod freshness;` + `pub use` |
| `crates/vertice-core/src/freshness.rs` | **Create** | §10 — `compare`, `evaluate`, the trait, `MapReferenceVersions` |
| `crates/vertice-core/src/installations.rs` | Modify | private `InstallSlot` replaced by the model type; `label()` moves with it; `resolve_slot` fills `slot`. **No probe, path or version-source behaviour changes** |
| `crates/vertice-core/Cargo.toml` | Modify | `semver` (MIT OR Apache-2.0, dependency-light) — the only new core dependency |
| `crates/vertice-app/src/freshness/{mod,upstream,fetch,cache}.rs` | **Create** | §6, §8, §9 — the only modules naming `reqwest`; `cache.rs` the only module that writes |
| `crates/vertice-app/src/commands.rs`, `lib.rs` | Modify | §11 |
| `crates/vertice-app/Cargo.toml` | Modify | `reqwest` (features per §4), `serde_json` |
| `crates/vertice-app/tests/read_only_audit.rs` | Modify | §14 — three commands; audit widened to all `src/**`, with one scoped exception |
| `crates/vertice-app/capabilities/default.json`, `tauri.conf.json` (CSP) | **Unchanged** | Review check |
| `deny.toml` | Modify | §5 ban entry; allow-list entries **only** if §4's gate forces one, with a comment |
| `Cargo.toml`, `rust-toolchain.toml`, `.github/workflows/ci.yml` | **Unchanged** | MSRV floor does not move (V4, §4) |
| `frontend/src/bindings/` | Regenerated | 5 new, 1 modified; never hand-edited |
| `frontend/src/lib/pages/ClientsPage.svelte` | Modify | badge, five renderings (§9) |
| `frontend/src/lib/i18n/catalogs.ts` | Modify | new `clients.*` keys, `en` + `es`; versions and package names are passthrough, never localized |
| `frontend/src/lib/scanDiagnostics.ts` | **Unchanged** | §12 — pinned by a test |

## 14. Testing strategy (`strict_tdd: true` — RED first)

The two load-bearing failing tests, written before any implementation:

1. `no_upstream_slot_is_never_up_to_date` — `ClaudeCodeBundled` with any installed version and any reference map yields `Unknown`. The §6 pin; the one verdict that must be impossible to get wrong.
2. `unavailable_source_yields_unknown_for_every_subject_and_zero_issues` — the offline path, which is the likeliest real-world first run.

| Layer | What | How |
|---|---|---|
| Unit (core) | `compare`: older → `Outdated{latest}`; equal → `UpToDate`; **`0.150.0-rc.1` vs `0.150.0` → `Outdated`**; **`0.151.0-rc.1` vs `0.149.1` → `UpToDate`** (§7, asserted explicitly, never left to the parser's default); MSIX-shaped directory name → `Unknown`; empty string → `Unknown`; garbage on either side → `Unknown`, never a panic | pure, in-module, no I/O |
| Unit (core) | `evaluate` over a `MapReferenceVersions` stub: `Found`/`Unavailable`/`NoUpstream` each map to the right verdict; **zero `ScanIssue` values produced in any case** | stub only |
| Contract (core) | `ClientInstallSlot` exhaustive-match test, mirroring the `Scope`/`ClientPresenceStatus` pattern; `Freshness` has exactly three variants | `tests/model_contract.rs` |
| Integration (core) | Existing `client_installations.rs` fixtures gain `slot` assertions; **`installations`, `issues` and ordering byte-identical to today** — the tripwire that §2's promotion changed nothing about detection | `scan_for(home, Windows)` |
| Unit (app) | `upstream.rs`: each slot maps to the §6 identity; `ClaudeCodeBundled` maps to **no request** | pure table test |
| Unit (app) | Response parsing against **recorded fixture payloads** (npm `/latest`, GitHub `releases/latest`): happy path, `rust-v` prefix stripped, missing field, wrong type, oversize body, truncated JSON | no network |
| Unit (app) | Cache: TTL respected; expired-plus-fetch-failure serves stale within 7 days and not beyond; corrupt file treated as empty; the resolved path is a child of `app_data_dir()` | `tempfile`-style dev-only dir |
| Audit (app) | `read_only_audit.rs` widened: commands are exactly `["scan","rescan","freshness"]`; permissions still exactly `["core:default"]`; the 16 mutation patterns are forbidden across **all** of `src/**` **except** `freshness/cache.rs`, which is separately asserted to contain **no absolute path literal and no env read**, deriving its path only from `app_data_dir()` | source-grep, as today (V6) |
| Live (app, **not in CI**) | One `#[ignore]`d test hitting the real endpoints, to catch upstream schema drift on demand. CA-17: never in CI, never on a default `cargo test` | manual |
| Frontend (Vitest) | Five badge renderings; `incidentCount` **unchanged** when a report is all-`Outdated`; the scan renders fully before any freshness result arrives | `App.test.ts`, `scanDiagnostics.test.ts` |
| i18n | `en` and `es` complete for the new keys; version strings and package names are passthrough | `locale.test.ts` |

Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, **`cargo deny check bans licenses`** (the one that carries this change's real risk), `cargo check` at 1.88, `npm run lint && npm run check && npm run test && npm run build`, bindings in sync.

## 15. Migration / rollout

**No migration.** The only persisted artefact is the cache-plus-setting document; deleting it loses nothing and the app degrades to a live lookup. Three chained PRs, matching the proposal's slices and each independently green:

1. **Core** — `ClientInstallSlot` promotion, `model/freshness.rs`, `compare`, `evaluate`, the trait, `semver`, bindings. Zero network code. Leaves `main` with a typed capability nothing consumes, which is self-consistent: a stub source reports `Unknown` honestly.
2. **App** — `reqwest` + TLS, `deny.toml`, upstream resolution, cache, the command, the setting. **The slice carrying the entire dependency risk (§4) and the read-only audit change (§14) — review it hardest.**
3. **Frontend** — badge, five states, i18n, first-run disclosure.

Rollback is the proposal's five ordered layers. A partial rollback fails at TypeScript compile time on a missing binding, never silently at runtime.

## 16. Open questions and corrections

- [x] Codex upstream identity — `openai/codex`, `releases/latest`, ordered `name` → de-prefixed `tag_name` → `Unknown`. §6
- [x] Bundled Claude Desktop upstream — **none**; permanent `Unknown`, no request. §6, closing condition recorded
- [x] HTTP client — `reqwest`, already a direct dependency of `tauri`. §4
- [x] `deny.toml` ban — yes, with `["vertice-app", "tauri"]`. §5
- [x] Subject key — `FreshnessSubject::ClientInstallation { slot, path }`, requiring a public `ClientInstallSlot`. §2
- [x] Cache policy — `app_data_dir()`, JSON, 6 h TTL, 7-day stale ceiling, corrupt-as-empty. §8
- [x] Prerelease semantics — three states; ahead-of-stable is `UpToDate`. §7
- [x] Timeout and retries — 5 s concurrent, zero retries; `pending` is frontend-only; "gave up" *is* `Unknown`. §9
- [ ] **The TLS backend is undecided and MUST NOT be guessed (U3).** §4 specifies the three commands and the ordering rule that decide it. This is the only thing standing between this design and a fully closed dependency picture, and it cannot be closed from the repository alone.
- [ ] **Correction to the proposal.** "Explicitly NOT Modified" must **drop `client-installation-detector`**: §2 promotes `InstallSlot` to a public model type and adds a field to `ClientPresence`. Detection *behaviour* is still untouched — probes, paths, version sources, ordering and issues are byte-identical — but the capability's published shape changes and `domain-model` gains five types plus one modified type, not the "new verdict and report types" the proposal anticipated.
- [ ] **Correction to the proposal.** It states `semver` is core's only new dependency and an HTTP client is the app's; `serde_json` is a second app-side addition (already in the graph, V1) for cache and response parsing.
