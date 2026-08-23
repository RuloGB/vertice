# Tasks: Add Codex Client Support

> Trace: **T7** (client installation detection), replaying **T4** (skill roots) and **T5/T6** (per-client agent adapter). Addresses **CA-11, CA-7, CA-12, CA-8**. Must-not-regress **CA-2/CA-3/CA-4, CA-6, CA-14**. Bounded by **CA-16, CA-17**.
> Authority for every decision below is `design.md`; do not reopen §0–§14. Section references (`§n`) point there.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~720-1115 (proposal Changed-Line Forecast) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | Three slices, each independently green and independently revertible (design §13) |
| Delivery strategy | chained |
| Chain strategy | slice-by-dependency |

Decision needed before apply: Yes
Chained PRs recommended: Yes
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Phases | Likely PR | Notes |
|------|------|--------|-----------|-------|
| A | `ClientKind::Codex` + `CodexStandalone` slot + `ReleaseDirectoryName` + installation fixtures + regenerated binding | 1, 3 | PR 1 | Self-contained; adds a presence row and nothing else (design §13.1). No dependency on B or C. |
| B | The `codex-skills` root: `skill_roots` 3→4 + fixtures | 4 | PR 2 | Near-zero implementation, meaningful test surface (design §13.2). Touches `ROOT_ORDER` — see sequencing note below. |
| C | `toml.rs` seam + `codex_agents.rs` + `codex_agent_root` + orchestrator wiring + `ROOT_ORDER`'s eighth entry | 2, 5, 6 | PR 3 | The only slice with a new dependency (design §13.3). Touches `ROOT_ORDER` — see sequencing note below. |
| — | `ROOT_ORDER` 6→8, tripwire, read-only audit, gates | 7, 8, 9, 10 | Lands with whichever of B/C merges second | `ROOT_ORDER` is touched by both B and C; whichever lands second updates the array and `consolidate.rs`'s pinning test together, in the same commit as its own roots.rs change (design §6.2, §13). |

Units A, B, and C have no code dependency on each other and MAY be implemented in parallel by different sessions/PRs. Phases 7-10 are sequential and MUST land after all three units are in.

## Phase 0: Fixture-Coverage Honesty (CA-17)

- [x] 0.1 Enumerate every scenario across the six delta specs (`codex-agent-scanner`, `skill-scanner`, `client-installation-detector`, `domain-model`, `workspace-architecture`, `scan-orchestration`) against design §10.3's fixture table. Confirm every scenario maps to a named fixture home listed there; report plainly if any scenario has no planned fixture before writing code.

## Phase 1: Core Model — `ClientKind::Codex` (T2, `domain-model` delta)

- [x] 1.1 Modify `crates/vertice-core/src/model/installation.rs`: add `ClientKind::Codex`; update the doc comment from "(Claude Code, OpenCode)" to name three clients. No `#[non_exhaustive]`.
- [x] 1.2 Run `cargo test -p vertice-core` to regenerate `frontend/src/bindings/ClientKind.ts` (three variants: `"claudeCode" | "openCode" | "codex"`). Never hand-edit; commit with the Rust enum in the same commit (domain-model spec requirement).
- [x] 1.3 Confirm no other `bindings/*.ts` file changed — a diff there means something leaked outside `model/`.

## Phase 2: TOML Seam (`workspace-architecture` delta) — Slice C, part 1

