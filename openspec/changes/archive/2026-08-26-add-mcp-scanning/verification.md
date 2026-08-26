# Verification Report: add-mcp-scanning

Date: 2026-08-26
Verifier: sdd-verify (openspec artifact store)

## Verdict

PASS WITH FINDINGS

0 CRITICAL, 1 WARNING, 2 SUGGESTIONS.

## Gates -- independently re-run in this session, real output

- `cargo fmt --all --check`: PASS (no output).
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS (`Finished` profile, zero warnings).
- `cargo test --workspace --locked`: PASS. 29 test binaries, every `test result: ok`, 0 failed.
  One test is `ignored` (`freshness_live_upstream_endpoints_still_match_the_documented_shape`,
  pre-existing, network-hitting, excluded from CI by design -- unrelated to this change).
- `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses`: PASS -- "bans ok, licenses ok".
  Same two pre-existing `license-not-encountered` warnings (BSD-2-Clause, ISC) and one
  `unused-wrapper` warning as the prior cycle; none touch this change's dependency tree (no new
  dependency was added -- confirmed below).
- `frontend/` `npm run lint`: PASS (eslint, no output).
- `frontend/` `npm run check`: PASS (svelte-check, 225 files, 0 errors, 0 warnings).
- `frontend/` `npm run test`: PASS (vitest, 14 files / 124 tests, all green).
- `frontend/` `npm run build`: PASS (vite build, `dist/` produced, 649ms).
- `git diff --stat Cargo.toml Cargo.lock deny.toml crates/vertice-app/capabilities/default.json`:
  empty -- no new dependency, no capability change, confirming the "no new Rust dependency, no new
  Tauri command" claim structurally, not just by assertion.
- `git status --porcelain crates/vertice-core/tests/fixtures/roots/reference/`: empty -- the
  69/25/22/3 reference-fixture pins are byte-identical, confirmed rather than merely re-read.

All gates the proposal's success criteria list are green. All frontend gates pass, including
`npm run check`, which contradicts `tasks.md` task 8.6's own record (see Finding WARNING-1
below) -- the current working tree is in a better state than the last thing `tasks.md` says about
it.

## Task completeness (105/105 claimed done)

Spot-checked against source, not merely trusted. Slices 1-8 were walked; the following were
independently confirmed by reading source rather than accepting the checkbox:

- **Slice 1 (model)**: `McpTransport` in `model/mcp.rs` is exactly the two-variant, value-free
  shape design section 2 specifies; `ComponentKind::Mcp`, `SearchRootKind::Mcp`,
  `Location.mcp_transport` all present; `identity_prefix` gains exactly one arm
  (`ComponentKind::Mcp => "mcp"`).
- **Slice 2 (`mcp.rs`/`json_merge.rs`)**: `sanitize_url` re-derived by hand against the seven-step
  rule and tried against IPv6, multiple `@`, `@`-in-query-only, empty authority, `scheme://@host`,
  `scheme://host@`, backslash, and `%40` -- every case matches design's documented expected output
  (detail in "Security property" below). `Lenient<T>`'s `visit_seq`/`visit_map` fully drain via
  `IgnoredAny` before returning `WrongType` -- the round-3 MapAccess-corruption defect (b) is
  confirmed still fixed, matching the reverted-to-original shape `tasks.md` 2.6.4 describes.
- **Slice 3-5 (three adapters)**: all four MCP modules carry
  `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]`; `clippy -D
  warnings` passing confirms this is enforced, not aspirational.
- **`ROOT_ORDER`**: 11 entries, and `root_order_matches_the_roots_module_in_order` builds its
  expectation by calling `roots::claude_mcp_root`/`opencode_mcp_root`/`codex_mcp_root` (plus the
  eight pre-existing calls) rather than hand-asserting the array -- pinned to `roots.rs`, exactly
  as V5/section 5.3 require.
- **Slice 7 (reference tripwire)**: confirmed directly, not re-read from the report -- `git status
  --porcelain` on `tests/fixtures/roots/reference/` is empty.
