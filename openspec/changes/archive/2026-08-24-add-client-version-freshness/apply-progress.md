# Apply Progress: Report Whether A Detected Client Installation Is Out Of Date

> Scope of this run: Phases 0-3 only (Slice 1 — core, pure, offline). Phases 4-8 (Slice 2 TLS/fetcher/cache/IPC, Slice 3 frontend, cross-slice gates) are NOT started.

## Mode

Strict TDD — RED then GREEN for every behavioral test, honoring `tasks.md`'s explicit ordering.

## Completed Tasks

- [x] 0.1 — Fixture-coverage mapping (see "Fixture/Stub Mapping" below). No core test requires network.
- [x] 1.1-1.8 — `ClientInstallSlot` promoted to `model::ClientInstallSlot`, public, exhaustively matchable; `ClientPresence.slot` added; every hand-constructed `ClientPresence` literal (in `installations.rs` unit tests) updated; every relevant assertion in `client_installations.rs` gained a `slot` expectation; dedicated tripwire test added.
- [x] 2.1-2.5 — `model/freshness.rs` created (`Freshness`, `FreshnessSubject`, `FreshnessCheck`, `FreshnessReport`); wired into `model/mod.rs`; bindings regenerated — exactly 5 new `.ts` files + 1 modified (`ClientPresence.ts`), confirmed via `git status`.
- [x] 3.1-3.8 — `semver` added to `vertice-core/Cargo.toml`; `freshness_compare.rs` and `freshness_evaluate.rs` written RED-first (confirmed failing to compile before `src/freshness.rs` existed); `crates/vertice-core/src/freshness.rs` created with `ReferenceLookup`, `ReferenceVersions`, `MapReferenceVersions`, `compare`, `evaluate`; wired into `lib.rs`; all RED tests now GREEN.
- [~] 3.9 — Gates run; see "Gate Results" below. `cargo deny` could not be executed (tool not installed in this environment).

## TDD Cycle Evidence

| Task | RED (test written first, confirmed failing) | GREEN (implementation added, test passes) | REFACTOR |
|---|---|---|---|
| 1.1 | `client_install_slot_is_exhaustively_matchable_without_a_wildcard_arm` in `model_contract.rs` — written against `ClientInstallSlot`, which did not exist; confirmed as a compile failure (see note below) | `model/slot.rs` added; test passes | `cargo fmt` reformatted `installations.rs`/test files after mechanical edits; no logic change |
| 1.2/1.8 | `client_installations.rs` extended with `.slot` assertions on every presence-record-checking test, against a field (`ClientPresence.slot`) that did not exist yet | `presence.rs` gained `pub slot: ClientInstallSlot`; every `ClientPresence` literal in `installations.rs` updated to populate it | n/a |
| 1.7 | `slot_promotion_leaves_detection_output_unchanged_except_for_the_new_field` written against the not-yet-existing `slot`/`label()` pairing | Passes once 1.3-1.4 land: asserts exact probe-table slot order per fixture and `record.label == record.slot.label()` for every record across 15 fixture homes | n/a |
| 2.1 | `freshness_is_exhaustively_matchable_without_a_wildcard_arm` written against the not-yet-existing `Freshness` type | `model/freshness.rs` added; test passes | n/a |
| 3.2 | `freshness_compare.rs` written against the not-yet-existing `vertice_core::freshness::compare` | `src/freshness.rs::compare` added; all 8 scenarios pass, including both prerelease scenarios (`0.150.0-rc.1` vs `0.150.0` → `Outdated`; `0.151.0-rc.1` vs `0.149.1` → `UpToDate`) | n/a |
| 3.3 | `no_upstream_slot_is_never_up_to_date` in `freshness_evaluate.rs`, written against the not-yet-existing `evaluate`/`MapReferenceVersions` | `evaluate` + `MapReferenceVersions` added; passes for all four probed installed-version shapes (valid, absurdly-large, garbage, empty) | n/a |
| 3.4 | `unavailable_source_yields_unknown_for_every_subject_and_zero_issues`, same file, same not-yet-existing symbols | Passes: an empty `MapReferenceVersions` degrades every subject to `Unknown` | n/a |
| 3.8 | `evaluate_maps_each_reference_lookup_variant_to_the_right_verdict`, same file | Passes: `Found` runs `compare`, `Unavailable`/`NoUpstream` both degrade to `Unknown` | n/a |

