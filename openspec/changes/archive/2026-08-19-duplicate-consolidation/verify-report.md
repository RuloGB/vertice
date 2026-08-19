# Verify Report: duplicate-consolidation (T8)

Test line one.
Test line two with backtick `code` and apostrophe it's here.

## Verdict: PASS WITH WARNINGS

All mechanical gates pass, all 19 tasks are truthfully checked off, all 17 spec scenarios trace to a passing test with one exception (a scenario covered only partially by its integration test) and one scenario with no dedicated multi-component test. No CRITICAL structural or correctness defect found. Findings below are process/coverage WARNINGs and one perf/style SUGGESTION.

**Note on scenario count**: the spec has **17** Given/When/Then scenarios, not 20 as stated in the task brief. Verified by grep -c against spec.md.

## Gate Evidence (all executed for real, output captured)

| Gate | Command | Result |
|---|---|---|
| Format | cargo fmt --all --check | PASS - no output, no diff |
| Lint | cargo clippy --workspace --all-targets -- -D warnings | PASS - Finished dev profile, zero warnings |
| Tests | cargo test --workspace --locked | PASS - 12 consolidate unit tests + 7 consolidation integration tests + all pre-existing T2-T7 suites (85 lib tests, 22 in a second target, 18/14/9/8/24/13/7/1 across integration files) all green, 0 failed |
| Deny | cargo deny check bans licenses | PASS - bans ok, licenses ok (two unrelated license-not-encountered warnings for BSD-2-Clause/ISC, non-blocking, pre-existing) |
| Frontend | npm run lint && npm run check && npm run test && npm run build | PASS - eslint clean; svelte-check 169 files/0 errors/0 warnings; vitest 2/2 passed; vite build succeeded (3 chunks emitted) |

## Invariant Checks

| Invariant | Check | Result |
|---|---|---|
| crates/vertice-core/src/model/ byte-identical | git diff --stat -- crates/vertice-core/src/model | clean, zero diff |
| frontend/src/bindings/ byte-identical | git diff --ignore-space-at-eol -- frontend/src/bindings, line count 0 | clean - the M flags in git status are LF-CRLF line-ending metadata only (Windows checkout), not content changes |
| crates/vertice-core/src/roots.rs unchanged | git diff --stat -- roots.rs | clean |
| Cargo.toml/Cargo.lock/deny.toml unchanged | git diff --stat | clean |
| No is_duplicate field anywhere | grep -rn is_duplicate crates/ frontend/src | zero matches |
| No Ord/PartialOrd derive added to ComponentId | read identity.rs line 17 | Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS only |
| No std::fs/std::io/std::env/clock in consolidate.rs | grep for those tokens | zero matches |
| No hashing/content comparison in consolidate.rs | grep for hash/read_to_string/fs:: | zero matches |
| vertice-core imports nothing from tauri | grep -rn tauri crates/vertice-core | only a doc-comment sentence stating the invariant |
| lib.rs diff is exactly one line | git diff --stat -- lib.rs plus read | pub mod consolidate; correct alphabetical position, no crate-root re-export |

## Scenario Coverage Matrix (17 scenarios)

