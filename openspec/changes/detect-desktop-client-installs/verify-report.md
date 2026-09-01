# Verify Report: Detect desktop client installations

Date: 2026-09-01
Verified against: uncommitted working tree, `openspec/changes/detect-desktop-client-installs/`

## Summary

**0 CRITICAL, 1 WARNING, 1 SUGGESTION.** All gates I actually ran (Rust + frontend, all
four/five commands) passed. Every scenario in the three delta specs maps to a named,
passing test. Structural invariants hold (no `tauri` in core, no forbidden imports under
`model/`, no new dependency, no unauthorized filesystem writes, bindings in sync). Phase 10
(manual oracle) remains correctly and explicitly open -- it requires physical access to the
user's second machine and cannot be closed here.

## 1. Gates -- actually run, real output

All commands below were executed in this session, not assumed from `apply-progress.md`.

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | **PASS** -- no output, no diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** -- clean, 0 warnings |
| `cargo test --workspace --locked` | **PASS** -- all test binaries green, 0 failures. Includes `asar_fixture_integrity.rs` (15/15), `client_installations.rs` (40/40), `model_contract.rs` (20/20, includes exhaustive `ClientInstallSlot` match with `OpenCodeDesktop`), lib unittests (95 passed, 1 ignored, see note below) |
| `cargo test --workspace --locked -- --ignored` | The 1 ignored test (`freshness_live_upstream_endpoints_still_match_the_documented_shape`) fails in this environment due to live GitHub rate-limiting (403) -- a pre-existing, network-dependent, ignored oracle test, unrelated to this change |
| `cargo deny check bans licenses` | **PASS** -- bans ok, licenses ok; same two pre-existing informational license/wrapper warnings noted in `tasks.md` 11.1, unrelated to this change |
| `npm run lint` (frontend/) | **PASS** -- clean |
| `npm run check` (frontend/) | **PASS** -- 286 files, 0 errors, 0 warnings |
| `npm run test` (frontend/) | **PASS** -- 28 files, 214 tests, all passed |
| `npm run build` (frontend/) | **PASS** -- dist/ emitted |

No stray frontend/src/node_modules was created (checked directly). cargo and cargo-deny
both required PATH prefixes not on the default shim PATH -- consistent with the known
environment gotcha, not a project defect.

## 2. Spec conformance -- scenario-to-test mapping

### client-installation-detector delta

| Scenario | Test |
|---|---|
| The two npm slots resolve with no enumeration | pre-existing coverage, unaffected |
| The Codex slot's candidate roots are composed from home alone | pre-existing coverage, unaffected |
| The OpenCode desktop slot's path is composed from home alone | `home_without_the_desktop_root_yields_not_detected_and_zero_issues` + probe wiring proven by `scan_for_emits_five_records_in_probe_table_order` |
| An npm slot's version comes from package.json | pre-existing coverage, unaffected |
| The bundled slot's version comes from the directory name | pre-existing coverage, unaffected |
| OpenCode desktop version extraction succeeds | `happy_fixture_yields_one_detected_opencode_desktop_installation` |
| Every version-extraction failure mode degrades to Detected, never NotDetected | `every_remaining_opencode_desktop_fixture_matches_its_taxonomy_row` (11 failure cases) + `opencode_desktop_root_without_a_readable_archive_is_detected_with_no_installations` (no-asar) |
| An oversized header degrades with a Warning, every other failure with an Error | `oversized_header_degrades_with_a_warning_not_an_error` (Warning) + the taxonomy sweep (Error for the other 11) |
| A machine with no clients yields five notDetected records and zero issues | `nothing_yields_five_not_detected_records_and_zero_issues` |
| The scan always emits exactly five presence records on Windows | `scan_for_emits_five_records_in_probe_table_order` + the taxonomy sweep's per-case records.len() assertion |
| OpenCode desktop root existing yields Detected; absent yields NotDetected | `happy_fixture_yields_one_detected_opencode_desktop_installation` (present) + `home_without_the_desktop_root_yields_not_detected_and_zero_issues` (absent) |
| A bundled slot with two coexisting versions keeps both in one record | pre-existing coverage, unaffected |
| A candidate root that exists but yields nothing is Detected, not NotDetected | pre-existing coverage, unaffected (npm-slot case); OpenCode-desktop equivalent is the taxonomy sweep |
| ScanReport.installations equals the flattened presence records | pre-existing coverage, unaffected |
| A home with no Codex installation yields a NotDetected Codex record | pre-existing coverage, unaffected |

