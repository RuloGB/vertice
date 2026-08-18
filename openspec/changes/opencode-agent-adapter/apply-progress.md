# Apply Progress: OpenCode Agent Adapter (T6)

Batch: first (no prior apply-progress existed). Mode: **Strict TDD**. Artifact store: **openspec** — `tasks.md` in this directory carries the authoritative `[x]` marks; this file is the persisted narrative/evidence record.

## Status

**41/43 tasks complete.** The only two incomplete items (0.5, 3.5) are the MSRV-floor (`1.88.0`) sub-checks, honestly reported as **NOT RUN** because that toolchain is not installed locally (`rustup toolchain list` shows only `stable`/`1.97.1`). Per the executor's instructions, these were not claimed as passing and no toolchain was installed to force them. They are deferred to CI's existing `msrv` job, which `design.md` §5.2 names as the authoritative gate for this check.

`Ready for verify`, with the MSRV gap flagged as a known, honestly-reported limitation for the verify phase to weigh.

## Phase 0 — Dependency Gate (BLOCKING) — evidence

- **0.1** `jsonc-parser = { version = "0.33", default-features = false }` added to `crates/vertice-core/Cargo.toml`. Resolved to `jsonc-parser v0.33.1`. `serde` feature NOT enabled.
- **0.2** `PATH="$HOME/.cargo/bin:$PATH" cargo deny check licenses` → **`licenses ok`** (two pre-existing `license-not-encountered` warnings for `BSD-2-Clause`/`ISC`, unrelated to this change — those allow-list entries simply have no crate using them yet).
- **0.3** `cargo tree -p jsonc-parser -i` → **empty transitive tree**: `jsonc-parser v0.33.1` has zero dependencies of its own (confirmed via `cargo tree -p vertice-core -p jsonc-parser`, which shows `jsonc-parser` as a leaf). Matches design §5.2's expectation exactly.
- **0.4** Maintenance status, verified via the crates.io API on 2026-08-18: repository `github.com/dprint/jsonc-parser`, license `MIT`, latest version `0.33.1` published **2026-07-26** (~3 weeks before this cycle), 50 published versions total, 9.3M total downloads / 2.7M recent downloads. Actively maintained by the `dprint` tooling org. Evidence bar met.
- **0.5** **NOT RUN** — see Status above.
- **0.6** Decision checkpoint: 0.2–0.4 passed, 0.5 not run (not failed). Per the executor's explicit environment guidance, proceeded to Phase 1 with `jsonc-parser` as the dependency, flagging the MSRV gap rather than silently treating it as green.
- **0.7** Final crate decision: **`jsonc-parser` 0.33.1, MIT license**, verified 2026-08-18 as above. Recorded here for the PR body.

## Phase 1 — JSONC Seam, `roots.rs` Resolver, Fixture Tree

All tasks 1.1–1.12 complete.

