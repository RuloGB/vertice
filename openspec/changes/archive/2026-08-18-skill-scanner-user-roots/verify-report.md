# Verify Report: skill-scanner-user-roots (T4)

**Change**: `skill-scanner-user-roots`
**Branch**: `feat/t4-skill-scanner-user-roots` (5 commits ahead of `main`, not pushed)
**Verified**: 2026-08-18, by `sdd-verify`
**Strict TDD**: active (`openspec/config.yaml` -> `strict_tdd: true`)

## Overall Verdict: **PASS**

All 10 skill-scanner spec requirements and the domain-model delta are implemented and covered by
passing runtime tests.

---

## Gate Results (re-run independently, real output)

| Gate | Command | Result |
|---|---|---|
| Rust fmt | cargo fmt --all --check | PASS - no output |
| Rust lint | cargo clippy --workspace --all-targets -- -D warnings | PASS - Finished dev profile, 0 warnings |
| Rust tests | cargo test --workspace --locked | PASS - 84 total: 41 lib, 14 frontmatter_reader, 8 model_contract, 13 skill_scanner, 7 yaml_behavior, 1 yaml_seam_invariant, 0 doctests |
| Dependency policy | cargo deny check bans licenses | PASS - bans ok, licenses ok (2 pre-existing unrelated license-not-encountered warnings for BSD-2-Clause/ISC, not tied to walkdir) |
| Bindings drift | git diff --stat -- frontend/src/bindings | PASS - no content diff (the 14 M entries in git status are CRLF/LF line-ending noise only, confirmed via git diff on SearchRoot.ts producing zero lines) |
| Frontend lint | npm run lint (from frontend/) | PASS - eslint clean |
| Frontend types | npm run check | PASS - svelte-check: 169 files, 0 errors, 0 warnings |
| Frontend tests | npm run test | PASS - vitest: 1 file, 2 tests passed |
| Frontend build | npm run build | PASS - vite build, built in 189ms |

All gates match the orchestrator's independently-observed results. No disagreement.

---

## Spec Compliance Matrix - specs/skill-scanner/spec.md

