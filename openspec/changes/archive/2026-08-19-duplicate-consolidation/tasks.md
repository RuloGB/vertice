# Tasks: Duplicate Consolidation

> Trace: **T8** (`internal-docs/plan-desarrollo-poc.md:191-207`) / closes **CA-2**, **CA-4**, contributes to **CA-3** (with T11), **CA-8** (with T4); bound by **CA-16** (read-only) and **CA-17** (fixture-based). Design: `openspec/changes/duplicate-consolidation/design.md`. Spec: `openspec/changes/duplicate-consolidation/specs/duplicate-consolidation/spec.md`.
> Core-only — no IPC, no Tauri command, no frontend source change. `crates/vertice-core/src/model/`, `frontend/src/bindings/`, `crates/vertice-core/src/roots.rs`, `Cargo.toml`/`Cargo.lock`/`deny.toml` are **all unchanged by design** (design §2, §4, §10). If any task below appears to require editing one of these, STOP and flag it.
> `strict_tdd: true`. The 69→25 assertion, exact location-count assertions, and the conservation assertion land as failing tests **before** `consolidate.rs` exists. The precedence fixture and its test land before the precedence code.
> Environment note: `cargo` may not resolve on PATH here. Report gate status honestly (pass / fail / not-run), never assumed passing.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~245–375 (proposal forecast) |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending — not needed at Low risk |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | `consolidate.rs` (`ROOT_ORDER`, `LocationKey`, sort-then-fold, precedence, output sort), `lib.rs` wiring, unit tests, `tests/consolidation.rs` integration suite against `reference/`, new precedence fixture | PR 1 (single PR) | Base `main`. Fully self-contained; no dependency on T9/T10/T11 |

## Phase 1: RED — Unit Tests For The (Not-Yet-Existing) Module

