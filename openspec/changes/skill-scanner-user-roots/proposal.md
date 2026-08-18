# Proposal: Skill Scanner over User Roots

> Plan trace: **T4** (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md:110-128`.
> Acceptance criteria: closes **CA-6** (no plugin skill appears), **CA-8 (partial)** (`_shared` appears as a skill), **CA-9** (`~/.config/opencode/skill/` present and empty produces neither error nor entry), **CA-14** (no project-scope component appears). Contributes to **CA-12** (unreadable files are reported with their path, scan continues) and is bound by **CA-16** (read-only) and **CA-17** (fixture-based tests on three platforms). The 69-on-disk entry count is asserted here; the 25-unique consolidation is **T8**.

## Intent

T3 delivered a leaf primitive: `frontmatter::read(&Path)` turns one file into typed fields or a `ScanIssue` (`crates/vertice-core/src/frontmatter.rs:70-77`). Nothing calls it. The crate still cannot answer the PoC's first real question — *what is installed on this machine?* — because no module resolves a root, no module walks a tree, and no module assembles a `Component`.

T4 closes that gap for skills. It is the first module in the crate that **discovers** paths from disk rather than receiving them, which makes it the first place where root resolution, walking policy, and the "a root that does not exist is an absence, not a failure" contract must be decided.

Two constraints make this less mechanical than it looks. First, client roots **cannot** be derived from OS conventions: OpenCode uses `~/.config/opencode` on Windows and Claude Code uses `~/.claude`, neither `%APPDATA%` (`internal-docs/alcance-poc-vertice.md:106`). The rule the scope document derives is explicit — the scanner never infers foreign paths from OS conventions, it only applies them to Vertice's own data directory. Second, `SearchRoot` as merged in T2 (`crates/vertice-core/src/model/location.rs:46-50`) is `{ id, path, kind }` and cannot express "this root was looked for and is not on disk", which CA-9 and the "report clients not found rather than omitting them silently" requirement (`alcance-poc-vertice.md:29`) both need. T4 is therefore **not purely additive**.

## Scope

### In Scope

- A new `vertice-core` module (working name `roots`) owning **home-directory resolution plus hardcoded relative suffixes** for the three roots: `~/.claude/skills/`, `~/.agents/skills/`, `~/.config/opencode/skills/`, plus the singular alias `~/.config/opencode/skill/` treated as the same root (the glob extracted from the OpenCode binary is `{skill,skills}/**/SKILL.md`). Sibling of `model/`, mirroring how `frontmatter.rs` is the filesystem-touching sibling — root resolution cannot live in `model/`, whose module-doc allow-list forbids `std::env` and `std::fs` (`crates/vertice-core/src/model/mod.rs:8-15`).
- A **recursive** walker producing one candidate per `SKILL.md` found under a root. Detection rule, verbatim from the plan: **if there is a `SKILL.md`, it is a skill.** No name-convention heuristic, so `_shared` enters as an ordinary skill (**CA-8 partial**, finding 6).
- `Component` assembly for skills: `id` via `ComponentId::derive`, `kind: Skill`, `scope: User`, one `Location { path: Some(_), root, origin: File }`, `description` from the T3 reader.
- A **model change** letting `ScanReport::roots_scanned` distinguish *absent* from *present-and-empty* from *present-with-entries*. Shape is a design decision; the requirement is that an absent or empty root is recorded and produces **no `ScanIssue` and no component** ("sin error y sin entradas", plan line 118).
- Two-tier fixtures under `crates/vertice-core/tests/fixtures/roots/` (the location T3 reserved, `design.md:180-183`): a small semantic set (singular `skill/` alias, empty root, absent root, `_shared`, one skill present in all three roots) plus one minimal-content tree totalling **69** `SKILL.md` entries for the literal count criterion.
- Fixture-first TDD tests for CA-6, CA-8-partial, CA-9, CA-14, the 69 count, and the "one unreadable file does not stop the walk" guarantee.

### Out of Scope

- **Consolidation of 69 on-disk entries into 25 unique components** and duplicate marking — **T8**. T4 deliberately emits duplicates.
- Agents of any client — **T5** (Claude Code) and **T6** (OpenCode).
- Client installation detection and versions — **T7**.
- `ScanReport` assembly, `duration_ms`, and the scan-wide "one bad adapter does not abort" orchestration — **T9**. T4 returns roots, components, and issues; it does not build the report.
- IPC exposure, Tauri commands, any frontend surface — **T10**. The regenerated bindings in this change are a byproduct of the model edit, not a feature.
- MCP servers, project scope, and **every write operation** — out of the PoC entirely (`alcance-poc-vertice.md:33-36`).
- macOS/Linux path revalidation — **T16**. Ground truth here is one Windows machine (`alcance-poc-vertice.md:71`).

## Capabilities

### New Capabilities

- `skill-scanner`: root resolution for the three user roots plus the singular alias, recursive `SKILL.md` discovery, the "`SKILL.md` ⇒ skill" detection rule, absent/empty-root reporting, and skill `Component` assembly at `Scope::User`.

### Modified Capabilities

- `domain-model`: `SearchRoot` (or an adjacent type) gains the ability to represent a root that was resolved but not found on disk, so `roots_scanned` can report unfound clients instead of silently omitting them.

## Approach

**Resolve the home directory, concatenate hardcoded suffixes — never an OS config-dir convention.** This is the single highest-value decision in T4 and it is already settled by evidence: `opencode debug paths` shows XDG layout on Windows, and Claude Code sits at `~/.claude` (`alcance-poc-vertice.md:106`). A `config_dir()`-style call would return `%APPDATA%` on Windows and find zero skills. OS-idiomatic directory logic remains reserved for Vertice's own app-data directory.

**A new sibling module, not an extension of `model/` or `frontmatter.rs`.** `model/`'s purity invariant is mechanical and stated in its module doc; `frontmatter.rs` is deliberately caller-agnostic and path-agnostic (`design.md:111`), and putting walking logic inside it would give the leaf reader knowledge of caller intent it was designed not to have.

**Recursive walk, not fixed-depth `read_dir`.** On the reference machine all 69 files sit exactly one level below their root, but OpenCode's own glob is `{skill,skills}/**/SKILL.md`. Implementing depth-1 would be fitting the code to one observation instead of to the client's documented behavior. The flat reality is a *fixture-realism* note, not a licence to hardcode depth.

**CA-6 is satisfied structurally, not by exclusion logic.** Verified this session on the reference machine: **no `~/.claude/plugins/` directory exists**, and plugin-provided skills live under none of the three roots. Scoping the walk to exactly three roots therefore already guarantees no plugin skill appears. T4 writes **no active plugin-exclusion filter** — inventing one would be code defending against a case it cannot reproduce. Caveat: verified on one Windows machine only; T16 revalidates, and if a plugin root surfaces there, exclusion becomes a T16 delta rather than speculative code today.

**CA-14 is satisfied by construction.** All three roots live under the user's home; T4 never constructs a project-scoped root and only ever emits `Scope::User`. No filtering logic. The test asserts the absence, not a filter.

**An absent root is not a `ScanIssue`.** The plan is explicit. Issues are for things that went wrong; a client the user has not installed is a fact about the machine. This is why the model needs a place to say "looked for, not there" — and why `ScanIssue` is the wrong place to say it.

**Two fixture tiers, never one.** Semantic fixtures stay small and readable so a reviewer can see what each case proves; the 69-entry tree exists solely to satisfy a count and carries minimal content. Per T3's inherited rule, no walker is ever aimed at `fixtures/frontmatter/` — that would couple T4's counts to T3's deliberately-broken files.

**Dependencies: candidates, not commitments.** `dirs` (home resolution) and `walkdir` (recursive walk) both already appear transitively in `Cargo.lock`, neither is banned by `deny.toml`, and both carry allow-listed licenses. Hand-rolled alternatives exist — `std::env::var("USERPROFILE"/"HOME")` and a recursive `std::fs::read_dir` — and T3's precedent was to hand-roll rather than re-trigger crate governance. Design decided `std::env::home_dir()` (not `dirs`) plus `walkdir`; see `design.md` §3. **Verified at apply time**: `cargo deny check bans licenses` passes (`bans ok, licenses ok`) with `walkdir` promoted to a direct dependency — no `deny.toml` change was needed.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/roots.rs` (name TBD) | New | Root resolution, walker, `Component` assembly |
| `crates/vertice-core/src/lib.rs` | Modified | Declare the new module |
| `crates/vertice-core/src/model/location.rs` | **Modified** | `SearchRoot` gains absent/found representation |
| `frontend/src/bindings/*.ts` | Regenerated | Byproduct of the model edit; CI gates on drift |
| `crates/vertice-core/src/frontmatter.rs` | Unchanged | Consumed as-is; first real caller |
| `crates/vertice-core/tests/fixtures/roots/` | New | Semantic set + 69-entry tree |
| `crates/vertice-core/tests/` | New | Walker and root-resolution suites |
| `crates/vertice-core/Cargo.toml`, `deny.toml` | Possibly modified | Only if design adopts `dirs`/`walkdir` |
| `vertice-app`, frontend source, CI workflows | Unchanged | No IPC, no command, no gate change |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Model change to `location.rs` regenerates bindings and trips the CI drift gate if not committed | Med | Regenerate with `cargo test -p vertice-core` and commit bindings in the same PR; never hand-edit `frontend/src/bindings/` |
| 69-entry fixture tree inflates the diff past the 400-line review budget | High (line count) / Low (cognitive load) | Slice into PRs — fixtures+RED tests first, implementation second, as T3 did (PRs #3 and #5). See forecast below |
| Symlinked skill directory causes an infinite loop or a duplicate entry | Low-Med | Symlink-following policy MUST be an explicit documented decision in design, never an unexamined crate default; a fixture is impractical on the Windows CI leg, so the policy is asserted by unit-level contract |
| `walkdir` fails the `cargo deny` gate | Low — **did not materialize**: verified locally, `bans ok, licenses ok` | Design records the hand-rolled fallback; not needed |
| CA-6 conclusion rests on one Windows machine with no `~/.claude/plugins/` | Med | Recorded as a structural claim with its evidence and its limit; T16 revalidates on macOS/Linux and adds exclusion only if a real plugin root appears |
| BOM-prefixed `SKILL.md` silently skipped as a `Warning` (T3's known false negative, `design.md:270`) | Low-Med | In scope to *decide*, see Open Questions |
| Fixture tree drifts from the real 69/25 distribution and T8's assertions inherit the error | Med | Fixture layout mirrors the exact split recorded at `alcance-poc-vertice.md:146-153` (22 in three roots, 3 in one, none in exactly two) |

## Open Questions

**Resolved by the product owner (2026-08-18):**

- **Absent-root reporting shape.** For the PoC a root reports as *not found by path* (e.g. `~/.claude/skills not found`); `SearchRoot` does NOT carry a client display label. This keeps the `model/location.rs` change minimal and its TypeScript surface small. Naming the client in the UI is deferred to the UI phases, which already know the client from `SearchRootKind`.
- **Symlink-following policy — evidence.** No symlinks exist under any of the three roots on the reference Windows machine (verified 2026-08-18). Design documents "do not follow" as the explicit choice; no symlink fixture is required.

**Committed to resolving in `sdd-design`:**

- The exact shape of the `SearchRoot` model change (new field, new enum, or a wrapper type) and its TypeScript surface.
- **Severity escalation** — T3 left it open whether T4 escalates a leaf `Warning` on a discovered `SKILL.md` to `Error` (`design.md:271`). T4 is the layer that knows the file was expected to be a skill, so the decision belongs here and nowhere later.
- **Non-UTF-8 paths.** T4 is the first module that discovers paths from disk, so T2's `path: None` plus lossy-rendering contract becomes reachable for the first time. Design states the behavior even if no fixture can portably produce it.
- Whether to take `dirs`/`walkdir` or hand-roll both.
- Module and function names; whether the walker returns `(Vec<Component>, Vec<ScanIssue>)` or an owned result type.

**Deferred, with target:**

- **UTF-8 BOM handling** (`design.md:270`) — deferred to **T16**, where real-machine validation on three platforms determines whether a BOM-prefixed `SKILL.md` occurs in practice. Design records the deferral and the current behavior (skipped as a `Warning`) so it is a known state, not a surprise.
- **Plugin-root exclusion logic** — deferred to **T16**. Structurally unnecessary today; revisited only if macOS/Linux validation surfaces a plugin root inside one of the three scanned roots.

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. Fixtures and failing tests land before implementation, and the 69-entry tree must exist before any assertion counts it. This is what makes the two-PR slice below the natural shape rather than an artificial split.

## Changed-Line Forecast

| Bucket | Est. lines |
|---|---|
| `roots` module implementation | 150–250 |
| Tests (root resolution, walker, CA cases) | 250–350 |
| `model/location.rs` + regenerated bindings | 25–40 |
| Semantic fixtures (~10 files) | 50–70 |
| 69-entry tree (69 files × ~4 lines) | ~280 |
| **Total** | **~750–1000** |

**400-line budget risk: High by line count, Medium by reviewer load.** The 69-entry tree is ~280 added lines across 69 near-identical four-line files — bulk, not complexity: a reviewer verifies the *generator rule and the distribution*, not 69 diffs. But honesty matters more than the rationalization: even excluding that tree the change lands around 500–700 lines, which still exceeds the budget on its own. **Recommendation: two chained PRs**, matching the T3 precedent — (1) fixtures + RED tests + the `model/location.rs` change with regenerated bindings; (2) the `roots` module turning them GREEN. Final slicing is `sdd-tasks`'s call.

## Rollback Plan

Not purely additive — the model edit crosses two of the three layers.

- **Core**: delete the new module, its tests, and `tests/fixtures/roots/`; revert the `pub mod` line in `lib.rs`. `frontmatter.rs` and `yaml.rs` are read-only inputs, untouched by the revert.
- **Model + bindings (the load-bearing part)**: reverting `model/location.rs` requires re-running `cargo test -p vertice-core` to regenerate `frontend/src/bindings/*.ts` and committing the result. Reverting the Rust file **without** regenerating leaves the bindings drifted and CI red. This must be one atomic revert of both.
- **App (`vertice-app`)**: zero impact — no command registered, no capability change, `capabilities/default.json` untouched.
- **Frontend source**: zero impact — no component consumes `SearchRoot` yet, so a binding-shape change has no call sites to break. This window closes at T10; reverting is cheapest now.
- **CI / supply chain**: if design adopts `dirs`/`walkdir`, rollback also reverts `Cargo.toml`, `Cargo.lock`, and any `deny.toml` delta. If it hand-rolls, there is nothing to revert.

Reverting the branch restores the exact post-T3 state. No persisted data and no IPC contract depend on any of it.

## Dependencies

- **T2** (domain model, `Component`, `Location`, `SearchRoot`, `Scope`) — complete and archived.
- **T3** (`frontmatter::read`) — complete and archived; `frontmatter-reader` spec merged. T4 is its first caller.
- **Blocks**: T8 (consolidation 69 → 25), T9 (`ScanReport` assembly). Informs T5/T6 on walker conventions.

## Success Criteria

- [ ] Over fixtures reproducing the reference installation, the scanner produces exactly **69** on-disk skill entries, un-consolidated.
- [ ] No component from a plugin-provided root appears; the proposal records that this holds **structurally** via root scoping, with no exclusion filter written (**CA-6**).
- [ ] `_shared` appears as an ordinary skill, with no name-convention filtering anywhere in the code (**CA-8 partial**).
- [ ] A fixture root named `skill/` (singular) that exists and contains zero `SKILL.md` produces **no `ScanIssue` and no component**, and is still recorded as scanned (**CA-9**).
- [ ] An absent root is recorded as scanned-and-not-found, distinguishable from present-and-empty, and produces no issue (**CA-9**, `alcance-poc-vertice.md:29`).
- [ ] No component carries `Scope::Project` or `Scope::Local`; a fixture containing a project-shaped `.claude/skills/` tree outside the three roots yields nothing (**CA-14**).
- [ ] One unreadable or corrupt `SKILL.md` inside a walked tree yields a `ScanIssue` carrying its path while every sibling skill is still discovered (**CA-12 partial**).
- [ ] Root resolution derives no path from an OS config-dir convention; the three suffixes are hardcoded relative to the home directory only.
- [ ] No `File::create`, `OpenOptions::write`, or equivalent anywhere in the new module (**CA-16**).
- [ ] All tests read from `crates/vertice-core/tests/fixtures/roots/`; no test reads the author's machine, and no walker is aimed at `fixtures/frontmatter/` (**CA-17**).
- [ ] `frontend/src/bindings/*.ts` are regenerated and committed; the CI bindings-drift gate is green.
- [x] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and `cargo deny check bans licenses` verified locally on Windows during `sdd-apply` (all clean/green); the three-platform CI matrix still confirms macOS/Linux, per T16.
