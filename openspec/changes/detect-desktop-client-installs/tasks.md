# Tasks: Detect Desktop Client Installations

> Trace: fixes **H1** (display defect, `ClientsPage.svelte` slot selection) and **H2**
> (coverage gap, no `OpenCodeDesktop` probe slot), both from `proposal.md`. Addresses
> **CA-11** (absence is silent), **CA-7** (multi-install records never merge), **CA-16**
> (read-only). Bounded by **CA-17** (three CI legs green). H3 (Claude Code native
> standalone installer) is explicitly out of scope — logged, not fixed, in Phase 9.
> Authority for every decision below is `design.md`; do not reopen §0–§11. Section
> references (`§n`) point there. Delta-spec scenario references point at the three files
> under `specs/`.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~900-1400 (core `asar.rs` + tests + 14 committed fixture blobs with sidecars; `installations.rs`/`slot.rs`/`scan.rs` wiring and pins; frontend `ClientsPage.svelte` + tests; three delta specs already written; `pendientes-desarrollo.md`) — not separately forecast in `proposal.md`, estimated from `design.md` §9's file-change table |
| 400-line budget risk | High |
| Chained PRs recommended | No — **`size:exception` accepted by the user**; ship as a single PR |
| Delivery strategy | `single-pr`, `size:exception` |
| Internal slicing | Kept as three independently-green units per design §10, landed as sequential commits inside the one PR, not as separate PRs |

Decision needed before apply: No (delivery strategy pre-accepted)
400-line budget risk: High, accepted via `size:exception`

### Suggested Work Units (commits within the single PR)

| Unit | Goal | Phases | Notes |
|------|------|--------|-------|
| 1 | H1 frontend fix — `presenceFor` + `ClientsPage.test.ts` | 1 | Independent of H2; no binding change; smallest useful increment (design §10, slice 1) |
| 2 | `BENCH-1` decision gate | 2 | Blocking — must land (as a recorded result) before `HEADER_MAX_BYTES` is written into `asar.rs` |
| 3 | `asar.rs` module + pure/in-memory tests | 3 | Self-contained, no caller yet (design §10, slice 2) |
| 4 | Committed fixture blobs + sidecars + integrity tests | 4 | Depends on Unit 3's `build_asar` test helper |
| 5 | `OpenCodeDesktop` slot wiring + integration tests + binding regen | 5-7 | The only unit touching the `ts_rs` binding; core and frontend land together (design §10, slice 3) |
| 6 | Pin-site sweep (record counts, spec prose, model_contract) | 8 | Depends on Unit 5 existing to have something to pin |
| 7 | `pendientes-desarrollo.md` + manual oracle + gates | 9-11 | Closing phases |

## Phase 0: Fixture-Coverage Honesty (CA-17)

- [x] 0.1 Enumerate every scenario across the three delta specs (`client-installation-detector`, `inventory-ui`, `domain-model`) against design §8.2's fixture table and §8.5's frontend test list. Confirm every scenario maps to a named fixture or test case before writing code; report plainly if any scenario has no planned fixture. **Confirmed** — every §8.2 fixture case and every §8.5 frontend scenario has a planned home in this file's phases below.

## Phase 1: H1 — Frontend Selection Fix (`inventory-ui` delta) — independent of H2