| # | Requirement | Verdict | Evidence |
|---|---|---|---|
| 1 | User Root Set Is Fixed and Hardcoded | PASS | roots.rs:61-67 hardcodes .claude/skills, .agents/skills, .config/opencode/skills via per-segment PathBuf::push (roots.rs:70-73, 94-102); no dirs/config-dir API imported anywhere in roots.rs. Covered by opencode_root_resolves_under_home_never_a_platform_config_dir (skill_scanner.rs:57-72, ok) and singular_and_plural_opencode_roots_are_one_logical_root (skill_scanner.rs:77-90, ok) |
| 2 | SKILL.md Presence Is the Sole Detection Rule | PASS | skills.rs:112 matches only entry.file_name() equal to SKILL.md, no name heuristic. Covered by underscore_prefixed_directory_is_an_ordinary_skill (skill_scanner.rs:95-105, ok) |
| 3 | Traversal Is Recursive | PASS | WalkDir::new(scan_path) unbounded depth (skills.rs:92-95). Covered by nested_skill_two_levels_deep_is_discovered (skill_scanner.rs:110-116, ok) |
| 4 | Symbolic Links Are Not Followed | PASS | follow_links(false) explicit at skills.rs:93. Covered structurally (no portable fixture, as design section 6 records) by walk_never_follows_symlinks_by_default_walkdir_setting (skill_scanner.rs:123-137, ok) - a determinism/non-duplication proxy test, honestly weaker than a direct symlink fixture but consistent with the design documented limitation |
| 5 | Absent and Empty Roots Produce No Issue and No Component | PASS | walk_one ErrorKind::NotFound branch returns silently (skills.rs:66-68, no issue). Covered by absent_roots_yield_zero_components_zero_issues_all_not_found (skill_scanner.rs:142-154, ok) and present_empty_root_yields_zero_components_zero_issues_and_is_found (skill_scanner.rs:159-177, ok) - both fixtures inspected together satisfies the distinguishable scenario |
| 6 | Every Skill Component Has Scope::User | PASS | skills.rs:133 constructs Scope::User unconditionally, the only value ever constructed in this module. Covered by every_component_is_user_scoped_and_project_decoy_is_excluded (skill_scanner.rs:183-194, ok) |
| 7 | No Plugin-Provided Skill Appears In The Result | PASS | Structural - only the three hardcoded roots are ever walked (roots.rs:61-67); no exclusion filter exists to test, matching the design stated approach. Covered by plugin_decoy_outside_the_three_roots_is_excluded (skill_scanner.rs:200-207, ok) |
| 8 | Per-File Parsing Failures Do Not Abort The Scan | PASS | Err(issue) results in issues.push(escalate(issue)) (skills.rs:141) inside the loop, continue implicit - walk proceeds. Covered by corrupt_skill_yields_an_issue_and_does_not_stop_the_walk (skill_scanner.rs:213-230, ok): 1 component (good) plus 1 issue (path contains broken) |
| 9 | Scanner Performs No Writes | PASS | Grep across crates/ for File::create, OpenOptions, fs::write, create_dir*, remove_* returns zero matches. Covered by full_scan_leaves_the_fixture_tree_unchanged (skill_scanner.rs:235-243, ok), byte-for-byte comparison |
| 10 | Reference Fixture Set Produces Exactly 69 On-Disk Entries | PASS | Independently recounted (not trusting the test alone): find on reference dir for SKILL.md files counts 69; per-root split .claude 23 / .agents 24 / .config 22, matching internal-docs/alcance-poc-vertice.md:74-79 exactly; 25 distinct name: frontmatter values via independent grep and sort -u count. No dedup/consolidation logic exists in skills.rs, so the 69 is genuinely un-consolidated. Covered by reference_fixture_tree_yields_69_entries (skill_scanner.rs:262-275, ok) |

## Domain-Model Delta - specs/domain-model/spec.md

| Requirement | Verdict | Evidence |
|---|---|---|
| SearchRoot Distinguishes Absent From Present | PASS | SearchRootStatus enum with Found and NotFound at model/location.rs:58-64; SearchRoot.status field at location.rs:50; no client-label field on SearchRoot (fields are id, path, kind, status only, location.rs:46-51). All three scenarios covered by dedicated unit tests: absent_search_root_is_constructible_with_not_found_status (location.rs:86-95, ok), search_roots_differing_only_in_status_are_unequal (location.rs:101-116, ok), existing_fields_are_unchanged_in_type_and_value (location.rs:122-137, ok) |

---

## Design Coherence - design.md

| Decision | Verdict | Evidence |
|---|---|---|
| Two sibling modules roots.rs + skills.rs, no crate-root re-export | PASS | lib.rs gains pub mod roots and pub mod skills (per git show 83cb47c diff), confirmed no re-export of roots or skills contents in lib.rs |
| std::env::home_dir() for home resolution | PASS | roots.rs:32 calls std::env::home_dir() directly, no dirs crate import; clippy is clean with no deprecation warning, confirming design section 3 un-deprecation claim at rustc 1.97.1 |
| walkdir for the recursive walk | PASS | skills.rs:11 imports walkdir::WalkDir; cargo deny confirms walkdir resolved with no new crate added to the graph |
| SearchRootStatus closed 2-variant enum (Found, NotFound) | PASS | location.rs:61-64, exactly as designed |
| Uniform Error escalation on every discovered-SKILL.md issue | PASS | escalate() at skills.rs:151-156 unconditionally sets severity to IssueSeverity::Error, path and reason untouched via struct-update syntax. Unit test escalate_maps_every_severity_to_error (skills.rs:178-192) asserts both Warning to Error and Error to Error, path and reason preserved |
| Owned SkillScan struct with roots, components, issues, not a tuple | PASS | skills.rs:26-30, no Serialize or TS derive (correctly matches T3 SkillFrontmatter non-model precedent) |
| home passed as a parameter everywhere except home_dir() | PASS | skill_roots(home: Path) (roots.rs:61) and scan(home: Path) (skills.rs:36) both take home; grep for std::env:: across roots.rs/skills.rs shows exactly one call site (roots.rs:32, inside home_dir()). No test sets or reads an environment variable, confirmed by grep for set_var and env::var in the test files, zero matches |
| Root ids hardcoded, never path-derived | PASS | claude-skills, agents-skills, opencode-skills are string literals at roots.rs:63-65, 111; unit test root_ids_are_stable_and_never_path_derived (roots.rs:157-175) constructs roots under two different home values and asserts identical ids |
| model purity, no std::fs or std::env import | PASS | model/mod.rs:1-20 doc-declares the allow-list; model/location.rs:1-6 imports only std::path::PathBuf, serde, ts_rs::TS, no std::fs, std::env, or SystemTime anywhere in model |
| No regex frontmatter parsing (yaml seam) | PASS | grep for regex across crates/vertice-core/src/ returns only a doc comment (frontmatter.rs:43, describing itself as regex-free), no crate import; yaml_seam_invariant.rs test (only_yaml_module_imports_serde_norway) still passes, unaffected by this change |

