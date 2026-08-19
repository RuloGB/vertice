# Design: Duplicate Consolidation

> Trace: **T8** (`internal-docs/plan-desarrollo-poc.md:191-207`) / closes **CA-2**, **CA-4**, and (with T11) **CA-3**, (with T4) **CA-8**; bound by **CA-16** (read-only) and **CA-17** (versioned fixtures, three platforms).
> Proposal: `openspec/changes/duplicate-consolidation/proposal.md`. Inherits T4's design (**T4D**, canonical root order, fixture discipline), T5D/T6D §"no shared scanner abstraction before T9" and T7D §7 (determinism) as closed decisions.
> `rules.design` coverage: core data model impact (§2 — **none**, and that is load-bearing); core/Tauri isolation for the CLI pathway (§1); IPC contract surface (§2 — **empty set**); per-OS paths (§1 — **none**: this module resolves no path); `ScanIssue` taxonomy and error paths (§8 — **empty by construction**, with the argument).
> **Environment note.** `cargo` did not resolve on PATH in this phase and no command was executed. Nothing below was verified by compiling. §0 separates what was verified by reading the repository from what is asserted by reasoning.

## 0. Verified by reading the repository

| # | Statement | Basis |
|---|---|---|
| V1 | `Component.locations` is already `Vec<Location>` and the type's doc already states the aggregated contract ("N locations sharing one `id`, never N separate components") | `model/component.rs:9-25` |
| V2 | `ComponentId` derives `Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS` — **no `Ord`, no `PartialOrd`** | `model/identity.rs:17-19` |
| V3 | Both file scanners emit exactly one single-element `locations` vector per on-disk find, `provenance_hint: None`, `scope: Scope::User` | `skills.rs:128-140`, `agents.rs:181-193` |
| V4 | `opencode_agents::assemble_component` already emits **N locations for one component** (one per declaring config file), so "one member = one location" is not an invariant | `opencode_agents.rs:190-209` |
| V5 | `agents::emit_embedded_components` emits `path: None`, `origin: Embedded`, root id `claude-embedded-agents` — a location with no file | `agents.rs:204-216`, `roots.rs:94-105` |
| V6 | `roots::skill_roots`/`agent_roots`/`opencode_agent_root` all take `home: &Path` and call `probe`, i.e. **they touch the filesystem**; their ids are hardcoded and test-asserted in order | `roots.rs:61,82,166,198,263-343` |
| V7 | The OpenCode skill root carries **two scan paths under one root id** (`skills/` plural and `skill/` singular), so two distinct locations can share one `SearchRootId` | `roots.rs:132-157,214-226` |
| V8 | `frontmatter` performs **no trimming and no blank-collapsing**: a missing key yields `None`, and a folded scalar retains its trailing `\n` (asserted verbatim) | `frontmatter.rs:26-29`, `tests/frontmatter_reader.rs:45-73` |
| V9 | MSRV floor is `1.88`, edition 2021; `unsafe_code = "deny"` workspace-wide | `Cargo.toml:7-8,19-20` |

**V6 is the sharpest constraint in this design and it is easy to miss**: the canonical root order lives in functions that require a `home` and hit the disk. A pure, `home`-free `consolidate` therefore **cannot call them**. §4 is the consequence.

## 1. Technical approach

One new pure module, no dependency, no `model/` edit, no scanner edit.

```
                                    vertice-core
 frontend ──IPC──> vertice-app ──>  ├── model/        (pure data, zero I/O)  ← UNCHANGED §2
                                    ├── roots         ← UNCHANGED, not even called §4
 future vertice-cli ───────────>    ├── skills / agents / opencode_agents  ← UNCHANGED
                                    └── consolidate   (NEW — zero I/O, zero clock)

   skills::scan(home).components  ─┐
   agents::scan(home).components  ─┼─> Vec<Component>  ──> consolidate ──> Vec<Component>
   opencode_agents::scan(..)      ─┘   (69 for the             (T8)          (25, sorted)
                                        reference fixture)
                                        ▲ concatenation is T9's job, not T8's
```

**CLI isolation.** `consolidate` adds no dependency (`Cargo.toml`, `Cargo.lock`, `deny.toml` byte-identical), imports only `crate::model` and `std`, and reads no ambient environment. `vertice-core` still imports nothing from `tauri`, so `cargo deny check bans licenses` is unaffected and a future `vertice-cli` binary calls the identical function with the identical semantics. **CA-16 structurally**: the module's disk surface is the **empty set** — no `std::fs`, no `std::io`, no `std::env`, no `SystemTime`. It cannot write because it cannot open.