- [x] 2.1 Add to `crates/vertice-core/Cargo.toml`: `toml_seam = { package = "toml", version = "1", default-features = false, features = ["parse", "serde"] }` (§5.1-5.2). Run `cargo deny check bans licenses` — expect pass with `deny.toml` byte-identical (V1b).
- [x] 2.2 (RED) Write `crates/vertice-core/tests/toml_behavior.rs`, mirroring `tests/yaml_behavior.rs`: multiline `"""…"""` preserved verbatim, escapes, a missing required field surfacing as an error, an unknown key ignored. Fails because `src/toml.rs` does not exist yet.
- [x] 2.3 (GREEN) Create `crates/vertice-core/src/toml.rs` (§5.3): `TomlError` enum with one `#[from]` `Parse(toml_seam::de::Error)` variant, `pub fn from_str<T: DeserializeOwned>(input: &str) -> Result<T, TomlError>`. No serialization function exposed — read-only by construction.
- [x] 2.4 Add `pub mod toml;` to `crates/vertice-core/src/lib.rs`. Confirm 2.2 now passes.
- [x] 2.5 (RED then GREEN) Write `crates/vertice-core/tests/toml_seam_invariant.rs`, a line-for-line analogue of `tests/yaml_seam_invariant.rs`: greps `use toml_seam` / `toml_seam::`, excluding only `src/toml.rs` (parent == `src/` and file name == `toml.rs`). Confirm it fails against a deliberately-broken fixture (a second module naming `toml_seam`) before it is trusted, then confirm it passes against the real tree.

## Phase 3: Installation Detection — `CodexStandalone` Slot (T7, `client-installation-detector` delta) — Slice A

- [x] 3.1 (RED) Add fixture homes under `crates/vertice-core/tests/fixtures/client-installations/`: `codex-installations/single-release`, `.../two-releases`, `.../prerelease`, `.../unknown-triple`, `.../empty-releases`, `.../stale-version-json`, `.../nothing` (design §10.3). Every release directory MUST contain at least one file (`.gitkeep`), since git does not track empty directories (§10.3 closing note). Assert each directory's on-disk existence in a dedicated test, mirroring `skill_scanner.rs:36-52`.
- [x] 3.2 (RED) Add unit tests, no I/O, for `split_release_dir_name` over the §3.2 table including both no-match rows (`0.151.0-riscv64-unknown-linux-gnu`, `x86_64-pc-windows-msvc` alone, `nightly`) and the prerelease case `0.150.0-rc.1-x86_64-pc-windows-msvc` → `0.150.0-rc.1`. This is the RED test that kills "split on the first `-`" (design §12, item 2).
- [x] 3.3 (RED) Add integration tests in `tests/client_installations.rs`:
  - `two_release_directories_yield_two_unmerged_installations` over `two-releases` (**CA-7**)
  - `prerelease_release_directory_name_yields_the_full_prerelease_version` over `prerelease`
  - `home_without_codex_yields_not_detected_and_zero_issues` over `nothing` (**CA-11**)
  - unknown-triple / empty-releases → `Detected` + 0 installations + 1 `Error` carrying the directory's path (§3.3)
  - `stale-version-json` → reported version equals the release directory name, never `latest_version`
  - update the existing "machine with no clients" test: 3 `NotDetected` records → 4
- [x] 3.4 (GREEN) Modify `crates/vertice-core/src/installations.rs` (§3, §4):
  - `VersionSource::ReleaseDirectoryName` variant
  - `const CODEX_TARGET_TRIPLES: [&str; 2] = ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"]` with the manual-maintenance doc comment (§3.2)
  - pure `fn split_release_dir_name(name: &str) -> Option<&str>`, no I/O
  - `InstallSlot::CodexStandalone` appended after `OpenCodeNpm`; label exactly `"Codex CLI (standalone)"` (§4)
  - `InstallSlot::client()` gains `CodexStandalone -> ClientKind::Codex` (the second of the two exhaustive-match sites)
  - new `resolve_codex_slot`, a structural sibling of `resolve_bundled_slot`, not a parameterization of it (§3.1) — enumerates `<home>/.codex/packages/standalone/releases/` one level deep, never follows a symlink, sorts children byte-wise on `file_name()`
  - module doc "three slots" → "four"