- [x] 1.1 Create `crates/vertice-core/src/consolidate.rs` with only a module doc comment and a `#[cfg(test)] mod tests` block — no implementation yet. Wire `pub mod consolidate;` into `crates/vertice-core/src/lib.rs` (one line, no crate-root re-export, matching `lib.rs`'s existing style).
- [x] 1.2 [RED] Unit test: `ROOT_ORDER` (once introduced) equals, in order, the ids of `roots::skill_roots(home)` ++ `roots::agent_roots(home)` ++ `roots::opencode_agent_root(home)` for a synthetic non-existent `home` — design §4's pin test. This fails to compile until `ROOT_ORDER` exists (task 2.1). — *spec: canonical root order (design §4)*
- [x] 1.3 [RED] Unit test: a `Location` with an unknown `SearchRootId` (e.g. `"unknown-root"`) ranks last, deterministically, and never panics; `None` path sorts before `Some` under the same root id (V5/V7). — *spec: "Locations within a component follow canonical root order"*
- [x] 1.4 [RED] Unit tests for field precedence: (a) earlier root `description: None`/empty, later root non-empty → later wins; (b) earlier root `description: Some("\n")` (whitespace-only), later root real → later wins, never `Some("\n")`; (c) all copies blank → first member's value preserved verbatim, including `Some("")` vs `None`. — *spec: "First-Non-Empty Field Precedence"*
- [x] 1.5 [RED] Unit test: the same duplicate-copy set fed in two shuffled input orders produces byte-identical merged field values — the anti-"last write wins" tripwire. — *spec: "Precedence is independent of input arrival order"*
- [x] 1.6 [RED] Unit test: a skill and an agent sharing the same `name` remain two separate components after `consolidate` (kind is part of identity). — *spec: "A skill and an agent sharing a name are not merged"*
- [x] 1.7 [RED] Unit test: case and NFC/NFD name variants collapse into one group with 2 locations, using only `ComponentId::derive` — no normalization code added in this module. — *spec: "Case and NFC/NFD name variants collapse to one component"*
- [x] 1.8 [RED] Unit tests for edges: empty `Vec<Component>` → empty output; single-component input → that component with `locations.len() == 1`. — *spec: "Edge Cases Are Explicit"*
- [x] 1.9 [RED] Unit test: two components sharing a display `name` are ordered by `ComponentId` (`id.as_str()`), not arrival order. — *spec: "Two components sharing a display name are ordered by identity"*
- [x] 1.10 **Checkpoint.** Run `cargo test -p vertice-core --locked` and confirm 1.2–1.9 fail (compile error or assertion failure) — `consolidate` does not exist yet. Record actual output; if `cargo` is unavailable, state that explicitly.

## Phase 2: GREEN — Implement `consolidate`

- [x] 2.1 Add the private `const ROOT_ORDER: [&str; 6]` (design §4) with the six ids in `skill_roots`-then-`agent_roots` order. Pass 1.2.
- [x] 2.2 Implement `LocationKey = (rank: usize, root_id: &str, path: Option<&Path>)` and the rank lookup (unknown id → `ROOT_ORDER.len()`, never a panic). Pass 1.3.
- [x] 2.3 Implement sort-then-fold grouping: sort the owned `Vec<Component>` by `(id.as_str(), member key)`, fold contiguous equal-id runs — no `HashMap`, no `BTreeMap` (design §5). Member key = the member's sorted `Vec<LocationKey>` then its raw `name`.
- [x] 2.4 Implement per-field precedence (`name`, `description`, `provenance_hint`, `scope`) per design §6: first member whose value's `trim().is_empty() == false` (or, for `scope`, first member unconditionally); winning value stored verbatim, never rewritten. Pass 1.4, 1.5.
- [x] 2.5 Implement `locations` union: concatenate every member's locations, sort by `LocationKey`, never deduplicate, never elect a winner.
- [x] 2.6 Implement kind-agnostic behavior (no kind filter — `ComponentKind` is already part of the grouping key). Pass 1.6.
- [x] 2.7 Implement output ordering: `out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.as_str().cmp(b.id.as_str())))`. Pass 1.7, 1.8, 1.9.
- [x] 2.8 Assemble the public surface: `#[must_use] pub fn consolidate(components: Vec<Component>) -> Vec<Component>`. No `Result`, no `ScanIssue`, no panicking path (no `unwrap`/`expect`/indexing by computed index) — design §8.
- [x] 2.9 **Checkpoint.** Run `cargo test -p vertice-core --locked` and confirm all of 1.2–1.9 now pass.

## Phase 3: RED → GREEN — Integration Suite Against The Reference Fixture

- [x] 3.1 [RED] Create `crates/vertice-core/tests/consolidation.rs`. Write the CA-2 assertion first: `skills::scan` over `tests/fixtures/roots/reference/` (69 entries) piped through `consolidate` yields exactly **25** components. Confirm it fails to compile before 2.8 lands (or run before Phase 2 if sequencing this file first is preferred — either order satisfies strict TDD as long as it is observed RED before this task's own GREEN).
- [x] 3.2 [RED→GREEN] Add the CA-3 assertion: exactly **22** components have `locations.len() == 3`, each `locations` in canonical root order. — *spec: "Exact location-count distribution over the reference fixture"*
- [x] 3.3 [RED→GREEN] Add the CA-4 assertion: exactly **3** components have `locations.len() == 1` — `claude-only-01`, `agents-only-01`, `agents-only-02` — and **zero** have `locations.len() == 2`.
- [x] 3.4 [RED→GREEN] Add the conservation assertion: sum of `locations.len()` across the output equals the input length (69). — *spec: "Total location count is conserved"*
- [x] 3.5 [RED→GREEN] Add the CA-8 assertion: the `_shared` fixture skill (`tests/fixtures/roots/underscore-shared/`) consolidates like any other name, with no name-prefix filtering anywhere in `consolidate.rs` (structural review, not just the test).
- [x] 3.6 [RED→GREEN] Add a determinism assertion: two consecutive `consolidate` calls over the same input yield byte-identical output vectors; run on all three CI legs implicitly via the existing matrix.
- [x] 3.7 **Checkpoint.** Run `cargo test -p vertice-core --locked --test consolidation` and confirm 3.1–3.6 all pass.

## Phase 4: Precedence Fixture (RED Before Precedence Code Is Exercised End-to-End)

- [x] 4.1 [RED] Create the new fixture home `crates/vertice-core/tests/fixtures/roots/precedence-description/` with two roots: `.claude/skills/blank-description/SKILL.md` using an empty folded block scalar (`description: >` with nothing under it, per design V8), and `.agents/skills/blank-description/SKILL.md` with a real, non-empty description. Build paths via `env!("CARGO_MANIFEST_DIR")` + per-segment `push`, never `"/"`-joined literals.
- [x] 4.2 [RED] In `tests/consolidation.rs`, add the real-pipeline precedence test: `skills::scan` over `precedence-description/` piped through `consolidate` yields one component whose `description` is the `.agents/skills` copy's value, not `Some("\n")`. This must exercise actual YAML parsing (the unit tests in 1.4 use hand-built `Component` values and cannot substitute for this). Confirm it is RED against a stub scan if run before Phase 2/3 land, or immediately green if run after — either way, note in the PR body that the fixture predates any hand-crafted-value shortcut.
- [x] 4.3 **Checkpoint.** Run `cargo test -p vertice-core --locked --test consolidation` and confirm 4.2 passes.

## Phase 5: Verification (Local, Pre-Commit Gates)

- [x] 5.1 `cargo fmt --all --check`.
- [x] 5.2 `cargo clippy --workspace --all-targets -- -D warnings` — `consolidate.rs` must be clean with no `#[allow]`.
- [x] 5.3 `cargo test --workspace --locked` — Phases 1–4 suites plus all existing T2–T7 suites green, with **zero edits** to any pre-existing test.
- [x] 5.4 `cargo deny check bans licenses` — **not run**: `cargo deny` is not installed on PATH (`error: no such command: 'deny'`), including with the rustup `.cargo/bin` prefix. Reported honestly as not-run, not assumed passing.
- [x] 5.5 **Read-only grep (CA-16)**: confirm no `std::fs`, `std::io`, `std::env`, `SystemTime`, or `Instant` anywhere in `consolidate.rs`. Confirmed — zero matches.
- [x] 5.6 **No-`Ord`-on-`ComponentId` invariant**: confirm `crates/vertice-core/src/model/identity.rs` has zero diff — no `Ord`/`PartialOrd` derive added. `git diff --exit-code -- crates/vertice-core/src/model` must be clean. Confirmed clean.
- [x] 5.7 **Bindings invariant**: `git diff --exit-code -- frontend/src/bindings` must be clean — no `ts-rs` regeneration, no new `is_duplicate` field. Confirmed clean.
- [x] 5.8 **`roots.rs` invariant**: `git diff --exit-code -- crates/vertice-core/src/roots.rs` must be clean — zero lines changed, confirming `ROOT_ORDER` is a pinned local constant, never a call into `roots.rs`. Confirmed clean.
- [x] 5.9 **Dependency invariant**: `Cargo.toml`, `Cargo.lock`, `deny.toml` byte-identical to pre-change state. Confirmed clean.
- [x] 5.10 **Fixture-only invariant (CA-17)**: grep confirms no test in `tests/consolidation.rs` or `consolidate.rs`'s unit tests reads a real-machine path or an environment variable; all fixture paths resolve under `tests/fixtures/roots/`. Only `env!("CARGO_MANIFEST_DIR")` (compile-time macro) is present — no `std::env`.
- [x] 5.11 From `frontend/`: `npm run lint && npm run check && npm run test && npm run build` — regression check only; no consumer of `consolidate` exists yet (T9/T10). All four commands passed.
