# Verify Report: add-codex-client-support

Date: 2026-08-24
Verifier: sdd-verify (openspec artifact store)

## Verdict

PASS WITH FINDINGS

0 CRITICAL, 2 WARNING (1 resolved pre-archive, 1 open), 1 SUGGESTION (open).

## Gates

Independently re-run in this session:

- frontend/ npm run lint: PASS (eslint, no output).
- frontend/ npm run check: PASS (svelte-check, 206 files, 0 errors, 0 warnings).
- frontend/ npm run test: PASS (vitest, 10 files / 96 tests, all green). Run from frontend/,
  no stray node_modules created.
- frontend/ npm run build: PASS (vite build, dist/ produced, 418ms).
- cargo deny check bans licenses (PATH-prefixed with ~/.cargo/bin, per known local-toolchain
  gotcha): PASS -- "bans ok, licenses ok". Two license-not-encountered warnings for
  BSD-2-Clause/ISC allow-list entries are pre-existing and unrelated to this change (no crate
  in the new toml_seam dependency tree uses either license).
- git diff --stat frontend/src/bindings/: confirms only ClientKind.ts has a real content
  diff (+4/-4 lines -- three-variant union); the other three files carry only the CRLF-vs-LF
  warning, no content diff.

Trusted from the reporting agent's independent pre-verification pass, not re-run here:
cargo fmt --all --check (PASS), cargo clippy --workspace --all-targets -- -D warnings (PASS),
cargo test --workspace --locked (PASS, 0 failures), no tauri reference in
crates/vertice-core, no File::create/OpenOptions in crates/vertice-core/src, toml_seam
confined to src/toml.rs, reference fixture trees byte-identical, end-to-end run against the
real machine's Codex CLI 0.149.0 install.

## Task completeness (48/48 claimed done)

Spot-checked against source, not merely trusted:

- Phase 1 (ClientKind::Codex): confirmed in model/installation.rs; ClientKind.ts regenerated
  correctly.
- Phase 2 (toml.rs seam): confirmed -- TomlError with one #[from] variant, from_str, no
  serializer exposed, toml_seam aliasing in Cargo.toml, toml_seam_invariant.rs present.
- Phase 3 (CodexStandalone slot, split_release_dir_name, CODEX_TARGET_TRIPLES): confirmed --
  see "Scrutiny" below for the longest-suffix-match verification.
- Phase 4 (codex-skills root, skill_roots 3 to 4): confirmed in roots.rs; doc comment says
  "four" throughout; codex-skills is index 3 as claimed.
- Phase 5 (codex_agents.rs adapter): confirmed -- flat walk, no embedded pseudo-root, no
  escalate, per-file continue isolation, no regex (source-inspection test present).
- Phase 6 (orchestrator wiring): confirmed -- scan.rs calls codex_agents::scan, three
  extends appended after the OpenCode-agent adapter.
- Phase 7 (ROOT_ORDER 6 to 8): confirmed -- 8-entry array, codex-skills at index 3,
  codex-agents last; pinning test extended with one push, matches design section 6.2 exactly.
- Phase 8 (reference-fixture tripwire): confirmed -- three layers all present, see "Scrutiny"
  below.
- Phase 9 (read-only audit): confirmed -- disk surface is exactly symlink_metadata, read_dir,
  DirEntry::file_type, read_to_string; no symlink in any new fixture (.gitkeep markers
  used throughout the empty/single-entry Codex release directories).
- Phase 10 (gates): all re-run above, all pass.

No task marked [x] was found to be hollow. The 48/48 claim is truthful.

## Spec-to-test traceability

Walked all six delta specs' scenarios against the actual test suite.

### domain-model, workspace-architecture, client-installation-detector

Every scenario in these three deltas maps to a real, passing test:

- ClientKind exhaustive match: tests/model_contract.rs.
- toml.rs sole-importer containment: tests/toml_seam_invariant.rs.
- MSRV/license floor: V1b verified by the reporting agent's pre-verification pass (cargo tree),
  and re-confirmed here by cargo deny check bans licenses passing with deny.toml untouched.
