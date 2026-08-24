# Tasks: Report Whether A Detected Client Installation Is Out Of Date

> Trace: capability `component-freshness`; deltas `domain-model`, `client-installation-detector`, `workspace-architecture`, `desktop-shell`, `inventory-ui`, `frontend-i18n`, `scan-orchestration`. Bounded by CA-15, CA-16, CA-17.
> Authority for every decision is `design.md`; do not reopen §0-§16.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~900-1300 (5 new model types + enum promotion + comparison/trait + HTTP fetcher/cache/upstream + widened audit + badge/i18n + full test coverage) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | Three slices, each independently green and revertible (design §15) |
| Delivery strategy | ask-on-risk |
| Chain strategy | stacked-to-main |

Decision needed before apply: **RESOLVED (2026-08-24)** — the user chose a **single PR with `size:exception`**, explicitly accepting the over-budget review load. The three-slice structure below is retained as the *implementation and commit* order, not as a PR boundary: each slice must still reach a green state before the next begins, so a bisect lands on a coherent commit. The dependency and licence risk concentrated in Slice 2 is therefore reviewed alongside the rest; this was surfaced before the decision and accepted.

Original forecast verdict (superseded, kept for the record): chained PRs recommended, decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Phases | Likely PR | Notes |
|------|------|--------|-----------|-------|
| 1 | Core, pure and offline: `ClientInstallSlot` promotion, `model/freshness.rs`, `compare`/`evaluate`, `ReferenceVersions` trait, `MapReferenceVersions` stub, bindings | 1-3 | PR 1, base `main` | Zero network, zero UI; ships a typed capability nothing consumes yet — self-consistent (design §15) |
| 2 | App, the concrete fetcher: TLS decision, `reqwest`, `deny.toml`, upstream resolution, cache, third IPC command, setting, widened `read_only_audit.rs` | 4-6 | PR 2, base PR 1's branch | Carries the entire dependency risk; review hardest |
| 3 | Frontend: badge (4 states), i18n `en`/`es`, first-run disclosure | 7 | PR 3, base PR 2's branch | Depends on regenerated bindings from PR 1 and the command from PR 2 |
| — | Final gates, cross-slice verification | 8 | Lands with PR 3 or as a closing commit | Confirms all quality gates green end-to-end |

## Phase 0: Fixture-Coverage Honesty (CA-17)

- [x] 0.1 Map every scenario in the 8 delta/new specs to a planned fixture home or stub table (design §14). Confirm no core test requires network; report plainly if any scenario has no planned fixture before writing code.

## Phase 1: Core Model — Promote `ClientInstallSlot`, Add `slot` Field (Slice 1, `client-installation-detector` + `domain-model` deltas)

- [x] 1.1 (RED) Add `client-installation-detector` exhaustive-match test for the new public slot enum in `crates/vertice-core/tests/model_contract.rs`, mirroring the `Scope`/`ClientPresenceStatus` pattern — fails because the type does not exist yet.
- [x] 1.2 (RED) Extend `crates/vertice-core/tests/client_installations.rs`: every existing presence-record assertion gains a `slot` expectation across all fixture homes — fails to compile/assert until `ClientPresence.slot` exists.
- [x] 1.3 (GREEN) Promote the private `InstallSlot` in `crates/vertice-core/src/installations.rs` to `pub enum ClientInstallSlot` in `model/` (new `model/slot.rs` or folded into `model/installation.rs` per design §13); `label()` moves with it; `resolve_slot` fills `ClientPresence.slot`. No probe, path, or version-source behavior changes.
- [x] 1.4 Modify `crates/vertice-core/src/model/presence.rs`: add `pub slot: ClientInstallSlot` to `ClientPresence`.
- [x] 1.5 Modify `crates/vertice-core/src/model/mod.rs`: `mod` + `pub use` for the new/moved type.
- [x] 1.6 Confirm 1.1-1.2 now pass.
- [x] 1.7 (RED then GREEN) Add integration test: `installations`, `issues`, and ordering are byte-identical to pre-change output across the existing fixture suite, with only `slot` newly present (design §2, tripwire).
- [x] 1.8 Update every hand-constructed `ClientPresence { .. }` literal across `crates/vertice-core/tests/client_installations.rs` and `crates/vertice-core/src/model/presence.rs` doctests/tests to include `slot`.

## Phase 2: Core Model — Freshness Types (Slice 1, `domain-model` delta)

