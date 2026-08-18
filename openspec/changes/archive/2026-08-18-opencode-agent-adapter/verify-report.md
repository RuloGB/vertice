# Verify Report: OpenCode Agent Adapter (T6)

Mode: **openspec artifact store**. Strict TDD: **active**. Change: `opencode-agent-adapter`.

## Verdict: **PASS** (was PASS WITH WARNINGS — both warnings closed, see Addendum)

All executed gates are green. All 43 tasks are either `[x]` or honestly marked NOT RUN with a stated reason (0.5, 3.5 -- MSRV floor, environment gap). No CRITICAL issue was found. Two WARNING-level spec-coverage gaps were found; neither weakens a load-bearing behavior. The self-declared TDD deviation (tasks 2.5-2.11) is judged acceptable.

## Gates -- actually re-run in this session, not taken on trust

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all --check` | Clean, no output |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Clean, no warnings |
| tests | `cargo test --workspace --locked` | All green. 63 vertice-core lib tests, 22 agent_scanner.rs (T5, unmodified), 14 frontmatter_reader.rs, 9 jsonc_behavior.rs, 8 model_contract.rs, 22 opencode_agent_scanner.rs, 13 skill_scanner.rs (T4, unmodified), 7 yaml_behavior.rs, 1 yaml_seam_invariant.rs. 0 failures |
| deny (bans/licenses) | `cargo deny check bans licenses` (PATH-prefixed) | bans ok, licenses ok. Two pre-existing license-not-encountered warnings (BSD-2-Clause, ISC) -- unrelated, no crate uses those allow-list entries yet |
| MSRV 1.88 | `cargo +1.88.0 check -p vertice-core` | NOT RUN. rustup toolchain list confirms only stable and 1.97.1 are installed -- the 1.88 floor toolchain is genuinely absent locally. Correctly not claimed as passing by apply. Deferred to CI's msrv job |
| frontend lint | `npm run lint` | Clean |
| frontend check | `npm run check` | 169 files, 0 errors, 0 warnings |
| frontend test | `npm run test` (vitest) | 2/2 passed |
| frontend build | `npm run build` | Succeeded |

## Invariants confirmed independently

| Invariant | Check | Result |
|---|---|---|
| model/bindings unchanged | `git diff --numstat -- frontend/src/bindings crates/vertice-core/src/model` | Empty -- zero lines, confirming this is CRLF-artifact-only elsewhere and this change touches neither path at all |
| CA-16 read-only | grep for File::create, OpenOptions::write, fs::write, create_dir, remove_ across jsonc.rs, opencode_agents.rs, and both new test files | Zero matches |
| Core purity (no tauri) | grep "tauri" in crates/vertice-core/Cargo.toml and src/ | Zero import/dependency matches (only a doc comment stating the invariant) |
| No regex | grep in jsonc.rs/opencode_agents.rs/Cargo.toml | Zero matches |
| JSONC crate confined to one module | grep -rln "jsonc_parser" crates/vertice-core/src/ | jsonc.rs only |
| No env/machine-path reads in tests | grep for set_var, env::var, home_dir(), literal home paths in the new test file and opencode_agents.rs | Zero matches -- every test resolves fixture paths via env!("CARGO_MANIFEST_DIR") |
| No #[derive(Deserialize)] DTO for an agent entry | grep -n "derive(Deserialize)" opencode_agents.rs | Zero matches (one doc-comment mention only) |
| pub surface minimal | grep "^pub " | jsonc.rs: JsonValue, JsoncError, parse. opencode_agents.rs: OpenCodeAgentScan, scan. merge_all/merge_two/DescriptionField/extract_description/read_agent_object/assemble_component all private. Matches design section 5.2/5.3/task 2.13 |
| probe/resolve_single/resolve_opencode untouched | Read roots.rs | opencode_agent_root (line 166) is a new sibling function; resolve_opencode (132), resolve_single (109), probe (198) unchanged in structure |
| T4/T5 suites still green, untouched | Test run output | skill_scanner.rs (13/13) and agent_scanner.rs (22/22) unmodified and green |
| Nothing committed yet | git status --short, git log -3 | Confirmed: all T6 files are modified/untracked, matching apply-progress's "proposed commit sequence, not executed" claim |

## Judgment call 1 -- the self-declared TDD deviation: ACCEPTABLE, not remediation-required

Apply discloses that tasks 2.5-2.11 (value-level description extraction, issue taxonomy, the 22-test integration suite) were "verified together rather than individually red-first." This is weighed against the one place task order was declared non-negotiable: tasks 2.1-2.3, the partial-override merge safeguard.

Verified independently:
- 2.1's #[cfg(test)] module in opencode_agents.rs contains the literal-level merge_all tests, including shared_key_partial_override_merges_per_field_not_per_object with an explicit doc comment: "This is the test that MUST fail against the naive stub above."
- apply-progress records an actual captured failing run: 3 of 10 tests failed against the stub, and names exactly the three (partial-override, overlay-only-key, case-distinctness) -- the ones that discriminate a correct per-key recursive merge from a whole-object-replace stub. This matches the fixture's actual shape (see judgment call 2).
- The stub itself (`inputs.iter().cloned().reduce(|_base, overlay| overlay)`) is visibly a whole-object-replace, so the RED-before-GREEN discipline was genuinely followed here, not merely narrated.

For 2.5-2.11, every behavior claimed is covered by a passing test (extract_description's 7 unit tests, the 22 integration tests) -- re-run and confirmed green in this session. There is no case here where implementation logic exists without a corresponding test; the relaxation is in the order tests were written relative to implementation, not in test existence. Given: (a) this deviation applies to extraction/wiring/issue-taxonomy code that is comparatively low-risk (a value-level match with no recursive structure), (b) the one place a subtle bug is invisible from output alone -- the recursive merge -- is exactly where the strict order WAS followed with captured evidence, and (c) the deviation is disclosed rather than hidden, this is judged acceptable and does not need remediation. It is a scope-driven relaxation on the low-risk subset, not a bypass of the safeguard the change exists to protect.

## Judgment call 2 -- is partial-override genuinely discriminating: YES, confirmed

Read the fixture pair directly:

partial-override/opencode.json:  "reviewer": { "description": "Reviews code for quality issues", "permission": { "edit": "ask", "bash": "deny" } }
partial-override/opencode.jsonc: "reviewer": { "permission": { "edit": "allow" } }

The overlay omits description entirely and omits permission.bash. A whole-object-replacement merge (base's entry fully replaced by overlay's entry when both keys exist) would produce a reviewer entry with no description and no permission.bash -- losing both fields the test asserts survive:
- partial_override_fixture_merges_per_field_not_per_object asserts component.description == Some("Reviews code for quality issues").
- The unit-level shared_key_partial_override_merges_per_field_not_per_object additionally asserts permission.bash == "deny" (base-only nested sibling survives) and permission.edit == "allow" (overlay's nested override wins).

Both assertions would fail under whole-object replacement (confirmed by apply's captured RED run) and both pass under the real recursive merge (confirmed by this session's own green test run). The fixture and its tests are genuinely discriminating, not decorative.

## Judgment call 3 -- spec coverage of the four "agent entry body" scenarios

Spec requirement "An Agent Entry's Body Can Never Prevent The Agent From Being Reported" lists four scenarios:

1. Entry with every observed field (description, mode, prompt, tools-object, hidden, permission) produces a component. Coverage found: unmodelled_fields_do_not_affect_description_extraction (unit) covers description+mode+prompt+tools-object+permission but NOT hidden in the same entry. reference/ fixture's alpha has description+mode+prompt+tools(object)+hidden:false, gamma has permission but no hidden. No single fixture/test entry combines all six fields together (in particular none combines hidden:true with permission). Verdict: covered functionally, but not literally as one combined case -- SUGGESTION, not a gap that changes behavior, since every individual field's non-effect is independently proven.

2. Empty body still produces a component. Coverage found: malformed-entry/ and unit tests don't include a literal {} body case as a named scenario; empty-agent/ tests the object-level empty ("agent": {}), which is a DIFFERENT case (the "Absent... Produce No Component" requirement). An entry whose VALUE is {} (not the whole agent object) is not separately fixture-tested, though extract_description on an empty object returns Absent correctly by construction (map.get("description") on an empty BTreeMap is None). Verdict: structurally guaranteed but not directly test-asserted at the component/fixture level -- WARNING, orphaned scenario, low severity.

3. Unexpected type -- description a number/object/null, AND tools a string rather than an object -- degrades the field, never the component, no issue claims "skipped". Coverage found: description_wrong_type_yields_wrong_type_never_absent (unit) covers Number, nested Object, Array, Bool, Null for description -- thorough. malformed-entry/ integration fixture covers description: 42 (number) at the integration level with the correct Warning, component still emitted. NO fixture or test anywhere sets tools to a string -- verified by grepping every fixture's "tools" occurrences, all are objects (reference/ only). Verdict: gap confirmed -- the tools-as-string half of this scenario has zero test coverage. WARNING (not CRITICAL: the implementation never reads tools at all per design section 5.4, so this is structurally safe by construction, but the spec names this exact case and no test proves it).

4. Unmodelled future field does not disturb the result. Coverage found: no fixture/test uses a genuinely novel field name; existing tests use only the six real-world field names (mode, prompt, tools, hidden, permission), all of which the implementation already treats identically to a hypothetical unknown field (none of them are read except description). Verdict: functionally proven by construction (the code path for "any key other than description" is uniform and untyped), but literally interpreted, this scenario asks for a field name the capability does not model, which was not exercised verbatim -- SUGGESTION.

## Any spec requirement with no test: none found beyond the two flagged above

Every other requirement in the spec (OpenCode Agent Root Resolves..., Root Status Is Found If Either..., JSON And JSONC Parsing..., hidden Is Never A Filtering Signal, The agent Object Is Merged Per Key..., One File Produces N Components, Component Assembly..., Malformed JSON In One File Isolates..., Absent Files..., Out-Of-Scope Top-Level Keys..., Component And Issue Ordering Is Deterministic, A Normalization Collision..., Scanner Performs No Writes, This Capability Introduces No Domain Model Change, Every Case Is Traceable To A Repository Fixture) has a directly corresponding passing test, confirmed by cross-reading each scenario against tests/opencode_agent_scanner.rs and the unit tests in opencode_agents.rs/roots.rs. The jsonc-only, jsonc-syntax, and partial-override fixtures marked non-negotiable by design section 0/10 all exist and are exercised by discriminating assertions, not merely present on disk.

## Task completeness

41/43 checked [x]; 0.5 and 3.5 correctly marked NOT RUN with a stated, verified-true reason (MSRV toolchain absent). No task is falsely marked complete -- cross-checked task claims against actual gate re-runs in this session, all matched.

## TDD Compliance (Strict TDD module)

| Check | Result | Details |
|---|---|---|
| TDD Evidence reported | Yes | apply-progress.md "TDD Cycle Evidence" table present |
| All tasks have tests | Yes | Every implementation task maps to a passing test file |
| RED confirmed (tests exist) | Yes | merge_all's literal tests and partial-override integration test both exist and were read directly in this session |
| GREEN confirmed (tests pass) | Yes | All 22 integration + 63 lib tests re-run green in this session |
| Triangulation adequate | Yes | Merge algorithm: 10 distinct cases (base-only, overlay-only, partial-override, array-replace, scalar/object both directions, null-non-delete, fold-zero, fold-one, case-distinctness). Extraction: 7 cases |
| Safety Net for modified files | Yes | roots.rs's 9 pre-existing tests re-run green with zero edits to that suite |

TDD Compliance: 6/6 checks passed, with the disclosed 2.5-2.11 ordering relaxation ruled acceptable above.

### Assertion Quality
No tautologies, no ghost loops, no assertion-free tests found on inspection of opencode_agents.rs's unit tests and tests/opencode_agent_scanner.rs. full_scan_leaves_the_reference_fixture_tree_unchanged and the two broken-* tests assert concrete values (path suffixes, severities, counts), not just is_ok()-style checks.

Assertion quality: All assertions verify real behavior.

## Issues

CRITICAL: None.

WARNING:
1. Spec scenario "An unexpected type in the body degrades the field, never the component" is only half-covered: description wrong-type is thoroughly tested (unit + integration), but the scenario's other named case -- tools as a string rather than an object -- has no fixture or test anywhere in the tree. Structurally safe (design section 5.4: tools is never read), but unproven by a test the spec explicitly names.
2. Spec scenario "An agent entry with an empty body still produces a component" has no fixture/test whose agent entry value is a bare {} (distinct from empty-agent/'s empty agent object, which is a different requirement). Structurally guaranteed by extract_description's empty-map handling but not directly asserted.

SUGGESTION:
1. No single fixture combines all six real-world fields (description, mode, prompt, tools-object, hidden:true, permission) on one entry, as spec scenario 1 literally describes. Coverage exists piecewise across reference/'s alpha/beta/gamma.
2. Spec scenario "unmodelled future field" uses only known real-world field names in every test; a literal unknown field name (e.g. a fictional key) was never used, though the code path is uniform for any non-description key.

## Files verified

- openspec/changes/opencode-agent-adapter/specs/opencode-agent-scanner/spec.md
- openspec/changes/opencode-agent-adapter/tasks.md
- openspec/changes/opencode-agent-adapter/apply-progress.md
- openspec/changes/opencode-agent-adapter/design.md
- crates/vertice-core/src/jsonc.rs, crates/vertice-core/src/opencode_agents.rs, crates/vertice-core/src/roots.rs
- crates/vertice-core/tests/jsonc_behavior.rs, crates/vertice-core/tests/opencode_agent_scanner.rs
- crates/vertice-core/tests/fixtures/roots/opencode-agents/** (18 files across 13 homes)

## Recommendation

Proceed to sdd-archive, or optionally close the two WARNING gaps (a tools-as-string fixture assertion and a literal empty-body-entry assertion) first -- neither blocks archive since both are structurally already correct and no CRITICAL issue exists.


---

## Addendum — WARNING gaps closed by the orchestrator

Both WARNING-level spec-coverage gaps identified above were confirmed independently and then closed, rather than carried into archive. Leaving a spec scenario without a test defeats the reason the scenario was written: these two were added to the spec mid-cycle precisely because they are unobservable on the real machine and therefore reachable only by fixture.

**What changed**

- `tests/fixtures/roots/opencode-agents/malformed-entry/.config/opencode/opencode.json` gained two entries:
  - `tools-as-string` — a well-formed entry whose `tools` is a **string** (the Claude Code shape, and the exact wrong-shape guess design §5.4 refuses to make), with a valid `description`.
  - `empty-body` — an entry whose value is a bare `{}`.
- `tests/opencode_agent_scanner.rs` gained two tests:
  - `tools_typed_as_a_string_leaves_the_component_and_its_description_intact` — asserts the component survives, its `description` is intact, and no issue names the entry. Proves an unread field of an unexpected type costs nothing.
  - `an_entry_with_an_empty_body_produces_a_component_and_no_issue` — asserts a component with no description and **no** `ScanIssue`, pinning design §8's rule that an absent field is absence, not unreadable metadata.
- `malformed_entry_fixture_still_emits_components_with_warnings` was updated for the widened fixture: 4 components, still exactly 2 `Warning` issues — the two well-formed additions raise none. The issue count staying at 2 while the component count rises to 4 is itself the assertion that a healthy entry in a fixture named "malformed" is not swept into the warning path.

**Both new tests passed on first run.** The gaps were in coverage, not in behavior — the implementation already handled both cases correctly. Their value is regression protection and spec traceability.

**Gates re-run after the change** (orchestrator, this session):

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace --locked` | **161 passed, 0 failed** (was 159; +2 new tests) |

**Unchanged from the report above:** MSRV 1.88 remains NOT RUN (floor toolchain absent locally; `rustup toolchain list` shows only `stable` and `1.97.1`), correctly deferred to CI's `msrv` job. Tasks 0.5 and 3.5 stay open for that reason. The ruling that the disclosed 2.5–2.11 TDD ordering relaxation is acceptable also stands.