## 2. Core data model impact: none

> **Decision: `crates/vertice-core/src/model/` is not opened. No `is_duplicate` field, no `Ord` derive on `ComponentId`, no new `TS` type. `frontend/src/bindings/*.ts` is byte-identical and no `ts-rs` regeneration is part of this change.**

`locations.len() > 1` is the duplicate signal (proposal, closed). Two temptations are ruled out here because both would be discovered mid-implementation:

| Temptation | Why it breaks the property | Verdict |
|---|---|---|
| `#[derive(PartialOrd, Ord)]` on `ComponentId` so groups can key a `BTreeMap` | A `model/` edit puts T8 on the bindings drift gate for a convenience. `id.as_str().cmp(..)` gives the same total order from outside (§5) | **Rejected** |
| `is_duplicate: bool` "for T11" | Redundant with `locations.len() > 1`; regenerates `Component.ts` | **Rejected** (proposal) |

**IPC contract surface: empty.** No Tauri command, no capability change, no frontend source file. T10 exposes; T8 computes.

## 3. Public surface

> **Decision: one free function, input taken by value.**

```rust
// crates/vertice-core/src/consolidate.rs

/// Merge components discovered under different search roots into one entry
/// per identity. Pure: no I/O, no clock, no ambient environment. Total: it
/// cannot fail and emits no `ScanIssue` (design §8).
#[must_use]
pub fn consolidate(components: Vec<Component>) -> Vec<Component>;
```

`lib.rs` gains exactly one line, `pub mod consolidate;`, with no crate-root re-export (matching `lib.rs:7-15`).

| Option | Consequence | Decision |
|---|---|---|
| **Free `consolidate(Vec<Component>) -> Vec<Component>`** | Matches the module-level-function precedent (`skills::scan`, `installations::scan`). Ownership lets the merge **move** names, descriptions and locations out of the members instead of cloning them; T9 concatenates the three scans into a `Vec` it has no further use for, so by-value is exactly its call shape | **Chosen** |
| `&[Component] -> Vec<Component>` | `Component` is not `Copy`; every surviving field would be cloned. It also invites `clippy::ptr_arg`-adjacent noise at call sites that own the vector, and buys nothing — no caller wants the un-consolidated list afterwards | **Rejected** |
| A `Consolidator` type / builder | There is no configuration and no state. A struct with one method and no fields is ceremony, and it would imply future knobs (content comparison) the proposal forbids | **Rejected** |

`#[must_use]` is included: the function's only effect is its return value, so discarding it is always a bug. No `#[allow]` of any kind is expected; the module must be clean under `cargo clippy --workspace --all-targets -- -D warnings`.

**Kinds: both.** One call consolidates skills **and** agents. `ComponentKind` is part of the identity key (`identity.rs:41-46`), so a skill and an agent sharing a name are structurally unmergeable — the function needs no kind parameter and gets no kind filter. All four T8 acceptance criteria are skill-shaped by coincidence of the reference installation, not by design.

## 4. The canonical root order, without calling `roots`

> **Decision: a private `const ROOT_ORDER: [&str; 6]` inside `consolidate.rs`, pinned to `roots.rs` by a test rather than by a call.**

```rust
const ROOT_ORDER: [&str; 6] = [
    "claude-skills", "agents-skills", "opencode-skills",       // roots::skill_roots
    "claude-agents", "claude-embedded-agents",                  // roots::agent_roots
    "opencode-agents",                                          // roots::opencode_agent_root
];
```