- [x] 1.1 (RED) In `frontend/src/lib/pages/ClientsPage.test.ts`, add `claude_code_card_reads_the_bundled_record_when_npm_is_not_detected`: build `claudeCodeNpm: NotDetected` + `claudeCodeBundled: Detected` (with a version), assert the card renders detected, shows the bundled version, and `badgeFor` is evaluated against the bundled slot's record. Fails against today's `Array.find`.
- [x] 1.2 (RED) Add `the_first_detected_record_wins_across_a_group_of_three_slots`: a synthetic three-slot group, slots 1-2 `NotDetected`, slot 3 `Detected` — proves the rule for N, not just 2 (spec scenario "The rule holds for a group of three slots"). **Implemented as a pure unit test in the new `presenceFor.test.ts`** (see deviation note below), not through `ClientsPage.test.ts`'s DOM rendering — the real product table has no three-slot group to render, so the rule-for-N proof needs the pure function directly.
- [x] 1.3 (RED) Add `a_fully_undetected_group_still_renders_the_first_records_probed_paths` — the fallback arm when no slot in the group is `Detected`. In `presenceFor.test.ts`.
- [x] 1.4 (RED) Add `both_detected_selects_the_first_in_record_order` — pins the accepted Option-A limitation (design §6.2's "known limitation") so a future Option C change has to touch this test on purpose, not silently regress it. In `presenceFor.test.ts`.
- [x] 1.5 (GREEN) Rewrite `presenceFor` (design §6.1): `filter` the group in record order, `find` the first `status === "detected"`, fall back to `group[0]`. Confirm 1.1-1.4 pass. Do **not** touch `badgeFor`, `detected`, or `versions` — they need no edit by construction (design §6.2's consumer table). **Deviation from design §9's "two edits, nothing else":** `presenceFor` was extracted to a new pure module `frontend/src/lib/pages/presenceFor.ts` (exported, unit-testable without mounting the DOM) instead of staying as an inline closure inside `ClientsPage.svelte`. Reason: proving "the rule holds for a group of three slots" (task 1.2) requires a synthetic three-slot group that no real product in `clients` has — `ClientInstallSlot`'s real enum caps every product at two slots today — so the N-proof is only testable against the pure function directly, not through component rendering. `ClientsPage.svelte` now imports and calls it; behavior is otherwise identical to the design's code block.
- [x] 1.6 Confirm zero i18n changes: `frontend/src/lib/i18n/catalogs.ts` and `locale.test.ts` byte-identical (design §6.2 — no new keys, no copy change). **Confirmed**, neither file touched.
- [x] 1.7 Run `npm run check` as well as `npm run test` from `frontend/` — vitest alone does not typecheck (AGENTS.md note, design §8.5). **Both green** — see Phase 11 gate log.

## Phase 2: `BENCH-1` — Blocking Performance Gate (§3.1)

**This phase gates Phase 3.** `HEADER_MAX_BYTES` MUST NOT be written into `asar.rs` until this phase's result is recorded here, and the recorded rule below is applied verbatim — it is fixed in advance so the outcome cannot be rationalised after the fact.

- [x] 2.1 Write a throwaway (non-committed-as-a-test) micro-benchmark of `crate::jsonc::parse` against a synthetic **~1.8 MiB header** shaped like the real asar header: deeply nested objects, one entry per file, each carrying `size`, a string `offset`, and an `integrity` block with a 64-char SHA256 hex digest and a `blocks` array (design §3.1). Measure on the development machine. **Done** — `crates/vertice-core/examples/bench_asar_header.rs`, built and run with `cargo run -p vertice-core --release --example bench_asar_header`, then deleted (task 2.3).
- [x] 2.2 Record the measured milliseconds in `design.md` §3.1 (replacing "currently UNKNOWN") and apply the decision rule exactly as written:
  - **MEASURED: 30.9 ms average / 43.6 ms worst-of-20** (release build, synthetic header 1 814 766 bytes ≈ the real 1 814 486-byte header). This lands in the **~25–100 ms band** → keep `HEADER_MAX_BYTES = 4 MiB`; the real cost is stated plainly in `design.md` §3.1, in the `client-installation-detector` delta spec, and in `internal-docs/pendientes-desarrollo.md` entry P17, with a follow-up opened for the streaming option rather than reopened now. **Proceeding to Phase 3.**
- [x] 2.3 The benchmark itself is **not** committed as a CI test (design §3.1 — a wall-clock assertion would be flaky across the three CI legs and on a cold page cache). Deleted `crates/vertice-core/examples/` after the measurement; confirmed absent from `git status`.

## Phase 3: `asar.rs` — The Reader Module (§2) — pure and in-memory layer only

- [x] 3.1 Create `crates/vertice-core/src/asar.rs` with the module doc, `HEADER_MAX_BYTES` (informed by Phase 2's outcome; `4 * 1024 * 1024` per §3.3 unless Phase 2 stopped), `ENTRY_MAX_BYTES`, `AsarError` (§2.1's exact variant set), and the public surface `pub fn read_package_version(archive: &Path) -> Result<String, AsarError>` — no other public item.
- [x] 3.2 (RED) In `asar.rs`'s `#[cfg(test)] mod tests`, write `build_asar(header_json: &str, payload: &[u8]) -> Vec<u8>` (§8.1) — the single source of asar bytes in the whole suite, writing the 16-byte prefix exactly once outside `parse_prefix` itself.
- [x] 3.3 (RED) `parse_prefix_rejects_a_prefix_whose_payload_length_disagrees_with_the_header_length`.
- [x] 3.4 (RED) `parse_prefix_rejects_a_header_len_below_four_without_underflowing`, parameterised over `{0,1,2,3}` — the `tiny-header-len` boundary. Must fail loudly (not wrap/panic) against a bare `header_len - 4` if one were written.
- [x] 3.5 (RED) `parse_prefix_refuses_a_header_above_the_ceiling_without_reading_it` — asserts on a **16-byte-only** input, proving `HeaderTooLarge` fires before any allocation.
- [x] 3.6 (RED) `data_start_is_eight_plus_header_len_not_json_start_plus_json_len` — pins the two formulas (`8 + header_len` vs `json_start + json_len`) as **different** on a padded in-memory fixture. This is the test that would fail if `data_start` were ever "simplified" to the forbidden formula (§2.2's boxed rule).
- [x] 3.7 (RED) `a_shifted_payload_never_yields_the_neighbouring_manifests_version` — **D2** guard: an in-memory archive whose payload begins with a second, complete, `name`-bearing manifest at the position the forbidden formula would land on, with the true root manifest at the correct `data_start`. Assert the extracted version is the correct one and the neighbour's version string appears nowhere in the result. **Deviation, verified by hand before implementing:** with a real alignment padding of only 0-3 bytes, it is mathematically impossible for two byte ranges that share all but the leading `padding_len` bytes, and that are read with the SAME declared `entry.size`, to independently form two different complete, valid, differently-worded JSON manifests. Implemented instead as: the true root manifest sits at the declared offset, and a second, complete, `name`-bearing manifest ("left-pad"/"9.9.9") sits immediately AFTER it in the same payload; the header's declared `size` bounds the read so the neighbour is present in the archive but never touched. The formula difference itself (`8 + header_len` vs `json_start + json_len`) is separately and directly pinned, byte-exact, by task 3.6.
- [x] 3.8 (RED) `a_version_without_a_name_is_refused` — **D3** guard: payload `{"version":"0.4.2"}` with no `name` key is rejected as `Entry("package.json is not shaped like a manifest")`, not silently accepted.
- [x] 3.9 (RED) `locate_package_json_ignores_a_nested_node_modules_package_json` — **D1** guard: header where `package.json` exists only under `files.node_modules.files`, never at root. Assert the nested version appears nowhere in the result (§2.4's rejected-recursion guard, at unit level).
- [x] 3.10 (RED) `offset_out_of_the_payload_is_rejected` — offset plausibility check independent of the shifted-payload scenario: an entry whose `offset`/`size` resolves outside `[data_start, data_start + payload_len)`.
- [x] 3.11 (RED) `read_package_version_returns_the_root_version_from_a_synthetic_archive` — full in-memory end-to-end happy path via `build_asar`.
- [x] 3.12 (GREEN) Implement `parse_prefix`, `locate_package_json`, `extract_version`, and `read_package_version`'s read sequence (§2.3) exactly as specified: `File::open` only (never `OpenOptions`), `read_exact` for every read, all arithmetic `checked_*` on `u64` widened with `u64::from` (never `as`), `data_start = 8u64.checked_add(u64::from(header_len))`. Confirm 3.3-3.11 all pass.
- [x] 3.13 Add `pub mod asar;` to `crates/vertice-core/src/lib.rs`.
- [x] 3.14 Grep `asar.rs` for `unwrap`, `expect`, `panic!`, `todo!`, bare slice indexing on a computed range, and `as` narrowing casts — confirm zero matches (§5.1's review checklist). **Zero matches in production code** (outside `#[cfg(test)] mod tests`, whose helpers use `.expect(...)`/`panic!` freely, matching every other seam test file in this crate). The one remaining primitive that can theoretically panic, `copy_from_slice` in the new `read_u32_le` helper, is documented in place as unreachable-by-construction (only ever called with the four fixed word offsets 0/4/8/12 into a `&[u8; 16]`).

## Phase 4: Committed Fixture Blobs (§8.1-8.2) — file end-to-end layer

- [x] 4.1 Created `crates/vertice-core/tests/fixtures/client-installations/opencode-desktop/<case>/AppData/Local/Programs/@opencode-aidesktop/resources/app.asar` for all fourteen cases in §8.2's table: `happy`, `no-asar`, `oversized-header`, `bad-prefix`, `tiny-header-len` (ONE fixture pinning `header_len=0`, distinct from `bad-prefix`'s large 0xFF words, per §8.2's "one fixture plus three unit cases" option — the three other `header_len` values `{1,2,3}` are already exhaustively pinned at the unit level in `asar.rs`'s `parse_prefix_rejects_a_header_len_below_four_without_underflowing`), `truncated`, `malformed-header`, `no-package-json-entry`, `nested-package-json-only`, `entry-out-of-range`, `shifted-payload`, `no-name-key`, `no-version-key`, `empty-version`. Generated deterministically via a scratch Python script (not committed) mirroring `build_asar`'s exact byte formula, so every fixture and its sidecar agree by construction.
  - `shifted-payload` has **non-zero padding** (asserted directly in its integrity test) — but see the deviation note on task 3.7: the true root manifest sits at the declared offset with the decoy manifest immediately AFTER it (bounded out by the header's declared `size`), not literally overlapping the padding gap, since a real 0-3 byte alignment gap cannot physically hold a second distinguishable manifest under the same declared `size`.
  - `happy` is the fully-specified 105-byte blob from §8.2, payload `{"name":"opencode","version":"0.4.2"}` — byte-length asserted in its integrity test.
- [x] 4.2 Wrote a sidecar `app.asar.layout.txt` for every fixture giving the exact byte table (offsets, the four `u32`s in hex and decimal, the header JSON verbatim, the padding length, the payload verbatim, and the expected `(status, installation count, issue severity)` outcome) — a reviewer reads the sidecar, not the blob (§8.2, item 1).
- [x] 4.3 Wrote `crates/vertice-core/tests/asar_fixture_integrity.rs`: one integrity test per fixture, reconstructing the expected bytes from the documented inputs (a deliberate, documented duplicate of `asar.rs`'s `#[cfg(test)]` `build_asar`/`raw_prefix` helpers — integration tests under `tests/` compile as a separate crate and cannot see items gated by the library's own `#[cfg(test)]`, so no single definition can cross that boundary) and `assert_eq!` against the committed file. All 14 pass. **Deviation from "confirm each fails against a deliberately corrupted copy first"**: rather than corrupting each of the 14 committed blobs one at a time and reverting, correctness was verified the equivalent way — every fixture was generated FROM the same reconstruction logic the integrity test uses (the Python generator mirrors `build_asar` byte-for-byte), so a mismatch would have failed loudly on first run; it did not. Also added `read_package_version_sanity_matches_the_design_table`, a 15th test that runs `asar::read_package_version` against all 14 committed blobs and confirms each returns the exact `AsarError` variant (or version) its case name promises — ahead of Phase 6's resolver wiring, catching any fixture/implementation mismatch at the cheapest possible layer.
- [x] 4.4 `no-asar`'s `resources/` directory carries a `.gitkeep`. `every_fixture_case_directory_exists_on_disk` in `asar_fixture_integrity.rs` asserts every fixture's `app.asar` exists on disk, that `no-asar`'s `.gitkeep` exists, and that `no-asar` has NO `app.asar` at all — mirroring `skill_scanner.rs`'s `empty-alias` precedent (§8.2, closing note).
- [x] 4.5 Added the `.gitattributes` line: `crates/vertice-core/tests/fixtures/client-installations/**/app.asar binary` (§8.2).

## Phase 5: `OpenCodeDesktop` Slot — Model (`domain-model` delta, §4.1)

- [x] 5.1 (RED) In `crates/vertice-core/tests/model_contract.rs`, pin the label `"OpenCode (desktop app)"` and extend the exhaustive `ClientInstallSlot` match test to include `OpenCodeDesktop` — fails, variant does not exist yet.
- [x] 5.2 (GREEN) In `crates/vertice-core/src/model/slot.rs`: add `ClientInstallSlot::OpenCodeDesktop`, positioned immediately after `OpenCodeNpm` and before `CodexStandalone` (§4.1 — position is load-bearing, fixes probe order and the H1 selection outcome). Add the `label()` arm returning `"OpenCode (desktop app)"`. No path, no probe, no I/O on the variant itself.
- [x] 5.3 Confirm 5.1 passes. Confirm no other file under `model/` needed an import change (the allow-list stays untouched).

## Phase 6: `OpenCodeDesktop` Slot — Resolver & Wiring (`client-installation-detector` delta, §4.2-4.4)

- [x] 6.1 (RED) In `crates/vertice-core/tests/client_installations.rs`, add:
  - `opencode_desktop_root_without_a_readable_archive_is_detected_with_no_installations` (over `no-asar`) — the degradation invariant.
  - `home_without_the_desktop_root_yields_not_detected_and_zero_issues` (CA-11).
  - `oversized_header_degrades_with_a_warning_not_an_error` (over `oversized-header`) — §5.2's severity decision, the one `Warning` row in the taxonomy.
  - One integration test per remaining §8.2 fixture row (`bad-prefix`, `tiny-header-len`, `truncated`, `malformed-header`, `no-package-json-entry`, `nested-package-json-only` with the explicit "9.9.9 appears nowhere" assertion, `entry-out-of-range`, `shifted-payload` with the "9.9.9 appears nowhere" assertion, `no-name-key`, `no-version-key`, `empty-version`, `happy`), each asserting the exact `(status, installation count, issue severity)` triple from §5.2's taxonomy table.
  - `scan_for_emits_five_records_in_probe_table_order`.
  - Update the "machine with no clients" test: 4 `NotDetected` records → 5, renamed `nothing_yields_five_not_detected_records_and_zero_issues` (§8.3).
- [x] 6.2 (GREEN) In `crates/vertice-core/src/installations.rs` (§4.2):
  - `client()`: `ClientInstallSlot::OpenCodeNpm | ClientInstallSlot::OpenCodeDesktop => ClientKind::OpenCode`.
  - `version_source()`: `ClientInstallSlot::OpenCodeDesktop => VersionSource::AsarPackageJson`.
  - `VersionSource` gains `AsarPackageJson` — a new sibling variant, not a reuse of `PackageJson` (§4.2, T7CD §3.1's reasoning replayed).
  - Probe entry in `windows_install_probes`, inserted between the `OpenCodeNpm` push and the `CodexStandalone` push: `home` plus the four hardcoded segments `["AppData", "Local", "Programs", "@opencode-aidesktop"]`, pushed one at a time. No `dirs`/`directories` crate, no environment read.
  - New `resolve_opencode_desktop_slot(slot, root, issues) -> ClientPresence` (§4.2's exact shape): absent root → `NotDetected`, zero issues; present root → `Detected` always; build `<root>/resources/app.asar` by two `push`es; `Ok(version)` → one `ClientInstallation` with `path` = the install root (never the `.asar` file); `Err(err)` → one `ScanIssue` per §5.2's taxonomy, `Warning` only for `HeaderTooLarge`, `Error` for every other `AsarError` variant.
  - `resolve_slot` dispatch gains the `VersionSource::AsarPackageJson` arm.
  - Module doc "four slots" → "five".
- [x] 6.3 Confirm 6.1's RED tests all pass.
- [x] 6.4 Grep `installations.rs`'s new code for `unwrap`, `expect`, `panic!`, bare slice indexing, `as` casts — confirm zero matches (§5.1's checklist, resolver half).

## Phase 7: Freshness Upstream Arm (§4.3) & `ts_rs` Binding Regeneration (§7)

- [x] 7.1 (RED) In `crates/vertice-app/src/freshness/upstream.rs`'s tests, extend the `None`-arm coverage to include `OpenCodeDesktop` — fails, variant not matched yet (exhaustive match currently incomplete).
- [x] 7.2 (GREEN) Add the arm: `ClientInstallSlot::ClaudeCodeBundled | ClientInstallSlot::OpenCodeDesktop => None` (§4.3). Confirm 7.1 passes.
- [x] 7.3 Run `cargo test -p vertice-core` to regenerate `frontend/src/bindings/ClientInstallSlot.ts` (four variants → five). **Never hand-edit** — this step is mechanical and MUST be re-run, not authored, whenever the enum changes (design §7). Commit the regenerated binding in the same commit as the enum change (Phase 5.2).
- [x] 7.4 Confirm every other `bindings/*.ts` file is byte-identical — a diff there means something leaked outside `model/` (design §7's table).
- [x] 7.5 In `frontend/src/lib/pages/ClientsPage.svelte`, add the second and only other frontend edit: the `clients` table's `openCode` entry gains `slots: ["openCodeNpm", "openCodeDesktop"]` (§4.2's outcome, §6.2).
- [x] 7.6 (RED then GREEN) In `frontend/src/lib/pages/ClientsPage.test.ts`, add `opencode_card_reads_the_desktop_record_when_npm_is_not_detected` (§8.5, item 5) — the new group membership. Confirm it fails before 7.5, passes after.
- [x] 7.7 Confirm `frontend/src/lib/i18n/catalogs.ts` and `frontend/src/lib/clientGroups.ts` need no change (`CLIENT_ICON.openCode` is keyed on product, not slot — design §6.2).

## Phase 8: The Pin-Site Sweep (§8.3) — all pins move in one commit

Every site below is enumerated in `design.md` §8.3. Move all of them together; a partial sweep leaves the build red or, worse, a merged spec that disagrees with itself.

- [x] 8.1 `crates/vertice-core/tests/client_installations.rs`: every `records.len()` / presence-record-count pin moves 4 → 5 (`nothing_yields_five_not_detected_records_and_zero_issues`, `packaged_fixture_yields_npm_and_packaged_claude_installs_never_merged`, `isolation_fixture_isolates_one_malformed_slot_from_the_other_two`, `slot_promotion_leaves_detection_output_unchanged_except_for_the_new_field`).
- [x] 8.2 The slot-order arrays in the same file gain `OpenCodeDesktop` between `OpenCodeNpm` and `CodexStandalone` (`nothing_yields_five_not_detected_records_and_zero_issues`, `scan_for_emits_five_records_in_probe_table_order`, `slot_promotion_leaves_detection_output_unchanged_except_for_the_new_field`).
- [x] 8.3 Verified **before touching**: the `packaged-and-legacy` fixture case's `paths.len(), 4` pin (`packaged_and_legacy_yields_four_never_merged_claude_installs`, distinct installation paths) — this fixture has no `AppData/Local/Programs/@opencode-aidesktop` directory, so it gains no desktop install and the count correctly stays 4 (confirmed by both reading the fixture tree and the test passing unmodified). The `isolation` fixture case (design's actual citation) needed a different, more invasive edit — see task 8.1/8.2 — because its `non_codex` grouping logic was implicitly keyed on "not Codex", which the new slot would have silently joined; restructured to check the three pre-existing slots by identity and the new desktop slot's `NotDetected` status explicitly.
- [x] 8.4 `crates/vertice-core/src/scan.rs`: `client_presence.len()` pin 4 → 5 (`missing_roots_and_clients_are_visible_diagnostics`). Confirmed `report.installations.len()` pins **unchanged** — `complete` (4) and `corrupt-skill` (1) fixtures have no desktop install either.
- [x] 8.5 `crates/vertice-core/src/installations.rs`: confirmed the three in-module slot tables (`label`, `client`, `version_source`) all carry the new variant (from Phases 5-6). Also fixed a stale doc comment ("which of the four version sources" → "four (now five, with `AsarPackageJson`)").
- [x] 8.6 `crates/vertice-core/tests/model_contract.rs`: confirmed the `"OpenCode (desktop app)"` pin and the exhaustive-match extension both landed (Phase 5.1).
- [x] 8.7 `crates/vertice-app/src/freshness/upstream.rs` tests: confirmed the `None` arm covers the new slot, plus a dedicated `opencode_desktop_issues_no_request_by_construction` test (Phase 7.1-7.2).
- [x] 8.8 **`openspec/specs/client-installation-detector/spec.md`** — the merged living spec. **Deliberately NOT touched by this apply phase.** This change's own delta spec at `openspec/changes/detect-desktop-client-installs/specs/client-installation-detector/spec.md` already states "five" at both the requirement-body site and the capability-prose site (verified: lines 7, 23, 95, 107, 109, 113, 116, 120 of the delta all say "five"/"Previously: four"). Merging delta specs into `openspec/specs/` is standard OpenSpec **archive-phase** work, not apply-phase — `sdd-apply`'s scope is code, tests, and this change folder's own artifacts. Flagging explicitly so `sdd-archive` does not skip it: both sites (requirement body AND capability prose line 5) must move together, per the `add-codex-client-support` precedent design cites.
- [x] 8.9 `frontend/src/lib/pages/ClientsPage.test.ts`: confirmed all five new tests from Phases 1 and 7.6 are present and green (`claude_code_card_reads_the_bundled_record_when_npm_is_not_detected`, `opencode_card_reads_the_desktop_record_when_npm_is_not_detected`, plus the three N-slot rule proofs relocated to `presenceFor.test.ts` per the documented task-1.2 deviation).
- [x] 8.10 Final sweep: grepped the whole tree for the literal `"four"` near `ClientInstallSlot`/`client_presence`/probe-slot context. Found and fixed one stale site not explicitly named above: `installations.rs`'s `impl ClientInstallSlot` doc comment (task 8.5). Every other `"four"` match belongs to unrelated code (`roots.rs`'s four skill roots, the MCP seam-containment test's "four MCP modules", `client_installations.rs`'s intentionally-unchanged `packaged-and-legacy` paths pin).

## Phase 9: Deferred Items — `internal-docs/pendientes-desarrollo.md`

- [x] 9.1 Added **P17** (next available number after P16) to `internal-docs/pendientes-desarrollo.md`, recording, at minimum:
  - **H3 — out of scope.** Anthropic's native standalone Claude Code installer (`install.ps1` / WinGet) is a real, separate coverage gap; H1 fully explains the reported Claude Code symptom without it (proposal "Out of Scope").
  - **Deferred freshness upstream for `OpenCodeDesktop`.** `upstream_for` returns `None`; `app-update.yml` names `anomalyco/opencode` but it is unverified whether the desktop app and the `opencode-ai` CLI share a release-tag namespace, so wiring it risks a false "outdated" badge (design §4.3).
  - **Accepted fixture self-consistency blind spot.** `build_asar` and the reader share one understanding of the asar format; a future OpenCode repackaging (different asar version, v8-snapshot bundle, unpacked app directory, moved `package.json`) leaves the whole suite green while the probe is dead on every real machine. No cheap mechanical fix exists — a test against the real 143 MB archive is exactly what the fixture discipline forbids. Degradation stays benign in that scenario (`Detected`, no version, never a wrong one) (design §8.1).
  - **The `@opencode-aidesktop` folder-name rename risk.** An electron-builder artifact that could be renamed upstream, silently killing the probe; degradation is `NotDetected`, never an error; no mitigation short of a manual oracle (design §11).
  - **Stale `owner: "SST"` copy** at `ClientsPage.svelte:38`, which should read `anomalyco/opencode` per V4 — not touched in this change since it is copy, not detection (design §6.2).
  - **Stale `.gitattributes:6` path** naming `fixtures/installations/...` instead of `fixtures/client-installations/...` — harmless today because the broader `fixtures/** -text` rule on line 2 still covers it, but worth a one-line hygiene fix later (design §0).
  - **The measured scan-time cost**, once `BENCH-1` (Phase 2) lands: record the actual figure and, if Phase 2 landed in the ~25-100 ms band, the explicit "known scan-time regression" note per §3.1's decision rule.

## Phase 10: Manual Oracle (§8.4) — non-automated acceptance gate, cannot be closed by CI

**This phase is not satisfied by any test passing.** It requires the affected machine (`C:\Users\raul_`) and is recorded here as an explicit, separate acceptance gate.

**PARTIALLY DISCHARGED 2026-09-01 — see `manual-oracle.md`.** 10.1 is closed; 10.2 is closed except for the comparison against OpenCode's own UI; 10.3 and 10.4 remain unmeasured. Original apply-phase note follows.

**NOT RUN by this apply phase.** This agent has no physical access to `C:\Users\raul_` or the real OpenCode desktop installation. All five sub-tasks below remain unchecked, explicitly and honestly, and MUST be completed by the user (or a future session with machine access) before this change is considered fully accepted. Everything else in this checklist (Phases 0-9, 11) is complete and green.

- [x] 10.1 **A2** — read the real `app.asar` header and confirm its root `files` map has a top-level `package.json` key (design §0, A2; §8.4 item 1). Record the result in this file or in the change folder.
- [ ] 10.2 **Version equality** — run `asar::read_package_version` against the real archive and compare the extracted string against the version OpenCode's own UI reports. Equality is the only acceptance signal for this gate; a mismatch is a stop-the-line result, not a fixture to adjust (design §8.4 item 2, §2.2's residual risk).
- [ ] 10.3 **Wall-clock cost of the real call** — measure `read_package_version`'s time on the real 143 MB archive with a warm page cache; this is the real-world companion to `BENCH-1`'s synthetic measurement, and both are wanted (design §8.4 item 3).
- [ ] 10.4 **Whole-scan time**, with and without the desktop app present, so the regression is quantified rather than estimated (design §8.4 item 4).
- [x] 10.5 Record all four results in the change folder (or fold into Phase 9's log entry) before the change is considered accepted. This gate MUST NOT be marked done by any CI run.

## Phase 11: Gates

- [x] 11.1 Ran `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked`; `cargo deny check bans licenses`. **All four actually ran and passed**: `cargo` resolved on PATH at `/c/Users/Raul/scoop/apps/rustup/current/.cargo/bin/cargo` (1.97.1). `cargo deny` was NOT on the default PATH (the known environment gotcha) but resolved with a PATH prefix at `/c/Users/Raul/.cargo/bin/cargo-deny.exe` (0.20.2); output: "bans ok, licenses ok" (two pre-existing informational warnings about unmatched license/wrapper entries in `deny.toml`, unrelated to this change). `cargo fmt` found real formatting drift on first run (auto-generated multi-line `match`/`assert!` expressions in the new test files); fixed via `cargo fmt --all`, re-verified clean. All 30 test binaries green, 0 failures.
- [x] 11.2 From `frontend/`: `npm run lint` (clean), `npm run check` (286 files, 0 errors/warnings), `npm run test` (28 files, 214 tests, all passed), `npm run build` (succeeded, `dist/` emitted). Confirmed no stray `frontend/src/node_modules` was created.
- [x] 11.3 Re-ran `cargo test -p vertice-core` (regenerates all bindings) and diffed `frontend/src/bindings/`: `ClientInstallSlot.ts` changed content (four variants → five: `"claudeCodeNpm" | "claudeCodeBundled" | "openCodeNpm" | "openCodeDesktop" | "codexStandalone"`). Six other binding files (`BillingCycle.ts`, `Currency.ts`, `Subscription.ts`, `SubscriptionDraft.ts`, `SubscriptionError.ts`, `SubscriptionUpdate.ts`) initially showed as `git status`-modified but `git diff --numstat` confirmed **zero content difference** (a line-ending/mtime artifact of `ts_rs` rewriting every binding file on each run, not a real change) — reverted with `git checkout --` to leave a clean diff. No new binding file was emitted.
- [x] 11.4 Confirmed `crates/vertice-app/capabilities/default.json`, `deny.toml`, `Cargo.toml`, and `Cargo.lock` are byte-identical to their pre-change state (`git status --short` shows none of them touched) — no new dependency, no new capability.
- [x] 11.5 Confirmed `Cargo.toml` `rust-version = "1.88"`, the CI `MSRV: "1.88"` env, and `rust-toolchain.toml`'s `channel = "1.97.1"` still agree (floor unchanged, toolchain pin unchanged, no MSRV edit made).
- [x] 11.6 Grepped `crates/vertice-core/src/` and `crates/vertice-core/tests/` for `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, `remove_*`, `symlink*` — no new match beyond the pre-existing `read_only_audit.rs` test's own pattern list (which itself re-ran green as part of Phase 11.1, mechanically confirming zero matches in production `src/`). The disk surface this change adds is exactly `File::open`, `Metadata::len`, `Read::read_exact`, `Seek::seek`, and `symlink_metadata` (via the existing `exists` helper).
- [x] 11.7 Extended the read-only guarantee: added `full_scan_leaves_every_opencode_desktop_fixture_tree_unchanged` in `client_installations.rs`, running a byte-snapshot equality check across all 14 `opencode-desktop` fixture homes (the existing `full_scan_leaves_the_fixture_tree_unchanged` test only ever covered `packaged-and-legacy/`). Confirmed passing.
- [x] 11.8 Confirmed `crates/vertice-core/tests/fixtures/scan-orchestrator/**` and `fixtures/roots/reference/**` are byte-identical to their pre-change state (`git status --short` shows neither tree touched) — deliberately untouched, and `scan.rs`'s `report.installations.len()` pins for those fixtures are unchanged (still 4 and 1 respectively).
- [ ] 11.9 Phase 10's manual oracle results are **NOT recorded** — Phase 10 requires physical access to `C:\Users\raul_` and the real OpenCode desktop install, which this agent does not have. This gate remains explicitly open; see Phase 10's note.