- [x] 3.5 Confirm 3.1-3.3's RED tests now pass.
- [x] 3.6 (RED then GREEN) Write `crates/vertice-core/tests/codex_version_source_invariant.rs`: no file under `src/` contains the strings `version.json` or `latest_version` (§10.4, item 2).
- [x] 3.7 Update `tests/model_contract.rs`: the `ClientKind` exhaustive-match test gains `Codex` (the first of the two exhaustive-match sites; closes domain-model's "ClientKind is exhaustively matchable" scenario).

## Phase 4: Codex Skill Root (T4 replay, `skill-scanner` delta) — Slice B

- [x] 4.1 (RED) Add fixture homes under `crates/vertice-core/tests/fixtures/roots/`: `codex-skills` (a `SKILL.md` carrying `disable-model-invocation`, `user-invocable`, `license`, `metadata.*`) and `codex-and-claude-same-name` (a skill named `shared` under both `.claude/skills` and `.codex/skills`).
- [x] 4.2 (RED) Update `tests/skill_scanner.rs`: `scan.roots.len() == 3` → `4` (line 149); a new test asserting the Codex `SKILL.md` with extra keys parses with the unmodelled keys ignored; a new test asserting `.codex/skills/` is the fourth resolved root and the relative order of the three existing roots is unchanged.
- [x] 4.3 (GREEN) Modify `crates/vertice-core/src/roots.rs`: `skill_roots` return type `[ResolvedRoot; 3] -> [ResolvedRoot; 4]`, appending `resolve_single(home, "codex-skills", SearchRootKind::Skill, [".codex", "skills"])` as the fourth and last entry. Update the module doc's "three" → "four" everywhere, including the CA-6/CA-14 scoping argument.
- [x] 4.4 Rename/extend `roots.rs`'s own tests: `skill_roots_always_returns_exactly_three_entries` → `..._four_entries`; `root_ids_are_stable_and_never_path_derived` gains the fourth id.
- [x] 4.5 Confirm 4.1-4.2's RED tests now pass.

## Phase 5: Codex Agent Adapter (T5/T6 replay, `codex-agent-scanner` delta) — Slice C, part 2

- [x] 5.1 (RED) Add fixture trees under `crates/vertice-core/tests/fixtures/roots/codex-agents/` (a new tree, never reused from `roots/agents/` or `roots/opencode-agents/`, per spec): a valid agent; an agent whose `developer_instructions` is a genuine multiline `"""…"""` string with embedded blank lines; a malformed `.toml`; a nested subdirectory containing a well-formed agent (must NOT be discovered); a non-`.toml` sibling file; an absent root; an empty root; extra/unmodelled keys including a nested table; a `corrupt/` home with one malformed `.toml` and one missing-`name` `.toml` alongside valid siblings; a not-a-directory root-shape case.
- [x] 5.2 (RED) Write `crates/vertice-core/tests/codex_agent_scanner.rs` covering every spec scenario: flat discovery of a direct `.toml`; nested file NOT discovered; non-`.toml` file ignored; absent root → zero components/issues; empty root → zero components/issues; multiline `developer_instructions` returned complete and byte-exact; a source-inspection test asserting no regex is used to parse `.toml` content; `Component` assembly shape (`kind: Agent`, `scope: User`, one `Location{path: Some(_), origin: File}`); per-file isolation — one malformed file yields one `Error` `ScanIssue` with its path and both siblings still discovered (**CA-12**); a full-scan read-only/tree-snapshot test proving the fixture tree is byte-for-byte unchanged afterward (**CA-16**).
- [x] 5.3 (GREEN) Modify `crates/vertice-core/src/roots.rs`: add `pub fn codex_agent_root(home: &Path) -> ResolvedRoot`, built from `resolve_single(home, "codex-agents", SearchRootKind::Agent, [".codex", "agents"])`, mirroring `opencode_agent_root`. It DOES emit a `SearchRoot` with `kind: Agent` (§6.1 — closes the open decision "Yes").
- [x] 5.4 (GREEN) Create `crates/vertice-core/src/codex_agents.rs` (§6.3):
  - `CodexAgentDocument { name: String, description: Option<String>, developer_instructions: Option<String> }` — `Deserialize`-only, no `TS`, no binding emitted. Not named `…Frontmatter` (§5.4).
  - `CodexAgentScan { roots: Vec<SearchRoot>, components: Vec<Component>, issues: Vec<ScanIssue> }`, `pub fn scan(home: &Path) -> CodexAgentScan`
  - walk shape: resolve root → `symlink_metadata` (`NotFound` ⇒ silent return, no issue) → assert directory → `read_dir` → `sort_by_key(DirEntry::file_name)` → per entry: extension `.toml` check (else skip silently) → UTF-8 path check → `read_to_string` → `crate::toml::from_str::<CodexAgentDocument>` → `Component`
  - no embedded pseudo-root; no `escalate` helper — every issue built at its failure site with severity already `Error` per the §7.1 table; per-file `continue` isolation
  - a file missing `name` is an `Error` for the whole file, never a fallback to the file stem
  - field mapping per §6.3's table; `developer_instructions` parsed and pinned by tests but deliberately dropped, never mapped onto `Component`
- [x] 5.5 Add `pub mod codex_agents;` to `crates/vertice-core/src/lib.rs`.
- [x] 5.6 Confirm 5.1-5.2's RED tests now pass.

## Phase 6: Orchestrator Wiring (T7, `scan-orchestration` delta) — Slice C, part 3

- [x] 6.1 (RED) Extend the `scan-orchestrator/complete` fixture: add `.codex/skills/<name>/SKILL.md`, `.codex/agents/<name>.toml`, and a `packages/standalone/releases/<version>-<triple>/` tree. Without this the two new roots resolve `NotFound` and `report.issues.is_empty()` fails on two new `Warning`s (§10.3).
- [x] 6.2 (RED) Update `scan.rs` orchestrator tests' counts (design §8, §10.3):
  - `complete`: `roots_scanned.len()` 6 → 8, `installations.len()` 3 → 4, `components.len()` 10 → 12, `report.issues.is_empty()` still holds
  - `missing-root-client`: roots 6 → 8, `path.is_none()` warnings 6 → 8, `client_presence.len()` 3 → 4, all still `NotDetected`
  - `corrupt-skill`: confirm unchanged (`installations.len() == 1` holds — no Codex tree in that fixture home)
- [x] 6.3 (RED) Add a scenario proving a same-named skill in a Codex root and a Claude Code root consolidates into one `Component` with two `Location`s (uses the `codex-and-claude-same-name` fixture from 4.1, exercised at the orchestrator/consolidation layer as `scan-orchestration` specifies).
- [x] 6.4 (RED) Add a scenario: a malformed Codex agent `.toml` in an otherwise well-formed fixture home does not abort the scan — one `Error` `ScanIssue` for the malformed file, every other adapter's valid results still present.
- [x] 6.5 (GREEN) Modify `crates/vertice-core/src/scan.rs`: call `crate::codex_agents::scan(home)`, appended after the OpenCode-agent adapter, and `extend` `roots_scanned`, `components`, `issues` from it — three `extend`s, exactly like the other three adapters. `codex_agents::scan` is infallible and returns owned buffers, so "one bad adapter never aborts the scan" holds structurally.
- [x] 6.6 Confirm 6.1-6.4's RED tests now pass.

## Phase 7: `consolidate.rs` — `ROOT_ORDER` 6 → 8 (T8 replay)

> Sequencing note: this phase depends on BOTH Phase 4 (`codex-skills` in `roots.rs`) and Phase 5 (`codex_agent_root` in `roots.rs`) having landed. If Slices B and C are applied as separate PRs, whichever lands second performs this phase and updates the pinning test in the same commit as its own `roots.rs` change — never after (design §6.2).

- [x] 7.1 (GREEN) Modify `crates/vertice-core/src/consolidate.rs`: `ROOT_ORDER` grows to 8 entries in the pinned concatenation order — `["claude-skills", "agents-skills", "opencode-skills", "codex-skills", "claude-agents", "claude-embedded-agents", "opencode-agents", "codex-agents"]` (§6.2). No merge logic, no `root_rank`, no `location_key`, no `merge_into` change.
- [x] 7.2 Update `root_order_matches_the_roots_module_in_order`: the `skill_roots` loop already iterates whatever the array holds (the fourth skill root flows in automatically); add one explicit push — `expected.push(crate::roots::codex_agent_root(&home).root.id.0.clone());` — after the `opencode_agent_root` push.
- [x] 7.3 Confirm `identity.rs`, `component.rs` need no logic change (explicit no-op check, per design §11's "Unchanged" row) — do not edit either file.

## Phase 8: Reference-Fixture Tripwire — the hard guard (CA-2/CA-3/CA-4)

- [x] 8.1 Confirm `crates/vertice-core/tests/fixtures/roots/reference/` and `tests/fixtures/scan-orchestrator/reference-volume/` are byte-identical to their pre-change state — no file added, changed, or removed (`git status`/`git diff` clean on both paths).
- [x] 8.2 Confirm `reference_fixture_tree_yields_69_entries`, its 25-id corroborator, and the CA-3/CA-4 (22-with-3-locations / 3-with-1-location) assertions in `skill_scanner.rs` remain textually unmodified. A diff to any of those four numbers is a stop-the-line signal, not a fixture update.
- [x] 8.3 (RED then GREEN) Add the new negative-existence assertion in `skill_scanner.rs`: `reference/.codex` MUST NOT exist on disk (design §10.1, tripwire 3). Confirm it fails against a deliberately-created `reference/.codex` directory in a scratch check, then confirm it passes against the real (untouched) tree.
- [x] 8.4 Verify, by reading `skills.rs:66-68`, that `skills::walk_one` returns silently with zero issues on the absent fourth root (`reference/.codex/skills`) — the 69/25/22/3 counts are structurally unaffected, not merely observed to be unaffected (design §10.1, tripwire 2). No code change; verification only.

## Phase 9: Read-Only Audit (CA-16)

- [x] 9.1 Grep `crates/vertice-core/src/` and `crates/vertice-core/tests/` for `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, `remove_*`, `symlink*` — confirm no new match outside the audit test's own pattern list. The disk surface added this change is exactly `symlink_metadata`, `read_dir`, `DirEntry::file_type`, `read_to_string` (design §11).
- [x] 9.2 Confirm the existing read-only tree-snapshot equality test is extended to cover the new Codex fixture homes (installations, skills, agents).
- [x] 9.3 Confirm no committed fixture contains a symlink or a junction anywhere in the new trees (design §10.2 — releases are enumerated directly, never followed).

## Phase 10: Gates

- [x] 10.1 Run `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked`; `cargo deny check bans licenses`. Report each gate's actual pass/fail; if `cargo` is not resolvable on PATH in the execution environment, say so plainly rather than reporting the gate as passing.
- [x] 10.2 From `frontend/` (never from `frontend/src/`, to avoid a stray `node_modules`): `npm run lint && npm run check && npm run test && npm run build`.
- [x] 10.3 Re-run `cargo test -p vertice-core` and diff `frontend/src/bindings/`: confirm only `ClientKind.ts` changed (three variants); every other `bindings/*.ts` file is byte-identical, and no new binding file was emitted.
- [x] 10.4 Confirm `crates/vertice-app/`, `capabilities/default.json`, and `deny.toml` are byte-identical to their pre-change state (§9.1, §11).
- [x] 10.5 Confirm `Cargo.toml` `rust-version`, the CI `MSRV` env, and `rust-toolchain.toml`'s `channel` still agree (no MSRV edit expected — V1b).