| # | Scenario | Test | Status |
|---|---|---|---|
| 1 | Module contains no I/O primitives | grep-verified structurally (correctly no dedicated automated test - structural claim) | COVERED (structural) |
| 2 | Reference fixture collapses 69 to 25 | consolidation.rs::reference_fixture_collapses_sixty_nine_entries_into_twenty_five_components | COVERED |
| 3 | Case/NFC-NFD variants collapse | consolidate.rs::tests::case_and_nfc_nfd_name_variants_collapse_to_one_component_with_two_locations | COVERED |
| 4 | Skill/agent same name not merged | consolidate.rs::tests::a_skill_and_an_agent_sharing_a_name_are_not_merged | COVERED |
| 5 | _shared consolidates like any other name (3 locations, one per root) | consolidation.rs::underscore_shared_existing_in_three_roots_consolidates_like_any_other_name | COVERED - W1 resolved |
| 6 | Exact location-count distribution (22x3, 3x1, 0x2) | exactly_twenty_two_components_have_three_locations_in_canonical_order + exactly_three_components_have_a_single_location_and_none_has_two | COVERED |
| 7 | Total location count conserved | total_location_count_is_conserved | COVERED |
| 8 | Locations follow canonical root order | rank-sorted assertion inside exactly_twenty_two_components test + consolidate.rs::tests::none_path_sorts_before_some_path_under_the_same_root | COVERED |
| 9 | Later root's non-empty description wins | consolidate.rs::tests::later_roots_non_empty_description_wins_over_earlier_roots_empty_one + real-pipeline real_pipeline_precedence_prefers_the_later_roots_real_description | COVERED |
| 10 | Whitespace-only description does not win | consolidate.rs::tests::whitespace_only_description_does_not_win_precedence + same real-pipeline test, exercises real Some newline from actual YAML | COVERED |
| 11 | Precedence independent of arrival order | consolidate.rs::tests::precedence_is_independent_of_input_arrival_order | COVERED |
| 12 | Single-location component not marked duplicated | consolidate.rs::tests::single_component_input_is_passed_through_with_one_location (implicit - no is_duplicate field exists to test against directly, correctly so) | COVERED (implicit by design) |
| 13 | Two same-name components ordered by identity | consolidate.rs::tests::two_components_sharing_a_display_name_are_ordered_by_identity | COVERED |
| 14 | Output order stable across shuffled inputs (multi-component) | none found | UNCOVERED - see Finding W2 |
| 15 | Empty input yields empty output | consolidate.rs::tests::empty_input_yields_empty_output | COVERED |
| 16 | Single-component input yields 1 location | consolidate.rs::tests::single_component_input_is_passed_through_with_one_location | COVERED |
| 17 | Divergent duplicate copies merge without content diff | later_roots_non_empty_description_wins_over_earlier_roots_empty_one (divergent description values merge via precedence only) plus structural grep confirming no byte comparison exists | COVERED |

## Findings

### WARNING W1 - _shared scenario is only partially exercised end-to-end
Spec text (spec.md lines 45-49) literally reads: GIVEN three _shared components, one per root, THEN they merge into one component with three locations. The only fixture exercising this (tests/fixtures/roots/underscore-shared/) has a _shared skill under ONE root only (.claude/skills/_shared/SKILL.md), and the test asserts locations.len() == 1. This proves _shared is not name-filtered out of the output (a real regression it would catch: if a _shared-style filter were added, the assertion underscore_shared.len() == 1 would fail), but it does NOT prove the 3-location merge behavior for that specific name - that behavior is only proven generically, via the shared-01..shared-22 fixtures in reference/ (which are not literally named _shared). CA-8's full claim (no filtering, merges across roots) is proven by the combination of two separate tests rather than by one literal test matching the spec's GIVEN clause. Design section 9 itself narrows the CA-8 test intent to "structural review, not just the test," so this is a legitimate, deliberate design narrowing - but it means the spec's literal scenario text is not what got implemented as a single test. Not blocking; recommend either updating the fixture to carry _shared in all three roots, or amending the spec's Given clause to match the narrower, already-agreed design intent.

**RESOLVED (post-verify, before PR).** The first option was taken: `tests/fixtures/roots/underscore-shared/` now carries `_shared` under all three skill roots, and the test — renamed `underscore_shared_existing_in_three_roots_consolidates_like_any_other_name` — now asserts three input components, one output component, `locations.len() == 3`, and the three root ids in canonical order. RED was observed before the fix (`the fixture must place _shared under all three roots`, left: 1, right: 3) by temporarily removing the two added roots. CA-8 is now proven by one literal test matching the spec's GIVEN clause, not by two tests in combination.

### WARNING W2 - Output order stability across shuffled inputs has no dedicated multi-component test
No test feeds the same set of two or more distinct-identity components in two different (shuffled) orders and asserts the output vector is identical. The closest tests are: precedence_is_independent_of_input_arrival_order (three components that all collapse into ONE output component - tests field-precedence stability, not multi-component order stability) and two_consecutive_calls_over_the_same_input_yield_identical_output (same input order run twice, not shuffled). The final sort_by(name, then id) step (consolidate.rs lines 127-131) is a total order over a unique key (ids are pairwise distinct post-grouping), so this is very likely correct by construction - but the specific spec scenario has no direct covering test. Recommend adding a small unit test: feed 3+ distinct-identity components in two different input orders, assert consolidate(order_a) == consolidate(order_b).