- Four-slot presence record, NotDetected-never-Error-on-absence, CA-7 (never merged),
  release-directory-name version extraction, prerelease safety, unparseable-name error path: all
  covered in tests/client_installations.rs's codex-installations/* fixture-driven tests
  (single-release, two-releases, prerelease, unknown-triple, empty-releases,
  stale-version-json, nothing).

Exception -- "A Malformed Codex Candidate Does Not Block Other Slots" (WARNING, see Findings
#2): the spec's own scenario requires "Claude Code npm, Claude Code bundled, and OpenCode npm
are all well-formed" alongside the broken Codex slot. No fixture combines a broken Codex release
name with well-formed sibling slots; codex-installations/unknown-triple/ contains only a
.codex/ tree, so the other three slots are trivially absent (NotDetected), not "well-formed
and detected". Isolation is real by construction (resolve_slot is called once per slot inside
an independent loop iteration in scan_for), but the spec's literal GIVEN clause has no covering
fixture.

### skill-scanner

Every scenario covered: fourth-root resolution, extra-keys-ignored parsing, root-ordering
(codex-skills last of four, existing three unchanged), and the untouched 69/25/22/3 reference
pins plus the new negative-existence tripwire.

### codex-agent-scanner

Every scenario except one maps cleanly:

- Flat discovery / nested-not-discovered / non-.toml-ignored:
  flat_discovery_ignores_nested_files_and_non_toml_siblings.
- Absent/empty root: absent_root_yields_zero_components_and_zero_issues,
  empty_root_yields_zero_components_and_zero_issues.
- Component assembly shape: component_assembly_shape_is_agent_user_one_file_location.
- No-regex: source_does_not_use_regex_to_parse_toml_content.
- Per-file isolation (CA-12): malformed_and_missing_name_files_are_isolated_from_valid_siblings.
- Read-only (CA-16): full_scan_leaves_the_fixture_tree_unchanged.