- `crates/vertice-core/src/jsonc.rs` created: `JsonValue` (`Object` is `BTreeMap<String, JsonValue>`), `JsoncError::Parse(String)`, `pub fn parse`. Sole importer of `jsonc_parser` in the crate (verified by grep — see Phase 3 below). Parser options set explicitly: comments on, trailing commas on, everything else (loose property names, missing commas, single-quoted strings, hex numbers, unary-plus numbers) off — true JSONC, not JSON5.
- `crates/vertice-core/tests/jsonc_behavior.rs` created, mirroring `tests/yaml_behavior.rs`. 9 tests, all pinning: line/block comments accepted, trailing comma accepted, unquoted property names rejected, duplicate keys resolve last-wins, syntax errors return `JsoncError` (never panic), and `BTreeMap` ordering determinism (`apple, mango, zebra` byte-wise, not insertion order).
- `crates/vertice-core/src/roots.rs`: added `pub fn opencode_agent_root(home: &Path) -> ResolvedRoot`, structurally mirroring `resolve_opencode`. `probe` reused unchanged; `resolve_single`/`resolve_opencode` untouched. Two new unit tests. T4/T5's existing 9-test `roots.rs` unit suite stays green with zero edits.
- Fixture tree created under `crates/vertice-core/tests/fixtures/roots/opencode-agents/` — all 13 homes from design §10: `absent-config`, `empty-config-dir` (with `.gitkeep` tripwire), `json-only`, `jsonc-only` **[non-negotiable]**, `partial-override` **[non-negotiable]**, `jsonc-syntax` **[non-negotiable]**, `broken-json`, `broken-jsonc`, `no-agent-key`, `empty-agent`, `normalize-collision`, `malformed-entry`, `reference` (7 agents across two files, one shared key `epsilon`, mirroring V2's real field shape). New tree — no reference into T4's or T5's fixture trees.
- `crates/vertice-core/tests/opencode_agent_scanner.rs` created with the disk-existence tripwire (`empty_config_dir_fixture_still_exists_on_disk`), GREEN with no `opencode_agents` module.
- `.gitattributes` confirmed unchanged — `-text` on `crates/vertice-core/tests/fixtures/**` already covers the new tree.
- **Gate 1.12**: `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace --locked` all green; `cargo deny check bans licenses` → `bans ok, licenses ok`; `git diff --numstat -- frontend/src/bindings` empty (see the CRLF note below for why `--exit-code` is the wrong check on this machine).

## Phase 2 — `opencode_agents.rs` Module (TDD, RED → GREEN)

### The non-negotiable checkpoint (tasks 2.1–2.3) — done in the required order

1. **2.1 [RED]**: Wrote `#[cfg(test)]` literal-level unit tests in `opencode_agents.rs` against `merge_all(inputs: &[JsonValue]) -> Option<JsonValue>` (ordered fold over a slice, never a two-named-parameter `merge(base, overlay)` — design §4's escape hatch preserved). Implemented `merge_all` as a **deliberately naive whole-object-replacement stub** (`inputs.iter().cloned().reduce(|_base, overlay| overlay)`) precisely so the tests fail on an assertion, not a compile error.
2. **2.2**: Wrote the `partial-override/` fixture integration test in `tests/opencode_agent_scanner.rs`, calling `opencode_agents::scan` (not yet existing → compile failure, expected per T5D's "poor RED" note — acceptable because 2.1 already supplies the assertion-level RED).
3. **2.3 [Checkpoint]** Ran `cargo test -p vertice-core --lib opencode_agents::` against the stub. **Observed and recorded the failure** before writing any real merge code:

   ```
   running 10 tests
   ...
   test opencode_agents::tests::overlay_only_key_survives ... FAILED
   test opencode_agents::tests::keys_differing_only_by_case_are_not_normalized_before_merging ... FAILED
   test opencode_agents::tests::shared_key_partial_override_merges_per_field_not_per_object ... FAILED

   ---- opencode_agents::tests::shared_key_partial_override_merges_per_field_not_per_object stdout ----
   thread '...' panicked at crates\vertice-core\src\opencode_agents.rs:108:9:
   assertion `left == right` failed: the base's non-overridden `description` field must survive a partial override
     left: None
    right: Some(String("from base"))

   test result: FAILED. 7 passed; 3 failed
   ```

   3 of 10 tests failed against the stub — exactly the ones exercising per-key merge behavior (partial-override, overlay-only-key, case-distinctness). The other 7 (base-only-key, array-replace, scalar/object-replace both directions, null-replace, fold-zero, fold-one) trivially pass under whole-object replacement too, which is the whole point: they do not discriminate. Only `partial-override`'s shape does.

4. **2.4 [GREEN]**: Implemented the real recursive `merge_two`/`merge_all` per design §6.2. All 10 unit tests pass. `partial_override_fixture_merges_per_field_not_per_object` integration test now compiles and passes.

### Remaining Phase 2 tasks (2.5–2.13)

Implemented and verified together rather than individually RED-first — **disclosed as a deviation from strict per-task TDD sequencing**, though every behavior below is covered by a passing test and the one place order was declared non-negotiable (2.1–2.3) was followed exactly as required.

- `DescriptionField` enum (`Absent`/`Present(String)`/`WrongType`) and `extract_description`: value-level extraction, `entry.get("description")` matched against `JsonValue::String` only. No `#[derive(Deserialize)]` struct for an agent entry anywhere in the module (grep-verified, task 3.9). 7 unit tests including `hidden_true_does_not_affect_extraction` (task 2.7/2.8) and `unmodelled_fields_do_not_affect_description_extraction`.
- `OpenCodeAgentScan { roots, components, issues }` and `pub fn scan(home: &Path) -> OpenCodeAgentScan`, per design §5.3's control flow: resolve root → per-path read/parse/extract with index retained for provenance → `merge_all` fold → one `Component` per merged key in sorted (`BTreeMap`) order.
- **Per-file `Location` provenance** (design §6.4): `scan` tracks which `scan_paths` index each surviving `agent` object came from, and for each merged key determines every declaring file by checking key presence in each surviving per-file map — not just the final merged winner. This produces exactly one `Location` per declaring file, ordered by `scan_paths` order. Verified by `reference/`'s `epsilon` carrying two `Location`s and `zeta`/`eta` carrying one each.
- Full `ScanIssue` taxonomy wired per design §8's table exactly — no `escalate` function (design §5.6): every issue is constructed at the point where caller context (which file, what was lost) is already in hand.
- `lib.rs` wiring: `pub mod jsonc;` and `pub mod opencode_agents;`, two lines, no crate-root re-export.
- **Refactor check (2.13)**: only `OpenCodeAgentScan` and `scan` are `pub` from `opencode_agents.rs`; only `JsonValue`, `JsoncError`, `parse` are `pub` from `jsonc.rs`. `merge_all`, `merge_two`, `DescriptionField`, `extract_description`, `read_agent_object`, `assemble_component` all private. `cargo clippy --workspace --all-targets -- -D warnings` clean (two clippy findings — a redundant closure and an unnecessary `.clone()` for a single-element slice — fixed during this pass).

### Integration suite (task 2.9)

22 tests in `tests/opencode_agent_scanner.rs`, one per requirement, covering every fixture: `json-only`, `jsonc-only` (non-negotiable), `jsonc-syntax` (non-negotiable), `broken-json`/`broken-jsonc` (CA-12 mirror pair), `absent-config`/`empty-config-dir` (NotFound-vs-Found-empty distinguishability), `no-agent-key`, `empty-agent`, `normalize-collision`, `malformed-entry` (Warning, not Error), `reference` (CA-5 pin: 7 components, 7 distinct ids, `zeta`/`eta` sourced only from `.jsonc`, `epsilon` carrying two `Location`s), determinism (two consecutive scans byte-identical, literal expected id order), read-only (`full_scan_leaves_the_reference_fixture_tree_unchanged`), and `roots.len() == 1` for every home.

## TDD Cycle Evidence

| Task(s) | Behavior | RED | GREEN | REFACTOR |
|---|---|---|---|---|
| 1.1–1.2 | `jsonc::parse` seam behaviors | Compile failure against non-existent `jsonc` module (observed) | `jsonc.rs` created, 9/9 pass | n/a |
| 1.3–1.4 | `BTreeMap` determinism | Same test file as 1.1, property of the chosen type | Passed with no new code | n/a |
| 1.5–1.6 | `opencode_agent_root` | Compile failure — function not found (observed) | Implemented, unit tests pass | 1.7: clippy clean, `probe`/`resolve_single`/`resolve_opencode` untouched |
| **2.1–2.3** | **Merge algorithm (`partial-override` safeguard)** | **Naive whole-object-replace stub; 3/10 tests FAILED, output recorded above** | 2.4: real recursive merge, 10/10 pass | n/a |
| 2.2 | `partial-override` integration test | Compile failure — `opencode_agents::scan` not found (observed) | Passes once `scan` exists | n/a |
| 2.5–2.8 | `description` extraction, `hidden` non-filtering | Not individually RED-first (deviation, disclosed above) | 7/7 unit tests pass on first run | n/a |
| 2.9–2.11 | Full integration suite, issue taxonomy | Not individually RED-first (deviation, disclosed above) | 22/22 integration tests pass on first run | 2.13: pub-surface check, clippy clean |

**Deviation disclosed honestly**: strict TDD's RED-before-GREEN was followed exactly for the one place task order was declared non-negotiable (2.1–2.3, the `partial-override` merge safeguard) and for the two structural additions with a clean not-yet-exists compile failure (1.1, 1.5, 2.2). For the extraction/issue-taxonomy/integration-suite work (2.5–2.11), tests and implementation were written and verified together rather than individually red-first, due to the scope of a single apply batch. Every behavior in the spec is covered by a passing test; nothing here was implemented without a corresponding test, but the strict per-task red-observation discipline was relaxed for that subset.

## Verification Gates (Phase 3) — actually run, results below

| Gate | Command | Result |
|---|---|---|
| 3.1 fmt | `cargo fmt --all --check` | Clean |
| 3.2 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Clean |
| 3.3 tests | `cargo test --workspace --locked` | All green: 63 lib tests (vertice-core), 22 `agent_scanner.rs` (T5, unmodified), 14 `frontmatter_reader.rs`, 9 `jsonc_behavior.rs`, 8 `model_contract.rs`, 22 `opencode_agent_scanner.rs`, 13 `skill_scanner.rs` (T4, unmodified), 7 `yaml_behavior.rs`, 1 `yaml_seam_invariant.rs` |
| 3.4 deny | `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` | `bans ok, licenses ok` |
| 3.5 MSRV | `cargo +1.88.0 check -p vertice-core` | **NOT RUN** — toolchain not installed |
| 3.6 read-only grep | `grep -nE "File::create\|OpenOptions::write\|fs::write\|create_dir\|remove_" jsonc.rs opencode_agents.rs *test files*` | Zero matches |
| 3.7 model/bindings diff | `git diff --numstat -- crates/vertice-core/src/model` and `-- frontend/src/bindings` | **Both empty.** Used `--numstat`, not `--exit-code`, per the verified environment fact that `core.autocrlf=true` makes `--exit-code` report dirty on these 16 pre-existing binding files for reasons unrelated to this change (CRLF-vs-LF only, zero content diff) |
| 3.8 seam invariants | grep for `jsonc_parser`, `serde_norway`, `regex`/`Regex`, `walkdir` in new modules | `jsonc_parser` imported only in `jsonc.rs` (the one comment-only mention in `jsonc_behavior.rs` is prose, not an import); no `serde_norway`/`regex`/`walkdir` in either new module |
| 3.9 no Deserialize DTO | grep `derive(Deserialize)` in `opencode_agents.rs` | Zero matches (one doc-comment mention only) |
| 3.10 platform note | n/a | Noted: Windows-verified only, macOS/Linux/`config.json`/null-delete/BOM/unquoted-names deferred to T16; a green T16 oracle contrast never covers the `.jsonc`-agent or comment/trailing-comma paths — only `jsonc-only`, `jsonc-syntax`, `partial-override` cover those |
| 3.11 frontend | `npm run lint && npm run check && npm run test && npm run build` (from `frontend/`) | All green: 0 lint errors, 0 svelte-check errors/warnings, 2/2 vitest tests, build succeeded |

## Files Changed

| File | Action | What Was Done |
|---|---|---|
| `crates/vertice-core/Cargo.toml` | Modified | Added `jsonc-parser = { version = "0.33", default-features = false }` |
| `Cargo.lock` | Modified | First lockfile movement since T2 (adds `jsonc-parser` only — empty transitive tree) |
| `crates/vertice-core/src/jsonc.rs` | Created | `JsonValue`, `JsoncError`, `parse` — sole importer of `jsonc_parser` |
| `crates/vertice-core/src/opencode_agents.rs` | Created | `OpenCodeAgentScan`, `scan`, `merge_all`/`merge_two`, `DescriptionField`/`extract_description`, `read_agent_object`, `assemble_component`, full issue taxonomy, 16 unit tests |
| `crates/vertice-core/src/roots.rs` | Modified | Added `opencode_agent_root` + 2 unit tests; `probe`/`resolve_single`/`resolve_opencode` untouched |
| `crates/vertice-core/src/lib.rs` | Modified | `pub mod jsonc;`, `pub mod opencode_agents;` |
| `crates/vertice-core/tests/jsonc_behavior.rs` | Created | 9 seam-behavior tests |
| `crates/vertice-core/tests/opencode_agent_scanner.rs` | Created | 22 fixture-driven integration tests |
| `crates/vertice-core/tests/fixtures/roots/opencode-agents/**` | Created | 13 synthetic homes (design §10) |
| `openspec/changes/opencode-agent-adapter/tasks.md` | Modified | 41/43 tasks marked `[x]`; 0.5/3.5 marked NOT RUN with explanation |
| `openspec/changes/opencode-agent-adapter/apply-progress.md` | Created | This file |

**Unchanged, confirmed**: `crates/vertice-core/src/model/**`, `frontend/src/bindings/**`, `crates/vertice-app/**`, `frontend/src/**`, `.github/workflows/**`, `.gitattributes`, `rust-toolchain.toml`, `deny.toml`.

## Proposed Commit Sequence (planned only — NOT executed; branch is `main`, no commit/push performed per instructions)

Per `work-unit-commits` and the design §12 seam, with the single-PR/`size:exception` decision from `tasks.md`'s Review Workload Forecast, commit granularity is the only reviewable structure left:

1. `deps(core): add jsonc-parser for JSON/JSONC parsing` — `Cargo.toml`, `Cargo.lock` only. Isolates the supply-chain-review-worthy change from all logic, per design §5.2/§12. PR body should carry the Phase 0 evidence (license MIT, empty transitive tree, maintenance date 2026-08-18, MSRV NOT RUN and why).
2. `feat(core): add JSON/JSONC parsing seam` — `src/jsonc.rs`, `tests/jsonc_behavior.rs`.
3. `feat(core): resolve the OpenCode agent config root` — `src/roots.rs` (`opencode_agent_root` + unit tests).
4. `test(core): add OpenCode agent scanner fixture tree` — `tests/fixtures/roots/opencode-agents/**`, plus the disk-existence tripwire and `.gitkeep` half in `tests/opencode_agent_scanner.rs`.
5. `test(core): pin the partial-override merge safeguard (RED)` — the literal-level `merge_all` tests plus the naive stub in `src/opencode_agents.rs`, and the `partial-override` integration test in `tests/opencode_agent_scanner.rs`. This commit is intentionally red-by-design at the assertion level (3/10 unit tests fail against the stub) — the RED evidence above is this commit's diff.
6. `feat(core): implement the OpenCode agent per-key merge (GREEN)` — replace the stub with the real recursive `merge_two`/`merge_all`.
7. `feat(core): scan OpenCode agent config files into components` — `OpenCodeAgentScan`, `scan`, value-level `description` extraction, full issue taxonomy, `lib.rs` wiring, and the remaining 20 integration tests.

Each commit compiles and its own test subset is green at that point in history, preserving RED-before-GREEN at commit 5→6 without a compile-failure RED anywhere in the sequence except transiently between commits 2→3→4 (each of which adds a symbol the next commit's tests need — matching T4/T5 precedent already accepted in this repo's history).

## Deviations from Design

None. Implementation matches `design.md` exactly, including the deliberately non-obvious choices: `merge_all` takes a slice (never `merge(base, overlay)`), `Object` is `BTreeMap`, no `escalate` function, no `#[derive(Deserialize)]` DTO, `Null` overlay replaces without deleting, keys never normalized before merging, one `Location` per declaring file.

## Issues Found

None blocking. The MSRV-floor sub-checks (0.5, 3.5) are an environment limitation, not a code or design issue, and are deferred to CI's `msrv` job as the design anticipated.

## Risks

- **MSRV at 1.88 is unverified locally.** If the crate or its API usage relies on anything above 1.88, CI's `msrv` job will catch it — this is the first time that job is meaningfully exercised by a new dependency since T2.
- **The recursive merge is the highest-consequence logic in this change** (per `tasks.md`'s Review Workload Forecast) and sits behind the largest diff volume. The `partial-override` RED-before-GREEN discipline is the safeguard the design relies on instead of reviewer attention alone — it was followed exactly.
- **Single-PR `size:exception`** means commit granularity (see Proposed Commit Sequence above) is the only remaining reviewable structure; this was planned but not executed, per instructions.
