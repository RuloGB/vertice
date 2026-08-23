# Tasks: Report Client Presence As A Typed Status, Not A Warning

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~550-700 (core model + detector rewrite + full test rewrites + frontend seam/components + i18n + bindings) |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (type contract must land with all consumers together) |
| Delivery strategy | exception-ok |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Full change (core → bindings → frontend → i18n → gate) | PR 1 | `size:exception` pre-accepted; splitting leaves `main` with a typed field nothing reads (design §11) |

## Phase 0: Fixture-Coverage Honesty (CA-17)

- [x] 0.1 Map every scenario in the 4 delta specs to one of the 14 existing fixture homes in `crates/vertice-core/tests/fixtures/client-installations/`. Confirm no new home is needed; report plainly if any scenario has none.
      **Confirmed**: `ls` on the fixture dir returned exactly the 14 homes named in design §7 (`isolation`, `legacy`, `no-version-key`, `non-claude-packages`, `nothing`, `npm-dir-no-package-json`, `opencode-npm`, `package-json-empty`, `package-json-unreadable`, `packaged`, `packaged-and-legacy`, `packaged-empty`, `packages-unreadable`, `two-packages`, `version-not-a-string`). No new home was needed; no fixture file was added, changed, or deleted.

## Phase 1: Core Model (T2 seam, domain-model delta)

- [x] 1.1 Create `crates/vertice-core/src/model/presence.rs`: `ClientPresence { label, probed_paths, status, installations }`, `ClientPresenceStatus { Detected, NotDetected }`, deriving `Serialize`/`Deserialize`/`TS`; respect `model/`'s import allow-list (no I/O, no clock).
- [x] 1.2 Modify `model/mod.rs` (`mod presence;` + `pub use`) and `model/report.rs` (`client_presence: Option<Vec<ClientPresence>>`); `installations` field unchanged.
- [x] 1.3 Run `cargo test -p vertice-core` to regenerate bindings: new `frontend/src/bindings/ClientPresence.ts`, `ClientPresenceStatus.ts`; modified `ScanReport.ts`. Never hand-edit; commit with the Rust types.
- [x] 1.4 (RED) Add `nothing_yields_three_not_detected_records_and_zero_issues` to `tests/client_installations.rs` (3 `NotDetected`, 0 installations, 0 issues on `nothing`) — fails to assert, not to compile (CA-11 pin).
- [x] 1.5 (RED) Add `bundled_slot_record_carries_every_coexisting_installation` over `packaged-and-legacy` — one record, `installations.len() == 3` (legacy(1) + packaged(2 versions); CA-7 pin).

## Phase 2: Detector (`client-installation-detector` delta)

- [x] 2.1 (GREEN) Modify `resolve_slot` in `installations.rs` to return a `ClientPresence` record per slot; delete the not-detected `Warning` push. `InstallSlot::label()` was NOT deleted — still builds 4 `Error` reasons and now fills `ClientPresence.label` too.
- [x] 2.2 (GREEN) Add private `flatten_presence(&Option<Vec<ClientPresence>>) -> Vec<ClientInstallation>` (`None`→`[]`, `Some`→concat in record order); this is the ONLY producer of `installations` — `resolve_slot` never pushes to it directly.
- [x] 2.3 Confirm 1.4/1.5 now pass.
- [x] 2.4 Unit test: `flatten_presence` order/empty behavior, in-module, no I/O.
- [x] 2.5 Rewrite `tests/client_installations.rs` per design §7 table (all 14 fixtures) to assert records, not reason strings.
- [x] 2.6 Add invariant test: flattened records' installations == `ScanReport.installations`, element-for-element, on `packaged-and-legacy` and `isolation`.
- [x] 2.7 Extend the existing `Unsupported` test with `client_presence.is_none()`; arm otherwise byte-identical (design §4 tripwire).

## Phase 3: Orchestrator

- [x] 3.1 (RED) Rewrite `scan.rs:129-154` (`missing_roots_and_clients_are_visible_diagnostics`): replace the `reason.ends_with("not detected")` count==3 assertion with presence-record assertions.
- [x] 3.2 (GREEN) Modify `scan.rs` to carry `client_presence` into `ScanReport`.

## Phase 4: Binding Regeneration Checkpoint

