# Proposal: Duplicate Consolidation

> Plan trace: **T8 — Consolidación de duplicados** (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md:191-207`.
> Acceptance criteria: **CA-2** — "25 skills appear, not 69" (`alcance-poc-vertice.md:161`); **CA-3** — "the 22 duplicated skills appear marked, each showing its three paths" (`:162`); **CA-4** — "`caveman`, `source-command-sdd-init` and `source-command-sdd-onboard` appear **without** a duplicate mark" (`:163`); **CA-8** — "`_shared` appears as one more skill, with no name-convention filtering, marked as duplicated" (`:167`). Bound by **CA-16** (read-only) and **CA-17** (versioned fixtures, three platforms). Mapping table `plan-desarrollo-poc.md:363-369` assigns CA-2 and CA-4 to T8 alone, CA-3 to T8 + T11, CA-8 to T4 + T8.

## Intent

This is **the first real product function of Vertice**, not a presentation detail (`alcance-poc-vertice.md:27`). Every adapter shipped so far (T4–T7) answers *what exists on disk*. None answers *what the user actually has*.

The gap is measurable on the reference installation and is already pinned in a fixture: `crates/vertice-core/tests/fixtures/roots/reference/` holds **69 `.md` files resolving to 25 unique component names** — 22 present in all three roots, 3 present in a single root, none in exactly two. Every scanner today deliberately emits one `Component` per on-disk find with a single-element `locations` vector; `openspec/specs/skill-scanner/spec.md` pins those 69 un-consolidated entries and explicitly names **T8** as the consumer that fixes it.

The product statement in `alcance-poc-vertice.md:54-55` is the whole change: a skill present in more than one root appears **once**, marked as duplicated, showing **all** the paths — no copy hidden, no "winner" elected. Vertice reports; it does not decide and it does not write.

Verified this cycle: **no consolidation code exists** (`grep -rn "consolidat" crates/` hits only a doc comment in `tests/frontmatter_reader.rs:63`), and **the T2 model already supports the outcome**. `Component.locations` is already `Vec<Location>` and `component.rs:9-12` already documents the aggregated contract. `ComponentId::derive(kind, name)` already normalizes trim → NFC → lowercase and is unit-tested for case and NFC/NFD collapsing. T8 makes an existing contract true; it does not extend it.

## Scope

### In Scope

- **One new pure module** `crates/vertice-core/src/consolidate.rs` — a sibling of `skills.rs` / `agents.rs` — exposing a single post-scan function of the shape `consolidate(Vec<Component>) -> Vec<Component>`.
- **Grouping by `Component.id`.** Identity is `(kind, name)` normalized, exactly as `model/identity.rs` already derives it. No second normalization, no new identity rule, no hashing.
- **Location merging that preserves EVERY entry.** N finds yield one `Component` with N `Location` values. No location is dropped, deduplicated away, or elected as the winner — CA-3 and CA-8 depend on all three paths staying visible.
- **A normative field-precedence rule for divergent duplicates**: for `name` (display form), `description`, `provenance_hint` and `scope`, take the **first present and non-empty value found while walking the roots in the canonical `roots.rs` order**, not simply the highest-priority root's value. A skill missing its description in `claude-skills` MUST NOT render blank when another root supplies one. This rule is specified and tested; it MUST NOT depend on scan arrival order.
- **Deterministic output ordering**, asserted by test. Both existing scanners already pay for stable ordering (`sort_by_file_name`, explicit `sort_by_key`) precisely because CI runs Linux + Windows + macOS; consolidation output and the merged `locations` vector follow the same canonical root order.
- **Zero model change.** "Duplicated" is derivable as `locations.len() > 1`. No new field, no `ts-rs` regeneration.
- **End-to-end tests against the existing `tests/fixtures/roots/reference/` tree** — no new fixture home required for the headline assertions.

### Out of Scope

- **Content comparison.** `locations.len() > 1` is the sole duplicate signal; T8 has zero content awareness. Distinguishing *identical* from *divergent* duplicates opens diffing and drift, explicitly deferred (`alcance-poc-vertice.md:67`, `plan-desarrollo-poc.md:199`). The known real case — `issue-creation`, two distinct versions across its three copies — is a **documented non-goal**, reserved for the "duplicado divergente" increment (`alcance-poc-vertice.md:193`).
- **An `is_duplicate: bool` model field.** Rejected: redundant with `locations.len() > 1`, and needless type-contract churn.
- **Any change to the scanners.** `skill-scanner` keeps emitting un-consolidated entries by its own merged spec; consolidation cannot live there anyway, since duplicates span three roots read by different adapters and the merge is only possible after all adapters have run.
- **Scan orchestration, `ScanReport` assembly, `duration_ms`, `ScanIssue` aggregation** — **T9**. T8 takes an already-flattened `Vec<Component>` and stays decoupled from wiring that does not exist yet.
- **The visual duplicate mark** — **T11**. T8 makes the data true; the UI renders it. CA-3 is shared T8 / T11 by the plan's own mapping.
- **IPC exposure, Tauri commands, any frontend surface** — T10.
- **Any write operation, project scope, MCP servers** — outside the PoC.

## Capabilities

### New Capabilities

- `duplicate-consolidation`: grouping scanned components by derived identity, merging locations without loss or election, the field-precedence rule for divergent duplicates, deterministic ordering, and the `locations.len() > 1` duplicate signal.

### Modified Capabilities

None. `domain-model` is consumed exactly as merged — `Component.locations` is already `Vec<Location>`. `skill-scanner`, `agent-scanner`, `opencode-agent-scanner` and `client-installation-detector` are untouched.

> A new capability spec (consistent with T4–T7, each of which got its own) rather than a `domain-model` delta. CA-2, CA-3, CA-4 and CA-8 are currently **unhomed in any spec**; closing that gap is part of this change.

## Approach

**One pure function, no I/O, no orchestration.** `consolidate` reads no file, resolves no path and touches no clock. It takes the flattened output of all scanners and returns the consolidated list. That signature is what keeps T8 independent of the T9 orchestrator and testable against a fixture-derived input with no filesystem in the loop beyond the scanners themselves.

**It lives in `src/consolidate.rs`, not in `model/` and not in the scanners.** `model/` is plain data with a declared import allow-list and zero I/O — a transformation over components is behavior, not data. The scanners are ruled out by their own merged spec and, more fundamentally, by the fact that no single adapter can see all three roots.

**Grouping key is `Component.id`, unchanged.** Identity is already `"{kind}:{normalized name}"`, derived from `(kind, name)` alone — never from `Location` or content. Consolidation is therefore a `HashMap`/`BTreeMap` fold over an existing, already-tested key. `_shared` needs no special case (CA-8): it has a `SKILL.md`, so it is a skill, and it groups like every other name.

**First non-empty wins, in canonical root order.** `roots::skill_roots()` returns a fixed `[ResolvedRoot; 3]` in an order already asserted by test (`roots.rs:61`, ~`:267`), and `agent_roots()` a fixed `[ResolvedRoot; 2]`. That fixed order — not arrival order — is the precedence spine. Per field, the first present and non-empty value wins. This is deliberately not "highest-priority root wins wholesale": partial metadata in the first root must not erase complete metadata from a later one.

**Nothing is elected among locations.** Field precedence resolves *display metadata only*. The `locations` vector is a union, emitted in canonical root order, and its length is the duplicate signal the UI will read.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/consolidate.rs` | New | Grouping, location union, field precedence, deterministic ordering |
| `crates/vertice-core/src/lib.rs` | Modified | One `pub mod` line |
| `crates/vertice-core/src/roots.rs` | Unchanged (read) | Canonical root order consumed as the precedence spine; no root id, path, kind or status changes |
| `crates/vertice-core/src/model/` | **Unchanged** | No new field; `locations` is already `Vec<Location>` |
| `frontend/src/bindings/*.ts` | **Unchanged** | No model edit ⇒ no `ts-rs` regeneration; the CI drift gate stays green untouched |
| `crates/vertice-core/src/skills.rs`, `agents.rs`, `opencode_agents.rs`, `installations.rs`, `frontmatter.rs`, `yaml.rs`, `jsonc.rs` | Unchanged | T8 consumes their output; it shares no code path with them |
| `crates/vertice-core/tests/fixtures/roots/reference/` | Unchanged, reused | Already carries the 69→25 shape; the CA-2/CA-3/CA-4/CA-8 oracle |
| `crates/vertice-core/tests/` | New | Consolidation suite: counts, location counts, precedence, determinism |
| `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Unchanged** | No new dependency |
| `vertice-app`, `frontend/` source | Unchanged | No IPC, no command, no capability change |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| A location is silently dropped while merging, so a duplicated skill shows two paths instead of three | **Med — the defining risk** | The headline test asserts **exact location counts** (22 groups with `locations.len() == 3`, 3 with `== 1`), not just the group count. A merge that loses a copy fails before it is written |
| Field precedence implemented as "last write wins" over an iteration, i.e. dependent on scan arrival order | Med | Precedence is a specified, RFC-2119 requirement tied to the fixed `roots.rs` order and covered by a fixture where an earlier root has an **empty** description and a later root has a real one |
| Output ordering differs across Linux / Windows / macOS ⇒ flaky CI | **Med — both prior scanners already hit this class** | Explicit sort, asserted by test; `HashMap` iteration order never leaks into the returned `Vec` or into `locations` |
| Consolidation slips into content comparison ("they're duplicates, let's check if they're equal") | Med | Non-goal stated in the proposal, in the spec, and enforced by the module reading no file at all — it has no bytes to compare |
| An `is_duplicate` flag gets added "for the UI" | Low-Med | Explicitly rejected here; success criteria require `frontend/src/bindings/` byte-identical |
| Two genuinely different components collapse into one because normalization is too aggressive | Low | No new normalization is introduced; `ComponentId::derive` is reused as merged and already unit-tested for case and NFC/NFD |
| A skill and an agent sharing a name get merged | Low | `kind` is part of the identity key by construction; a fixture pins it |
| Empty input, or a single component, mishandled | Low | Explicit tests for empty input and single-element input |

## Open Decisions

**Closed in this proposal:**

- **No model field.** `locations.len() > 1` is the duplicate signal. Zero binding regeneration is a hard constraint of this change, not a nice-to-have.
- **First non-empty value wins, walking canonical root order** — normative and tested, never arrival order.
- **All locations preserved**, in canonical root order.
- **No content comparison.** `issue-creation` divergence deferred to a later increment.
- **New capability spec `duplicate-consolidation`**, not a `domain-model` delta.
- **Determinism is a first-class, asserted requirement.**

**Committed to resolving in `sdd-design` — do not guess:**

- The exact signature and module surface (`consolidate` free function vs. a small type; whether it takes `Vec<Component>` by value or a slice).
- The ordering rule for the **returned component list** itself (by `ComponentId`, by display name, or by first-seen canonical order) — this becomes the list order the T11 UI inherits, so it is a product decision, not a habit.
- Whether "non-empty" for `description` means `None`-or-empty-string or `None`-or-whitespace-only.
- Whether the merged `name` display form and the merged `description` may come from **different** roots (per-field precedence) or must come from the same root. The proposal's default is per-field.
- Whether consolidation is applied to agents as well as skills in the same call (the T8 acceptance criteria are all skill-shaped, but the function is kind-agnostic by construction).

**Deferred, with target:**

- Visual duplicate marking and the three-path display — **T11**.
- Invocation from the scan orchestrator — **T9**.
- Identical vs. divergent duplicate detection — post-PoC increment 1.

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. The 69→25 assertion against `tests/fixtures/roots/reference/` and the exact location-count assertions land as failing tests **before** `consolidate.rs` exists. The empty-description precedence fixture lands before the precedence code.

## Changed-Line Forecast

| Bucket | Est. lines |
|---|---|
| `consolidate.rs` implementation + doc comments | 90–140 |
| Tests (counts, location counts, precedence, determinism, edge cases) | 140–200 |
| Small precedence fixture (empty vs. populated description) | 15–30 |
| `lib.rs` | 1–3 |
| **Total** | **~245–375** |

**Decision needed before apply: No. Chained PRs recommended: No. 400-line budget risk: Low.** The smallest change since T2: one pure module, no dependency, no model edit, no new fixture tree for the headline criteria. A single PR is appropriate. Final slicing is `sdd-tasks`'s call.

## Rollback Plan

Additive at every layer, and free at both the supply-chain and type-contract layers.

- **Core**: delete `consolidate.rs` and its test file; revert one `pub mod` line in `lib.rs`. Nothing else in the crate references it.
- **Model + bindings**: **nothing to revert.** No `model/` edit means no `ts-rs` regeneration; `frontend/src/bindings/` never moves, and the CI bindings-drift gate is untouched throughout.
- **Fixtures**: the `reference/` tree is reused, not modified — reverting removes only the small precedence fixture, if one is added.
- **CI / supply chain**: nothing to revert — `Cargo.toml`, `Cargo.lock` and `deny.toml` untouched; `cargo deny check bans licenses` unaffected; `vertice-core` still imports nothing from `tauri`.
- **App (`vertice-app`)**: zero impact — no command registered, `capabilities/default.json` untouched.
- **Frontend source**: zero impact — no IPC surface, no consumer yet.

Reverting the branch restores the exact pre-T8 state. No persisted data and no IPC contract depend on any of it.

## Dependencies

- **T2** (`Component`, `ComponentId::derive`, `Location`) — complete and archived; verified sufficient with **no change required**.
- **T4** (`skills`, `roots::skill_roots` canonical order), **T5** (`agents`, `agent_roots`), **T6** (`opencode_agents`) — complete and archived; T8 consumes their `Component` output and their root ordering.
- **Independent of T7**; may run in parallel with it (T7 produces installations, not components).
- **Blocks**: T9 (the orchestrator calls `consolidate` before assembling `ScanReport`), T11 (the duplicate mark), and the CA-2 / CA-3 / CA-4 / CA-8 claims.

## Success Criteria

- [ ] Feeding the flattened skill scan of `tests/fixtures/roots/reference/` through `consolidate` yields exactly **25** components from **69** inputs (**CA-2**).
- [ ] Exactly **22** of those components have `locations.len() == 3` (**CA-3**).
- [ ] Exactly **3** have `locations.len() == 1` — `claude-only-01`, `agents-only-01`, `agents-only-02` — and **none** has `locations.len() == 2`, matching the reference distribution (**CA-4**).
- [ ] Every input location survives: the sum of `locations.len()` across the output equals the input count (69). No copy is hidden and no winner is elected (`alcance-poc-vertice.md:55`).
- [ ] `_shared`-shaped names are consolidated like any other name, with **no** name-prefix or name-convention filtering anywhere in the module (**CA-8**).
- [ ] Components differing only by case or by NFC/NFD form collapse into one group, via `ComponentId::derive` and no additional normalization.
- [ ] A skill and an agent sharing the same name are **not** merged.
- [ ] Given a component whose first-root copy has an absent or empty `description` and whose later-root copy has one, the consolidated component carries the later root's description — proving first-**non-empty** precedence, not first-root precedence.
- [ ] Field precedence produces identical output regardless of the input vector's order (arrival order MUST NOT affect the result).
- [ ] The output component order and each `locations` order are deterministic and asserted, stable across Linux, Windows and macOS.
- [ ] Empty input yields empty output; single-component input yields that component with `locations.len() == 1`.
- [ ] The module contains **no** content reading, byte comparison, hashing of file contents, or any `std::fs` / `std::io` use (content comparison non-goal; **CA-16**).
- [ ] `crates/vertice-core/src/model/` and `frontend/src/bindings/` are **byte-identical** to their pre-change state; no `ts-rs` regeneration is part of this change.
- [ ] `Cargo.toml`, `Cargo.lock` and `deny.toml` are byte-identical; `cargo deny check bans licenses` passes and `vertice-core` still imports nothing from `tauri`.
- [ ] All tests read from `crates/vertice-core/tests/fixtures/`; no test reads the author's machine or sets an environment variable (**CA-17**).
- [ ] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` and `npm run test` pass on the three-platform CI matrix; the T3–T7 suites stay green.

## Proposal question round

The interactive question round could not be run from this phase. These are the product questions whose answers would change the proposal, with the assumption currently written into it. Answer, correct, or skip — a second round is available.

| # | Question | Assumption currently written in |
|---|---|---|
| 1 | When two copies of a skill carry **different descriptions**, is showing one of them (first non-empty, canonical root order) acceptable for the PoC, or must the user be told the copies disagree? | Showing one is acceptable; "they disagree" is content comparison, deferred to the divergent-duplicate increment |
| 2 | Should the consolidated list be ordered by display name (what a user scans visually) or by normalized identity (what is stable)? This becomes the T11 list order. | Left open for `sdd-design`, flagged as a product decision rather than an implementation detail |
| 3 | Is the three-path display expected to be ordered by root (`~/.claude`, `~/.agents`, `~/.config/opencode`) so the same skill always reads the same way? | Yes — canonical root order, asserted |
| 4 | Are agents expected to be consolidated too, or is duplication a skills-only concern in the PoC? | The function is kind-agnostic and consolidates both; all four T8 criteria happen to be skill-shaped |
| 5 | Does anything downstream need to know *which root* supplied the winning description, or is the merged component's provenance adequately covered by its location list? | Adequately covered; no per-field provenance is recorded, and adding it would break the no-model-edit property |