### WARNING W3 - Strict TDD RED state never observed for the 7 integration tests (self-reported by apply)
Per the brief, tests/consolidation.rs's 7 tests were written after consolidate.rs already existed and so never ran RED, violating the letter of strict_tdd: true and task 3.1's RED requirement. I independently assessed each of the 7 tests for whether it would actually fail against a plausible wrong implementation (a mandatory check per the brief, since a test that cannot fail is worse than no test):

- reference_fixture_collapses_sixty_nine_entries_into_twenty_five_components: would fail if grouping is wrong (e.g. no-op passthrough gives 69, or over-aggressive merge gives fewer than 25). Not vacuous.
- exactly_twenty_two_components_have_three_locations_in_canonical_order: would fail on a lost location or wrong ordering. Not vacuous.
- exactly_three_components_have_a_single_location_and_none_has_two: would fail if any group of 2 leaked through, or if the specific 3 names differ. Not vacuous.
- total_location_count_is_conserved: would fail on any dropped or duplicated location (the "winner elected" bug class). Not vacuous - this is the highest-value assertion in the suite.
- underscore_shared_fixture_skill_consolidates_like_any_other_name: would fail if a name-prefix filter excluded _shared. Not vacuous (see W1 for its scope limit).
- two_consecutive_calls_over_the_same_input_yield_identical_output: would fail on any HashMap-order-derived non-determinism. Not vacuous.
- real_pipeline_precedence_prefers_the_later_roots_real_description: would fail if precedence used a plain empty-string check instead of trim().is_empty(), since it exercises the real "description: >" empty-folded-scalar path via actual YAML parsing (confirmed directly by inspecting the fixture bytes, see below). Not vacuous.

None of the 7 is a tautology, a smoke test, or an assertion the code path cannot reach. The tests are real and would have failed against each's own designated wrong implementation - the process violation (no observed RED) does not, in this case, correlate with weak tests. Still, this is a genuine strict-TDD process deviation and should be recorded as such rather than waved through.

Additionally: no apply-progress artifact file exists under openspec/changes/duplicate-consolidation/ at all (only proposal.md, design.md, tasks.md, specs/). The TDD Cycle Evidence table this skill's strict-TDD module expects to cross-reference was not persisted as a file; the RED/GREEN self-report reached this verify phase only via the orchestrator's prompt text, not a stored artifact.

