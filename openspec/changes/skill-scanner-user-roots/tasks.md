# Tasks: Skill Scanner over User Roots

> Trace: **T4** (Phase 1 — Reading, `plan-desarrollo-poc.md:110-128`) / closes **CA-6** (no plugin skill appears), **CA-8 partial** (`_shared` is an ordinary skill), **CA-9** (absent/empty root — no issue, no component, distinguishable states), **CA-14** (no project-scope component); contributes to **CA-12 partial** (unreadable file reported, scan continues); bound by **CA-16** (read-only) and **CA-17** (fixture-based, three-platform tests).
> Design: `openspec/changes/skill-scanner-user-roots/design.md`. Inherits T3's fixture-per-case pattern (`archive/2026-08-18-skill-frontmatter-reader/design.md` §9) and its PR-boundary precedent (below).
> Core-only — no IPC, no Tauri command, no frontend source change. `npm run lint/check/test/build` still gate because `frontend/src/bindings/*.ts` changes.
> `strict_tdd: true`. Fixtures and failing tests land before the implementation that turns them green.
> Environment note, updated at apply time: `cargo` (1.97.1) IS available in the apply environment, unlike the authoring/planning environment this file's earlier hedges assumed. Every gate below was actually run locally and its real output is recorded in `apply-progress.md`: `cargo fmt --all --check` (clean), `cargo clippy --workspace --all-targets -- -D warnings` (clean, `std::env::home_dir()` compiles with no deprecation warning at MSRV 1.88/toolchain 1.97.1, confirming design.md §3's un-deprecation claim), `cargo test --workspace --locked` (all green), `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` (`bans ok, licenses ok` — `walkdir`'s `Unlicense OR MIT` resolves via the already-allow-listed `MIT`, no `deny.toml` change needed), and the frontend gates from `frontend/`.

## Work Units

| Unit | Goal | PR | Base | Notes |
|------|------|----|------|-------|
| 1 | `model/location.rs` (`SearchRootStatus`, `SearchRoot::status`) + regenerated bindings + domain-model tests (GREEN immediately — no `roots`/`skills` dependency) + full fixture tree (semantic set + 69-entry reference tier) + the fixture-integrity half of the `.gitkeep` tripwire | PR 1 (~350-450 lines, mostly fixture bulk) | `main` | No `roots.rs`/`skills.rs` yet. Every test in this unit compiles and passes standalone — see "PR Boundary Decision" below for why this unit contains no RED test for skill-scanner behavior. |
| 2 | `crates/vertice-core/src/roots.rs`, `crates/vertice-core/src/skills.rs`, `lib.rs` wiring, `walkdir` dependency (+ `deny.toml` contingency), `tests/skill_scanner.rs` full RED→GREEN suite, the status half of the `.gitkeep` tripwire | PR 2 (~400-550 lines) | PR 1 branch | Mirrors T3's resolved decision: code and the tests that justify it travel together, so no PR ships a module without the tests proving its acceptance criteria. |

### PR Boundary Decision — why not the proposal's literal split

The proposal's starting point was "PR1 = fixtures + RED tests + `model/location.rs`; PR2 = `roots`/`skills` turning them GREEN" (`proposal.md:124`). That literal split is **not mergeable**: `tests/skill_scanner.rs` calling `roots::skill_roots` or `skills::scan` before either module exists is a **compile error**, not a RED test — `cargo test --workspace --locked` fails the whole workspace build, not just the new suite, and CI has no way to distinguish "expected RED" from "broken build." This is the same failure mode T3 rejected its 3-PR split over (`archive/2026-08-18-skill-frontmatter-reader/tasks.md:22`): a reviewable PR cannot merge with a red or non-compiling CI run.

The adjusted boundary keeps PR 1 honestly green: the model change and its own tests need nothing from `roots`/`skills` to compile or pass (`SearchRootStatus` is a plain enum), and the fixture tree is inert data no test yet walks. Task 1.6 below (the fixture-integrity tripwire) is the one deliberate exception worth naming: it is split into two halves precisely so the disk-existence half can compile and pass in PR 1 while the status-assertion half — which needs `roots::skill_roots` — waits for PR 2. All skill-scanner-behavior RED tests (root resolution, walk policy, CA-6/8/9/12/14, the 69 count) are written and turned GREEN inside PR 2, same as T3's Phase 2+3 merge.

## Phase 1: Model Change, Bindings, and Fixture Tree (PR 1)

- [x] 1.1 In `crates/vertice-core/src/model/location.rs`, add `pub enum SearchRootStatus { Found, NotFound }` (`#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]`, `#[serde(rename_all = "camelCase")]`, same export attribute as `LocationOrigin`) and add `pub status: SearchRootStatus` to `SearchRoot`. — *domain-model spec: "SearchRoot Distinguishes Absent From Present"*
- [x] 1.2 Re-export `SearchRootStatus` from `crates/vertice-core/src/model/mod.rs` alongside the existing `location` re-exports.
- [x] 1.3 Run `cargo test -p vertice-core --locked` to regenerate `frontend/src/bindings/SearchRoot.ts` (modified: one new field, one new import) and `frontend/src/bindings/SearchRootStatus.ts` (new file). Commit both alongside 1.1/1.2 in the **same** commit — never split the Rust model edit from its regenerated bindings (proposal Risk row 1; design §2.3). Never hand-edit either `.ts` file.
- [x] 1.4 Add domain-model unit tests (in `model/location.rs` or a small `tests/` file) covering the three added scenarios verbatim: an absent-path `SearchRoot` is constructible with `status: NotFound` and no client-label field exists on the type; two `SearchRoot` values differing only in `status` (`Found` vs `NotFound`) compare unequal; a `SearchRoot` for a root with components keeps `id`/`path`/`kind` unchanged in type and value. These compile and pass without `roots.rs`/`skills.rs` — no RED step needed, the change is purely additive. — *domain-model spec, all three scenarios*
- [x] 1.5 Create the semantic fixture set under `crates/vertice-core/tests/fixtures/roots/` per design §8, one synthetic home per top-level directory, using `env!("CARGO_MANIFEST_DIR")` + per-segment `push` (never `"/"`-joined literals) when later referenced by tests:
  - `absent-roots/` — `.gitkeep` only (no `.claude`/`.agents`/`.config` at all)
  - `empty-alias/` — `.config/opencode/skill/.gitkeep` (CA-9, singular alias, present-and-empty)
  - `alias-populated/` — `.config/opencode/skill/demo/SKILL.md`
  - `underscore-shared/` — `.claude/skills/_shared/SKILL.md` (CA-8 partial)
  - `nested-skill/` — `.claude/skills/group/nested/SKILL.md` (recursion, depth 2)
  - `unreadable-entry/` — `.claude/skills/good/SKILL.md` + `.claude/skills/broken/SKILL.md` (a deliberate copy of a T3 corrupt fixture, never a walk target aimed at `fixtures/frontmatter/` itself — CA-12 + §5 escalation)
  - `project-decoy/` — `.claude/skills/real/SKILL.md` + `projects/demo/.claude/skills/fake/SKILL.md` (CA-14)
  - `plugin-decoy/` — `.claude/plugins/p/skills/x/SKILL.md` (CA-6)
- [x] 1.6a Create `empty-alias/`'s `.gitkeep` as in 1.5, and write the **disk-existence half** of the tripwire test now: `empty_alias_fixture_directory_still_exists_on_disk` asserting the directory is present on disk via `std::fs::metadata`. This half needs no `roots`/`skills` module and is GREEN in PR 1. (The `status == Found` half of the same tripwire lands in Phase 2 — see 2.7.)
- [x] 1.7 Create the tier-2 `reference/` fixture tree: 25 uniquely-named skills distributed 22 across all three roots, 1 only in `.claude/skills/`, 2 only in `.agents/skills/`, matching `alcance-poc-vertice.md:57-59, 79-81` exactly (23/24/22 per-root split, 69 total entries). Each `SKILL.md` is minimal, four lines (`name` + one-line `description`), generated by one rule so a reviewer verifies the rule and the distribution rather than 69 diffs.
- [x] 1.8 Confirm `.gitattributes` needs no change — line 2 already scopes `-text` to `crates/vertice-core/tests/fixtures/**`, which covers `roots/`.
- [x] 1.9 **[Gate, PR 1]** `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked` (all green, including 1.4's model tests and 1.6a's tripwire half); `git diff --exit-code -- frontend/src/bindings` is **not** expected clean here — confirm the diff is exactly `SearchRoot.ts` (modified) + `SearchRootStatus.ts` (new), nothing else. Report results; do not claim they ran if `cargo` is unavailable in the executing environment.

## Phase 2: `roots`/`skills` Modules — TDD (RED → GREEN) (PR 2)

- [x] 2.1 [RED] In `crates/vertice-core/src/roots.rs` (new), write `#[cfg(test)]` unit tests: alias grouping (`.config/opencode/skill/` and `.config/opencode/skills/` resolve to one `SearchRoot` id), root-id stability (hardcoded `SearchRootId` values, never path-derived), and the three-roots-always invariant (`skill_roots` returns exactly `[ResolvedRoot; 3]` for any `home`). No disk access — in-memory only.
- [x] 2.2 [GREEN] Add `walkdir = "2"` to `crates/vertice-core/Cargo.toml`. Implement `pub fn home_dir() -> Result<PathBuf, ScanError>` (using `std::env::home_dir()` per design §3 — the sole ambient-environment read in the crate) and `pub fn skill_roots(home: &Path) -> [ResolvedRoot; 3]` with `ResolvedRoot { root: SearchRoot, scan_paths: Vec<PathBuf> }`, hardcoded suffixes `.claude/skills/`, `.agents/skills/`, `.config/opencode/skills/` (+ singular alias in `scan_paths`), built with per-segment `PathBuf::push`, never OS config-dir APIs. Pass 2.1.
- [x] 2.2a **Dependency license contingency — verified, not needed.** Ran `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` locally once `walkdir` was added: output is `bans ok, licenses ok`. `walkdir`/`same-file`/`winapi-util`'s `Unlicense OR MIT` resolves via the already-allow-listed `MIT` half; `deny.toml` needed no change. Separately confirmed `std::env::home_dir()` does **not** trigger `clippy -D warnings` at this workspace's pinned toolchain (rustc 1.97.1) — the design §3 un-deprecation claim holds, so the `dirs = "6"` fallback was not exercised.
- [x] 2.3 [RED] Create `crates/vertice-core/tests/skill_scanner.rs`. One test (or tight group) per skill-scanner spec requirement, each pointed at its own synthetic-home fixture from Phase 1:
  - root resolution never touches an OS config-dir convention; OpenCode root is always `<home>/.config/opencode/skills/` (or alias)
  - singular/plural OpenCode alias scan as one logical root (`alias-populated/`)
  - `SKILL.md` presence is the sole detection rule; `_shared` is an ordinary skill, no name heuristic (`underscore-shared/`)
  - traversal is recursive; a `SKILL.md` two levels deep is discovered (`nested-skill/`)
  - symlinks are not followed (unit-level contract per design §6 — no portable fixture; assert `follow_links(false)` is set, not crate default)
  - `absent-roots/` yields zero components, zero issues, `status: NotFound` on all three roots
  - `empty-alias/` yields zero components, zero issues, `status: Found` (CA-9, second half of the tripwire — see 2.7)
  - absent and present-empty are distinguishable in one scan result (both fixtures inspected together)
  - every produced `Component` has `scope: Scope::User`; `project-decoy/`'s nested `.claude/skills/` outside the resolved roots yields nothing (CA-14)
  - `plugin-decoy/` yields nothing (CA-6, asserted structurally — no exclusion filter exists to test)
  - `unreadable-entry/` yields one `ScanIssue` (severity `Error`, escalated) carrying the corrupt file's path, and both sibling skills are still discovered (CA-12 partial)
  - a full scan leaves every fixture file's bytes unchanged (CA-16, read-only)
  - `reference/` yields exactly 69 produced-components-plus-issues, and (non-binding corroborator) 25 distinct `ComponentId`s
- [x] 2.4 [GREEN] In `crates/vertice-core/src/skills.rs` (new): `pub struct SkillScan { roots: Vec<SearchRoot>, components: Vec<Component>, issues: Vec<ScanIssue> }` (no `Serialize`/`TS` — same non-model status as T3's `SkillFrontmatter`-adjacent types) and `pub fn scan(home: &Path) -> SkillScan`. Walk each `ResolvedRoot.scan_paths` with `walkdir::WalkDir::new(..).follow_links(false).sort_by_file_name()`, matching `entry.file_name() == "SKILL.md"`, calling `frontmatter::read` per match and assembling `Component { id: ComponentId::derive(Skill, &fm.name), kind: Skill, scope: Scope::User, locations: vec![Location { path: Some(path), root: root_id, origin: File }], provenance_hint: None, .. }`. Pass 2.3.
- [x] 2.5 Implement `fn escalate(issue: ScanIssue) -> ScanIssue` (design §5): maps every `frontmatter::read` failure severity to `IssueSeverity::Error` uniformly, `path`/`reason` untouched. Add a direct unit test asserting the mapping for each of T3's severity classes (including the deferred-to-T16 BOM/`NoOpeningFence` case, now surfacing as `Error`).
- [x] 2.6 Implement the entry-level and root-probe error paths per design §7's table: `NotFound` on root probe → `status: NotFound`, no issue; any other `io::Error` on root probe → `status: Found` + `ScanIssue { severity: Error, path: Some(root), reason: "could not inspect search root: {io}" }`; unreadable subdirectory mid-walk → `ScanIssue` with the same-root walk continuing; root path exists but is not a directory → `ScanIssue`, walk continues to other roots.
- [x] 2.7 Implement the non-UTF-8 discovered-path case (design §7.1): skip the file, emit `ScanIssue { severity: Error, path: None, reason: "skipped a file whose path is not valid UTF-8: {lossy}" }`, never emit a `Component`. Add the `#[cfg(unix)]`-gated unit test on the path-conversion helper using `std::os::unix::ffi::OsStrExt::from_bytes` (design §7.1: no portable fixture exists; this is deliberately not exercised on the Windows CI leg). Also finish the `.gitkeep` tripwire from 1.6a: add `empty_alias_root_status_is_found` asserting `skill_roots` resolves `empty-alias/` to `status: Found` — this is the half that needed `roots.rs` to exist.
- [x] 2.8 Implement home-directory resolution failure (design §7.2): `home_dir()` returns `Err(ScanError::Internal { reason })` when `std::env::home_dir()` returns `None`, or when the resolved path is not UTF-8-representable. No new `ScanError` variant. Add a unit test constructing this failure without touching the real environment.
- [x] 2.9 Wire `pub mod roots;` and `pub mod skills;` in `crates/vertice-core/src/lib.rs` — two plain lines, no crate-root re-export, matching the existing `pub mod model; pub mod yaml;` style.
- [x] 2.10 [REFACTOR] Confirm `escalate`, walker internals, and any helper types stay appropriately private (only `SkillScan`, `scan`, `home_dir`, `skill_roots`, `ResolvedRoot` are `pub`); `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Phase 3: Verification (local, pre-commit gates)

- [x] 3.1 `cargo fmt --all --check`.
- [x] 3.2 `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 3.3 `cargo test --workspace --locked` — Phase 1 and Phase 2 suites, in-module units, all green.
- [x] 3.4 `cargo deny check bans licenses` — confirm green after 2.2a's contingency (if applied).
- [x] 3.5 **Read-only grep (CA-16, `rules.apply`)**: confirm no `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, or `remove_*` anywhere in `roots.rs`, `skills.rs`, or `tests/skill_scanner.rs`.
- [x] 3.6 Confirm `git diff --exit-code -- frontend/src/bindings` is clean (PR 2 introduces no further binding change — the shape settled in PR 1).
- [x] 3.7 From `frontend/`: `npm run lint && npm run check && npm run test && npm run build`. No frontend source consumes `SearchRoot`/`SearchRootStatus` yet (window closes at T10), so this gate is a regression check on the regenerated bindings compiling cleanly, not new behavior.
- [x] 3.8 **Platform note**: fixtures run on all three CI platforms via the existing matrix automatically. Windows is the only platform this session's reasoning is verified against (home-dir suffixes, `std::env::home_dir()` MSRV claim, absence of symlinks/junctions); macOS/Linux revalidation and the junction-vs-symlink question are explicitly deferred to **T16**, per design §9/§11 — no manual system verification is required here beyond noting that deferral.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines (logic + tests, excluding the 69-entry tree) | ~500–720 (`roots.rs` 150–250, `tests/skill_scanner.rs` + unit tests 250–350, `model/location.rs` + regenerated bindings 25–40, semantic fixtures ~10 files 50–70) |
| Estimated changed lines (69-entry minimal-content fixture tree, tracked separately) | ~280 (69 files × ~4 lines each — bulk, not complexity; a reviewer verifies the generator rule and the distribution, not 69 diffs) |
| **Total estimated changed lines** | **~780–1000** |
| 400-line budget risk | **High** by line count (both PR 1, once the 69-entry tree lands there, and PR 2 individually risk crossing 400); **Medium** by reviewer cognitive load — the 69-entry tree is mechanical bulk, and PR 2's logic is the only genuinely dense review surface |
| Chained PRs recommended | **Yes** |
| Decision needed before apply | **Yes** — `delivery_strategy: ask-on-risk` means the orchestrator must confirm the 2-PR chain (and the adjusted boundary above, which diverges from the proposal's literal split) before `sdd-apply` starts |

### Delivery Decision (product owner, 2026-08-18) — `single-pr` with `size:exception`

**Resolved: one PR, not a chain.** The product owner reviewed the forecast above and accepted a
`size:exception` for a single PR of ~780-1000 changed lines. The chained-PR analysis below is
retained as the rejected alternative and as the rationale record; it is NOT the plan.

Consequences for `sdd-apply`:

- `delivery_strategy: single-pr`, `size:exception` recorded. `chain_strategy` does not apply.
- **Strict TDD is unaffected.** RED -> GREEN still holds inside the PR, expressed as ordered
  commits rather than separate PRs: the fixture tree and the model change land first, the failing
  skill-scanner suite next, the `roots.rs`/`skills.rs` implementation last. Phase ordering in this
  file is unchanged; only the PR boundary is removed.
- The compile-order constraint that killed the proposal's original split still binds at commit
  level: no commit may leave `cargo test --workspace --locked` unable to *compile*. A RED test
  compiles and fails an assertion; a test calling a module that does not exist yet does not.
  The RED commit therefore lands together with the module skeletons (signatures plus
  `todo!()`/empty returns), not before them.
- The `model/location.rs` change and the regenerated `frontend/src/bindings/*.ts` remain one
  atomic commit, per the CI binding-drift gate.
- PR description MUST carry the `size:exception` label/justification, pointing at the forecast
  table above: ~280 of the changed lines are 69 near-identical four-line fixture files, so
  reviewer cognitive load is Medium, not High.

---

## Rejected alternative — chained PRs (retained for the record)

### Proposed slice boundaries (chain strategy: recommend `stacked-to-main`, PR 2 targets PR 1's branch)

- **PR 1 — Model change, bindings, and fixture tree** (~350–450 lines, dominated by the 69-entry tree). Delivers, independently: the `SearchRootStatus` enum and `SearchRoot::status` field with regenerated, committed bindings (closing the domain-model delta on its own), plus the full `tests/fixtures/roots/` tree with a GREEN, compiling test suite (domain-model unit tests + the disk-existence half of the `.gitkeep` tripwire). **Independently reviewable and mergeable**: nothing in this PR references `roots.rs`/`skills.rs`, so `cargo test --workspace --locked` passes on this branch alone, unlike the proposal's literal starting-point split (which would leave RED, non-compiling tests in PR 1 and fail CI).
- **PR 2 — `roots`/`skills` modules, RED→GREEN** (~400–550 lines). Delivers the skill-scanner capability end to end: root resolution, the recursive walker, `Component` assembly, severity escalation, and the full `tests/skill_scanner.rs` suite proving CA-6, CA-8 partial, CA-9, CA-12 partial, CA-14, and the 69-entry count. Targets PR 1's branch; both PRs merge to `main` in order.

Dependency diagram:

```
main
 └─ PR 1: model + bindings + fixtures        (this PR is 📍 for review-1)
     └─ PR 2: roots.rs + skills.rs + suite    (this PR is 📍 for review-2)
```