- **Slice 8 (gates)**: re-run independently above. Task 8.6 is the one place where the ticked
  record and the current code state disagree -- see Finding WARNING-1.

No other ticked task was found hollow.

## Spec-to-test traceability

Walked all four spec files' requirements against the actual test suite.

### `mcp-scanner`

Every requirement maps to a real, passing test:

- Key-names-only (`env`/`headers`): `fake_token_in_env_never_reaches_the_serialized_report`,
  the per-client `stdio-secret`/`remote-secret` scanner tests, and
  `fake_guard_holds_across_the_full_secret_bearing_fixture_tree` (whole-tree `FAKE` guard).
- URL sanitization (userinfo/query/fragment stripped, ambiguous authority rejected):
  `dirty_url_is_reduced_to_scheme_host_and_port`,
  `userinfo_containing_a_path_delimiter_is_rejected_not_truncated`, the 21
  `sanitize_url`-related unit tests in `mcp.rs`, and `claude/remote-userinfo-ambiguous-url`'s
  scanner-level test.
- `args` count-only: `token_bearing_argument_yields_only_a_count`,
  `opencode_array_command_maps_to_command_plus_arg_count`,
  `codex/args-non-string-element`'s scanner test.
- Transport on `Location`, one component per server name:
  `same_server_name_in_three_clients_yields_one_component_with_three_transports`, and the
  conservation property (sum of location counts equals input length) it also asserts.
- Identity unchanged: covered structurally -- `identity.rs`'s only touched line is the one match
  arm; no fixture-level identity test was strictly necessary and the three-client consolidation
  test above exercises it as a side effect (see SUGGESTION-2).
- Disabled-but-emitted: `disabled_flagged_entries_are_still_emitted_with_no_provenance_hint`
  against `shared/disabled-flagged`, using the machine-verified `enabled: bool` shape (M7/M9),
  not an invented flag.
- Malformed/wrong-typed degrade, never abort: `malformed_config_yields_one_error_with_a_fixed_
  reason_and_no_parser_text`, per-client `root-key-wrong-type`/`entry-wrong-type` tests, and the
  "unusable command falls back to Remote" requirement closed by all 9 matrix-cell unit tests
  (`matrix_command_*`) plus the three per-client `entry-unusable-command-valid-url` fixtures
  (Claude/OpenCode/Codex -- E3 fully closed, not just prose-claimed).
- Absent/empty root produces nothing: `home_without_any_mcp_configuration_yields_no_components_
  and_no_errors` against `shared/no-mcp-anywhere` -- CA-11.
- User-scope-only: asserted per-adapter (`scope: Scope::User` construction is unconditional in
  all three `mcp_*.rs` files -- confirmed by reading the `assemble` phase of each).
- No `FreshnessCheck` entry: no `FreshnessSubject` variant exists for MCP (confirmed --
  `model/freshness.rs` is untouched per `git diff --stat`), so this is closed structurally rather
  than by a runtime assertion; acceptable given the enum is closed and unmodified.
- Seam containment: `no_mcp_module_imports_the_jsonc_or_toml_crate_directly` (the fourth test in
  `mcp_no_error_interpolation_invariant.rs`), plus the pre-existing `yaml_seam_invariant.rs`-style
  coverage extended for free by the same containment class.