### Design section 7 error-taxonomy deviation - investigated, not substantiated

The task brief flagged a claimed apply-time deviation: that skills.rs collapses design section 7 root probe io::Error and entry-level walk error into one code path because no portable fixture separates them. This claim does not match the code. walk_one (skills.rs:60-144) implements these as two distinct branches with two distinct reason strings:

- Root probe (before the WalkDir loop): skills.rs:66-77, reason text "could not inspect search root", path is the scan_path.
- Entry-level walk error (inside the WalkDir loop): skills.rs:96-110, reason text "could not read directory entry", path is the entry or root.

Both correctly report status Found (the root-existence status is computed independently and earlier, in roots::probe, roots.rs:125-131) plus an Error-severity ScanIssue, matching design section 7 table rows exactly. apply-progress.md own Deviations from Design section (lines 127-131) states None, implementation matches design.md sections 4-9, which is consistent with what was found in the source. This flagged deviation appears to be a misdescription in the task brief, not an actual implementation gap. Noted as a SUGGESTION below: this specific claim should be dropped from the next verification brief, or if it originated from a real internal apply-agent note not persisted anywhere found, that note is now inconsistent with the merged code and should be reconciled.

One real, and separately honest, limitation: neither branch (permission-denied root probe, mid-tree unreadable subdirectory) has a portable CI fixture, exactly as design sections 7, 8, and 11 state - Windows cannot easily construct a permission-denied directory the way Unix can with chmod. This is a documented, accepted gap, not a silent one, and matches the design own admission.

---

## Task Completion - tasks.md (28 of 28 claimed complete)

Spot-checked rather than taken on faith:

| Task | Claim | Verified |
|---|---|---|
| 1.1-1.3 | SearchRootStatus plus status field plus regenerated bindings, one commit | Confirmed: git show feb7105 stat touches model/location.rs, model/mod.rs, tests/model_contract.rs, SearchRoot.ts, SearchRootStatus.ts together |
| 1.6a / 2.7 | gitkeep tripwire split across two commits (disk-existence half lands early, status-assertion half lands with roots.rs) | Confirmed: empty_alias_fixture_directory_still_exists_on_disk exists standalone and was the one test that passed in the RED commit; empty_alias_root_status_is_found (roots.rs:191-202) needs resolve_opencode, correctly landed with the GREEN commit |
| 1.7 | 69-entry reference tree, 23/24/22 split, 25 unique names | Independently recounted above, matches exactly |
| 2.1-2.4 | RED to GREEN cycle, real signatures with todo!(), then real bodies | Directly executed: checked out commit 83cb47c in an isolated git worktree, ran cargo test workspace locked, compiles cleanly, 3 roots tests fail via todo!() panic (message: not yet implemented, implemented in task 2.2), not a build break; ran cargo test for the skill_scanner integration file, 12 of 13 fail via todo!() panic (message: not yet implemented, implemented in task 2.4), 1 passes (the tripwire). This is a genuine RED state, not a broken build, and it exactly matches apply-progress.md claimed evidence. GREEN commit 5f13be6 touches only roots.rs and skills.rs, no test file edits, confirming implementation-only, no cheating by loosening assertions |
| 3.5 | Read-only grep, zero write calls | Independently re-ran the grep across all of crates/, not just the new files, zero matches |
| 3.6 | Bindings diff clean | Independently confirmed, git diff on bindings shows zero content lines, only CRLF warnings |
| 3.7 | Frontend gates green | Independently re-ran all four (lint, check, test, build) myself, this pass is genuinely new evidence, not inherited from the apply agent unverified claim |