All new scenarios have real, named, currently-passing coverage.

### inventory-ui delta

| Scenario | Test |
|---|---|
| A later slot detected while an earlier slot is not renders as detected | `claude_code_card_reads_the_bundled_record_when_npm_is_not_detected` (ClientsPage.test.ts) |
| No slot detected renders as not detected | `a_fully_undetected_group_still_renders_the_first_records_probed_paths` (presenceFor.test.ts) |
| The rule holds for a group of three slots, not just two | `the_first_detected_record_wins_across_a_group_of_three_slots` (presenceFor.test.ts) |
| OpenCode's card is driven by the same rule across its two slots | `opencode_card_reads_the_desktop_record_when_npm_is_not_detected` (ClientsPage.test.ts) |

Confirmed: two of these are pure unit tests in a new file (presenceFor.test.ts) rather than
DOM-rendering tests in ClientsPage.test.ts as tasks.md 1.2/1.3 originally specified -- the
documented deviation is sound: no real product group in clientGroups today has three slots,
so the N-proof needs the pure function directly. both_detected_selects_the_first_in_record_order
also exists, correctly pinning the accepted Option-A limitation. All five green.

### domain-model delta

| Scenario | Test |
|---|---|
| ClientInstallSlot is exhaustively matchable with five variants | `client_install_slot_is_exhaustively_matchable_without_a_wildcard_arm` (model_contract.rs) |
| ClientPresence can carry the new slot | covered structurally by every client_installations.rs test constructing an OpenCodeDesktop record; no dedicated isolated test but the type-level guarantee is enforced by the exhaustive-match test plus the model's plain-enum discipline |

No uncovered scenario found across the three delta specs.

## 3. The six contract scenarios (explicit checks)

- (a) H1 regression -- first slot NotDetected, later slot Detected renders detected with
  the detected slot's version: claude_code_card_reads_the_bundled_record_when_npm_is_not_detected.
  PASS.
- (b) No slot detected renders not detected -- a_fully_undetected_group_still_renders_the_first_records_probed_paths.
  PASS.
- (c) OpenCode desktop root present/absent -> Detected/NotDetected -- happy_fixture_yields_one_detected_opencode_desktop_installation /
  home_without_the_desktop_root_yields_not_detected_and_zero_issues. PASS.
- (d) Successful version extraction yields one ClientInstallation -- happy_fixture_yields_one_detected_opencode_desktop_installation
  asserts installations.len() == 1, version 0.4.2, path ends with @opencode-aidesktop
  (never app.asar). PASS.
- (e) Every version-extraction failure mode yields Detected with empty installations,
  never an error/panic/failed scan -- every_remaining_opencode_desktop_fixture_matches_its_taxonomy_row
  sweeps all 11 non-happy fixture cases plus no-asar is covered by a dedicated test; every
  case asserts status == Detected and the correct installation count/issue severity. cargo test
  ran clean (no panics) across all 30 test binaries. PASS.
- (f) Exactly five ClientPresence records on Windows -- scan_for_emits_five_records_in_probe_table_order
  (order-checked) and every other test in client_installations.rs and scan.rs pins .len() to 5.
  PASS.

## 4. Structural invariants