| Option | Consequence | Decision |
|---|---|---|
| Call `roots::skill_roots(home)` for the order | Requires a `home` parameter and **hits the filesystem** (V6): `probe` runs `symlink_metadata` per root. That destroys purity, makes the signature depend on data the function does not otherwise need, and makes the result depend on what exists on disk | **Rejected** |
| Add a `home`-free `roots::canonical_order()` | Modifies `roots.rs`, putting T4–T7's shared resolver on this change's regression surface for a constant list | **Rejected** |
| **Local `const` + a test asserting it equals the ids returned by `skill_roots`/`agent_roots`/`opencode_agent_root`** | Deliberate, recorded duplication (T7D §5.3's precedent). The duplication is inert data, and the test fails the moment a root is added, renamed or reordered | **Chosen** |

**The rank lookup must be total, and two real cases prove it.** `LocationRank` is computed as the index of the location's root id in `ROOT_ORDER`, or `ROOT_ORDER.len()` when the id is unknown. Unknown ids are not hypothetical hygiene — they are what a future root (T16, project scope) will look like before this constant is updated, and an unknown rank must degrade to "last, deterministically", never to a panic and never to an unstable position.

Ranking never mixes kinds in practice (a skill's locations come from skill roots), so the cross-kind interleaving of the array is unobservable. It is written in `skill_roots`-then-`agent_roots` order anyway, so a reader does not have to establish that.

**Location sort key** — total, and byte-wise rather than locale-collated (T7D §7):

```
LocationKey = (rank: usize, root_id: &str, path: Option<&Path>)
```

`root_id` is in the key because the fallback bucket holds many ids; `path` is in the key because **two locations can legitimately share a root id** (V7: the OpenCode plural/singular alias) and because `Option<&Path>` orders `None` before `Some` deterministically — the embedded pseudo-location (V5) therefore always sorts ahead of its file-backed siblings under the same root instead of floating.

## 5. Grouping: sort, then fold

> **Decision: sort the input once by `(id.as_str(), member key)`, then fold contiguous equal-id runs. No `HashMap` anywhere in the module.**

| Option | How hash order is kept out of the result | Decision |
|---|---|---|
| `HashMap<ComponentId, Vec<Component>>` | Only by a trailing total sort of both the output vector and each `locations` vector. Correct, but the guarantee lives in a sort a refactor can weaken, and `RandomState` re-randomizes per process, so a partial key would produce a run-to-run flake, not a platform flake | **Rejected** |
| `BTreeMap<String, Vec<Component>>` | Deterministic, but requires cloning each id into an owned `String` key because `ComponentId` has no `Ord` (V2) — and §2 forbids adding one | **Rejected** |
| **Sort-then-fold over the owned `Vec`** | Grouping order **is** the sorted order; there is no hash iteration to leak. Member precedence order falls out of the same sort, so the precedence walk needs no second ordering pass | **Chosen** |

Mechanically: `sort_by` comparing `a.id.as_str().cmp(b.id.as_str())` first (§2 — through `as_str`, no `Ord` derive), then the member key; then a single `into_iter()` loop accumulating a run while the id is unchanged. `slice::chunk_by` is available at MSRV 1.88 (V9) but operates on borrowed slices, so the owned loop is preferred over `chunk_by` + clone.

**Member key** = the member's own sorted `Vec<LocationKey>`, then its raw `name`. `Vec<T: Ord>` is `Ord`, so this is total; the `name` tiebreak covers the pathological case of two members with identical location sets. This is what makes the result **independent of arrival order**: nothing in the pipeline consults the input index.

## 6. Field precedence

> **Decision: per-field, first present and non-empty in member order, where "non-empty" means `trim().is_empty() == false`. The winning value is carried VERBATIM — never trimmed, never rewritten.**

| Field | Rule |
|---|---|
| `id`, `kind` | Identical across the group by construction (they are the grouping key); take the first member's |
| `name` | First member whose `name` is non-empty; if none, the first member's `name` verbatim |
| `description` | First member whose `Some(s)` has `s.trim()` non-empty; if none, the first member's `description` verbatim (preserving `None` vs `Some("")`) |
| `provenance_hint` | Same rule as `description` |
| `scope` | Not optional and not a string — precedence degenerates to **first member wins**. Unobservable in the PoC (every scanner emits `Scope::User`, V3), specified so it is not improvised |
| `locations` | Union of every member's locations, sorted by `LocationKey` (§4). Never deduplicated, never elected |

**Why "whitespace-only counts as empty", not "empty string counts as empty".** V8 is decisive: `frontmatter` does not trim, and a folded block scalar keeps its trailing newline — the suite asserts a description ending in `"\n"` verbatim. An empty folded scalar (`description: >` with nothing under it) therefore reaches `Component` as `Some("\n")`, which renders blank in T11 but is `Some` and non-empty under the narrow rule. Choosing the narrow rule would let a visually blank description from `claude-skills` suppress a real description from `.agents/skills`, which is precisely the failure the precedence rule exists to prevent. The predicate is a **selection** criterion only: the chosen value is stored as it was read, so consolidation never mutates a scanner's output and `Some("\n")` still survives when it is the only candidate.

**Why per-field and not per-root.** Per-root ("the highest-priority root supplying anything wins wholesale") would blank a description Vertice demonstrably knows, for the sake of a coherence property nothing consumes. The accepted cost, stated plainly: **the merged component may not equal any single on-disk copy**, and no per-field provenance is recorded. That is proposal Q5's answer — provenance is adequately carried by `locations`, and a per-field provenance field is a `model/` edit (§2).

## 7. Output ordering

> **Decision (orchestrator, post-proposal — closed): the returned list is sorted by display `name`, with `ComponentId` as a mandatory tiebreak. Each `locations` vector is in canonical root order (§4).**

```rust
out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.as_str().cmp(b.id.as_str())));
```

The tiebreak is not defensive padding. The sort key is a **derived** field — `name` is whatever §6's precedence elected — and two distinct components can legitimately share a display name: a skill and an agent called `triage` are correctly **not** merged (`identity.rs:81-86`) and then collide on name. Name alone leaves a real tie, and an unresolved tie is exactly how non-determinism leaks across the `[ubuntu-24.04, windows-2022, macos-14]` matrix that already bit both prior scanners. With the tiebreak the key is **unique** across the output (ids are pairwise distinct after grouping), so stable vs. unstable sorting is immaterial and no input-order residue can survive.

Comparison is byte-wise `str` ordering — **not** locale collation, **not** case-insensitive. `"Zeta"` sorts before `"alpha"`. Core guarantees determinism; human-facing collation is a **T11** presentation concern and T11 may re-sort freely.

## 8. Error paths: none, and why

> **Decision: `consolidate` returns a bare `Vec<Component>`. It produces no `ScanIssue`, has no `Result`, and no panicking path.**

The `ScanIssue` taxonomy is a taxonomy of **read failures**. This module reads nothing: no file, no directory, no environment variable, no clock. It has nothing to fail on. Returning `(Vec<Component>, Vec<ScanIssue>)` with a permanently empty second element would be a contract lie that T9 then has to merge into `ScanReport.issues`, and it would advertise a failure mode that cannot occur.

| Rejected issue | Why it is not an issue |
|---|---|
| "Descriptions of two copies disagree" | Content comparison — the explicit non-goal (proposal, `alcance-poc-vertice.md:67`). The module has no bytes to compare |
| "Unknown `SearchRootId` in a location" | §4 defines a total, deterministic fallback rank. An unknown root is a future root, not a defect |
| "Empty input" | The empty vector is a valid inventory; it returns empty |

No panics: no indexing, no `unwrap`, no `expect`, no slicing by computed index — every group is non-empty by construction (a run exists only because a member created it), so "first member" is always reachable through the iterator that built the run.

## 9. Testing strategy

`strict_tdd: true`; the counting and precedence tests land RED before `consolidate.rs` exists. Test command `cargo test && npm run test` — **not run in this phase** (`cargo` did not resolve on PATH).

| Layer | What | How |
|---|---|---|
| Unit — precedence | Blank/whitespace/`None` description in the earlier root, real one later → later value wins, verbatim; all-blank → first member's value preserved including `Some("")` vs `None` | `#[cfg(test)]` in `consolidate.rs`, hand-built `Component` values, no disk |
| Unit — rank totality | A `Location` with `SearchRootId("unknown-root")` sorts last, deterministically, and never panics; `None` path sorts before `Some` under the same root (V5, V7) | idem |
| Unit — root table pin | `ROOT_ORDER` equals the ids of `skill_roots` ++ `agent_roots` ++ `opencode_agent_root` for a synthetic non-existent `home` | idem — fails on any `roots.rs` rename/reorder (§4) |
| Unit — arrival independence | The same input reversed, and rotated, yields a byte-identical output vector | idem — the anti-"last write wins" tripwire |
| Unit — kind separation | A skill and an agent both named `triage` remain **two** components | idem |
| Unit — normalization reuse | Case variants and NFC/NFD variants collapse to one group with 2 locations, with **no** normalization code in the module | idem |
| Unit — edges | Empty input → empty output; single component → itself, `locations.len() == 1` | idem |
| Integration — CA-2 | `skills::scan` over `tests/fixtures/roots/reference/` → 69 components → `consolidate` → **exactly 25** | `tests/consolidation.rs` |
| Integration — CA-3 | **Exactly 22** with `locations.len() == 3`, each in canonical root order | idem |
| Integration — CA-4 | **Exactly 3** with `locations.len() == 1` (`claude-only-01`, `agents-only-01`, `agents-only-02`), and **zero** with `== 2` | idem |
| Integration — conservation | Sum of `locations.len()` across the output equals the input count (**69**). No copy hidden, no winner elected | idem — the defining-risk assertion |
| Integration — CA-8 | A `_shared`-shaped name consolidates like any other; **no** name-prefix or convention filter exists anywhere in the module | idem + structural review |
| Integration — precedence, real pipeline | New small fixture home: the `claude-skills` copy has a blank/folded-empty description, the `agents-skills` copy a real one → the real one survives. `Some("\n")` only arises through actual YAML, so this cannot be a unit test | idem |
| Determinism | Two consecutive `consolidate` calls yield byte-identical vectors; ordering assertions run on all three CI legs | idem |
| Read-only (CA-16) | Structural: the module contains no `std::fs`, `std::io`, `std::env`, `File::create` or `OpenOptions` | grep + review |
| Regression | T2–T7 suites stay green with **no edits at all**; `frontend/src/bindings/` untouched | existing suites |

New fixture: one small home under `crates/vertice-core/tests/fixtures/roots/` for the precedence case only. `tests/fixtures/roots/reference/` is **reused unmodified** — it is the CA-2/CA-3/CA-4 oracle (69 files → 25 names: 22×3, 3×1, 0×2). Fixture paths are built from `env!("CARGO_MANIFEST_DIR")` with per-segment `push`, never `"/"`-joined literals.

## 10. File changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/consolidate.rs` | **Create** | §3–§7: `ROOT_ORDER`, `LocationKey`, sort-then-fold, field precedence, output sort |
| `crates/vertice-core/src/lib.rs` | Modify | one line: `pub mod consolidate;` |
| `crates/vertice-core/tests/consolidation.rs` | **Create** | §9 integration suite |
| `crates/vertice-core/tests/fixtures/roots/<precedence home>/**` | **Create** | §9, blank-vs-populated description only |
| `crates/vertice-core/tests/fixtures/roots/reference/**` | **Unchanged, reused** | CA-2/CA-3/CA-4 oracle |
| `crates/vertice-core/src/roots.rs` | **Unchanged** | §4 — not called, not modified, not even a visibility change |
| `crates/vertice-core/src/model/**` | **Unchanged** | §2 |
| `frontend/src/bindings/**` | **Unchanged** | no `TS` type added; drift gate green with **no regeneration** |
| `crates/vertice-core/src/{skills,agents,opencode_agents,installations,frontmatter,yaml,jsonc}.rs` | **Unchanged** | T8 consumes their output; shares no code path |
| `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Unchanged** | no new dependency |
| `crates/vertice-app/**`, `frontend/src/**`, `.github/workflows/**` | **Unchanged** | no IPC, no command, no capability, no MSRV change |

**Migration: none.** No persisted data, no IPC contract, no consumer exists yet.

**Rollback.** Delete `consolidate.rs`, `tests/consolidation.rs` and the precedence fixture; revert one `lib.rs` line. Nothing else moves — no lockfile, no `deny.toml` entry, no `model/` edit, no binding regeneration, no `roots.rs` change. Reverting the branch restores the exact pre-T8 state.

## 11. Open questions

- [x] **Public surface** — free `consolidate(Vec<Component>) -> Vec<Component>`, `#[must_use]`, by value to move rather than clone. §3.
- [x] **"Non-empty"** — whitespace-only counts as empty (`trim().is_empty()`), because folded scalars carry a trailing `\n` (V8); the winner is stored verbatim. §6.
- [x] **Per-field vs. per-root precedence** — **per-field**, with the "merged component may match no single copy" cost recorded. §6.
- [x] **Agents too?** — **yes**, one kind-agnostic call; `kind` is in the identity key so cross-kind merging is impossible. §3.
- [x] **Root-rank lookup** — local `const ROOT_ORDER` pinned to `roots.rs` by a test, never a call (purity, V6); unknown ids get a total, last-place fallback rank. §4.
- [x] **Fold data structure** — sort-then-fold, no `HashMap` and no `BTreeMap`, so hash order cannot leak into the output or into `locations`. §5.
- [x] **Output ordering** — display `name`, `ComponentId` tiebreak, byte-wise. §7.
- [x] **Error paths** — none; no `ScanIssue`, no `Result`, no panic path. §8.
- [ ] **Human-facing collation of the component list** (accents, case, locale) — **T11**; core guarantees determinism, not linguistic ordering. §7.
- [ ] **Invocation from the scan orchestrator and `ScanReport` assembly** — **T9**.
- [ ] **Identical vs. divergent duplicate detection** (`issue-creation`) — post-PoC increment 1; requires content awareness this module refuses. §8.