- [x] 4.1 Re-run `cargo test -p vertice-core`; diff `ClientPresence.ts`, `ClientPresenceStatus.ts`, `ScanReport.ts`; confirm every other `bindings/*.ts` is unchanged.
      **Confirmed**: only `ClientPresence.ts` (new), `ClientPresenceStatus.ts` (new), and `ScanReport.ts` (added `clientPresence: Array<ClientPresence> | null`) changed. No other `bindings/*.ts` file was touched.

## Phase 5: Frontend Seam

- [x] 5.1 (RED) Rewrite `scanDiagnostics.test.ts:6-10`: drop the 3 hardcoded reason strings; add `incidentCount` cases (0 for NotDetected+notFound; non-zero for broken-`package.json` Error) and a case pinning `isUnavailableRootWarning`'s exact-string match plus a one-word-drift non-match.
- [x] 5.2 (GREEN) Modify `scanDiagnostics.ts`: delete `MISSING_CLIENT_REASONS`, `isMissingClientIssue`; `incidentCount` = `recoverableIssues.length` only. Keep `isUnavailableRootWarning` unchanged — load-bearing (V5), out of scope to remove.

## Phase 6: Components

- [x] 6.1 (RED) Update the diagnostics-panel test coverage in `App.test.ts` (`ScanIssueList` has no dedicated test file; it is exercised via `App scan route` tests): remove missing-client-section assertions.
- [x] 6.2 (GREEN) Modify `ScanIssueList.svelte`: drop the missing-client section.
- [x] 6.3 (RED) Add `App.test.ts` cases: 3-row table incl. `NotDetected`; two versions in one row with path `title` tooltips; `clientPresence: null` → unsupported-platform copy; rescan button invokes `rescan` and disables while loading.
- [x] 6.4 (GREEN) Modify `ScanPage.svelte`: replace "Detected installations" panel with always-visible supported-clients table (client/status/version(s), no path column, path as `title` tooltip); `notFound` roots lose `text-danger`; add `onReload` prop + rescan button mirroring `ComponentToolbar.svelte:28-35`, reusing `toolbar.reload`/`toolbar.reloading`, disabled while `status === "loading"`.
- [x] 6.5 (GREEN) Modify `App.svelte`: thread `onReload={() => void runScan("reload")}` into `ScanPage` (mirror `App.svelte:111,122`); update `App.test.ts` fixtures with `clientPresence`.
      **Deviation noted**: the pre-existing test "shows the incident indicator on both pages for a not-found root with zero issues (correctness-critical)" pinned the OLD behavior (a `notFound` root alone lights the indicator). The `inventory-ui` delta's "Incident Indicator on List Pages" requirement explicitly reverses this ("Previously: also fired on any `rootsScanned` entry with `status === 'notFound'`, with zero issues"). Updated the test to assert NO indicator for that case, matching the new spec's "No indicator from a not-found root alone" scenario.

## Phase 7: i18n

- [x] 7.1 (RED) Update `locale.test.ts:95-96`: remove `diagnostics.missingClient`, `scan.installationsTitle`, `scan.installationsEmpty` assertions; add presence-completeness assertions for the 5 new keys in `en`/`es`.
- [x] 7.2 (GREEN) Modify `catalogs.ts`: add `scan.clientsTitle`, `scan.clientDetected`, `scan.clientNotDetected`, `scan.clientVersionUnavailable`, `scan.clientsUnsupportedPlatform` (en+es); remove the 3 retired keys. Slot labels stay untranslated.

## Phase 8: Gates

- [x] 8.1 Read-only check (CA-16): grep source + tests for `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, `remove_*` — confirm none introduced. Only matches are inside `read_only_audit.rs`'s own pattern list (the audit test itself), which passed.
- [x] 8.2 Confirm Phase 0's fixture-coverage mapping still holds; no fixture added/changed/deleted.
- [x] 8.3 Run gates: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked`; `cargo deny check bans licenses`; from `frontend/` (never `frontend/src/`, to avoid a stray `node_modules`): `npm run lint && npm run check && npm run test && npm run build`. All gates ran and passed (see apply-progress for verbatim output notes). `cargo` resolved via `PATH="$PATH:/c/Users/Raul/.cargo/bin"`.
