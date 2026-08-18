# Apply Progress: Claude Code Agent Adapter (T5)

## Status

Phase 1 (PR 1 — `roots.rs` and fixture tree): COMPLETE.
Phase 2 (PR 2 — `agents.rs` module): COMPLETE.
Phase 3 (verification gates): COMPLETE — all gates ran and passed.

All 27 tasks in `tasks.md` are marked `[x]`.

## TDD Cycle Evidence

| Task | RED | GREEN | REFACTOR |
|---|---|---|---|
| 1.1-1.4 `roots::agent_roots` + `resolve_single` kind param | Unit tests added first against not-yet-existing `agent_roots` | `agent_roots` implemented (see Deviations below), `resolve_single` gained `kind: SearchRootKind`; `cargo test -p vertice-core roots::` green | `clippy -D warnings` clean; only `agent_roots` is new `pub` item in `roots.rs` |
| 1.7 tripwire | `tests/agent_scanner.rs` created with `empty_agent_root_fixture_directory_still_exists_on_disk` before any `agents.rs` code exists | Passes immediately (fixture-only) | N/A |
| 2.1-2.2 `escalate` + `ensure_utf8_path` unit tests | Tests for `escalate_maps_every_severity_to_error`, `non_utf8_path_component_fails_the_utf8_check` (unix), `utf8_path_passes_the_utf8_check` written alongside `agents.rs`'s first draft | Both functions implemented, structurally mirroring `skills::escalate`/`skills::ensure_utf8_path`; `cargo test -p vertice-core --lib agents::` green | Functions stay private |
| 2.3-2.6 full `agent_scanner.rs` integration suite | 20 integration tests written against the fixture tree, covering every `agent-scanner` spec requirement | `agents::scan` implemented (flat `read_dir` + sort, embedded-list gating, `ScanIssue` taxonomy); one test (`folded_description_is_parsed_in_full`) initially asserted no `\n` at all — corrected to match the yaml seam's documented folded-scalar behavior (trailing `\n` retained, matching `tests/yaml_behavior.rs`'s `folded_scalar_joins_lines_with_spaces`) — this was a test-authoring error, not an implementation bug | `clippy -D warnings` caught one `needless_lifetimes` in the test helper `file_backed`, fixed by eliding the lifetime |

## Deviations from Design

- **`agent_roots` construction (design §5.1).** The design's sketch calls `resolve_single` twice, once per root. `resolve_single`'s `suffix` parameter is `[&str; 2]` (matching the two-segment skill-root suffixes: `[".claude", "skills"]`, `[".agents", "skills"]`), which cannot express the embedded pseudo-root's single-segment suffix (`[".claude"]`) without changing the parameter's shape. Implemented instead as: `resolve_single` unchanged for the on-disk `claude-agents` root (still two-segment), and the embedded pseudo-root's path build + probe inlined directly in `agent_roots` using the same private `probe()` helper `resolve_single` itself uses. Net effect matches every stated invariant: `resolve_single` stays private, `probe` stays private, `agent_roots` is the only new `pub` item in `roots.rs`, both root ids are hardcoded and never path-derived, and the embedded root's `scan_paths` is empty. This is a mechanical implementation choice, not a behavioral deviation — verified by the unit tests in task 1.1 asserting the exact same shape the design specifies.

No other deviations. All spec requirements in `agent-scanner/spec.md` are covered by the fixtures and the 20-test integration suite; the `file-backed-only` filtering discipline (never `components.is_empty()`) is applied throughout, per design §4 and the spec's explicit warning.

## Files Changed

| File | Action | What Was Done |
|---|---|---|
| `crates/vertice-core/src/roots.rs` | Modified | Added `pub fn agent_roots(home: &Path) -> [ResolvedRoot; 2]`; gave `resolve_single` a `kind: SearchRootKind` parameter; updated the two skill-root call sites to pass `SearchRootKind::Skill` explicitly; added `agent_roots_returns_exactly_two_entries_with_stable_ids` and `agent_root_ids_are_stable_and_never_path_derived` unit tests |
| `crates/vertice-core/src/agents.rs` | Created | `AgentScan`, `AgentFrontmatter`, `scan(home) -> AgentScan`, `EMBEDDED_CLAUDE_AGENTS` const, flat `read_dir`-based walk with collect-then-sort ordering, `escalate`, `ensure_utf8_path`, embedded-component emission gated on `<home>/.claude` presence, plus 3 unit tests |
| `crates/vertice-core/src/lib.rs` | Modified | Added `pub mod agents;` |
| `crates/vertice-core/tests/agent_scanner.rs` | Created | 20 integration tests over the fixture tree, one (or a tight group) per `agent-scanner` spec requirement, plus the `.gitkeep` tripwire disk-existence test |
| `crates/vertice-core/tests/fixtures/roots/agents/**` | Created | Ten synthetic-home fixtures: `absent-root/` (`.gitkeep` only), `empty-root/` (`.claude/agents/.gitkeep`), `tools-scalar/` (`reviewer.md`), `folded-description/` (`summarizer.md`), `missing-optional/` (`minimal.md`), `broken-frontmatter/` (`good.md` + `broken.md`), `nested-decoy/` (`flat.md` + `group/nested.md`), `non-agent-entries/` (`real.md` + `notes.txt` + `.DS_Store` + `subdir/.gitkeep`), `shadowing/` (`Plan.md`), `reference/` (17 files, `reference-agent-01.md`…`reference-agent-17.md`, none colliding with the six embedded names) |
| `openspec/changes/2026-08-18-claude-code-agent-adapter/tasks.md` | Modified | All 27 tasks marked `[x]` |

Confirmed unchanged (per design §2 / task 3.6): `crates/vertice-core/src/model/**` and `frontend/src/bindings/**` — `git diff --exit-code` on both returns clean (exit 0; only pre-existing CRLF line-ending warnings, no content diff).

## Gate Results (all ran, all passed)

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all --check` | Clean |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Clean |
| test | `cargo test --workspace --locked` | All green — 20 new `agent_scanner.rs` tests, 5 new `roots.rs`/`agents.rs` unit tests, plus T4's 13 `skill_scanner.rs` tests and 7 pre-existing `roots.rs` unit tests all still green |
| deny | `cargo deny check bans licenses` (required `PATH="$HOME/.cargo/bin:$PATH"` prefix — `cargo-deny` is not on the default PATH in this environment) | `bans ok, licenses ok` |
| read-only grep | manual grep for `File::create`, `OpenOptions::write`, `fs::write`, `create_dir`, `remove_` | No matches in `agents.rs`, `roots.rs`, or `tests/agent_scanner.rs` |
| model/bindings diff | `git diff --exit-code -- crates/vertice-core/src/model frontend/src/bindings` | Exit 0, clean |
| yaml seam invariant | `cargo test -p vertice-core --test yaml_seam_invariant` + grep for `serde_norway` in `agents.rs` | Passes; no `serde_norway` import in `agents.rs` |
| walkdir/regex structural check | grep for `walkdir::`, `regex::`, `Regex::` in `agents.rs` | No matches |
| frontend regression | `npm run lint && npm run check && npm run test && npm run build` (from `frontend/`) | All pass: 0 lint errors, 0 svelte-check errors/warnings, 2/2 vitest tests pass, build succeeds |

## Remaining Tasks

None. All 27 tasks complete.
