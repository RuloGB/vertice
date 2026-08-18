# Verify Report: claude-code-agent-adapter (T5)

**Change**: `2026-08-18-claude-code-agent-adapter`
**Branch**: `main` (all T5 work uncommitted in the working tree)
**Verified**: 2026-08-18, by `sdd-verify`
**Strict TDD**: active (`openspec/config.yaml` -> `strict_tdd: true`)
**Artifact store note**: Engram MCP tools were unavailable in this session (per task instructions). `mem_search`/`mem_save` were not attempted; this report is persisted only as a file in the change folder, matching the archived precedent format.

## Overall Verdict: **PASS-WITH-FINDINGS**

All 15 `agent-scanner` spec requirements are implemented and covered by passing runtime tests, all four required gates genuinely pass, and both invariant-critical properties (no domain-model change, no writes) hold. One WARNING (a coverage gap against one explicit spec scenario) and two SUGGESTIONs are recorded below; none is CRITICAL and none blocks archive.

---

## Gate Results (re-run independently, real output)

| Gate | Command | Result |
|---|---|---|
| Rust fmt | cargo fmt --all --check | PASS - no output, exit 0 |
| Rust lint | cargo clippy --workspace --all-targets -- -D warnings | PASS - Finished dev profile, 0 warnings |
| Rust tests | cargo test --workspace --locked | PASS - all suites green: 45 lib unit (incl. 2 new agents::tests:: + 2 new roots::tests::agent_* on this Windows leg; a third agents::tests:: case is cfg(unix)-gated and not exercised here), 20 agent_scanner.rs, 13 skill_scanner.rs (T4 regression, unchanged), 14 frontmatter_reader.rs, 8 model_contract.rs, 7 yaml_behavior.rs, 1 yaml_seam_invariant.rs, 0 doctests |
| Dependency policy | cargo deny check bans licenses (via PATH prefix) | PASS - bans ok, licenses ok (2 pre-existing unrelated license-not-encountered warnings for BSD-2-Clause/ISC, not tied to this change) |
| Model/bindings drift | git diff --exit-code -- crates/vertice-core/src/model frontend/src/bindings | PASS - both exit clean; the 16 M entries git status reports on frontend/src/bindings/*.ts are CRLF/LF line-ending noise only (git diff --stat produces only line-ending warnings, zero content lines) |

npm run lint / check / test / build was NOT independently re-run by this verify pass (Rust-only scrutiny was the assignment's focus, and apply-progress.md already reports it green with no new binding to consume); flagged as NOT RE-RUN rather than claimed.

All gates that were re-run match apply-progress.md's claims. No disagreement.

---

## Spec Compliance Matrix - specs/agent-scanner/spec.md

| # | Requirement | Verdict | Evidence |
|---|---|---|---|
| 1 | Agent Root Resolves Under The Home Directory | PASS | roots.rs:82-106 builds <home>/.claude/agents via per-segment push, kind: SearchRootKind::Agent; no config-dir API imported. Covered by agent_root_resolves_under_home_with_agent_kind (ok) |
| 2 | A Direct .md File Under The Root Is An Agent, Detected Flat | PASS | agents.rs:167 filters file_type.is_file() && extension == "md", no recursion (std::fs::read_dir, not walkdir). Covered by direct_md_file_under_root_is_discovered, nested_md_file_is_not_discovered, non_md_file_directly_under_root_is_ignored (all ok) |
| 3 | Absent and Empty Agent Roots Produce No Issue and No Component (file-backed only) | PASS | walk_agents_root returns silently on ErrorKind::NotFound (agents.rs:100); empty dir yields an empty entries vec, no issue. Both integration tests filter via file_backed() (origin == File), never is_empty() on the raw component set. Covered by absent_root_yields_zero_file_backed_components_and_zero_issues and empty_root_yields_zero_file_backed_components_and_is_found (both ok), which also assert the two roots' SearchRootStatus values differ |
| 4 | Agent Frontmatter Data Contract | PASS | AgentFrontmatter { name: String, description: Option<String>, model: Option<String>, tools: Option<String> } (agents.rs:50-59); parsing delegated to unmodified frontmatter::read::<AgentFrontmatter> (agents.rs:180). Covered by tools_comma_separated_scalar_deserializes_as_one_string, missing_model_and_tools_is_not_a_failure, folded_description_is_parsed_in_full (all ok) |
| 5 | On-Disk Agent Component Assembly | PASS | agents.rs:180-193: kind: Agent, scope: User, one Location { path: Some(path), origin: File, root: root_id }, id derived from (kind, name) via ComponentId::derive. Covered by valid_on_disk_agent_produces_correctly_shaped_component (ok) |
| 6 | Embedded Agents Are Emitted From A Fixed, Named List | PASS, with a coverage gap (see WARNING 1) | EMBEDDED_CLAUDE_AGENTS const (agents.rs:29-36), gated on embedded_root.root.status == Found (agents.rs:72-75), each with path: None, origin: Embedded, Location.root a non-empty SearchRootId. Covered by embedded_agents_appear_when_agent_root_absent_but_claude_dir_present, no_embedded_agents_when_claude_dir_absent, embedded_and_on_disk_agents_distinguishable_by_origin_and_path, embedded_component_root_is_a_valid_search_root_id (all ok) - but see WARNING 1 for what the first of these does NOT actually prove |
| 7 | A User Agent File Shadowing An Embedded Agent Name Produces Two Components | PASS | No consolidation logic anywhere in agents.rs; both components independently pushed. Covered by shadowing_user_agent_and_embedded_agent_both_appear (shadowing/Plan.md, ok): 2 components share ComponentId::derive(Agent, "Plan"), one Embedded/None, one File/Some(_) |
| 8 | Per-File Parsing Failures Do Not Abort The Walk | PASS | Err(issue) => issues.push(escalate(issue)) (agents.rs:194), loop continues to next entries item. escalate unconditionally sets severity: Error, path/reason untouched (agents.rs:226-231), unit-tested by escalate_maps_every_severity_to_error. Covered by corrupt_agent_yields_an_issue_and_does_not_stop_the_walk (broken-frontmatter/, ok): good discovered, exactly one file-path-carrying issue containing broken |
| 9 | Non-UTF-8 Discovered Paths Are Guarded | PASS | ensure_utf8_path split out for direct unit testing (agents.rs:237-243, cfg(unix)-gated portable-fixture-free case per T4 precedent); on failure, path: None, lossy string in reason, walk continues (agents.rs:171-178). No integration fixture (none possible, matches T4D 7.1) |
| 10 | Scanner Performs No Writes | PASS | Grep across agents.rs, the roots.rs diff, and agent_scanner.rs for File::create, OpenOptions, fs::write, create_dir*, remove_* - zero matches, independently re-run. Covered by full_scan_leaves_the_fixture_tree_unchanged (ok) |
| 11 | This Capability Introduces No Domain Model Change | PASS | git diff --exit-code on model and bindings both clean, independently re-run. AgentFrontmatter has no TS/Serialize derive; model/tools are parsed and dropped at Component assembly, never promoted |
| 12 | Reference Fixture Set Produces Exactly 17 On-Disk Agent Components | PASS | Independently recounted: find on reference/.claude/agents/ lists exactly 17 .md files, none named to collide with the 6 embedded names. Covered by reference_fixture_yields_17_on_disk_and_23_total_with_23_distinct_ids (ok): 17 file-backed, 23 total, 23 distinct ids |
| 13 | Every Case Is Traceable To A Repository Fixture | PASS, with the same gap as #6 | All ten listed fixture cases exist on disk and none reuses a skill-scanner or frontmatter fixture. See WARNING 1: the specific scenario ".claude present, .claude/agents absent" has no dedicated fixture directory |

---

## Focused Scrutiny Items (from the task brief)

1. The one known design deviation (agent_roots construction). VERIFIED as described, and mechanically correct. resolve_single's suffix: [&str; 2] (roots.rs:109) is a fixed-size array - it genuinely cannot accept the embedded root's one-segment suffix [".claude"] without a signature change. The implementation keeps resolve_single unchanged (still private, still 2-segment, still used for the on-disk claude-agents root and the three skill roots) and builds the embedded pseudo-root's path/probe inline inside agent_roots using the private probe() helper directly (roots.rs:82-106). All stated invariants survived, independently confirmed: resolve_single private, probe private, agent_roots is the only new pub item in roots.rs (grep for "^pub " confirms), both root ids are string literals never derived from home ("claude-agents", "claude-embedded-agents", unit-tested by agent_root_ids_are_stable_and_never_path_derived), and the embedded root's scan_paths is vec![] (roots.rs:102-103, asserted by agent_roots_returns_exactly_two_entries_with_stable_ids). Judgment: acceptable as implemented - a mechanically-forced deviation, not a design-intent change. See SUGGESTION 1 for the one loose end it leaves (design.md's 5.1 code sketch is now stale).

2. The ordering test. VERIFIED as genuine. component_order_matches_sorted_file_name_order (agent_scanner.rs:383-399) runs over reference/ - 17 files, not a single-file fixture - and asserts the file-backed component name sequence equals its own sorted clone. Because entries.sort_by_key(std::fs::DirEntry::file_name) (agents.rs:150) is a real precondition for this assertion to hold reliably across read_dir's OS-dependent yield order, removing that sort would make this test flaky-to-failing. This is the strongest kind of ordering proof available without mocking read_dir - confirmed adequate.

3. The embedded gate - three scenarios. Two of three are directly and correctly tested; the third has a coverage gap. See WARNING 1 below for full detail - in short: (a) "zero embedded when <home>/.claude absent" is tested (no_embedded_agents_when_claude_dir_absent, absent-root/, ok); (b) "embedded present when .claude exists and .claude/agents exists-and-is-empty" is tested (embedded_agents_appear_when_agent_root_absent_but_claude_dir_present, empty-root/, ok) but its NAME claims to test "agent root absent", which it does not - empty-root/.claude/agents/ exists on disk (it is present-and-empty, not absent); (c) the spec's literal scenario - <home>/.claude exists AND <home>/.claude/agents/ does NOT exist - has no fixture and no test anywhere in the suite. Code-path analysis (not test evidence) shows the implementation would handle it correctly: walk_agents_root's symlink_metadata on the agents path returns NotFound and the function returns early with no issue (agents.rs:98-100), while embedded_status is probed independently against <home>/.claude, not against the agents subdirectory (roots.rs:90-92) - so the six embedded components would still be emitted. But this is an inference from reading the code, not a passing test proving it, and it is the exact scenario named in the spec.

4. The is_empty() trap. VERIFIED absent. file_backed() (agent_scanner.rs:36-41) filters c.locations.iter().all(|l| l.origin == LocationOrigin::File) and every absent/empty-root assertion goes through it; no bare scan.components.is_empty() is used for either of those two cases. (The one place a bare scan.components.is_empty() IS used - no_embedded_agents_when_claude_dir_absent, agent_scanner.rs:253 - is correct there: when <home>/.claude itself is absent, the spec requires the full component set, embedded included, to be empty, so a bare is_empty() is the right assertion for that specific case, not the forbidden one.)

5. The .gitkeep tripwire. VERIFIED present and effective. absent-root/.gitkeep and empty-root/.claude/agents/.gitkeep both exist on disk (confirmed via find), and neither is matched by git check-ignore (exit 1 = not ignored), so both would be tracked once staged. The tripwire test empty_agent_root_fixture_directory_still_exists_on_disk (agent_scanner.rs:49-62) asserts std::fs::metadata(...).is_dir() on empty-root/.claude/agents/ before any scanner code runs, named for its own failure, exactly per design 10. Note: as of this verify pass, the entire tests/fixtures/roots/agents/ tree is untracked in git status - expected for an unstaged branch, not a defect, but the tripwire's protective value only begins once these files are actually committed.

6. CA-5 partial (23 components). VERIFIED both by test and independent recount: reference/.claude/agents/ contains exactly 17 .md files (recounted via find), the six embedded names never collide with any of them, and reference_fixture_yields_17_on_disk_and_23_total_with_23_distinct_ids confirms 17 file-backed + 23 total + 23 distinct ComponentIds at runtime.

7. CA-13. VERIFIED. embedded_and_on_disk_agents_distinguishable_by_origin_and_path asserts the origin/path pairing holds for every location of every component in a mixed scan (tools-scalar/, which yields both an on-disk agent and the six embedded), with no name-based branch anywhere in agents.rs.

8. CA-12 partial. VERIFIED. corrupt_agent_yields_an_issue_and_does_not_stop_the_walk over broken-frontmatter/ (one corrupt name: corrupt-yaml file with an unclosed YAML list, one well-formed sibling good.md) confirms exactly one path-carrying Error issue referencing broken, and good is still discovered as a component.

9. AgentFrontmatter shape. VERIFIED exactly as specified: tools: Option<String> (not Vec<String>), model: Option<String>, struct lives in agents.rs (agents.rs:50-59), not frontmatter.rs (confirmed unchanged - frontmatter.rs untouched, not in the diff). tools/model are read but never written onto Component - the on-disk assembly block (agents.rs:180-193) constructs Component with no tools/model field reference, and Component itself has no such field (unchanged model, confirmed above).

10. Shadowing. VERIFIED intended, not a bug, as directed - see requirement 7 above and design 9's explicit rationale, which the implementation and test both follow without any consolidation logic.

11. ScanIssue path attribution. VERIFIED exactly per design 8's table. A failing read_dir iterator item (agents.rs:139-147) is attributed path: Some(scan_path.to_path_buf()) - the root, not None - with reason "could not read directory entry: {io}". None is reserved solely for the non-UTF-8-path case (agents.rs:174). No unit or integration test exercises the bare-iterator-error path directly (no portable way to force a mid-read_dir Err on committed fixtures), consistent with T4's same acknowledged gap for its own equivalent row - not flagged as a new issue.

12. Task completion spot-check. All 27 tasks in tasks.md are marked [x]; spot-checked against real evidence rather than the checklist alone:
- Task 1.9 / 3.1-3.4 (gate claims): independently re-run in this session, all four gates genuinely pass (see Gate Results table above) - not taken on the apply agent's word.
- Task 2.3 (20-test integration suite): counted directly from cargo test output - exactly 20 tests in agent_scanner.rs, matching the claim.
- Task 3.6 (model/bindings invariant): independently re-run git diff --exit-code, both clean - not taken on faith.
- Task 3.5 (read-only grep): independently re-run the grep across agents.rs, roots.rs, agent_scanner.rs - zero matches, matching the claim.
- Task 1.6/1.7 (fixture tree + tripwire): independently walked the fixture tree via find and content-inspected all ten case directories plus every one of the 17 reference/ files - matches the claimed content shapes (tools: Read, Grep, Glob, Bash, folded description: >, missing-optional, corrupt YAML, shadowing name: Plan) exactly.
- Task 2.7 (pub mod agents;): confirmed in lib.rs.
No task was found to be falsely ticked. The one genuine gap found (WARNING 1) is a fixture/test coverage gap, not a falsely-claimed task - no task in tasks.md claims the specific ".claude present, .claude/agents absent" fixture exists; it was never on the list task 1.6 itself enumerates, so this is a spec-vs-fixture-list gap that predates task completion, not a task marked done that wasn't.

---

## Design Coherence - design.md

| Decision | Verdict | Evidence |
|---|---|---|
| Two-root model (claude-agents walked, claude-embedded-agents probed-only) | PASS | roots.rs:82-106, scan_paths: vec![] for the embedded root |
| Embedded list gated on <home>/.claude presence, not unconditional | PASS | agents.rs:72-75 |
| AgentFrontmatter in agents.rs, not frontmatter.rs | PASS | confirmed, frontmatter.rs untouched |
| No shared skills+agents scanner abstraction | PASS | agents.rs duplicates escalate/ensure_utf8_path structurally rather than importing from skills.rs; zero cross-import between the two modules besides shared model/frontmatter/roots |
| resolve_single stays private, agent_roots is the only new pub item | PASS | confirmed by grep, see Scrutiny Item 1 |
| Flat walk, std::fs::read_dir, no walkdir in agents.rs | PASS | grep for walkdir:: in agents.rs matches only doc-comment prose, no import |
| Collect-then-sort ordering | PASS | agents.rs:150, entries.sort_by_key(std::fs::DirEntry::file_name) |
| Uniform Error escalation | PASS | escalate at agents.rs:226-231, unconditional |
| ScanIssue taxonomy for the two read_dir-specific rows | PASS | see Scrutiny Item 11 |

design.md 5.1's code sketch (resolve_single called twice with an unmodified signature) is now inaccurate relative to the actual implementation - see SUGGESTION 1.

---

## Issues

CRITICAL: none.

WARNING:
1. Spec scenario coverage gap: "<home>/.claude present, <home>/.claude/agents/ absent" is untested. agent-scanner/spec.md's "Embedded Agents Are Emitted From A Fixed, Named List" requirement has an explicit scenario: "GIVEN a home where <home>/.claude exists but <home>/.claude/agents/ does not - WHEN the scanner runs - THEN it still produces exactly six components with origin: Embedded and path: None." No fixture directory in tests/fixtures/roots/agents/ has a .claude/ directory without a .claude/agents/ subdirectory beneath it (independently confirmed: find -maxdepth 3 -type d shows all ten fixture cases have .claude/agents/ present). The test named for this scenario, embedded_agents_appear_when_agent_root_absent_but_claude_dir_present (agent_scanner.rs:227-243), actually runs against empty-root/, where .claude/agents/ exists as an empty directory - a different code path (read_dir returns Ok with zero entries) from the spec's literal "does not exist" case (symlink_metadata returns NotFound, agents.rs:98-100). Code inspection strongly suggests the untested path is correct (the embedded gate probes <home>/.claude directly and independently of the agents subdirectory's existence, roots.rs:90-92), but this is inference, not test evidence, for the exact scenario the spec names. Recommend adding one fixture (e.g. claude-dir-only/ with .claude/.gitkeep and no agents/ subdirectory) plus a correctly-scoped test before this is archived as fully spec-complete, or renaming the existing test to accurately describe what it covers and explicitly noting the remaining gap in tasks.md/apply-progress.md.

SUGGESTION:
1. design.md 5.1's code sketch - agent_roots calling resolve_single twice with an unmodified signature - no longer matches the implementation (see Scrutiny Item 1). The deviation is well-documented in apply-progress.md's "Deviations from Design" section and is mechanically forced, not a design-intent change, so this is cosmetic. Still, a reader who consults only design.md (not apply-progress.md) would be misled about the actual roots.rs shape. Consider a one-line amendment to 5.1 before archive, per this project's own convention of keeping design.md as a faithful as-built record.
2. As of this verify pass, crates/vertice-core/src/agents.rs, tests/agent_scanner.rs, tests/fixtures/roots/agents/**, and the whole openspec/changes/2026-08-18-claude-code-agent-adapter/ folder are untracked (?? in git status), and roots.rs/lib.rs are modified-but-unstaged. Not a defect - the task context states this work is intentionally uncommitted - but noted so the .gitkeep tripwire's protective value (Scrutiny Item 5) is understood to begin only once these files are actually staged and committed.

---

## Skill Resolution

No registry skill in .atl/skill-registry.md matched Rust/Cargo verification work; none was loaded. Verification proceeded using the sdd-verify phase skill and the shared sdd-phase-common protocol only, with terminal-based Rust gate execution (cargo fmt/clippy/test/deny) and direct source/fixture inspection substituting for a missing stack-specific skill. Engram MCP tools (mem_search/mem_save) were unavailable in this session per the task's explicit instruction; this report was written directly to the change folder instead, matching the two prior archived cycles' file-based fallback shape.