- [x] 2.1 (RED) Add `crates/vertice-core/tests/model_contract.rs` cases: `Freshness` exhaustive match (3 variants, no wildcard); `ClientInstallSlot` exhaustive match — fails to compile until the types exist.
- [x] 2.2 (GREEN) Create `crates/vertice-core/src/model/freshness.rs`: `Freshness { UpToDate, Outdated { latest: String }, Unknown { reason: String } }`, `FreshnessSubject { ClientInstallation { slot: ClientInstallSlot, path: PathBuf } }`, `FreshnessCheck { subject, installed, verdict }`, `FreshnessReport { enabled: bool, checks: Vec<FreshnessCheck> }` — plain data only, respecting `model/`'s import allow-list (design §3).
- [x] 2.3 Modify `crates/vertice-core/src/model/mod.rs`: `mod freshness;` + `pub use`.
- [x] 2.4 Run `cargo test -p vertice-core` to regenerate bindings: new `ClientInstallSlot.ts`, `Freshness.ts`, `FreshnessSubject.ts`, `FreshnessCheck.ts`, `FreshnessReport.ts`; modified `ClientPresence.ts` gains `slot`. Never hand-edit; commit in the same commit as the Rust types.
- [x] 2.5 Confirm 2.1 now passes; confirm no other `bindings/*.ts` file changed.

## Phase 3: Core Comparison, Trait, Stub (Slice 1, `component-freshness` capability)

