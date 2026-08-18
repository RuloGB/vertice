# Proposal: OpenCode Agent Adapter

> Plan trace: **T6** (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md:151-167`.
> Acceptance criteria: **CA-5 (the other half)** — "the 17 Claude Code agents **and the OpenCode agents, the latter read by merging `opencode.json` with `opencode.jsonc`**; an agent defined only in the `.jsonc` appears in the list" (`alcance-poc-vertice.md:164`). Contributes to **CA-12** (a malformed file yields a `ScanIssue` carrying its path and interrupts nothing) and is bound by **CA-16** (read-only) and **CA-17** (fixture-based tests on three platforms). Implements finding **2** (`alcance-poc-vertice.md:112`, verified) and finding **1** (`alcance-poc-vertice.md:110`).

## Intent

T5 closed one half of CA-5: Claude Code agents are inventoried. The other half — OpenCode agents — has no adapter, and the plan's acceptance table (`plan-desarrollo-poc.md:366`) assigns CA-5 to **T5 / T6 jointly**. Until T6 lands, CA-5 cannot be claimed, and T8's consolidation would consolidate an inventory that is knowingly incomplete.

T6 is not "T5 again with a different path". It is the first adapter where **the cardinality inverts**: finding 1 records that an OpenCode agent is an entry inside a JSON object, not a file. One file produces N components. Every adapter so far has walked a directory and parsed one component per file. T6 parses one file and emits many, and it does so from **two files that must be merged**.

The merge is the load-bearing behavior, and it is the reason this change carries a **higher correctness bar than T5**. Finding 2 is empirically verified: `opencode.json` and `opencode.jsonc` are merged sequentially with the last one winning **only on conflicting keys**, not by whole-object replacement. A T5-shaped mistake — skipping a corrupt file — costs one missing agent, visible and self-explanatory. A T6-shaped mistake — implementing the merge as replacement — **silently drops or duplicates real agents that both files legitimately declare**, and the inventory looks plausible while being wrong. The proposal therefore treats the per-key merge as a specified behavior with its own fixture, not as an implementation detail.

## Scope

### In Scope

- A new `vertice-core` module (working name `opencode_agents`) mirroring the shape T4 and T5 settled: `scan(home) -> OpenCodeAgentScan { roots, components, issues }`, infallible, `home: &Path` passed in so no test reads the author's machine.
- Resolution of the OpenCode configuration root and the two config files inside it, `opencode.json` and `opencode.jsonc`, reported with `SearchRootKind::Agent` and the absent / present-and-empty / present-with-entries status discipline `SearchRootStatus` already encodes.
- **JSON parsing** of `opencode.json` and **JSONC parsing** of `opencode.jsonc` — comments and trailing commas — behind a single named seam in the new module, mirroring how `yaml.rs` isolates `serde_norway`.
- **A new runtime dependency** for JSONC (candidate: `jsonc-parser`, named by the plan at `plan-desarrollo-poc.md:157`) and, if the chosen approach requires it, promotion of `serde_json` from `[dev-dependencies]` to `[dependencies]`. See Approach — this is the first new runtime dependency since T2 and is called out deliberately.
- **The per-key merge of the `agent` object**, `opencode.json` then `opencode.jsonc`, last-wins **per key**, with keys present in only one file surviving unchanged.
- Extraction of the `agent` key, one `Component` per merged agent key: `kind: ComponentKind::Agent`, `scope: Scope::User`, `id: ComponentId::derive(Agent, name)`, one `Location` per component.
- Malformed-file isolation: a malformed `opencode.json` yields a `ScanIssue` and **does not prevent** reading and emitting the agents of `opencode.jsonc`, and symmetrically.
- Versioned fixtures under a **new** tree (`crates/vertice-core/tests/fixtures/roots/opencode-agents/` or equivalent), never reusing T4's or T5's homes: agent only in `.json`; agent only in `.jsonc`; the same key in both with **partial-key override**; `.jsonc` with comments and trailing commas; malformed `.json` with a healthy `.jsonc`; the symmetric malformed `.jsonc`; both files absent; the `agent` key missing; the `agent` key present but empty; a reference fixture pinning the CA-5 assertion.
- Fixture-first TDD tests for CA-5 (an agent defined **only** in the `.jsonc` appears alongside the `.json`-only ones), for the per-key merge, and for per-file failure isolation.

### Out of Scope

- **Cross-adapter deduplication with Claude Code agents.** T6 performs only the intra-adapter `.json` + `.jsonc` merge. Two same-named agents from different clients will collide at `ComponentId` level **by design** — exactly the precedent T5's design §9 set for embedded/on-disk shadowing. Resolution is **T8**.
- **A shared scanner abstraction across skills and agents.** Already rejected, on the record, citing T6 by name — see Approach. T6 does not re-litigate it and does not attempt it.
- The `config.json` step of the merge chain. Finding 2 describes `config.json` → `opencode.json` → `opencode.jsonc`; the plan's T6 scope (`plan-desarrollo-poc.md:156`) and CA-5 name only the last two. See Open Decisions.
- **MCP servers**, which live in the same config files and are what finding 2 was empirically verified against. Reading a neighbouring key would be an out-of-scope PoC feature (`openspec/config.yaml`, proposal rule 3). T6 reads `agent` and nothing else.
- Client installation detection and versions — **T7**.
- Consolidation and duplicate marking — **T8**.
- `ScanReport` assembly, `duration_ms`, the "one bad adapter does not abort the scan" orchestration — **T9**.
- IPC exposure, Tauri commands, any frontend surface — **T10**.
- Project scope, `Scope::Project`, `Scope::Local`, and every write operation — outside the PoC.
- macOS/Linux path revalidation and the `opencode debug config` oracle contrast — **T16**, manual, never an automated test (`alcance-poc-vertice.md:132`).

## Capabilities

### New Capabilities

- `opencode-agent-scanner`: OpenCode config root and config-file resolution, JSON/JSONC parsing behind a seam, the per-key `agent` merge across `opencode.json` and `opencode.jsonc`, agent `Component` assembly at `Scope::User`, and per-file malformed-config isolation.

### Modified Capabilities

None expected. `domain-model` is consumed exactly as merged; `agent-scanner` (T5, Claude Code) is not touched. If `roots.rs` needs a helper for a **file-backed** root rather than a directory-backed one, that is an implementation-level addition, not a change to any merged requirement — the three skill roots and the two Claude Code agent roots keep their ids, paths, kinds and statuses.

## Approach

**The merge is per key, not per file — and it gets its own fixture that would fail under whole-object replacement.** Finding 2 (`alcance-poc-vertice.md:112`) is marked VERIFIED and explicitly records that the initial hypothesis (one file wins) was **wrong**. An adapter that picks one of the two files "produces an incomplete agent inventory". The fixture that proves this cannot be "an agent in each file" — that case passes under both a correct merge and a naive concatenation. The discriminating fixture is **the same agent key declared in both files with different sub-fields**: under a correct per-key merge the surviving object carries the `.json` fields that the `.jsonc` did not override; under whole-object replacement they vanish. That fixture is mandatory, and it is the single most load-bearing test in this change.

**No shared skills+agents scanner — inherited as a closed decision, not re-opened.** T5's design already rejected a shared `scan_root(root, walk_policy, parse)` abstraction, and the recorded reason names T6 directly: *"T6's OpenCode agents are entries in a JSON object with no directory and no file per component, so a 'walk a directory, parse a file' abstraction is provably wrong for one third of its known future callers before it ships"* (`openspec/changes/archive/2026-08-18-claude-code-agent-adapter/design.md:195`). T6 is the case that decision was made for. It therefore duplicates the adapter shape — own module, own DTO, own `escalate`, own UTF-8 guard — exactly as T5 duplicated T4's. **Consolidation, if ever, is T9's job**, once all adapters exist and their real common shape is observable rather than guessed. Three near-identical adapters is the intended state at the end of Phase 1, not accidental debt.

**A new runtime dependency, called out rather than slipped in.** T5's design treated "no new dependency" as a load-bearing property: its rollback plan states `Cargo.toml`, `Cargo.lock` and `deny.toml` are untouched, and its success criteria assert no new dependency. **T6 breaks that streak, and it is the first change to do so since T2.** JSONC — comments and trailing commas — is not parseable by a strict JSON reader, and hand-rolling a comment stripper would be the JSON equivalent of the regex frontmatter parser that finding 7 already forbids: it works until a `//` appears inside a string literal. The consequence must be visible: this change pulls `deny.toml` and `cargo deny check bans licenses` into its own review scope, the license of the chosen crate is a gate that can fail CI, and `Cargo.lock` moves. The crate choice, its transitive tree, and its license are a **design-phase decision with the same evidence bar T1 applied to the YAML crate** (`plan-desarrollo-poc.md:48`): actively maintained, verified at decision time, not assumed from the plan's suggestion.

**Core purity is unaffected — and that is worth stating.** `deny.toml` bans `tauri`/`tauri-build` outside `vertice-app`. A JSONC parser is a pure parsing crate with no runtime, no I/O and no platform coupling, so the ban is not approached. The gate that can realistically go red here is **licenses**, not bans. Reviewers should look there.

**Malformed-file isolation is stricter than T5's.** T5 skipped a corrupt file and lost one agent. T6 must lose one *file's worth* of agents and still emit the other file's — the plan states it flatly: "a malformed JSON produces a `ScanIssue` and does not prevent reading the other file" (`plan-desarrollo-poc.md:165`). This forbids the obvious implementation of "read both, merge, then parse the merged text": parsing must happen per file, and the merge must operate on already-parsed values so that one failure removes one input from the merge instead of aborting it.

**Severity escalation follows T5's precedent.** Under a config file whose `agent` key is the declaration of the user's agents, a file that fails to parse is a set of agents **missing from the inventory**, not a warning. `IssueSeverity::Error` with the file path, mirroring `skills::escalate` and `agents::escalate`. An **absent** file is not an issue at all — absence is reported through root status, never as a `ScanIssue`, the rule T4 established for the empty `~/.config/opencode/skill/` (CA-9).

**Ambient environment stays confined to `roots::home_dir()`.** T6 reads no environment variable of its own, and specifically does not consult `XDG_CONFIG_HOME` or any OS config-directory convention. The hard rule at `alcance-poc-vertice.md:106` and `plan-desarrollo-poc.md:179` is that the scanner never derives foreign paths from OS conventions — it applies them only to Vertice's own data directory. Every root is `home` plus a hardcoded relative suffix, which is also what keeps fixture assertions machine-independent (CA-17).

**Detection rule: presence of the key, no name heuristics.** If the merged `agent` object has a key, it is an agent. No filtering by prefix, by underscore, or by "looks like a template" — the same discipline finding 6 forced for `_shared`. Guessing by name convention is how an inventory tool starts lying.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/opencode_agents.rs` (name TBD) | New | Config-file resolution, JSON/JSONC parsing, per-key merge, `Component` assembly |
| `crates/vertice-core/src/lib.rs` | Modified | Declare the new module |
| `crates/vertice-core/src/roots.rs` | **Modified** | New `opencode_agent_root(home)` resolver following `resolve_opencode`'s two-path shape; reuses the existing `probe` helper unchanged |
| `crates/vertice-core/src/agents.rs`, `skills.rs`, `frontmatter.rs`, `yaml.rs` | **Unchanged** | No shared abstraction extracted; frontmatter and YAML are irrelevant to a JSON adapter |
| `crates/vertice-core/src/model/` | **Expected unchanged** | `Agent`, `Scope::User`, `Location`, `ScanIssue` all merged in T2 |
| `frontend/src/bindings/*.ts` | **Expected unchanged** | No model edit ⇒ no regeneration; the CI drift gate should stay green untouched |
| `crates/vertice-core/Cargo.toml` | **Modified** | New JSONC dependency; possible `serde_json` dev → runtime promotion |
| `Cargo.lock` | **Modified** | First lockfile movement from a feature change since T2 |
| `deny.toml` / `cargo deny check bans licenses` | **In review scope** | License of the new crate and its transitive tree is a CI gate |
| `crates/vertice-core/tests/fixtures/roots/opencode-agents/` | New | Ten fixture homes, new tree, no reuse of T4/T5 |
| `crates/vertice-core/tests/` | New | Merge, parsing, isolation and CA-5 suites |
| `vertice-app`, `frontend/` source | Unchanged | No IPC, no command, no capability change |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Merge implemented as whole-object replacement, silently dropping agents | Med — **the defining risk of this change** | A partial-key-override fixture written **before** the merge function; it fails under replacement and passes only under per-key merge |
| Config file path asserted from assumption instead of the oracle | **Med-High — unresolved, see Open Decisions** | Path pinned in `sdd-design` from `opencode debug paths` output, not inferred. **Structural mitigation:** the `opencode-agents` root reports its probed path with `Found`/`NotFound`, so a wrong path presents as a named path not found rather than as a silently empty inventory. The manual oracle check remains, but is no longer the only signal |
| Agent entry shape guessed instead of read from a real config | **Med — unresolved, see Open Decisions** | DTO fields taken from a real `opencode.json`; over-strict typing would make every real agent fail to deserialize, the exact failure mode T5 avoided by verifying `tools` was a scalar |
| New dependency fails `cargo deny check licenses` on CI | Low-Med | License verified at crate-selection time in `sdd-design`, before any code; treated as a gate, not a formality |
| Chosen JSONC crate is unmaintained or heavy | Low-Med | Same evidence bar T1 applied to the YAML crate: maintenance verified at decision time; kept behind a one-module seam so it is swappable, as `yaml.rs` proved |
| Reading the whole config exposes MCP servers or other out-of-scope keys | Low | Only the `agent` key is extracted; a test asserts a config carrying an `mcp` key produces no component from it |
| Same-named agent in Claude Code and OpenCode collides at `ComponentId` | Med | **Intended.** Recorded here so T8 inherits a known case, not a surprise — same posture as T5's shadowing |
| Adapter duplication across T4/T5/T6 read as copy-paste debt in review | Med | Deliberate and cited to a merged design decision; the extraction point is T9 |
| macOS/Linux config location differs from Windows | Med | OpenCode uses XDG on every platform per `opencode debug paths`, which is what makes this *less* platform-fragile than T7; still closed in **T16** |

## Open Decisions

**Closed in this proposal:**

- **No shared skills+agents scanner** — inherited from T5's design §5.4 rejection, which names T6 as its justifying case. Not re-opened.
- **No cross-adapter deduplication in T6.** The `.json`+`.jsonc` merge is intra-adapter only; `ComponentId` collisions with Claude Code agents are emitted and left for T8.
- **Malformed input isolates per file**, not per scan; parsing happens per file so one failure removes one merge input.
- **A new runtime dependency is accepted**, with `deny.toml` / `cargo deny check licenses` explicitly in this change's review scope.
- **The root reports the probed path.** This adapter contributes exactly one `SearchRoot`, id `opencode-agents`, whose `path` is the canonical probed config file and whose `status` is `Found` if **either** config file exists. Both probed paths are carried in `ResolvedRoot.scan_paths`. This is not a new pattern: `roots::resolve_opencode` (crates/vertice-core/src/roots.rs:130) already resolves the OpenCode *skill* root exactly this way — canonical path on the `SearchRoot`, alias alongside it in `scan_paths`, `Found` if either exists. T6 reuses that shape for a file-backed root instead of a directory-backed one. Consequence: a wrong path resolution surfaces as a root reporting a concrete path with `status: NotFound`, which is distinguishable from a correctly-resolved root that found no agents — the empty-versus-wrong ambiguity is closed in the model that already exists.

**Committed to resolving in `sdd-design` — do not guess:**

- **The exact config file path.** `plan-desarrollo-poc.md:156` names `opencode.json` and `opencode.jsonc` **without a parent directory**. `alcance-poc-vertice.md:106` establishes that OpenCode's config directory is `~/.config/opencode` on every platform including Windows, confirmed by `opencode debug paths`. That makes `~/.config/opencode/opencode.json` the strong candidate, but **it is not written down anywhere as fact** and this proposal does not assert it. Resolve from the `opencode debug paths` / `opencode debug config` oracle. Getting this wrong produces an inventory that is empty rather than incorrect — no fixture test can detect it, only the manual oracle can.
- **The shape of an agent entry** inside the `agent` object (`description`? `model`? `prompt`? `tools`? nested objects?). Must come from a **real** `opencode.json`, not from symmetry with Claude Code's frontmatter. T5's `tools`-is-a-scalar finding is the precedent: an assumed type would make every real agent fail to deserialize and vanish, with the blame landing on the merge.
- **Whether `config.json` participates.** Finding 2 describes a three-step chain; T6's scope and CA-5 name two files. Decide and record explicitly rather than dropping it silently.
- **Which JSONC crate**, its license, and its transitive tree — with maintenance verified at decision time.
- **Whether `serde_json` is promoted** to a runtime dependency or the JSONC crate covers both files.
- **Whether an agent name that normalizes to an existing key** (two JSON keys colliding after NFC + lowercase) is reported or silently collapsed.

**Deferred, with target:**

- **Oracle contrast against `opencode debug config`** — **T16**, manual, never automated (`alcance-poc-vertice.md:132`).
- **Cross-client duplicate marking** — **T8**.

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. Fixtures and failing tests land before implementation. Specifically: the partial-key-override fixture must exist and fail before the merge function is written, and the JSONC fixture carrying real comments and trailing commas must exist before the parser seam is chosen.

## Changed-Line Forecast

| Bucket | Est. lines |
|---|---|
| `opencode_agents` module implementation | 150–220 |
| JSON/JSONC parsing seam + doc comment | 30–50 |
| Merge function + doc comment | 30–50 |
| Tests (parsing, merge, isolation, CA-5) | 200–300 |
| Fixtures (~10 homes, small JSON files) | 60–100 |
| `Cargo.toml` / `Cargo.lock` / `deny.toml` | 5–20 (plus lockfile churn) |
| **Total** | **~475–740** |

**Decision needed before apply: Yes. Chained PRs recommended: Yes. 400-line budget risk: Medium-High.** Comparable to T5. Natural slice, matching the T3/T4/T5 precedent: (1) the dependency addition, `cargo deny` green, the parsing seam, fixtures and RED tests; (2) the merge and `Component` assembly turning them GREEN. Slicing the dependency into the first PR is deliberate — it isolates the supply-chain review from the logic review. Final slicing is `sdd-tasks`'s call.

## Rollback Plan

Additive at the model layer, but **not free at the supply-chain layer** — this is the one place T6's rollback is more expensive than T5's.

- **Core**: delete `opencode_agents.rs`, its tests and its fixture tree; revert one `pub mod` line in `lib.rs` and any `roots.rs` helper added.
- **`roots.rs`**: revert the added helper, if any. T4's and T5's existing suites are the regression guard and must stay green untouched.
- **Model + bindings**: nothing to revert. No model type changes expected, so `frontend/src/bindings/*.ts` are untouched and the CI drift gate cannot go red on this change.
- **CI / supply chain** — **the difference from T5**: revert the JSONC dependency in `crates/vertice-core/Cargo.toml`, regenerate `Cargo.lock`, and revert any `deny.toml` allow-list entry added for its license. This is mechanical but must not be forgotten: leaving an unused dependency behind keeps a license gate and an advisory surface for code that no longer exists.
- **App (`vertice-app`)**: zero impact — no command registered, `capabilities/default.json` untouched.
- **Frontend source**: zero impact — no IPC surface, no consumer.

Reverting the branch restores the exact post-T5 state, including the dependency tree. No persisted data and no IPC contract depend on any of it.

## Dependencies

- **T2** (`Component`, `ComponentId::derive`, `Location`, `Scope::User`, `ScanIssue`/`IssueSeverity`, `SearchRoot`/`SearchRootKind::Agent`) — complete and archived; expected sufficient with no change required.
- **T4** (`roots::home_dir`, `ResolvedRoot`, `SearchRootStatus`, escalation and UTF-8-guard patterns) — complete and archived; reused by pattern, not by extraction.
- **T5** (adapter shape, escalation posture, shadowing precedent, the rejected shared abstraction) — complete and archived; T6 inherits its decisions rather than revisiting them.
- **Does not depend on T3** — no frontmatter, no YAML. First adapter in the project that touches neither.
- **Blocks**: T8 (consolidation), T9 (`ScanReport` assembly), and the CA-5 claim. Independent of T7; may run in parallel with it.

## Success Criteria

- [ ] An agent defined **only** in `opencode.jsonc` appears in the result alongside agents defined **only** in `opencode.json` (**CA-5**, the half T5 could not close).
- [ ] A fixture declaring the **same agent key in both files with different sub-fields** yields one component whose non-overridden `.json` fields survive — the assertion that fails under whole-object replacement and passes only under a per-key merge (**finding 2**).
- [ ] A malformed `opencode.json` yields exactly one `ScanIssue` at `IssueSeverity::Error` carrying its path, while every agent declared in `opencode.jsonc` is still emitted; and the symmetric case holds (**CA-12 partial**).
- [ ] A `.jsonc` fixture carrying real `//` comments and a trailing comma parses successfully and yields its agents.
- [ ] Both config files absent, and the config directory absent, each produce zero components and **zero `ScanIssue`s**, remaining distinguishable through root status (the CA-9 discipline generalized).
- [ ] A config file present with **no `agent` key**, and one with an **empty `agent` object**, each produce zero components and zero issues.
- [ ] A config file carrying an `mcp` key (or any non-`agent` key) produces no component from it — out-of-scope PoC features stay out.
- [ ] Every emitted component has `kind: Agent`, `scope: Scope::User`, and `id == ComponentId::derive(Agent, name)`, derived from the key alone and never from the file it came from.
- [ ] The adapter reports one `SearchRoot` with id `opencode-agents` whose `path` is the probed config file path and whose `status` is `Found` when either config file exists; a fixture home with neither file yields that same root with `status: NotFound` and its path still populated, so "looked in the wrong place" is distinguishable from "found nothing to report".
- [ ] Component and issue ordering is deterministic across runs and platforms.
- [ ] The new dependency is declared with its license verified, and `cargo deny check bans licenses` passes; `vertice-core` still imports nothing from `tauri`.
- [ ] The JSON/JSONC crate is imported in exactly one module, the way `serde_norway` is confined to `yaml.rs`.
- [ ] `crates/vertice-core/src/model/` and `frontend/src/bindings/` are byte-identical to their pre-change state; the CI bindings-drift gate is green with no regeneration.
- [ ] No `File::create`, `OpenOptions::write`, or equivalent anywhere in the new module (**CA-16**).
- [ ] No regular expression is used to strip comments or parse either config file (**finding 7's rule, applied to JSONC**).
- [ ] All tests read from `crates/vertice-core/tests/fixtures/`; no test reads the author's machine, sets an environment variable, or reuses T4's or T5's fixture homes (**CA-17**).
- [ ] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` and `cargo deny check bans licenses` pass on the three-platform CI matrix; T4's and T5's existing suites stay green.

## Proposal question round

The interactive question round could not be run from this phase. These are the product questions whose answers would change the proposal, with the assumption currently written into it. Answer, correct, or skip — a second round is available.

| # | Question | Assumption currently written in |
|---|---|---|
| 1 | Should `config.json` participate in the merge chain, or is the two-file merge the intended PoC slice? | Two files only, matching CA-5's wording; `config.json` listed as out of scope and flagged for design |
| 2 | If an OpenCode agent and a Claude Code agent share a name, is showing two entries acceptable until T8, or does it read as a bug worth avoiding now? | Two entries, deliberately, matching T5's embedded/on-disk shadowing precedent |
| 3 | Is a new runtime dependency acceptable here, given T5 treated "no new dependency" as a property worth protecting? | Accepted, with `deny.toml` and the license gate pulled into review scope |
| 4 | ~~If the config file path turns out to be wrong, the inventory is silently **empty**, not wrong. Is a manual oracle check at design time enough, or should this change ship a louder signal?~~ | **Answered: report the probed path in root status.** Closed in Open Decisions. Verification found `resolve_opencode` already establishes this shape for the OpenCode skill root, so it costs one resolver function and no model change |
| 5 | Are OpenCode agent fields (`model`, `prompt`, `tools`) expected to reach the UI, or is name + description the first product slice? | Name + description; extra fields deferred, since `Component` has no field for them and adding one would break the "no model edit, no binding regeneration" property |