- Fixture-tree distinctness and per-requirement traceability: `fixtures/mcp/` is a wholly new tree
  distinct from `fixtures/roots/`; design section 10.4's fixture-to-requirement map was walked and
  every row has a corresponding test (see "Design corrections" below for the one row,
  `claude/empty-root-key`, that needed E4's late trace -- now closed).

### `domain-model`

- `McpTransport` closed, value-free, exhaustively matchable: confirmed by direct source read
  (above) plus `crates/vertice-core/tests/model_contract.rs`'s exhaustive-match tests.
- `SearchRootKind` mirrors `ComponentKind` 1:1 for three kinds: both enums now have exactly three
  variants with matching names -- confirmed by reading `component.rs`/`location.rs` directly.
- `Location.mcp_transport` optional, kind-conditional, with the degraded-entry carve-out: the
  carve-out text is present in `specs/domain-model/spec.md`'s "Location Carries An Optional,
  Kind-Conditional Transport" requirement exactly as design section 0.6 describes it was
  resolved.
- `provenance_hint` opacity re-affirmed: `mcp_claude.rs`/`mcp_opencode.rs`/`mcp_codex.rs`'s
  `assemble` phases construct `provenance_hint: None` unconditionally (confirmed by the same
  read as the scope check above); `disabled_flagged_entries_are_still_emitted_with_no_
  provenance_hint` pins it against a fixture using the real `enabled` flag.
- TypeScript contract: `ComponentKind.ts`, `SearchRootKind.ts`, `Location.ts`, new `McpTransport.ts`
  all differ as expected (`git status`); `Component.ts` carries only a doc-comment collateral
  diff from the widened doc comment (expected, not a leak). No other binding changed in content
  (`UserSettings.ts`'s diff is CRLF-only noise, confirmed above).

### `scan-orchestration`

- Fourth adapter class wired into `scan_for`; `mcp-same-name-three-clients` orchestrator fixture
  and its test close the "same-named MCP server across three clients" scenario at the orchestrator
  level (not just the adapter level) -- a genuine, non-redundant addition beyond the per-adapter
  test.
- Malformed-MCP-does-not-abort: covered per-client at the adapter level; no dedicated
  orchestrator-level fixture combines a malformed MCP file with well-formed siblings from *other
  adapter classes* (skills/agents) in one home -- see Finding SUGGESTION-1 below, a minor gap in
  the same shape as the prior Codex cycle's WARNING-2 finding.
- Logging never mutates `ScanReport`/`ScanIssue`, never leaks a secret: closed by
  `mcp_secrets_never_reach_the_scan_report_log_across_the_full_fixture_tree` in
  `crates/vertice-app/src/commands.rs`'s test module, exercising 8 secret-bearing fixture homes
  through `log_scan_report_with`'s capturing closure.

### `workspace-architecture`

- Sole-importer containment for a third consumer class: `no_mcp_module_imports_the_jsonc_or_
  toml_crate_directly` plus the full `cargo test --workspace` pass.
- No secret crosses the public surface: closed by the `McpTransport` shape (structural) plus the
  whole-tree `FAKE` guard (empirical) -- both independently confirmed above.

## Design section 6.3's 3x3 matrix -- all nine cells implemented and independently tested

Confirmed by direct source read (`crates/vertice-core/src/mcp.rs:718-845`) that all nine
`matrix_command_{absent,usable,unusable}_url_{absent,valid,unsanitizable}` unit tests exist and
pass, and that `discriminate_transport`'s implementation is the single total enumeration design
section 6.3 specifies (not restated piecemeal). Cross-checked against section 7.1's error
taxonomy table row by row -- no contradiction found between the two, consistent with design's
2026-08-25 alignment note. The two rounds of adversarial-review-found uncovered cells (E1, both
directions) are closed and each direction has its own regression test
(`unusable_command_with_a_valid_url_falls_back_to_remote_not_none*` for one direction,
`matrix_command_usable_url_unsanitizable` for the other).

## The two defects found during the cycle -- verified still fixed

**(a) `sanitize_url`'s userinfo/authority ordering leak.** Read the current implementation
(`crates/vertice-core/src/mcp.rs:56-152`) line by line and independently re-derived its behavior
against the design's documented worst case and several not explicitly enumerated there:

| Input | Expected (design) | Traced result |
|---|---|---|
| `https://tok3n/@host.example/mcp` | `None` | tail `/@host.example/mcp` contains `@` -> rejected. Matches. |
| `https://a@b@host/path` (multiple `@`) | not explicitly tabled | tail has no `@`; `rsplit_once('@')` on the candidate authority takes the last `@`, yielding host `host` -- correct per step 5's "last `@`" rule. |
| `https://host/path?x@y` (`@` in query only) | not explicitly tabled | candidate authority `host` (cut at `/`), tail `/path?x@y` contains `@` -> rejected, consistent with the design's stated "never guess" invariant. |
| `https://@/x` | `None` | authority resolves to empty string after stripping userinfo -> rejected by the empty-host check. |
| `scheme://@host` | not explicitly tabled | userinfo-free authority = `host` -> accepted as `scheme://host`, correctly treating an empty userinfo as absent. |
| `scheme://host@` (trailing `@`, empty tail) | not explicitly tabled | userinfo-free authority = empty string -> rejected by the empty-host check. |
| host containing `\` | rejected (section 3.1 step 6's forbidden-char list) | `is_forbidden_host_char` includes the backslash -> rejected. Matches. |
| `%40` in place of a literal `@` | passes through unmodified (documented residual) | the function operates on literal `@` bytes only; `%` is not in the forbidden-char set -> passes through, matching the documented, citation-free assumption. |

No leak reproduced in any tried case. The defect stays fixed.

**(b) `Lenient<T>`'s seq/map retry not draining `MapAccess`.** Confirmed by direct source read
(`crates/vertice-core/src/mcp.rs:334-348`): `visit_seq`/`visit_map` unconditionally fully drain
via `next_element::<IgnoredAny>()`/`next_entry::<IgnoredAny, IgnoredAny>()` before returning
`Lenient::WrongType` -- there is no retry-into-`T::deserialize` path over seq/map shapes, exactly
the reverted-to-original form `tasks.md` 2.6.4 records. `mcp_codex.rs`'s three dedicated,
hand-rolled seq-/map-shaped types (`McpServersField`, `CodexEntrySlot`,
`LenientArgCount`/`LenientKeyNames`) were spot-checked to confirm they exist as a separate
mechanism, not a reintroduction of the generic retry. The defect stays fixed.

## Security property -- verified adversarially

- `McpTransport` genuinely has no field capable of holding a value (confirmed above -- `model/
  mcp.rs` read directly).
- `sanitize_url` matches design's seven steps exactly and resists every hand-tried break attempt
  (table above).
- `ScanIssue.reason` interpolation is mechanically bounded to a 6-name allow-list (server key,
  client label, path, spelled multiple ways) by `mcp_no_error_interpolation_invariant.rs`, which
  is structural (parses every `format!`/`write!` call's format string) rather than a literal
  `{err}` grep -- closes exactly the defeat class (renaming the bound identifier) that a literal
  grep would miss.
- No unguarded indexing or `.unwrap()`/`.expect()` over a redact-phase value: enforced at compile
  time by `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` in all
  four MCP modules (confirmed present in each file), re-confirmed textually by the invariant
  test's second assertion, and independently confirmed by `cargo clippy -D warnings` passing
  clean.
- Fixtures carry fake secrets containing `FAKE`: 13 fixture files matched by `grep -rl FAKE`
  across `tests/fixtures/mcp/`; a targeted grep for plausible-looking secret patterns (`ghp_`,
  `Bearer sk-`, hex-looking tokens) without `FAKE` found zero matches. No fixture was authored
  from a real config file -- this cannot be independently verified from the repository alone, but
  design section 0.5's authorization record and the shape-only (no-value) methodology are
  consistent with the synthetic vocabulary actually committed.

## Structural invariants

- `model/` purity: `model/mcp.rs` imports only `serde` and `ts_rs` -- within the declared allow-
  list. No `std::fs`/`std::io`/`std::env`/clock in `model/`.
- Only `jsonc.rs`/`toml.rs` import their format crates: confirmed by
  `no_mcp_module_imports_the_jsonc_or_toml_crate_directly` passing, plus `cargo test --workspace`
  keeping the two pre-existing seam-invariant tests green.
- Read-only invariant (CA-16): `git status --porcelain` on the reference fixture tree is empty;
  `cargo test --workspace` includes `read_only_audit.rs`'s mechanical audit, green.
- No new dependency: `Cargo.toml`/`Cargo.lock`/`deny.toml` byte-identical (`git diff --stat`
  empty); `cargo deny check bans licenses` passes.
- `ROOT_ORDER` matches its pinning test: confirmed above, built by calling `roots.rs` functions,
  not hand-asserted.
- ts_rs bindings in sync, no stale orphan: `cargo test -p vertice-core` was already run as part
  of `cargo test --workspace` above and produced no working-tree diff beyond what `git status`
  already shows (`ComponentKind.ts`, `SearchRootKind.ts`, `Location.ts`, `McpTransport.ts`,
  `Component.ts`'s doc-only line, `UserSettings.ts`'s CRLF noise) -- no new orphan `.ts` file
  appeared.
- `crates/vertice-app/` and `capabilities/default.json`: byte-identical except test-only
  additions inside `#[cfg(test)]` blocks in `commands.rs` -- confirmed by reading the file's
  diff shape (the 8-fixture whole-tree guard and the earlier single-fixture anchor are both
  inside `#[cfg(test)] mod tests`).

## Known accepted gaps -- confirmed still true and correctly scoped

- Freshness is a non-goal for MCP: `FreshnessSubject` is unmodified (`model/freshness.rs` has no
  diff); no MCP component can appear in a `FreshnessCheck` because there is no variant to build
  one from.
- Enabled/disabled not modeled, disabled servers still emitted: confirmed by
  `disabled_flagged_entries_are_still_emitted_with_no_provenance_hint` and by reading that no MCP
  adapter branches on `enabled`/`disabled` anywhere (grep for `"enabled"` inside the three
  adapter files' non-test code found no conditional use).
- User scope only: every adapter constructs `Scope::User` unconditionally (confirmed above).
- Copilot out of scope: no `ClientKind::Copilot` exists; `installations.rs` is untouched
  (`git diff --stat` shows no change).
- Four residual assumptions (A4, A5', A8', A10) ship unconfirmed, as designed: confirmed by
  reading `mcp_opencode.rs`/`mcp_codex.rs` -- an absent `environment`/`headers`/`http_headers` key
  produces no keys and no issue, never an error, matching design section 7.1's "absent map is not
  a fault" row. This makes the four residuals genuinely safe to ship unconfirmed, as claimed.

## Findings

### WARNING-1 -- `tasks.md` 8.6 is stale: it claims `npm run check` fails, but the current working tree passes it, via an undocumented `frontend/src` source change outside this change's own declared scope

`tasks.md` task 8.6 records: "`npm run check` FAILS with 11 type errors... Left unfixed,
deliberately, and flagged here rather than silently reported as a passing gate -- the frontend
cycle needs this before it plans." That is no longer true. `npm run check` passes cleanly in this
session (225 files, 0 errors, 0 warnings), because the current working tree already contains
fixes for exactly those 11 errors:

- `frontend/src/lib/pages/ComponentKindPage.svelte` -- `KIND_ROUTE` gained a third entry,
  `mcp: "mcp"`, satisfying the now three-variant `Record<ComponentKind, RouteId>`. This is a
  real `frontend/src` source-code change, not a binding regeneration.
- `frontend/src/App.test.ts`, `frontend/src/lib/filterComponents.test.ts`,
  `frontend/src/lib/inventory.test.ts` -- every hand-written `Location` fixture literal gained
  `mcpTransport: null` to satisfy the now-required field.

This directly contradicts the proposal's stated scope ("Backend only. ... No `frontend/src/`
source change is planned") and design section 9.1 / section 11's file table ("`frontend/src/`
(source) ... Unchanged -- Separate cycle") and the proposal's own success-criteria checklist
("`frontend/src/` outside `bindings/` is byte-identical"). None of these four files are listed
anywhere in `tasks.md`'s 105 tasks, and no task or design amendment records this change. It is
small, mechanical, and arguably the correct fix -- an unhandled `"mcp"` value was the documented
expected failure mode, and this is precisely how a reader would resolve it -- but as committed it
is undocumented scope creep against an explicit, repeatedly-stated boundary of this change, and
`tasks.md` 8.6's record of the gate result is now factually wrong.

Not CRITICAL: the change is minimal, correct, and does not touch anything security-relevant (no
MCP secret handling in `ComponentKindPage.svelte`). It does not block archive on its own, but it
does mean `tasks.md` cannot be archived as written without a correction, and the "frontend cycle
is a separate cycle" claim in the proposal needs a footnote or amendment before archive, since one
frontend file already changed inside this "backend only" change.

Suggested resolution before archive: either (a) add a task recording this fix explicitly and
correct 8.6's claim, updating the proposal/design's "no frontend source change" language to note
this one minimal, load-bearing exception, or (b) revert the four files to their pre-fix state and
accept that `npm run check` fails as originally documented, deferring the fix to the frontend
cycle as originally planned. Shipping the current state silently, with `tasks.md` still asserting
the opposite, is the part that should not survive to archive.

### SUGGESTION-1 -- No orchestrator-level fixture combines a malformed MCP config with well-formed sibling adapters from a different class (skills/agents)

`scan-orchestration`'s "malformed MCP config does not abort the scan" scenario is well covered
within the MCP adapter family (a malformed Claude config alongside well-formed OpenCode/Codex MCP
configs -- confirmed by the per-client `malformed` fixtures and their scanner tests), and
`scan-orchestrator/complete` proves all four adapter classes coexist when everything is
well-formed. But no fixture combines a malformed MCP file with well-formed skill and agent
fixtures in the same orchestrator home to directly exercise CA-12 isolation across adapter
classes for MCP specifically (the existing `corrupt-skill`/`corrupt-codex-agent` fixtures predate
this change and do not carry MCP content). This is the same shape of gap the prior Codex cycle's
`verify-report.md` WARNING-2 flagged and judged low-risk for the same structural reason: isolation
is guaranteed by `scan_for`'s independent per-adapter `extend` calls (confirmed by reading
`scan.rs`), not by fixture composition, so the functional risk is very low. Low priority,
consistent with the precedent's disposition.

### SUGGESTION-2 -- `identity.rs`'s "unchanged rule" requirement has no dedicated fixture-level identity test beyond the three-client consolidation test

`mcp-scanner`'s "Component Identity Is The Config Key, Unchanged Rule" requirement's own scenario
("two MCP fixtures for the same server key under different clients... both produce the same
`ComponentId`") is exercised only as a side effect of
`same_server_name_in_three_clients_yields_one_component_with_three_transports`, which asserts one
component with three locations -- that necessarily implies matching `ComponentId`s, but no test
independently asserts the `ComponentId` string itself (e.g. `mcp:github`) the way `identity.rs`'s
own unit tests do for skills/agents. Very low priority -- the identity function itself is provably
unchanged (`identity.rs`'s only diff is the one match arm), so there is little independent
behavior left to test, but a one-line assertion on the derived ID string would make the "unchanged
rule" claim self-verifying rather than inferred.

## Recommendation

**WARNING-1 RESOLVED 2026-08-26, in the "document and keep" direction, by explicit user decision.**
`tasks.md` 8.6 was rewritten to record what actually happened instead of the superseded plan: the
11 type errors were fixed with a minimal type unblock, because CI's `frontend` job runs
`npm run check` and leaving it red would have made the branch unmergeable. The amendment to the
"backend only" scope is now stated openly in 8.6, including exactly what was touched
(`ComponentKindPage.svelte`'s `KIND_ROUTE` gains `mcp: "mcp"`; three test files' `Location`
fixtures gain `mcpTransport: null`) and what was deliberately NOT touched (no MCP page, no
transport rendering, no UI for secret key names -- that work is P4 in
`internal-docs/pendientes-desarrollo.md`). The task record and the code state now agree.

With that reconciled, this change **is ready for `sdd-archive`**. Both SUGGESTIONs are
non-blocking and carry forward as known follow-ups.

Every other check in this report -- the two previously-found defects, the 3x3 matrix, the security
property, the structural invariants, and all four spec files' requirement-to-test coverage -- is
clean.