**Note on RED verification method.** Tasks 1.1/1.2/1.7/2.1's RED state is a *compile* failure (the type/field does not exist), not a runtime assertion failure — this is the same mechanism `model_contract.rs`'s existing exhaustive-match tests use (e.g. `Scope`), and is the correct RED shape for a type that doesn't exist yet: `cargo test` cannot run at all until the type is added, which is exactly the tripwire these tests are for. Tasks 3.2/3.3/3.4/3.8 were confirmed as an actual RED run: `cargo check -p vertice-core --all-targets` failed with `error[E0583]: file not found for module 'freshness'` before `src/freshness.rs` was created, then all target tests passed once it was added (verified via `cargo test -p vertice-core --locked --test freshness_compare --test freshness_evaluate`).

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `crates/vertice-core/src/model/slot.rs` | Created | `pub enum ClientInstallSlot` (4 variants) + `label()`, promoted from `installations.rs`'s private `InstallSlot` |
| `crates/vertice-core/src/model/freshness.rs` | Created | `Freshness`, `FreshnessSubject`, `FreshnessCheck`, `FreshnessReport` — plain data, `model/`'s import allow-list respected |
| `crates/vertice-core/src/model/presence.rs` | Modified | `ClientPresence` gains `pub slot: ClientInstallSlot` |
| `crates/vertice-core/src/model/mod.rs` | Modified | `mod slot; mod freshness;` + `pub use` for both |
| `crates/vertice-core/src/freshness.rs` | Created | `ReferenceLookup`, `ReferenceVersions` trait, `MapReferenceVersions` stub, `compare` (total, pure), `evaluate` (total, pure, sync) |
| `crates/vertice-core/src/installations.rs` | Modified | `InstallSlot` removed; `ClientInstallSlot` (from `model/`) used throughout; `client()`/`version_source()` moved into an `impl ClientInstallSlot` block local to this module; every `ClientPresence` literal populates `slot`; test literals updated |
| `crates/vertice-core/src/lib.rs` | Modified | `pub mod freshness;` added |
| `crates/vertice-core/Cargo.toml` | Modified | `semver = "1"` added — the only new core dependency |
| `crates/vertice-core/tests/model_contract.rs` | Modified | Added `client_install_slot_is_exhaustively_matchable_without_a_wildcard_arm`, `freshness_is_exhaustively_matchable_without_a_wildcard_arm` |
| `crates/vertice-core/tests/client_installations.rs` | Modified | Added `.slot` assertions to every presence-record-checking test; added the slot-promotion tripwire test |
| `crates/vertice-core/tests/freshness_compare.rs` | Created | 8 scenario tests for `compare` |
| `crates/vertice-core/tests/freshness_evaluate.rs` | Created | 3 tests for `evaluate` (the two design §14 load-bearing pins + the mapping test) |
| `frontend/src/bindings/ClientInstallSlot.ts` | Created (regenerated) | Closed union, 4 variants |
| `frontend/src/bindings/Freshness.ts` | Created (regenerated) | Closed union, 3 variants |
| `frontend/src/bindings/FreshnessSubject.ts` | Created (regenerated) | One populated variant: `{ slot, path }` |
| `frontend/src/bindings/FreshnessCheck.ts` | Created (regenerated) | `{ subject, installed, verdict }` |
| `frontend/src/bindings/FreshnessReport.ts` | Created (regenerated) | `{ enabled, checks }` |
| `frontend/src/bindings/ClientPresence.ts` | Regenerated | Gains `slot: ClientInstallSlot` |
| `Cargo.lock` | Modified | `semver 1.0.28` added to the lock |

Confirmed via `git status --short frontend/src/bindings`: exactly the 5 new files plus the 1 modified file — no other binding drifted.

## Fixture/Stub Mapping (Task 0.1)

No core test in this slice requires network access — every test is either a fixture-driven integration test over the existing committed fixture tree (Phase 1), or an in-memory unit/contract test with no I/O (Phases 2-3). Concretely:

- `client-installation-detector`'s new "slot discriminator" requirement is exercised over the **existing** `crates/vertice-core/tests/fixtures/client-installations/**` fixture tree — no new fixtures needed, since the requirement is purely about the shape of already-scanned output.
- `domain-model`'s new type/binding scenarios are exercised by `model_contract.rs`, which is (per its own module doc) fixture-free and constructs values in memory only.
- `component-freshness`'s core-side requirements (three-valued verdict, total comparison, prerelease ordering, reference-source abstraction, no-upstream-never-UpToDate, degradation-to-Unknown, zero-diagnostic-channel-entries) are all exercised through `MapReferenceVersions`, the in-memory stub `ReferenceVersions` implementation that ships in core per design §10 — never a real HTTP call.
- The remaining `component-freshness` scenarios (cache policy, network failure shapes, upstream field-ordering extraction, the disclosure/opt-out UX, the live-endpoint test) belong to Phases 4-7 (Slices 2-3), out of scope for this run, and will need their own fixture/stub plan when that work starts — most explicitly the response-fixture table for `fetch.rs` (task 5.4) and the `#[ignore]`d live test (task 6.8), which CA-17 requires to never run in `cargo test --workspace`.

## Deviations from Design

None. Implementation matches `design.md` §2, §3, §7, §10 exactly:

- The subject-key decision (§2: promote `InstallSlot` to `model::ClientInstallSlot`, add `ClientPresence.slot`) was implemented as specified.
- `client()`/`version_source()` — detection-only behavior that design did not ask to promote — stayed in `installations.rs` as a separate `impl ClientInstallSlot` block, keeping `model/` limited to the identity enum and its display `label()` (which design §2 explicitly says "moves with it").
- `model/freshness.rs`'s four types (§3) match verbatim; serde uses the default externally-tagged encoding for `Freshness`/`FreshnessSubject` (no `#[serde(tag = ...)]`), matching design's framing that this is the *first* externally-tagged data-carrying enum in `model/`, not a departure into internally/adjacently-tagged encoding.
- The prerelease rule (§7) is implemented via unmodified `semver::Version` `Ord`, which already gives the correct total order without special-casing (a prerelease sorts before its own release) — both dedicated scenarios pass.
- The core↔app seam (§10) is exactly the trait/stub/free-function shape specified; `MapReferenceVersions` is Vec-based rather than `HashMap`-based (the design's comment `/* subject -> ReferenceLookup */` left the storage detail open) — this avoids requiring `Hash` on `FreshnessSubject`/`ClientInstallSlot`, which design does not ask for and which would be more surface than this slice needs.

## Issues Found

- **`cargo-deny` is not installed in this environment.** `cargo deny --version` → `error: no such command: 'deny'`. Task 3.1 and 3.9 both call for `cargo deny check bans licenses`; neither could be executed here. `semver` is MIT OR Apache-2.0 per its published metadata, which is on the workspace's existing `deny.toml` allow-list (`MIT`, `Apache-2.0`), and it added no transitive dependency of its own (checked via `Cargo.lock` — `semver 1.0.28` has zero new dependency edges), so there is no structural reason to expect this gate to fail. But it was **not run**, and that gap should be closed by whoever has `cargo-deny` available before this lands, per the phase's own instruction not to report a gate as passing without running it.
- No other issues. The design and specs were internally consistent for this slice; no defect found in the plan for Phases 0-3.

## Gate Results (end of Slice 1, task 3.9)

```
cargo fmt --all --check
```
PASS (after one `cargo fmt --all` pass to normalize mechanical multi-field literal edits — re-ran `--check` clean afterward).

```
cargo clippy --workspace --all-targets -- -D warnings
```
PASS — `vertice-core` and `vertice-app` both clean, zero warnings.

```
cargo test --workspace --locked
```
PASS — every test crate in the workspace green, including the new `freshness_compare.rs` (8/8), `freshness_evaluate.rs` (3/3), the extended `client_installations.rs` (31/31, including the new tripwire), and the extended `model_contract.rs` (12/12). `vertice-app`'s existing suite (including `read_only_audit.rs`) is unaffected — this slice touches only `vertice-core`.

```
cargo deny check bans licenses
```
**NOT RUN** — `cargo-deny` is not resolvable on this machine's PATH (`cargo deny` → "no such command: `deny`"). Reported honestly per instruction rather than claimed as passing.

## Remaining Tasks

- [ ] Phase 4 (Slice 2, blocking) — TLS backend decision (`rustls-tls` vs `native-tls`), which requires running `cargo tree`/`cargo deny check bans licenses`/`cargo check` for each candidate. Explicitly out of scope for this run per the orchestrator's instructions.
- [ ] Phase 5 — the concrete fetcher (`vertice-app/src/freshness/{upstream,fetch,cache,mod}.rs`).
- [ ] Phase 6 — IPC command, setting, widened `read_only_audit.rs`.
- [ ] Phase 7 — frontend badge, i18n, first-run disclosure.
- [ ] Phase 8 — cross-slice gates and final verification.

## Workload / PR Boundary

- Mode: single PR with `size:exception` (resolved 2026-08-24, recorded in `tasks.md`).
- Current work unit: Unit 1 (Phases 1-3) — "Core, pure and offline". Fully complete and green in isolation, matching design §15's description of a self-consistent state: a typed capability nothing yet consumes, backed by an honest stub.
- Boundary: this batch starts from a clean `feat/client-version-freshness` branch (no prior apply-progress) and ends with Phase 3 fully green. The next batch picks up at Phase 4.
- Estimated review budget impact: this slice alone is well under the 400-line PR budget on its own; the full single-PR delivery (all three slices) will exceed it, per the tasks artifact's own forecast, and the `size:exception` decision already accounts for that.

## Status

12/8-phase-total tasks-groups complete (Phases 0-3, tasks 0.1-3.9, with 3.9 partial pending `cargo-deny` availability). Ready for the next apply batch (Phase 4) once resumed — not yet ready for `sdd-verify`, since Phases 4-8 are unstarted and the design's TLS-backend decision (§4) is still open by design.

## Orchestrator gate verification (2026-08-24, post-slice-1)

The slice-1 apply run reported its gates green having run **only the Rust gates**. Two corrections, both verified by re-running the commands:

- **`cargo deny check bans licenses` DID run and passes** — result `bans ok, licenses ok`. The run reported it as unavailable ("no such command: `deny`"); the cause is environmental, not missing tooling: `CARGO_HOME` is the scoop path while `cargo-deny.exe` lives in `~/.cargo/bin`. Prefix it: `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses`. The `semver` addition is clean. This risk is CLOSED, not outstanding.
- **`npm run check` FAILED with 5 errors** and was not run by the apply phase. Task 1.8 ("update every hand-constructed `ClientPresence` literal") was completed on the Rust side but missed the **TypeScript** side: `frontend/src/App.test.ts` built five `ClientPresence` object literals without the new required `slot` field. Fixed by the orchestrator: slots assigned by label (`claudeCodeNpm` ×3, `claudeCodeBundled`, `openCodeNpm`), field ordered first to mirror the Rust struct.

**Lesson for the remaining slices**: `npm run test` (Vitest) does **not** typecheck, so it passed all 98 tests while the type error stood. Only `npm run check` catches a published-model-type change that breaks frontend consumers. Any slice touching a `ts_rs`-exported type MUST run the frontend gates, not just the Rust ones.

Verified end state of slice 1 — all gates green:

- `cargo fmt --all --check` — OK
- `cargo clippy --workspace --all-targets -- -D warnings` — OK
- `cargo test --workspace --locked` — OK (all suites pass)
- `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` — bans ok, licenses ok
- `npm run lint` — OK; `npm run check` — 212 files, 0 errors; `npm run test` — 98 passed; `npm run build` — OK

## Slice 2 (Phases 4-6) — App, the concrete fetcher, IPC command, widened audit

Scope of this run: Phases 4-6 only, per the orchestrator's instructions. Phases 7-8 (frontend, cross-slice gates) are NOT started.

### Phase 4 — TLS backend decision

**Already resolved by the orchestrator before this run started**, per the task prompt: `native-tls` chosen by measurement against `rustls` (see the dependency comment block in `crates/vertice-app/Cargo.toml`, already present on disk at the start of this run). Nothing to redo. **Important caveat, restated from the task prompt**: only the **Windows** leg is locally verified. The Linux leg needs system OpenSSL headers for `native-tls`; it has not been confirmed in this environment and must be confirmed on the first CI run. `rustls`'s predicted risk (`ring`'s non-SPDX licence) never materialized either way — `reqwest` 0.13's rustls path defaults to `aws-lc-rs`, not `ring`.