| Invariant | Check | Result |
|---|---|---|
| vertice-core imports no tauri | grepped crates/vertice-core/src/ | Only one hit: a doc comment in lib.rs stating the prohibition itself. No actual import. PASS |
| Nothing under model/ imports std::fs/std::io/std::env | grepped crates/vertice-core/src/model/ | Only doc-comment mentions of the allow-list, no real imports. PASS |
| No new crate dependency | git diff --stat Cargo.toml Cargo.lock crates/*/Cargo.toml | Empty diff -- byte-identical. PASS |
| No File::create / OpenOptions write outside app data dir (CA-16) | grepped crates/vertice-core/src/ for creation/write/removal calls | Zero matches. PASS |
| frontend/src/bindings/ matches cargo test -p vertice-core regeneration, no stale orphans | Re-ran the full cargo test --workspace (regenerates bindings); git status shows ClientInstallSlot.ts with a real content diff (four to five variants) and six other binding files flagged by git status but confirmed zero real content diff via git diff --numstat (all 0/0 changed lines) -- a known ts_rs line-ending re-touch artifact, not a real change. 35 binding files total, matches the expected set, no orphan | PASS |

## 5. Record-count pin sweep (design section 8.3)

Verified every named site actually moved 4 -> 5:

- crates/vertice-core/tests/client_installations.rs -- 5 sites (lines 63, 195, 428, 1185, and the taxonomy-sweep's per-case assertion at 1242), all pin 5.
- crates/vertice-core/src/scan.rs:168 -- client_presence.len() pins 5.
- installations.rs's three in-module slot tables (label/client/version_source) all carry OpenCodeDesktop.
- model_contract.rs's exhaustive match includes OpenCodeDesktop.
- crates/vertice-app/src/freshness/upstream.rs tests cover the None arm for the new slot.
- Delta spec openspec/changes/detect-desktop-client-installs/specs/client-installation-detector/spec.md -- both the requirement-body site (line 130, "exactly five records") and the distinct capability-prose site (line 7, "five slots") say five.

One additional site swept and confirmed correctly deferred, not missed:
openspec/specs/client-installation-detector/spec.md (the merged living spec, not the
delta) still says "four" at its capability-prose line and its requirement body. This is
expected and correctly documented -- tasks.md task 8.8 explicitly records this as
out-of-scope for the apply phase and flags it for sdd-archive's merge step. I confirmed by
grep that no other repository site outside openspec/specs/ or the archived-change
directories still says "four" in a ClientInstallSlot/client_presence/probe-slot context.
This is a real, correctly-tracked open item for the next phase, not a defect of this one.

## 6. The offset-formula guard -- WARNING

The pin exists and is correct: data_start_is_eight_plus_header_len_not_json_start_plus_json_len
(crates/vertice-core/src/asar.rs:415) calls the actual production parse_prefix on a
padded fixture (padding length 1, confirmed non-zero) and asserts prefix.data_start == 68
(the correct 8 + header_len result) and prefix.data_start != json_start + json_len (67).
This is a byte-exact, direct regression guard on production code -- if data_start's formula
were ever "simplified" to the forbidden one, this test fails immediately. This part is solid.

The shifted-payload fixture's guarantee is weaker than the design's literal intent, as
tasks.md's own deviation note anticipates. The design (2.2, 8.2) describes a fixture
where a decoy manifest sits exactly at the position the forbidden formula would compute,
so that a reintroduced formula bug would silently return the decoy's plausible-but-wrong
version (9.9.9). The implemented fixture instead places the decoy after the true
manifest, inside the same payload, relying on the entry's declared size to bound the read
and exclude it. I traced the arithmetic: with a real 1-2 byte alignment padding, the forbidden
formula computes an address before data_start (inside the padding, not the payload), so
even a reintroduced formula bug would not land on the appended-after decoy -- it would instead
read a truncated/garbled buffer likely to fail JSON parsing outright (Malformed), not return
9.9.9. This means the guard test's assertion (9.9.9 appears nowhere) would likely still pass
even if the forbidden-formula bug were reintroduced -- it is not, in isolation, a regression
guard for that specific bug.

This is not a critical gap, because the separate data_start_is_eight_plus_header_len test
independently and directly pins production's parse_prefix to the correct formula, byte-exactly.
Between the two tests, the formula bug is caught (by the first) and an entry-boundary over-read
is caught (by the second) -- two real, distinct guarantees, just not the single combined
"wrong formula silently returns a plausible wrong version" demonstration the design originally
asked for. The apply phase's own documentation of this deviation (task 3.7, with the
"mathematically impossible... under the same declared size" reasoning) is honest and
technically correct. Flagging as WARNING: the two tests together are adequate, but a reader
relying on shifted-payload's name and the design text alone would over-estimate what that
specific fixture demonstrates. Worth a one-line comment update rather than a code change.

## 7. The apply phase's five recorded deviations -- judged

1. presenceFor extracted to a new module (task 1.5) -- Acceptable. Behavior identical;
   necessary to unit-test the N=3 rule since no real product group has 3 slots. No spec or
   architecture impact.
2. shifted-payload fixture placement (task 3.7) -- Acceptable with the caveat in section 6
   above (WARNING, not a breach). The combined test pair still closes the practical risk.
3. Integrity-test verification method (task 4.3, "confirm each fails against a corrupted
   copy first") -- Acceptable. The stated alternative (generation-from-the-same-reconstruction
   logic, verified by first-run success) is methodologically equivalent for catching a bit-rot
   or hand-edit regression; the 15th sanity test (read_package_version_sanity_matches_the_design_table)
   adds real extra coverage.
4. Isolation-fixture restructure instead of a simple count bump (task 8.3) -- Acceptable and
   in fact necessary: the original "not Codex" grouping logic would have silently absorbed the
   new slot, which is exactly the kind of latent bug the design's "pins move in lockstep"
   discipline exists to catch. Correctly identified and fixed.
5. openspec/specs/ merge deferred to sdd-archive (task 8.8) -- Correct scoping per OpenSpec
   convention; not a deviation from the apply phase's actual scope, and explicitly flagged for
   the next phase (confirmed still needed in section 5 above).

None of the five rises to a contract breach.

## 8. Phase 10 -- manual oracle (open, correctly so)

Left open, as required. Phase 10 (section 8.4 of design.md) requires physical read access
to the real 143 MB app.asar on the affected machine -- A2's root-package.json-key shape,
version-string equality against OpenCode's own UI, wall-clock cost of the real call, and
whole-scan time with/without the desktop app present. None of these can be verified by this
agent or by CI. tasks.md items 10.1-10.5 remain unchecked, and I did not attempt them. This
is the correct state, not a defect.

## Suggestion

- Consider adding a one-line clarifying comment on
  shifted_payload_never_yields_the_neighbouring_manifests_version_in_the_report (or renaming
  it) to state explicitly that it guards the entry-size-boundary read, not the data_start
  formula itself -- the formula is guarded by the sibling test -- so a future reader doesn't
  over-attribute what this specific fixture proves. This is cosmetic; no functional change
  needed.

## Result

- status: done
- executive_summary: All Rust and frontend gates pass (0 failures across 30 Rust test
  binaries and 214 frontend tests); every delta-spec scenario has real, named, passing
  coverage; structural invariants hold; 0 CRITICAL, 1 WARNING (the shifted-payload fixture's
  guarantee is narrower than the design's literal text, though the combined test pair still
  closes the risk), 1 SUGGESTION (a doc-comment clarification). Phase 10's manual oracle is
  correctly left open, requiring physical machine access.
- artifacts: openspec/changes/detect-desktop-client-installs/verify-report.md
- next_recommended: sdd-archive -- no CRITICAL blockers; archive should perform the deferred
  openspec/specs/client-installation-detector/spec.md merge (both the requirement-body and
  capability-prose "four" to "five" sites) as part of its own scope, and Phase 10 should
  remain tracked as a follow-up outside the automated SDD pipeline.
- risks: (1) Phase 10's manual oracle is unresolved and requires the user's physical machine
  -- not blocking archive, but the change is not fully accepted until it is closed. (2) The
  shifted-payload fixture's guarantee is narrower than originally specified (WARNING, not
  CRITICAL -- see section 6).
- skill_resolution: none (file-based OpenSpec artifact store; no mem_* tools used per
  instructions)