Gap -- "A genuine multiline developer_instructions value is returned complete" (WARNING, see
Findings #1): the spec's scenario literally says "GIVEN a fixture Codex agent .toml file
whose developer_instructions key is a triple-quoted multiline string ... WHEN the file is
parsed ... THEN the resulting value is the complete, unmodified multiline string." The fixture
built for exactly this purpose
(tests/fixtures/roots/codex-agents/complete/.codex/agents/planner.toml, containing a multiline
value with an embedded blank line) is never read for its developer_instructions value by any
test -- codex_agent_scanner.rs only checks that planner is discovered (its name and
description), and the multiline-preservation assertion instead lives in tests/toml_behavior.rs,
against a hand-written inline string literal, not against the fixture file. design.md section 12
names this exact test (codex_agent_with_multiline_developer_instructions_yields_the_complete_value)
as the number-1 load-bearing RED test to write "before any implementation" -- it does not exist
under that name or any equivalent that reads the fixture. The underlying seam behavior is
genuinely proven (the inline-literal test is real and passes), so this is not a functional
defect, but the spec scenario as written, and the fixture purpose-built for it, are not actually
exercised together.

### scan-orchestration

Every scenario covered, including the two apply-executor-flagged extra fixture homes
(codex-claude-same-skill, corrupt-codex-agent) -- both are sound additions: design section 10.3's
table is scoped to per-adapter fixtures, not orchestrator-level cross-adapter scenarios, so these
two fixtures were necessary to test scan-orchestration's own added scenarios and are not
scope creep.

## Scrutiny items

1. CODEX_TARGET_TRIPLES extraction -- CONFIRMED correct. split_release_dir_name
   (installations.rs:196-206) iterates the closed 2-entry table and calls
   name.strip_suffix(&format!("-{triple}")) per entry, returning the first non-empty match.
   This is a genuine longest-known-suffix strip against a table, not a split-on-first-hyphen: for
   0.150.0-rc.1-x86_64-pc-windows-msvc, strip_suffix("-x86_64-pc-windows-msvc") correctly
   yields 0.150.0-rc.1, verified by the passing test split_release_dir_name_is_prerelease_safe.
   Since neither triple in the table is a suffix of the other, iteration order does not create
   an ambiguity the design didn't anticipate.

2. Failure path -- CONFIRMED. An unrecognised release directory name (unknown-triple
   fixture) yields Detected + 0 installations + 1 Error ScanIssue carrying the directory's
   path (codex_installation_from_release_dir, installations.rs:676-708), verified by
   unknown_triple_yields_detected_zero_installations_and_one_error_with_its_path.

3. CA-11 vs CA-7 -- CONFIRMED. Absent Codex slot to NotDetected
   (home_without_codex_yields_not_detected_and_zero_issues, and the isolation-fixture reuse
   test below). Multiple release directories to N independent, unmerged installations
   (two_release_directories_yield_two_unmerged_installations).

4. Isolation, adapter and orchestrator layers -- CONFIRMED. codex_agents.rs's per-entry loop
   continues on every failure arm before pushing to components; tested by
   malformed_and_missing_name_files_are_isolated_from_valid_siblings (adapter layer) and
   malformed_codex_agent_does_not_abort_the_scan (orchestrator layer, scan.rs).

5. ROOT_ORDER -- CONFIRMED. 8 entries, codex-skills at index 3 (before claude-agents),
   codex-agents last. root_order_matches_the_roots_module_in_order builds its expectation
   from roots::skill_roots ++ roots::agent_roots ++ roots::opencode_agent_root ++
   roots::codex_agent_root, so it is pinned to roots.rs, not hand-asserted -- a "Codex is
   last" test would indeed have been wrong, and this test is not that.

6. Reference-fixture tripwire -- CONFIRMED, all three layers present. (1) The 69/25/22/3
   pinning tests are textually unmodified. (2) V5 (silent-return on absent root) is structural,
   not merely observed -- codex_agents.rs and skills.rs both return early on
   ErrorKind::NotFound before pushing any issue, so a reference/.codex/skills that doesn't
   exist cannot move the counts. (3) reference_fixture_has_no_codex_directory asserts
   !reference/.codex.exists(); reasoning through it: if a future contributor added
   reference/.codex/skills/some-skill/SKILL.md, this assertion fails immediately with a named
   message, before the 69/25 counts would otherwise silently drift -- the tripwire does what it
   claims.

7. SkillFrontmatter stayed permissive -- CONFIRMED. grep -rn "deny_unknown_fields"
   crates/vertice-core/src/ returns no matches.

8. Two self-flagged apply-executor items -- both judged sound.
   (a) Reusing the isolation fixture (predates Codex, has no .codex/ tree) to assert the
   Codex slot resolves NotDetected while the other three resolve Detected (broken-or-not) is
   a legitimate, low-cost reuse -- it doesn't need a dedicated fixture since "no .codex/
   anywhere" is exactly what nothing/absence already tests, and layering it onto isolation
   additionally proves Codex's absence doesn't perturb the other three's pre-existing statuses.
   (b) The two extra orchestrator fixtures beyond design section 10.3's table are justified --
   see the "scan-orchestration" traceability note above.

9. Platform split -- CONFIRMED. grep -n "cfg\|HostPlatform" crates/vertice-core/src/roots.rs
   shows no platform branch; codex_agent_root/skill_roots' codex-skills entry both resolve
   unconditionally via resolve_single, exactly like every other component root. Only
   installations.rs's HostPlatform::current() gates installation detection.

## Findings

### WARNING #1 -- Spec scenario "genuine multiline developer_instructions" not exercised via its purpose-built fixture

codex-agent-scanner spec's own scenario requires the assertion to run against "a fixture Codex
agent .toml file". tests/fixtures/roots/codex-agents/complete/.codex/agents/planner.toml was
built for exactly this (per design.md section 10.3's fixture table and task 5.1), but no test
reads its developer_instructions value -- the actual byte-exact-preservation assertion lives
only in tests/toml_behavior.rs against an inline string literal. design.md section 12 names a
specific test, codex_agent_with_multiline_developer_instructions_yields_the_complete_value, as
the number-1 load-bearing RED test; it does not exist. Functionally low-risk (the seam behavior
is proven generically, and CodexAgentDocument is a thin pass-through), but it is a real
traceability gap: a future regression in CodexAgentDocument's field-level deserialization (e.g.
an accidental #[serde(skip)] or a typo'd field name) would not be caught by any test that
exercises the real adapter path end-to-end with a real multi-line fixture file.

Suggested fix: add an assertion in codex_agent_scanner.rs that parses planner.toml through
crate::toml::from_str::<CodexAgentDocument> (or exposes the value some other way) and checks the
multiline string is complete and byte-exact, including the embedded blank line.

**RESOLVED (2026-08-24, pre-archive).** The named test now exists at
`crates/vertice-core/tests/codex_agent_scanner.rs::codex_agent_with_multiline_developer_instructions_yields_the_complete_value`,
under exactly the name design.md section 12 specified. It reads
`tests/fixtures/roots/codex-agents/complete/.codex/agents/planner.toml` from disk, parses it
through the `toml.rs` seam into `CodexAgentDocument`, and asserts `developer_instructions`
byte-exactly (`"You are a planning agent.\n\nFollow these steps:\n\n1. Read the request.\n2.
Draft a plan.\n"`), plus `name` and `description`. It passes. `.gitattributes` marks the
fixture tree `-text`, so those bytes are LF on every platform and the assertion is not
line-ending-fragile. Gates re-run after adding it, all green: `cargo fmt --all --check` PASS,
`cargo clippy --workspace --all-targets -- -D warnings` PASS, `cargo test --workspace --locked`
PASS (21 test binaries, 0 failures). This traceability gap is closed.

### WARNING #2 -- Spec scenario "malformed Codex candidate does not block other slots" has no fixture combining a broken Codex slot with well-formed siblings

The client-installation-detector delta's scenario explicitly requires "Claude Code npm, Claude
Code bundled, and OpenCode npm are all well-formed" alongside the unparseable Codex directory.
No fixture under tests/fixtures/client-installations/codex-installations/ includes npm/bundled
installations; unknown-triple/ contains only a .codex/ tree. Isolation is structurally
guaranteed by the code shape (resolve_slot runs once per grouped probe, independently, inside
scan_for's .map()), so the functional risk is very low, but the spec's literal GIVEN clause is
untested as written.

Suggested fix: either add npm/bundled fixture content to codex-installations/unknown-triple/
(or a new combined fixture), or add a single integration test that composes an existing
well-formed slot's segments with a broken-Codex .codex/ tree under one temp/fixture home.

### SUGGESTION -- Reference-tripwire layer 2 has no explicit unit test, only a documented proof

Tripwire layer 2 (V5: absent reference/.codex/skills produces zero issues structurally) is
verified in this report by reading skills.rs/codex_agents.rs's early-return-on-NotFound code
paths, per task 8.4's "verification only, no code change" instruction -- which is compliant with
design.md section 10.1. This is not a defect, but a cargo-test-visible assertion (e.g. an
explicit walk_one/walk_agents_root unit test on a nonexistent path returning empty results)
would make this guarantee self-verifying on every CI run rather than resting on source-reading.
Low priority; the existing absent_root_yields_zero_components_and_zero_issues test in
codex_agent_scanner.rs already covers the Codex-adapter half of this claim.

## Recommendation

Neither WARNING blocks archive on its own -- both are traceability gaps around already-proven
underlying behavior, not evidence of a functional defect. WARNING #1 has been fixed and resolved
pre-archive (see the RESOLVED note above). WARNING #2 remains open and is lower priority given the
structural isolation argument, but should be tracked if a follow-up touches
installations.rs's slot-resolution loop. The SUGGESTION also remains open, low priority. Both are
carried forward as known follow-ups in the archive report.