### Phase 5 — the concrete fetcher

New files, all in `crates/vertice-app/src/freshness/`:

| File | What |
|---|---|
| `upstream.rs` | `UpstreamIdentity` (`Npm{package}` / `GitHubReleases{owner,repo}`), `upstream_for(slot)` implementing design §6's table verbatim, `cache_key()`/`request_url()`. Pure, no I/O. |
| `fetch.rs` | `parse_npm_latest`, `parse_github_latest_release` (pure parsing over `&[u8]`, tested exclusively against literal fixture strings — never network), `MAX_RESPONSE_BYTES` (256 KiB, enforced before parsing), `build_client` (3s connect / 5s total, zero retries, `User-Agent: vertice/<crate version>`), `fetch_reference` (the one function that performs a live request — every failure mode degrades to `ReferenceLookup::Unavailable`, never a panic or propagated error). |
| `cache.rs` | `FreshnessStore{ enabled, disclosure_seen, cache: HashMap<String, CacheEntry> }`, `store_path(app_data_dir)`, `load`/`save` (one whole-file `fs::write`, corrupt-as-empty), `is_fresh`/`is_within_stale_ceiling` (TTL 6h, stale ceiling 7d, both as named constants). |
| `mod.rs` | `build_report(app_data_dir, presence)` — the orchestration: setting off → `{enabled:false, checks:[]}`, no cache read, no request; setting on → build the distinct set of upstream identities actually needed (never more than what's installed, never duplicated per-installation) → per-identity cache-hit-within-TTL or live fetch (each on its own `tauri::async_runtime::spawn` task — concurrent without a new crate dependency) or stale-serve-within-7-days → `vertice_core::freshness::evaluate`. |

Dependencies added to `crates/vertice-app/Cargo.toml`: `serde` (workspace, derive), `serde_json = "1"`, `semver = "1"` — all three already present in the lock via `tauri`/`vertice-core` transitively (V1, design §16); declaring them directly is what lets `fetch.rs`/`cache.rs` name them. `deny.toml` gained `{ name = "reqwest", wrappers = ["vertice-app", "tauri"] }`, with a comment extending the existing `tauri-build` precedent (`tauri` must be listed as a legitimate direct parent or the ban false-positives on the pre-existing graph). No new allow-list entry — confirmed by `cargo deny check bans licenses` (see Gate Results).

### Phase 6 — IPC command, setting, widened audit

- `crates/vertice-app/src/commands.rs`: added `pub(crate) fn map_join_error` visibility widening (was private), a new private `scan_installations()` helper (`spawn_blocking`-offloaded client-installation scan, reusing `map_join_error`), and `#[tauri::command] pub async fn freshness(app: tauri::AppHandle) -> Result<FreshnessReport, ScanError>`. **Deviation from task 6.2's literal wording**, recorded honestly: the whole `freshness` function is not itself one `spawn_blocking` call, because it awaits async network I/O (via `crate::freshness::build_report`) after the blocking scan step — blocking-offloading an async-awaiting function is not meaningful. Only the blocking half (`scan_installations`) is `spawn_blocking`-offloaded, which is the same rationale `run_scan` already uses for the same reason (filesystem walk on the blocking pool, not the async executor).
- `crates/vertice-app/src/lib.rs`: `mod freshness;` added; `invoke_handler` is `generate_handler![commands::scan, commands::rescan, commands::freshness]`.
- `crates/vertice-app/tests/read_only_audit.rs`: rewritten per design §14. `commands == ["scan", "rescan", "freshness"]`. The forbidden-mutation-pattern scan is widened from two hardcoded files to every `.rs` file under `crates/vertice-app/src/**` via a hand-rolled recursive directory walk (no new dependency — `walkdir` is already transitively present via `tauri` but was deliberately not added as a direct dependency for a one-off test helper). `freshness/cache.rs` is the one scoped exception, per design's own instruction; it gets a separate, positive check instead (`app_data_dir` referenced, no `std::env::`, no literal absolute-path marker). **Widened, never weakened**: every one of the original 16 forbidden patterns is still checked, unchanged, now over a strictly larger file set.
- A `strip_cfg_test_blocks` helper was added to the audit test itself, stripping `#[cfg(test)] mod { ... }` bodies (brace-depth counting, string-literal-aware) before pattern-matching. **Why**: without it, the widened audit produced false positives on this slice's own test scaffolding (`std::fs::create_dir_all`/`std::env::temp_dir()` used by `cache.rs`'s and `mod.rs`'s unit tests to build a scratch stand-in for `app_data_dir()`). The audit's subject is the *production* command surface (as it always was — the original two-file version simply never had test code that happened to trip it); stripping test blocks makes that intent explicit rather than accidental.

### TDD cycle evidence (Slice 2)

| Task | RED | GREEN |
|---|---|---|
| 5.2/5.3 | `upstream.rs` tests written against not-yet-existing `upstream_for`/`UpstreamIdentity` — compile failure | `upstream.rs` created; all pass |
| 5.4/5.5 | `fetch.rs` tests written against not-yet-existing `parse_npm_latest`/`parse_github_latest_release` — compile failure | `fetch.rs` created; all pass, including the size-ceiling-before-parsing and raw-prefix-never-used-as-is scenarios |
| 5.6/5.7 | `cache.rs` tests written against not-yet-existing `FreshnessStore`/`load`/`save` — compile failure | `cache.rs` created; all pass |
| 6.1 | `commands.rs` tests written against a not-yet-existing `commands::freshness`/`scan_installations` — compile failure | `commands.rs` extended; both pass, including the all-`Unknown`, zero-network `ClaudeCodeBundled` case |
| 6.5/6.6 | `read_only_audit.rs` rewritten to assert `commands == [..., "freshness"]` and to scan `freshness/cache.rs` for `app_data_dir` — fails against pre-6.2/5.7 source (command not yet exported; file not yet present) | Confirmed passing once 6.2/5.7 landed, after fixing two self-inflicted false positives (command declaration order affecting the asserted `Vec` order; test-scaffolding `create_dir_all`/`env::temp_dir()` tripping the widened scan — both fixed as described above) |
| 6.7 | `scan_never_produces_a_freshness_shaped_issue_and_runs_independently_of_it` written and immediately green (no code change needed — the guarantee is structural, this test pins the observable half) | n/a |
| 6.8 | n/a — this is itself the live test, `#[ignore]`d from the start | Confirmed excluded from the default run (`1 ignored` in the workspace test output) |

### Files changed (Slice 2)

| File | Action |
|---|---|
| `crates/vertice-app/src/freshness/upstream.rs` | Created |
| `crates/vertice-app/src/freshness/fetch.rs` | Created |
| `crates/vertice-app/src/freshness/cache.rs` | Created |
| `crates/vertice-app/src/freshness/mod.rs` | Created |
| `crates/vertice-app/src/commands.rs` | Modified — `freshness` command, `scan_installations`, `map_join_error` visibility, new tests |
| `crates/vertice-app/src/lib.rs` | Modified — `mod freshness;`, widened `invoke_handler` |
| `crates/vertice-app/Cargo.toml` | Modified — `serde`, `serde_json`, `semver` added |
| `crates/vertice-app/tests/read_only_audit.rs` | Rewritten per design §14 |
| `deny.toml` | Modified — `reqwest` ban entry with `["vertice-app", "tauri"]` wrappers |
| `Cargo.lock` | Modified — reqwest's dependency tree materialized (native-tls/schannel etc.), `serde`/`serde_json`/`semver` edges added for `vertice-app` |

### Deviations from design

- **Task 6.2's exact "spawn_blocking-offloaded" wording** could not apply to the whole `freshness` command as literally written, because the command awaits async network I/O after the blocking scan step (see above). Only the blocking half is offloaded. This is a deviation from the task's literal phrasing, not from design §11, which specifies only the command's signature and error-mapping reuse — both honoured exactly.
- **No IPC command exists yet to mutate `FreshnessStore.enabled`/`disclosure_seen`.** Design §11/§16 describe the setting and disclosure flag living in the persisted document (done, task 6.4), but neither design nor Phase 6's task list calls for a *mutation* command in this slice — the read path (`freshness` command reads `store.enabled`) is complete and tested (`disabled_setting_yields_an_empty_disabled_report...`), but nothing in this codebase can yet flip that flag from `false` back to `true` or mark the disclosure seen except by hand-editing the JSON file. This is very likely Phase 7's job (frontend wiring needs *some* way to write these flags) and is flagged here so it isn't assumed done.
- **`build_report` takes an already-scanned `presence: Option<Vec<ClientPresence>>`, not an app-handle-driven scan itself.** This was a deliberate seam choice (not explicitly mandated by design, which shows `freshness(app)` calling `scan_for` inline in its pseudocode at §1) to keep `mod.rs`'s orchestration logic testable without needing a real or mocked `tauri::AppHandle` — the actual scan-and-resolve-app-data-dir wiring lives in `commands.rs`'s `freshness` function, which is the thinnest possible layer over `build_report`. Functionally equivalent to §1's pseudocode; structured differently for testability.

### Issues found

- **`native-tls`'s Linux leg is unverified**, restated from the task prompt's own caveat — flagging again here since it's a real open risk for the next CI run, not closed by this slice.
- **No setting-mutation IPC command** (see Deviations above) — Phase 7 will need one; not assumed to exist.
- **`crate::freshness` module is private (`mod freshness;`, not `pub mod`)**, matching `mod commands;`'s existing visibility. This means task 6.8's live test had to live *inside* `fetch.rs` (same-crate access) rather than as an external `tests/freshness_live.rs` file, which would need `pub` visibility to link against `vertice_app_lib::freshness`. Functionally equivalent — `cargo test -p vertice-app --lib -- --ignored` still reaches it, and it is still excluded from the default `cargo test --workspace` run — but it is a structural placement difference from a literal reading of "a `crates/vertice-app` test" in task 6.8, worth flagging since a reviewer might expect a `tests/` file.

### Gate Results (end of Slice 2, task 6.9)

```
cargo fmt --all --check
```
PASS.

```
cargo clippy --workspace --all-targets -- -D warnings
```
PASS — after fixing two `clippy::field_reassign_with_default` findings in test setup code (`cache.rs`, `mod.rs`), both switched to struct-update syntax.

```
cargo test --workspace --locked
```
PASS — every existing suite still green; `vertice-app`'s lib tests: 36 passed, 1 ignored (the live test, 6.8), 0 failed. `read_only_audit.rs`: 1/1 passed.

```
PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses
```
PASS — `bans ok, licenses ok`. Two informational warnings, not failures: `license-not-encountered` for `BSD-2-Clause`/`ISC` (pre-existing allow-list entries not exercised by any current dependency — harmless), and `unused-wrapper` for `tauri` under the new `reqwest` ban entry on this specific check invocation (the `wrappers` list is still correct — see design §5's own reasoning for why `tauri` must be listed even if a given target/feature combination doesn't exercise that edge every time).

### Frontend gates (run from `frontend/`, even though this slice touched no frontend code — confirming no accidental drift)

```
npm run lint
```
PASS — no output, zero findings.

```
npm run check
```
PASS — 212 files, 0 errors, 0 warnings. Confirms no `ts_rs`-exported type changed in this slice (none did — Slice 2 is Rust/app-only).

```
npm run test
```
PASS — 98 passed (unchanged from Slice 1 — this slice added no frontend code).

```
npm run build
```
PASS.

`frontend/src/bindings/` diff confirmed unchanged since Slice 1: still exactly the 5 new files (`ClientInstallSlot.ts`, `Freshness.ts`, `FreshnessCheck.ts`, `FreshnessReport.ts`, `FreshnessSubject.ts`) plus the 1 modified file (`ClientPresence.ts`) — verified via `git status --short frontend/src/bindings`.

`capabilities/default.json` and `tauri.conf.json` confirmed byte-identical to pre-change via `git status --short` (zero diff, not merely "not intentionally touched").

### Status

Phases 0-6 complete and green (Slice 1 + Slice 2). Phases 7-8 (frontend, cross-slice gates) remain, per this run's explicit scope boundary. Ready for the next apply batch (Phase 7) once resumed.

## Slice 3 completion by the orchestrator (2026-08-24)

The Slice 3 agent run terminated early on an account spend limit, not a code failure. It had already landed: the `freshness_settings` / `set_freshness_settings` IPC commands, the `FreshnessSettings` binding, `frontend/src/lib/freshness.ts`, the `en`/`es` catalog entries, and a complete RED `ClientsPage.test.ts`. It stopped immediately before modifying `ClientsPage.svelte`.

Verified on resume: `read_only_audit.rs`'s command assertion was **extended to the exact five-command list**, not loosened — checked directly, since that was the standing instruction.

Completed by the orchestrator:

- **`ClientsPage.svelte`** — the badge (`data-testid="freshness-badge"`) with four states, the opt-out toggle, and the first-run disclosure. `Unknown` carries no `role="alert"`: "we could not tell" is not a failure the user caused. Disabling the toggle clears any rendered verdict as well as stopping requests.
- **Slot-keyed presence lookup.** `presenceFor` previously matched on `record.label` with `toLowerCase().includes(...)` — exactly the string matching the `client-installation-detector` delta spec forbids. It now matches on `ClientPresence.slot`, with each card declaring its slot set. This was pre-existing code, but leaving it would have made the badge depend on display copy.
- **`ClientsPageHarness.svelte`** (new, test-only). The RED test called `provideI18n(...)` outside a component and seeded `mount`'s `context` with `Symbol.for("vertice.i18n")`. Both are wrong under Svelte 5: `setContext` is init-only, and the key `createContext()` mints is internal and unreachable from a test. The harness provides the context during initialisation, mirroring `App.svelte`. **Only the mounting mechanism changed — no assertion was altered, relaxed, or removed.**

### Verified gate results — all run, all green

- `cargo fmt --all --check` — OK
- `cargo clippy --workspace --all-targets -- -D warnings` — OK
- `cargo test --workspace --locked` — 23 suites ok, 0 failures
- `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` — bans ok, licenses ok
- `npm run lint` — OK · `npm run check` — 216 files, 0 errors, 0 warnings · `npm run test` — 11 files, 106 tests · `npm run build` — OK
- `ClientsPage.test.ts` — 6/6

### Browser verification, and its limit

Served the Vite dev server and opened the AI Clients page. The setting row renders with the correct accessible name, the Spanish catalog renders correctly on locale switch, and **the console is clean** — the Tauri IPC calls fail outside the desktop shell and are caught, producing no unhandled rejection. That exercises the degradation path.

**Limit, stated plainly:** the badge itself cannot be seen outside the Tauri runtime, because it needs a real `freshness` IPC response. Its four states are covered by the six component tests asserting exact copy and the absence of an alert role, but they have **not** been observed in the running desktop app. Confirm visually with `npx --prefix frontend tauri dev` before release.

### Still open

- `native-tls`'s Linux CI leg remains unverified locally (Windows only).
- The pre-existing `inventory-ui` spec/code drift (clients table rendered on two routes) is untouched and tracked separately.

### Phase 8 verification results

- **8.1 bindings** — `frontend/src/bindings/` contains exactly the expected delta: `ClientPresence.ts` modified, plus **six** new files (`ClientInstallSlot`, `Freshness`, `FreshnessCheck`, `FreshnessReport`, `FreshnessSubject`, `FreshnessSettings`). No other binding changed. *Deviation from the task's literal wording:* it predicted five. `FreshnessSettings` is the sixth, created when Slice 3 added the settings commands — the task was written before that type existed. Count is correct; the plan's number was stale.
- **8.2 MSRV three-way agreement** — intact and untouched: `Cargo.toml` `rust-version = "1.88"`, CI `MSRV: "1.88"`, `rust-toolchain.toml` `channel = "1.97.1"` (a newer exact pin over the floor, as documented). No MSRV edit was needed, matching design §4's prediction.
- **8.3 capability and CSP** — `crates/vertice-app/capabilities/default.json` and `tauri.conf.json` are both byte-identical to pre-change: no diff at all. The grant stays `core:default` and the CSP was never relaxed.
- **7.3 `scanDiagnostics.ts`** — byte-identical to pre-change. `Outdated` never entered the incident channel.

## Defect found by the user and fixed (2026-08-24)

**Reported:** unchecking "Check for newer versions" and re-checking it left every client card stuck on the pending copy ("Comprobando..."), and it only recovered after navigating away from the AI Clients page and back.

**Cause:** the lookup ran exactly once, in the component's initialisation block. `toggleEnabled` cleared `freshness` on disable but nothing re-fetched on re-enable, so `badgeFor` sat in its pending branch — `settings.enabled === true`, `freshness === null`, `lookupFailed === false` — indefinitely. Leaving the page destroyed the component, so remounting re-ran the init block and masked the bug.

**Fix:** the lookup is extracted into `loadFreshness()`, called both from init and from `toggleEnabled` when the setting is switched back on.

**Second defect fixed alongside it, not reported but adjacent:** every lookup now carries a token, and a response whose token is stale is discarded. Without it, disabling the check *while a request was in flight* would let the late response paint a verdict after the user opted out — the same class of bug, in the direction that actually matters for the privacy posture.

**Tests (RED first, both failing before the fix):**
- `re-runs the check when the setting is switched back on, instead of staying pending` — asserts `fetchFreshness` is called twice and the verdict copy returns, with the pending copy gone. This is the user's exact report.
- `ignores an in-flight response that lands after the check is switched off` — pins the token guard.

Gates after the fix: `cargo fmt` OK, `cargo test --workspace --locked` no failures, `npm run lint` OK, `npm run check` 216 files / 0 errors, `npm run test` 11 files / **108** tests, `npm run build` OK.

## Second user-reported defect: Codex could never be validated (2026-08-24)

**Reported:** the Codex card always said the version could not be validated, even though `openai/codex` publishes releases normally.

**Root cause — mine, not GitHub's.** `MAX_RESPONSE_BYTES` was a single shared 256 KiB ceiling applied to every upstream before parsing. Measured against the real endpoints:

| Upstream | Real payload | Old ceiling | Result |
| --- | --- | --- | --- |
| npm `opencode-ai` | 2,042 B | 256 KiB | fine |
| npm `@anthropic-ai/claude-code` | 3,301 B | 256 KiB | fine |
| GitHub `openai/codex` `releases/latest` | **272,440 B** | 256 KiB | **rejected → `Unavailable` → `Unknown`** |

GitHub's `releases/latest` embeds the release's entire asset array. That release carries **160 assets**; the only fields this code reads, `name` and `tag_name`, sit before byte ~1,715. The ceiling was rejecting ~270 KiB of data never looked at, and the mapping to `openai/codex` was correct all along.

**Fix:** the ceiling is now per upstream kind — npm 64 KiB (still ~20x its real payload), GitHub 4 MiB (~15x). Simply nudging one shared number would have broken again the next time OpenAI adds a target triple. The rationale is documented at the constants: the ceiling bounds memory against a hostile endpoint, it is not a tightness contest, and a ceiling that rejects a legitimate upstream does not protect anyone — it just reports `Unknown` forever, which is the exact failure this feature exists to remove.

**Tests:** `a_github_release_with_a_large_asset_array_still_yields_its_version` builds a >256 KiB release payload with a 160-entry asset array and asserts the version still resolves; it asserts its own fixture exceeds the old ceiling, so it cannot silently stop being a regression test. The two oversize tests were updated to the new per-kind messages and still pin that an over-ceiling body is rejected before parsing.

### The systemic finding, which matters more than the bug

**An existing test already covered this and could not catch it.** `freshness_live_upstream_endpoints_still_match_the_documented_shape` performs the real lookup against both registries and asserts `Found` for `openai/codex` — but it is `#[ignore]`d so it never runs in CI, correctly, since CA-17 forbids network access in the test suite. Proven, not assumed: temporarily restoring the 256 KiB ceiling makes that test fail with `Unavailable { reason: "...exceeds the ... size ceiling" }` — the user's exact symptom.

So this class of defect — upstream payload shape, size, or schema drift — has **no automated detector at all**, by design. The mitigation is procedural, and should be treated as a release step rather than a nice-to-have:

```
cargo test -p vertice-app --lib -- --ignored freshness_live
```

Run it before every release and after any change to the upstream table or the parsers. It is the only thing standing between a registry changing shape and every user silently seeing `Unknown`.

### A related weakness worth naming

The user saw only the word "Unknown". The `reason` string ("GitHub response exceeds the … ceiling") was carried in the report the whole time but is **not surfaced in the UI**, so a diagnosable failure looked like an inexplicable one. Not fixed here — it is a UI change beyond this defect's scope — but a badge tooltip or diagnostics row carrying `reason` would have turned this bug report into a self-diagnosis.

Gates after the fix: `cargo fmt --all --check` OK · `cargo clippy -D warnings` OK · `cargo test --workspace --locked` no failures (42 in `vertice-app`) · `cargo deny check bans licenses` bans ok, licenses ok · `npm run check` 216 files / 0 errors · `npm run test` 109 tests · live opt-in test passes against the real registries.