No task found to be falsely ticked.

---

## Specific Scrutiny Items (from the task brief)

- CA-9 tripwire: genuine and effective. gitkeep files are committed (git ls-files confirms both absent-roots gitkeep and empty-alias opencode skill gitkeep are tracked). The tripwire test asserts the directory existence independent of scanner code, so if the gitkeep were ever lost, this test fails loudly rather than the CA-9 test silently degrading. Confirmed correctly designed and present.
- 69-entry count: independently recounted from disk (not from the test), matches exactly, 23/24/22 split matches alcance-poc-vertice.md. No consolidation or dedup logic found in skills.rs.
- CA-6 / CA-14: no plugin path, no project-scope component possible, structurally, only 3 hardcoded roots are ever walked, Scope::User is the only value constructed in the entire module (single occurrence). Confirmed by both source inspection and passing tests.
- CA-16 read-only: zero write-call matches anywhere in crates/.
- Architecture invariants: vertice-core has zero tauri references; model/ imports only the allow-listed items; roots.rs is the sole environment reader (one std::env:: call site in the whole crate, inside home_dir()).
- No regex frontmatter parsing: confirmed, no regex crate import anywhere in vertice-core.
- Declared design deviation: investigated and found NOT to match the code, see Design Coherence section above. The implementation is more faithful to design section 7 than the task brief framing suggested.
- Strict TDD evidence: directly verified by checking out the RED commit in an isolated worktree and running the real test suite, it compiles and fails via todo!() panics, not a build break, and no implementation code preceded it (commit order: feb7105 model change, then d46965a fixtures, then 83cb47c RED, then 5f13be6 GREEN, then ca05146 docs).

---

## Issues

CRITICAL: none.

WARNING:
1. The task brief claimed design deviation (root-probe vs entry-level error collapse) does not match the actual code, both are implemented as distinct branches. This is a positive finding for the implementation but indicates either a stale or incorrect note somewhere upstream of this verification pass, or a miscommunication in the apply agent self-report that was never written into apply-progress.md (which itself says None for deviations). Recommend reconciling wherever that claim originated before it resurfaces in a future audit.
2. roots::probe() and skills::walk_one() each independently call std::fs::symlink_metadata on the same root path, the root existence and status is computed once in roots.rs to set SearchRoot.status, then re-probed in skills.rs to decide whether to emit a ScanIssue. This is not a correctness bug (verified both branches agree and together satisfy the design section 7 table), but it is a minor duplication of I/O and logic across the two modules that a future refactor could thread through ResolvedRoot instead. Not spec- or design-mandated to avoid, so not blocking.

SUGGESTION:
1. walk_never_follows_symlinks_by_default_walkdir_setting is a determinism and non-duplication proxy test for the symlinks-are-not-followed requirement, not a direct symlink-following test, this is an honest, design-acknowledged limitation (no portable CI fixture), not a gap introduced by the apply.

---

## Skill Resolution

No registry skill in .atl/skill-registry.md matched Rust/Cargo verification work; none was loaded. Verification proceeded using the sdd-verify phase skill and the shared sdd-phase-common protocol only, with terminal-based Rust and npm gate execution and direct source inspection substituting for a missing stack-specific skill.
