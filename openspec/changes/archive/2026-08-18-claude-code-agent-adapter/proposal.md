# Proposal: Claude Code Agent Adapter

> Plan trace: **T5** (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md:132-147`.
> Acceptance criteria: **CA-5 (partial)** (the 17 on-disk agents of the reference installation appear, asserted over equivalent fixtures) and **CA-13** (embedded components, if shown, are marked and offer no action they do not support). Contributes to **CA-12** (a corrupt file yields a `ScanIssue` carrying its path and interrupts nothing) and is bound by **CA-16** (read-only) and **CA-17** (fixture-based tests on three platforms). Closes open decision **2** of `plan-desarrollo-poc.md:387`.

## Intent

The crate can answer *what skills are installed* (T4) but not *what agents are installed*. `ComponentKind::Agent` exists in the model, is exercised only by identity unit tests (`crates/vertice-core/src/model/identity.rs:74-83`), and no adapter has ever produced one. T5 makes the second half of the plan's "reads the agents" objective (`alcance-poc-vertice.md:25`) real for Claude Code.

T5 is also the first adapter that must emit a component **with no file behind it**. Finding 4 (`alcance-poc-vertice.md:118`, verified) records six embedded Claude Code agents — `Explore`, `Plan`, `general-purpose`, `statusline-setup`, `claude`, `claude-code-guide` — that exist in no directory. `Location.path: Option<PathBuf>` and `LocationOrigin::Embedded` were merged in T2 **for this task** (`crates/vertice-core/src/model/location.rs:9-31`, verified) and no adapter has exercised either. Whether they ever get exercised is the open decision this proposal must close.

Verified this cycle: **the domain model needs no change.** `ComponentKind::Agent`, `SearchRootKind::Agent`, `LocationOrigin::Embedded` and `Location.path: Option<_>` are all already merged. Unlike T4, T5 is purely additive at the model layer — no binding regeneration is expected beyond none at all.

## Scope

### In Scope

- A new `vertice-core` module (working name `agents`) mirroring the shape T4 settled: `scan(home) -> AgentScan { roots, components, issues }`, infallible, `home` passed in so no test reads the author's machine.
- Resolution of the single Claude Code agent root `~/.claude/agents/`, `kind: SearchRootKind::Agent`, with the same absent / present-and-empty / present-with-entries reporting T4 established via `SearchRootStatus`.
- A **flat** walk (`std::fs::read_dir`, non-recursive) over `*.md` directly under that root. See Approach — this is a deliberate divergence from T4.
- `AgentFrontmatter { name, description, model, tools }` supplied by T5 into the **unchanged** generic `frontmatter::read<T: DeserializeOwned>` (`crates/vertice-core/src/frontmatter.rs:72-77`, whose doc comment already names T5 as its intended second caller). No edit to T3 code.
- `Component` assembly for on-disk agents: `kind: Agent`, `scope: User`, one `Location { path: Some(_), root, origin: File }`.
- **The six embedded agents as a hardcoded const list**, emitted as components with `origin: Embedded` and `path: None`. Decision closed below.
- A minimal, mechanical change to `crates/vertice-core/src/roots.rs`: `resolve_single` is currently **private and hardcodes `SearchRootKind::Skill`** (`roots.rs:70-87`, verified). It must become `pub(crate)` and take the kind as a parameter. Nothing else in `roots` changes.
- Versioned fixtures under `crates/vertice-core/tests/fixtures/roots/agents/` as synthetic homes, per the plan: valid agents, broken frontmatter, absent fields — plus an empty root, an absent root, and an absent `<home>/.claude` client directory.
- Fixture-first TDD tests for CA-5-partial (a 17-file fixture root yields exactly 17 on-disk components), CA-13 (embedded components are distinguishable by `origin`), and per-file failure isolation.

### Out of Scope

- **A shared, parametrized scanner across skills and agents.** Deliberately deferred — see Approach.
- OpenCode agents (`agent` key inside `opencode.json`/`.jsonc`, merge semantics, JSONC parsing) — **T6**.
- Client installation detection and versions — **T7**.
- Consolidation and duplicate marking across skills and agents — **T8**.
- `ScanReport` assembly, `duration_ms`, the scan-wide "one bad adapter does not abort" orchestration — **T9**. T5 returns roots, components and issues, exactly as T4 does.
- **Suppressing actions in the UI for embedded components.** T5 makes embedded components *identifiable* (`origin: Embedded`, `path: None`); rendering them as marked and action-less is the second half of CA-13 and lands in **T11/T13** (`plan-desarrollo-poc.md:300`).
- Splitting `tools` into a structured list — see Open Decisions.
- IPC exposure, Tauri commands, any frontend surface — **T10**.
- Project scope, MCP servers, and every write operation — outside the PoC.
- macOS/Linux path revalidation and the `claude agents` oracle contrast — **T16**.

## Capabilities

### New Capabilities

- `agent-scanner`: Claude Code agent root resolution, flat `*.md` discovery, agent frontmatter fields (`model`, `tools`), agent `Component` assembly at `Scope::User`, and the embedded-component contract (`origin: Embedded`, `path: None`).

### Modified Capabilities

None. The `roots.rs` change is implementation-level: no requirement of the merged `skill-scanner` spec changes, the three skill roots keep their ids, paths, kinds and statuses, and `frontmatter-reader` and `domain-model` are consumed exactly as merged.

## Approach

**Embedded agents are hardcoded as a fixed const list — the open decision, closed here.** `plan-desarrollo-poc.md:138` demands this decision be taken and recorded inside T5's cycle. Three reasons decide it: (1) CA-13 is phrased as "embedded components, **if shown**, are marked" (`alcance-poc-vertice.md:172`) — omitting them makes the criterion vacuous rather than satisfied; (2) the oracle contrast in T16 is `claude agents`, which reports 23 active with 6 embedded (`alcance-poc-vertice.md:150`), so an inventory of 17 would be visibly incomplete against the only oracle this adapter has; (3) T2 merged `path: Option` and `LocationOrigin::Embedded` explicitly for finding 4, and omitting them leaves both unexercised by any adapter through the whole PoC, so the first real test of that contract would arrive at UI time. **Accepted cost, recorded rather than hidden**: the list is manual maintenance. If Anthropic adds, removes or renames an embedded agent, Vertice is silently wrong until someone re-runs the T16 manual oracle. The list must therefore live in one named const with a comment stating its provenance and its verification date, never scattered.

**A flat walk, not T4's recursive one.** Claude Code agents are `~/.claude/agents/<name>.md` — one level, no nesting (`plan-desarrollo-poc.md:137`). T4 chose recursion because OpenCode's *own documented glob* is `{skill,skills}/**/SKILL.md`; that evidence does not exist for agents, and reusing recursion here would be inheriting a decision instead of making one. A nested `.md` under `~/.claude/agents/` is not a documented Claude Code agent, and discovering it would invent inventory entries.

**No shared skills+agents scanner abstraction now — a deliberate deferral, not an oversight.** The two walks differ in the one thing a shared abstraction would have to own: recursion policy and filename rule. Worse, T6's agents are *entries inside a JSON object*, not files at all, so a "walk directory, parse file" abstraction would be provably wrong for one third of its future callers before it ships. This follows the project's own precedent of rejecting the `RootScan` wrapper in T4's design (§2.2). The right moment to extract, if ever, is **T9**, when all adapters exist and their real common shape is observable rather than guessed.

**`AgentFrontmatter.tools` is a `String`, not a `Vec<String>` — verified empirically, not assumed.** Against the 17 real files in `~/.claude/agents/` on the reference machine (2026-08-18): all 17 files exist, matching CA-5's expected count; all 17 declare both `model:` and `tools:`; `tools` is a **comma-separated scalar** (`tools: Read, Grep, Glob, Bash`), *not* a YAML sequence; `model` is a plain scalar (`sonnet`). Typing `tools` as `Vec<String>` would make every real agent fail to deserialize and vanish from the inventory — CA-5 would fail with the blame landing on the walker.

**`model` and `tools` are `Option` despite 100% presence in that sample.** Both are documented as optional by Claude Code, and the plan requires a missing-field fixture (`plan-desarrollo-poc.md:139`). This applies T3's already-merged rule verbatim: a field whose absence does not invalidate the component must not remove the component from the inventory (`frontmatter-reader` spec, and T3's proposal Approach). `name` stays required — identity derives from it.

**Frontmatter goes through the T3 seam, no exception.** Also verified on the reference machine: `description` frequently uses a folded block scalar (`description: >`) spanning several lines — the exact construct that made regex parsing forbidden (finding 7, CA-10). T5 adds no parsing of its own.

**Parse failures escalate to `Error`, mirroring T4.** Under `~/.claude/agents/`, a `.md` file is an agent by the same "presence is the detection rule" logic T4 applied to `SKILL.md`; a file that fails to parse is an agent missing from the user's inventory, not a warning. Reuse the shape of `skills::escalate` (`crates/vertice-core/src/skills.rs:146-156`).

**Non-UTF-8 path guarding is carried over as-is** from `skills::ensure_utf8_path` (`skills.rs:158-168`): a discovered path that is not UTF-8 yields an `Error` issue with `path: None` and a lossy rendering, and the walk continues.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/agents.rs` (name TBD) | New | Agent root, flat walk, `AgentFrontmatter`, embedded const list, `Component` assembly |
| `crates/vertice-core/src/lib.rs` | Modified | Declare the new module |
| `crates/vertice-core/src/roots.rs` | Modified (small) | `resolve_single` becomes `pub(crate)` and takes `SearchRootKind` |
| `crates/vertice-core/src/frontmatter.rs` | Unchanged | Second caller; generic signature already fits |
| `crates/vertice-core/src/skills.rs` | Unchanged | No shared abstraction extracted |
| `crates/vertice-core/src/model/` | **Unchanged** | `Agent`, `Embedded`, `path: Option` all already merged in T2 |
| `frontend/src/bindings/*.ts` | **Unchanged** | No model edit ⇒ no regeneration, unlike T4 |
| `crates/vertice-core/tests/fixtures/roots/agents/` | New | Valid, broken, missing-field, empty root, absent root, absent client dir, 17-file set |
| `crates/vertice-core/tests/` | New | Agent walker and assembly suites |
| `Cargo.toml`, `deny.toml`, CI workflows | Unchanged | No new dependency (`std::fs::read_dir`, no `walkdir`) |
| `vertice-app`, `frontend/` source | Unchanged | No IPC, no command, no capability change |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Hardcoded embedded list drifts from the real Claude Code build | Med (over time) | Accepted and recorded as a known limitation; single named const with provenance + verification date; T16 manual `claude agents` contrast is the only detector |
| `tools` typed as a list, breaking all 17 real agents | Low — **already prevented**: verified as a comma-separated scalar | A fixture carrying the literal real-world form (`tools: Read, Grep, Glob, Bash`) is written before the struct |
| Embedded components without a `SearchRootId` have nowhere sensible to attach | Med | `Location.root` shape for an embedded component is an explicit design decision, not an unexamined default |
| Two identically named agents (one embedded, one on disk shadowing it) produce two components with the same `ComponentId` | Med | T5 deliberately emits both, exactly as T4 emitted 69 un-consolidated entries; resolution is **T8**. Recorded so T8 does not inherit a surprise |
| `roots.rs` change ripples into merged `skill-scanner` behavior | Low | Signature-only change; T4's existing suite is the regression guard and must stay green untouched |
| Flat walk proves wrong if Claude Code later nests agents | Low | Documented as evidence-based (`plan-desarrollo-poc.md:137`); revisit at T16 with real-machine validation, not speculatively today |
| Fixture agent set drifts from the real 17 and CA-5 asserts a fiction | Low-Med | Fixture count is pinned to the figure recorded at `alcance-poc-vertice.md:150` and verified on disk this cycle |

## Open Decisions

**Closed in this proposal:**

- **Embedded agents: hardcoded fixed list, not omitted** (`plan-desarrollo-poc.md:387`, open decision 2). Rationale and accepted cost in Approach. This closes the decision the plan assigned to T5's cycle.
- **`tools` is a `String`** — empirically verified, not a design preference.
- **`model` and `tools` are `Option`** — follows T3's merged optional-field rule.
- **No shared skills+agents scanner** — deferred to T9 at the earliest, with the reason recorded.

**Committed to resolving in `sdd-design`:**

- What `Location.root` holds for an embedded component: the Claude Code agent root id, a distinct synthetic id, or a model-level absence. Affects the `SearchRoot` list T9 aggregates.
- Whether `tools` is additionally split into a `Vec<String>` at the `Component` layer, or carried verbatim as the scalar the file declares. **Do not decide silently** — splitting is a product decision about what the UI will show (T11), and carrying it verbatim is the smaller first slice.
- Whether `model`/`tools` reach `Component` at all in the PoC, given `Component` has no field for them today and `provenance_hint: Option<String>` is documented as opaque display text. Adding a field would break this change's "no model edit, no binding regeneration" property.
- Module and function names; whether `AgentScan` is a distinct type or the same shape as `SkillScan`.
- Whether a non-`.md` file or a subdirectory under the agent root is silently skipped or reported.

**Deferred, with target:**

- **Embedded-component action suppression in the UI** — **T11/T13**, the second half of CA-13.
- **Oracle contrast against `claude agents`** — **T16**, manual, text-only (no `--json`).

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. Fixtures and failing tests land before implementation. The 17-file fixture set must exist before any assertion counts it, and the `tools: Read, Grep, Glob, Bash` fixture must exist before `AgentFrontmatter` is written.

## Changed-Line Forecast

| Bucket | Est. lines |
|---|---|
| `agents` module implementation | 130–200 |
| Embedded const list + doc comment | 25–40 |
| Tests (root, walk, assembly, CA cases) | 200–300 |
| `roots.rs` signature change + call sites | 10–20 |
| Semantic fixtures (~8 files) | 40–60 |
| 17-agent fixture set (17 × ~6 lines) | ~100 |
| **Total** | **~500–720** |

**Decision needed before apply: Yes. Chained PRs recommended: Yes. 400-line budget risk: Medium-High.** Smaller than T4 (no 69-file tree, no model edit, no binding churn), but still above budget. Natural slice, matching the T3/T4 precedent: (1) fixtures + RED tests + the `roots.rs` signature change; (2) the `agents` module turning them GREEN. Final slicing is `sdd-tasks`'s call.

## Rollback Plan

Additive at the model layer — materially cheaper to revert than T4.

- **Core**: delete `agents.rs`, its tests, and `tests/fixtures/roots/agents/`; revert one `pub mod` line in `lib.rs` and the added `kind` parameter in `roots.rs`.
- **`roots.rs`**: revert `resolve_single` to private with a hardcoded `SearchRootKind::Skill`. Sole cross-module coupling; T4's suite proves the revert is clean.
- **Model + bindings**: **nothing to revert.** No model type changes, so `frontend/src/bindings/*.ts` are untouched and the CI drift gate cannot go red on this change. This is the load-bearing difference from T4's rollback.
- **App (`vertice-app`)**: zero impact — no command registered, `capabilities/default.json` untouched.
- **Frontend source**: zero impact — no IPC surface, no consumer.
- **CI / supply chain**: no dependency added, so `Cargo.toml`, `Cargo.lock` and `deny.toml` are untouched.

Reverting the branch restores the exact post-T4 state. No persisted data and no IPC contract depend on any of it.

## Dependencies

- **T2** (`Component`, `Location`, `LocationOrigin::Embedded`, `ComponentKind::Agent`, `SearchRootKind::Agent`) — complete and archived; verified sufficient with no change required.
- **T3** (`frontmatter::read<T>`) — complete and archived; T5 is the generic signature's second caller and its stated justification.
- **T4** (`roots::home_dir`, `ResolvedRoot`, `SearchRootStatus`, escalation and UTF-8-guard patterns) — complete and archived.
- **Blocks**: T8 (consolidation, which must handle embedded/on-disk name collisions), T9 (`ScanReport` assembly). Independent of T6/T7; may run in parallel with them.

## Success Criteria

- [x] A fixture root containing 17 agent `.md` files yields exactly **17** on-disk agent components, `kind: Agent`, `scope: User`, each with one `Location { path: Some(_), origin: File }` (**CA-5 partial**).
- [x] The **six** embedded agents appear as components with `origin: Embedded` and `path: None`, distinguishable from on-disk agents by that pair alone, with no filename or name-convention heuristic (**CA-13**, core half).
- [x] The embedded list is a single named const carrying its provenance and verification date; the manual-maintenance cost is documented in `design.md` as a known limitation, not implied.
- [x] A fixture whose frontmatter carries `tools: Read, Grep, Glob, Bash` deserializes successfully into a scalar `tools` field.
- [x] A fixture with `name` and `description` but no `model` and no `tools` yields `Ok` with both as `None`, not a `ScanIssue`.
- [x] A fixture whose `description` is a folded block scalar (`description: >`) returns the complete description, parsed through `yaml::from_str` (**CA-10** inherited).
- [x] A broken-frontmatter agent file yields one `ScanIssue` at `IssueSeverity::Error` carrying its path, while every sibling agent in the same directory is still discovered (**CA-12 partial**).
- [x] An absent `~/.claude/agents/` and a present-but-empty one each produce no `ScanIssue` and no **file-backed** component, and remain distinguishable in the reported root status. Asserted by filtering on `origin == File`, never by `components.is_empty()` — when `<home>/.claude` exists the six embedded components are still emitted, so an empty-set assertion would pass today and forbid CA-13 tomorrow.
- [x] A home with no `<home>/.claude` at all produces zero components and zero issues: the six embedded agents are gated on the client directory being present, so an uninstalled client is never reported as inventory.
- [x] A `SKILL.md`-shaped nested tree under the agent root produces no extra component — the walk is flat by construction.
- [x] The new module contains no `serde_norway` import, no regular expression, and no `walkdir` dependency.
- [x] `crates/vertice-core/src/model/` and `frontend/src/bindings/` are byte-identical to their pre-change state; the CI bindings-drift gate is green with no regeneration.
- [x] No `File::create`, `OpenOptions::write`, or equivalent anywhere in the new module (**CA-16**).
- [x] All tests read from `crates/vertice-core/tests/fixtures/`; no test reads the author's machine, sets an environment variable, or is aimed at `fixtures/frontmatter/` or `fixtures/roots/` (**CA-17**).
- [x] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and `cargo deny check bans licenses` pass on the three-platform CI matrix; T4's existing skill-scanner suite stays green.