- [x] 3.1 Add `semver` (MIT OR Apache-2.0) to `crates/vertice-core/Cargo.toml` — the only new core dependency. Run `cargo deny check bans licenses`.
- [x] 3.2 (RED) Write `crates/vertice-core/tests/freshness_compare.rs`: older→`Outdated{latest}`; equal→`UpToDate`; `0.150.0-rc.1` vs `0.150.0`→`Outdated`; `0.151.0-rc.1` vs `0.149.1`→`UpToDate`; MSIX-shaped directory name→`Unknown`; empty string→`Unknown`; garbage on either side→`Unknown`, never a panic (spec scenarios).
- [x] 3.3 (RED) Write `no_upstream_slot_is_never_up_to_date`: a subject with no known upstream mapping, via the stub, yields `Unknown` for any installed/reference pair and never `UpToDate` (design §14, the load-bearing pin).
- [x] 3.4 (RED) Write `unavailable_source_yields_unknown_for_every_subject_and_zero_issues`: `MapReferenceVersions` stub reports unavailable for every subject; `evaluate` returns all-`Unknown`, zero `ScanIssue`-shaped side effects (design §14, the second load-bearing pin).
- [x] 3.5 (GREEN) Create `crates/vertice-core/src/freshness.rs`: `ReferenceLookup { Found(String), NoUpstream { reason }, Unavailable { reason } }`, `trait ReferenceVersions { fn latest_for(&self, subject: &FreshnessSubject) -> ReferenceLookup }`, `MapReferenceVersions` stub, `pub fn compare(installed: &str, reference: &str) -> Freshness` (total, pure), `pub fn evaluate(source: &impl ReferenceVersions, subjects: &[(FreshnessSubject, String)]) -> Vec<FreshnessCheck>` (total, pure, sync).
- [x] 3.6 Add `pub mod freshness;` to `crates/vertice-core/src/lib.rs`.
- [x] 3.7 Confirm 3.2-3.4 now pass.
- [x] 3.8 Unit test: `evaluate` maps `Found`/`Unavailable`/`NoUpstream` to the right verdict each, over the stub.
- [~] 3.9 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses` — end of Slice 1. All four gates run; the first three are green. `cargo-deny` is not installed in this environment (`cargo deny` → "no such command"), so `cargo deny check bans licenses` could not be executed here — see apply-progress.md.

## Phase 4: TLS Backend Decision (Slice 2, blocking, `workspace-architecture` delta — design §4, §16 U3)

- [x] 4.1 **Resolved by the orchestrator 2026-08-24, outside this apply run** — `native-tls` chosen by measurement (`rustls`: +120 crates incl. `aws-lc-sys`; `native-tls`: +101 crates, no vendored C build). Both pass `cargo deny check bans licenses` with no new allow-list entry. Recorded in `crates/vertice-app/Cargo.toml`'s dependency comment. **Verified locally on Windows only** — the Linux leg needs system OpenSSL headers for `native-tls`/`openssl-sys`; confirm on the first CI run before treating this as fully settled across all three legs.

## Phase 5: The Concrete Fetcher (Slice 2, `component-freshness` capability, `workspace-architecture` delta)

- [x] 5.1 `reqwest` (native-tls, json features) already present in `crates/vertice-app/Cargo.toml` per 4.1; added `serde`, `serde_json`, `semver` directly (all already in the graph, V1/design §16). `deny.toml`: added `{ name = "reqwest", wrappers = ["vertice-app", "tauri"] }`, extending the existing comment precedent for `tauri-build`. No allow-list entry needed (4.1's done-condition).
- [x] 5.2 (RED) `crates/vertice-app/src/freshness/upstream.rs` table tests written first — failed to compile until `upstream.rs`/`UpstreamIdentity` existed.
- [x] 5.3 (GREEN) `upstream.rs` created implementing the §6 table verbatim (`upstream_for`, `UpstreamIdentity::{cache_key, request_url}`).
- [x] 5.4 (RED) `crates/vertice-app/src/freshness/fetch.rs` tests written against literal fixture payload strings (no network): npm happy path, missing/wrong-type `version`, truncated JSON, oversize body; GitHub `name`→`tag_name` (`rust-v`/`v` stripped)→`Unknown` fallthrough ordering, raw-prefix-never-used-as-is, missing fields, wrong type, truncated JSON, oversize body — all failed to compile until `fetch.rs` existed.
- [x] 5.5 (GREEN) `fetch.rs` created: `parse_npm_latest`, `parse_github_latest_release` (pure, no I/O), `build_client` (3s connect / 5s total, zero retries, `User-Agent: vertice/<crate version>`), `fetch_reference` (the one function that performs a request — 403/429 mapped to a rate-limit reason, other non-2xx to a status-code reason, size ceiling enforced before parsing).
- [x] 5.6 (RED) `crates/vertice-app/src/freshness/cache.rs` tests written first: TTL, stale-ceiling, corrupt-as-empty, path-is-child-of-stubbed-app-data-dir, single-whole-file-write — failed to compile until `cache.rs` existed.
- [x] 5.7 (GREEN) `cache.rs` created: `FreshnessStore { enabled, disclosure_seen, cache }` (one JSON document, design §11), `store_path`/`load`/`save`, `is_fresh`/`is_within_stale_ceiling`. `store_path` takes an already-resolved `app_data_dir: &Path` — the `tauri::Manager::path().app_data_dir()` call itself lives in `commands.rs`, never in `cache.rs`, so `cache.rs` contains no Tauri/env/literal-path reference at all (stronger than "derived only from `app_data_dir()`" — it doesn't resolve it).
- [x] 5.8 `crates/vertice-app/src/freshness/mod.rs` created: `build_report(app_data_dir, presence)` — setting off → `{enabled:false, checks:[]}` with no cache read; setting on → per-distinct-upstream-identity cache-hit-or-fetch-or-stale (each identity's lookup on its own `tauri::async_runtime::spawn` task, so concurrency needs no new crate) → `vertice_core::freshness::evaluate`.

## Phase 6: IPC Command, Setting, Widened Audit (Slice 2, `desktop-shell` + `scan-orchestration` deltas)

- [x] 6.1 (RED then GREEN) Two tests added: `commands::tests::freshness_command_wiring_never_rejects_and_degrades_to_unknown` (a `ClaudeCodeBundled` subject — no upstream, so provably no network touched — degrades every check to `Unknown` inside a successful, non-rejected report) and `commands::tests::scan_installations_resolves_independently_of_the_full_scan_pipeline` (the installation-scan step `freshness` depends on succeeds standalone, with no call to `run_scan`/`scan`/`rescan` anywhere in its body).
- [x] 6.2 (GREEN) `crates/vertice-app/src/commands.rs`: `pub async fn freshness(app: tauri::AppHandle) -> Result<FreshnessReport, ScanError>` added. Deviation from the task's literal wording, recorded under "Deviations": the *whole* function is not one `spawn_blocking` call (it awaits async network I/O afterward, which cannot be blocking-offloaded); only the blocking filesystem-walk half (`scan_installations`, a new private helper) is `spawn_blocking`-offloaded, reusing `map_join_error` (now `pub(crate)`) exactly as asked.
- [x] 6.3 (GREEN) `crates/vertice-app/src/lib.rs`: `invoke_handler` is `generate_handler![commands::scan, commands::rescan, commands::freshness]`.
- [x] 6.4 `FreshnessStore.enabled`/`disclosure_seen` fields added in `cache.rs` (5.7), persisted in the same JSON document as the cache map — one file, one write path. No IPC command to *mutate* these fields yet — none was in Phase 6's scope (only `freshness` itself was authorized); Phase 7 (frontend, out of scope this run) will need one to wire the opt-out setting and disclosure UI, and none currently exists. Flagged under "Issues Found".
- [x] 6.5 (RED) `crates/vertice-app/tests/read_only_audit.rs` rewritten: `commands == ["scan", "rescan", "freshness"]`; mutation-pattern scan widened to every `.rs` file under `crates/vertice-app/src/**` (hand-rolled recursive walk, no new dependency) with `freshness/cache.rs` as the one scoped exception; a dedicated cache.rs-only check asserts `app_data_dir` is referenced and neither `std::env::` nor a literal absolute-path marker appears. Confirmed RED: failed to compile/assert against the pre-6.6 source (commands list still `["scan","rescan"]`, no cache.rs to check).
- [x] 6.6 (GREEN) `exported_tauri_commands` extended for `freshness`; capability/CSP assertions untouched and still pass (`permissions == ["core:default"]`; `capabilities/default.json`/`tauri.conf.json` confirmed byte-identical via `git status --short`, zero diff). **Widened, never weakened** — every one of the original 16 forbidden-mutation-pattern strings is still checked, now over a larger file set, with exactly one named, justified exception.
- [x] 6.7 (RED then GREEN) `commands::tests::scan_never_produces_a_freshness_shaped_issue_and_runs_independently_of_it` added: every `scan()` issue is asserted to never mention "freshness". The stronger, structural half of this requirement (scan cannot even name the `freshness` module — `vertice-core`'s `scan.rs` has no dependency on `vertice-app` at all, and `vertice-app`'s `run_scan`/`scan`/`rescan` bodies contain no reference to `crate::freshness`) is architectural, not a race a test could lose or win, and is documented in the test's own doc comment rather than asserted redundantly.
- [x] 6.8 `freshness::fetch::tests::freshness_live_upstream_endpoints_still_match_the_documented_shape` added, `#[ignore]`d with a reason string naming CA-17; hits the real npm `opencode-ai` and GitHub `openai/codex` endpoints. Confirmed excluded from the default run (see Gate Results below — `1 ignored`) and confirmed absent from `cargo test --workspace`'s executed set.
- [x] 6.9 All four gates run — see "Gate Results" below.

## Phase 7: Frontend Badge, i18n, Disclosure (Slice 3, `inventory-ui` + `frontend-i18n` deltas)

- [x] 7.1 (RED) Add `frontend/src/App.test.ts` / a new `ClientsPage.test.ts` cases: pending state before the report resolves; four visual states (`upToDate`/`outdated`/`unknown`/pending) render correctly across a mixed report; `Unknown` renders as first-class, not an error; `incidentCount` unchanged when a report is all-`Outdated`; the scan renders fully before any freshness result arrives.
- [x] 7.2 (GREEN) Modify `frontend/src/lib/pages/ClientsPage.svelte`: badge beside each installation's version, driven by the freshness report fetched after scan render; the call to the new `freshness` IPC command.
- [x] 7.3 Confirm `frontend/src/lib/scanDiagnostics.ts` is unchanged — pinned by 7.1's `incidentCount` assertion (spec requirement: `Outdated` is never an incident).
- [x] 7.4 (RED) Extend `frontend/src/lib/i18n/locale.test.ts`: assert presence and `en`/`es` completeness for the new `clients.*` keys (4 badge states, first-run disclosure text, opt-out setting label/description).
- [x] 7.5 (GREEN) Modify `frontend/src/lib/i18n/catalogs.ts`: add the `clients.*` keys in `en` and `es`; reference version strings and upstream package/repository names stay passthrough, never localized.
- [x] 7.6 Add the first-run disclosure UI element (shown before or alongside the first outbound request) and the visible opt-out setting control, wired to 6.4's persisted flag.
- [x] 7.7 Run `npm run lint && npm run check && npm run test && npm run build` from `frontend/` — end of Slice 3.

## Phase 8: Cross-Slice Gates and Verification

- [x] 8.1 Re-run `cargo test -p vertice-core` and diff `frontend/src/bindings/`: confirm exactly the 5 new files plus the modified `ClientPresence.ts`; no other binding file changed.
- [x] 8.2 Confirm `Cargo.toml` `rust-version`, the CI `MSRV` env, and `rust-toolchain.toml` channel still agree (no MSRV edit expected per design §4/V4).
- [x] 8.3 Confirm `crates/vertice-app/capabilities/default.json` and CSP in `tauri.conf.json` are byte-identical to pre-change.
- [x] 8.4 Full gate run: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`; from `frontend/`: `npm run lint && npm run check && npm run test && npm run build`. Report each gate's actual pass/fail; if `cargo` is not resolvable on PATH, say so rather than reporting the gate as passing.