### Fixture byte inspection for W3's real-pipeline claim (verified directly)
crates/vertice-core/tests/fixtures/roots/precedence-description/.claude/skills/blank-description/SKILL.md frontmatter is exactly two lines: "name: blank-description" then "description: >" with nothing under it - a folded block scalar with no content, genuinely triggering the trim().is_empty() vs plain empty-string distinction (per frontmatter.rs's documented no-trimming behavior). This is not a case where the description is simply absent; it is present-but-blank, which is precisely what the whitespace-only precedence rule needs to be tested against. Confirmed genuinely exercising the intended edge case, not a weaker "field is None" substitute.

### SUGGESTION S1 - member_key allocates on every comparator call (consolidate.rs lines 56-67)
member_key is called twice per sort_by comparison (consolidate.rs lines 107-111), each call cloning every location's root_id (String) and path (Option of PathBuf), plus component.name.clone(). This is O(n log n) allocation calls, each proportional to the member's location count (nearly always 1). At PoC scale (dozens to low hundreds of components per scan) this is invisible; it is not a correctness defect and does not block. If component counts grow materially (e.g. project-scope roots in a future phase, or many more clients), a Schwartzian-transform precomputation (sort keys computed once, paired with each Component) would remove the repeated allocation. Non-blocking; recommend as a follow-up, not a gate.

### Correctness review - merge_into one-way fold (consolidate.rs lines 86-100)
Verified: input is sorted by (id, member_key) before folding (consolidate.rs lines 107-111), and member_key's primary component is each member's own sorted Vec of LocationKey (consolidate.rs lines 56-67). For every scanner in this codebase, a pre-consolidation Component carries exactly one Location (skills.rs, agents.rs; V3 in design.md), so sorting members by their single LocationKey IS sorting by canonical root order (root_rank is the first field of the key) - the fold therefore walks members in true canonical-root order, and "first non-empty wins" via a single forward pass is correct. The one documented exception (opencode_agents::assemble_component, V4) can already emit N locations for one component pre-consolidation, but all such locations share the opencode-agents root id, so ties there are broken by path only within the same rank - order among a single scanner's own pre-merged locations does not affect the canonical-order guarantee that other scanners' single-location members are compared against. No bug found.

### Totality review - no panics possible (full file read)
No unwrap, no expect, no indexing by computed index, no slicing. root_rank uses position().unwrap_or(ROOT_ORDER.len()) which is total. out.last_mut() pattern-matches Some/None explicitly. Confirmed total.

### ROOT_ORDER pin test (consolidate.rs lines 184-200) - verified real, not self-referential
root_order_matches_the_roots_module_in_order calls the actual crate::roots::skill_roots, agent_roots, opencode_agent_root functions (with a synthetic non-existent home) and builds "expected" from their real return values, then compares against the local ROOT_ORDER constant. This is a genuine cross-check against roots.rs - renaming or reordering a root id in roots.rs would change "expected" and fail this test, since "expected" is computed from the live function, not copy-pasted. Confirmed real, not tautological.

## CA Traceability

| CA | Claim | Assertion | Status |
|---|---|---|---|
| CA-2 | 69 to 25 | reference_fixture_collapses_sixty_nine_entries_into_twenty_five_components | passing |
| CA-3 | 22 with three locations | exactly_twenty_two_components_have_three_locations_in_canonical_order | passing |
| CA-4 | 3 with one location, 0 with two | exactly_three_components_have_a_single_location_and_none_has_two | passing |
| CA-8 | _shared consolidated, no name filtering | underscore_shared_existing_in_three_roots_consolidates_like_any_other_name | passing, both halves covered (W1 resolved) |
| CA-16 | read-only | grep for std::fs/std::io/std::env/clock, zero matches | confirmed |
| CA-17 | fixture-only, no env vars, no machine dependence | all fixture paths built via CARGO_MANIFEST_DIR compile-time macro, not std::env; grep confirms no std::env use | confirmed |

## Task Completion

All 19 checkable tasks in tasks.md (Phases 1-5) are marked done. Cross-checked against actual repository state:
- Phase 1 unit tests (1.2-1.9) exist verbatim in consolidate.rs's cfg-test block and all pass.
- Phase 2 implementation (2.1-2.8) matches design.md sections 3-8 exactly (ROOT_ORDER, LocationKey, sort-then-fold, precedence, output sort, must_use free function, no Result).
- Phase 3/4 integration suite (3.1-4.3) exists in tests/consolidation.rs and the new precedence-description fixture, all 7 tests pass.
- Phase 5 gates (5.1-5.11) were independently re-run in this verify pass; all pass except 5.4 (cargo deny), which tasks.md marked "not run" due to PATH - I re-ran it successfully in this session (bans ok, licenses ok) since ~/.cargo/bin is on this session's PATH. Tasks.md's honest "not-run" self-report for that item is now superseded by a real passing result.
No unchecked or falsely-checked task found.

## Design Coherence

Implementation matches design.md decisions exactly: free function by value (section 3), local ROOT_ORDER const pinned by test not by call (section 4), sort-then-fold with no HashMap/BTreeMap (section 5), per-field first-non-empty precedence with verbatim storage (section 6), name-then-id output sort (section 7), no Result/ScanIssue/panic path (section 8). File changes match section 10's table exactly - no unlisted file touched, no listed-unchanged file touched.

## Risks (for archive decision)

None CRITICAL. W1 and W2 are coverage gaps relative to the spec's literal scenario text but do not indicate a functional defect - independent source review (merge_into correctness, totality, ROOT_ORDER pin authenticity) found the implementation sound. W3 is a process deviation, assessed and found not to have produced weak tests. S1 is a non-blocking performance note.

## Recommendation

Safe to archive. Optionally address W1/W2 with two small additional tests before archive (low effort, high confidence value) - but neither is a functional regression risk, so this is the orchestrator's/user's call, not a gate.
