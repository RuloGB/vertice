# Design: OpenCode Agent Adapter

> Trace: **T6** (`internal-docs/plan-desarrollo-poc.md:151-167`) / closes the second half of **CA-5**; contributes to **CA-12**; bound by **CA-16** and **CA-17**. Implements findings **1** and **2** (`alcance-poc-vertice.md:110,112`).
> Proposal: `openspec/changes/opencode-agent-adapter/proposal.md`. Inherits T4's design (`openspec/changes/archive/2026-08-18-skill-scanner-user-roots/design.md`, **T4D**) and T5's (`openspec/changes/archive/2026-08-18-claude-code-agent-adapter/design.md`, **T5D**). T5D §5.4 (no shared scanner abstraction) and §9 (duplicate identity is emitted, not resolved) are **inherited as closed decisions** and are not re-litigated here; §5.4 is *strengthened* with new evidence in §5.5, which is a different act from reopening it.
> `rules.design` coverage: core data model impact (§2), core/Tauri isolation for the CLI pathway (§1), per-OS paths (§11), `ScanIssue` taxonomy and error paths (§8), IPC contract surface (§2 — **none**, and that is load-bearing).
> **Environment note.** `cargo` is not on PATH in the authoring environment and this phase had no shell. **No claim below was verified by compiling, and no crate was verified against crates.io from here.** Everything marked *verified* in §0 came from a real inspection of the reference machine performed for this phase; everything about the JSONC crate is marked **VERIFY-BEFORE-APPLY** in §5.2 and carries a named fallback. That distinction is the whole point of the section — do not collapse it.

## 0. What was verified this cycle, and what it forces

Both of the proposal's open questions that blocked design are **closed by observation**, not by inference. The observations are recorded here because several decisions below are downstream of them and would otherwise look arbitrary.

| # | Observation on the reference machine | Consequence in this design |
|---|---|---|
| V1 | `~/.config/opencode/opencode.json` (76 KB) and `~/.config/opencode/opencode.jsonc` (421 B) both exist; the XDG layout holds on Windows exactly as `alcance-poc-vertice.md:106` predicted | §3, §11. Path is **resolved**, not assumed. macOS/Linux remain T16 |
| V2 | 18 real entries under `agent`. `description` string 18/18, `mode` string 18/18 (`subagent` ×17, `primary` ×1), `prompt` string 18/18, `tools` **object** 18/18, `hidden` bool 17/18, `permission` object 1/18 | §5.4 (no typed DTO), §5.5 |
| V3 | `tools` is an **object** here; T5 verified `tools` is a **scalar string** in Claude Code frontmatter | §5.5 — a shared DTO would be actively wrong, as a fact rather than as an argument |
| V4 | The real `opencode.jsonc` has **no `agent` key**, no comments and no trailing commas. It is an MCP-only overlay | §10 — stated loudly: the T16 oracle **cannot** exercise the `.jsonc`-agent path or the comment/trailing-comma path. Fixtures are their only defense |
| V5 | `opencode.json` top-level keys: `$schema`, `agent`, `mcp`, `permission`, `share` | §6.1 — extracting only `agent` is grounded in real data; the "an `mcp` key produces no component" test has a real basis, not a hypothetical one |
| V6 | All 18 real agent names are lowercase and NFC-stable — no normalization collision occurs in practice | §9 — the behavior is unreachable from real data but reachable from a crafted config, so it is **designed and fixture-tested**, never left undefined |

**V4 is the uncomfortable one and it is stated first on purpose.** A green `opencode debug config` contrast at T16 will say nothing whatsoever about JSONC comment handling or about agents declared in the overlay — the two behaviors CA-5 names by wording. A reviewer or a future maintainer who reads "T16 verified against the real client" as coverage of those paths will be wrong. The fixture suite in §10 is the **sole** defense for them, which is why §10 marks three fixtures as non-negotiable rather than illustrative.

**Product decision already taken, restated so no implementer re-derives it**: `hidden: true` **MUST NOT** exclude an agent. 17 of 18 real entries carry it; filtering on it would reduce OpenCode's contribution to CA-5 to a single agent. `hidden` is a picker-visibility hint inside OpenCode, not a statement about installation, and Vertice inventories what is installed. The field is not read at all (§5.4).

## 1. Technical approach

Two new sibling modules, one new resolver in `roots.rs`, one new dependency, and **nothing else**.

```
                                     vertice-core
 frontend ──IPC──> vertice-app ──>   ├── model/        (pure data, zero I/O)  ← UNCHANGED, §2
                                     ├── roots         (+ opencode_agent_root; probe reused as-is)
 future vertice-cli ────────────>    ├── skills        (T4, untouched)
                                     ├── agents        (T5, untouched — no shared abstraction, §5.5)
                                     ├── frontmatter   (T3, untouched — irrelevant to JSON)
                                     ├── yaml          (serde_norway seam, untouched)
                                     └── jsonc         (JSONC seam — NEW, §5.2)

 roots::opencode_agent_root(home) ─> ResolvedRoot { root.path = <base>, scan_paths = [<base>, <overlay>] }
                                             │
   opencode_agents::scan(home) ──────────────┤
                                             ├─ read+parse <base>    ─┐
                                             │      (isolated failure)│  §6 deep merge
                                             ├─ read+parse <overlay> ─┘  of the `agent` object only
                                             │
                                             ├──> Vec<Component>   (sorted by raw key, §7)
                                             └──> Vec<ScanIssue>   (§8)
```

**The CLI pathway is preserved unchanged.** `opencode_agents::scan` takes `home: &Path` and returns owned data. It performs **no ambient-environment read at all**: `roots::home_dir()` remains the single such call in the crate, one layer up, and every test bypasses it by passing a fixture path. In particular this adapter does **not** consult `XDG_CONFIG_HOME`, `%APPDATA%`, `dirs::config_dir()` or any OS convention — the hard rule at `alcance-poc-vertice.md:106` and `plan-desarrollo-poc.md:179`. Both scan paths are `home` plus hardcoded relative segments pushed one at a time.

**Core purity survives trivially.** The new dependency is a pure parsing crate: no runtime, no I/O, no platform coupling, nothing that could pull `tauri` transitively. `cargo deny check bans` is not approached. The gate that can realistically go red is **licenses** (§5.2).

**`model/` purity survives trivially**, because `model/` is not edited (§2). All disk access lives in `opencode_agents.rs`; `jsonc.rs` is a pure `&str -> Result<JsonValue, _>` function with no I/O of its own, exactly as `yaml.rs` is.

## 2. Core data model impact: none — and the IPC surface is empty

**None.** `Component`, `ComponentId`, `ComponentKind::Agent`, `Location`, `LocationOrigin::File`, `Scope::User`, `ScanIssue`, `IssueSeverity`, `SearchRoot`, `SearchRootKind::Agent` and `SearchRootStatus` are consumed exactly as merged in T2. No field is added, no variant is added, no doc comment in `model/` is edited.

Mechanical consequences, all checkable by a reviewer without running anything:

- **No `TS`-derived type is introduced.** `JsonValue` (§5.2), `JsoncError` and `OpenCodeAgentScan` derive neither `Serialize` nor `TS`.
- **`frontend/src/bindings/*.ts` is byte-identical** after this change. **No binding regeneration is performed or required**, and CI's `git diff --exit-code -- frontend/src/bindings` step stays green untouched. Like T5, this change *cannot* go red on binding drift.
- **No IPC contract surface.** No Tauri command is registered, `crates/vertice-app/capabilities/default.json` is untouched, and no frontend source file changes. `rules.design`'s "detail IPC contract surface (commands, events)" is satisfied by **the empty set**, with the mechanical proof above rather than by assertion. IPC exposure is T10.
- **CA-16, structurally.** The complete disk surface of this change is `std::fs::symlink_metadata` (via the existing private `roots::probe`) and `std::fs::read_to_string`. There is **no `File::create`, no `OpenOptions`, no `fs::write`, no `create_dir*`, no `remove_*`** anywhere in the new modules — nor in the tests, which read committed fixtures and never materialize a temp tree. `rules.apply`'s grep finds nothing.

**What would break these properties**, stated so a reviewer rejects them on sight:

| Temptation | Why it breaks the property | Verdict |
|---|---|---|
| Promote `mode`, `prompt`, `tools` or `permission` onto `Component` | `Component` derives `TS`; one new field regenerates `Component.ts` and puts this change on the bindings drift gate | **Rejected.** Product slice is name + description (proposal Q5). These fields are not even parsed (§5.4) |
| A new `LocationOrigin` variant for "declared inside a config file" | regenerates `LocationOrigin.ts`; and `File` already means exactly "backed by a file on disk", which a config-declared agent is | **Rejected** |
| A third `SearchRootStatus` for "config file present but unparseable" | `location.rs:53-57` fixes the two-valued design: unreadable-but-present is `Found` **plus** a `ScanIssue`, never a third status | **Rejected** — the model already says this |
| Derive `TS` on `OpenCodeAgentScan` "for later" | manufactures a binding for a type T9 destructures and never sends | **Rejected** |

## 3. Decision: one root, and which file is canonical

The proposal closed the root *shape*. It left open **which of the two files `SearchRoot.path` carries**, and that is a real question because `SearchRoot.path` is singular while this adapter reads two files.

> **Decision: exactly one `SearchRoot`, id `opencode-agents`, `kind: SearchRootKind::Agent`, `path = <home>/.config/opencode/opencode.json`, `scan_paths = [<home>/.config/opencode/opencode.json, <home>/.config/opencode/opencode.jsonc]` in that order, `status: Found` iff *either* file exists.**

This is not a new pattern. `roots::resolve_opencode` (`roots.rs:132-157`) already resolves the OpenCode *skill* root this exact way: canonical path on the `SearchRoot`, alias alongside it in `scan_paths`, `Found` if either exists. T6 reuses that shape for a **file**-backed root instead of a directory-backed one, which costs one resolver function and no model change.

**Why `opencode.json` is canonical**, since the T4 criterion does not transfer. For skills the tie-break was "the documented plural form". There is no documented-vs-alias relationship here — both files are legitimate. Three arguments, in decreasing weight:

1. **It is the base of the merge chain.** Finding 2 orders the chain `… → opencode.json → opencode.jsonc`, last-wins. The base is the file that carries the configuration; the overlay is, by construction, partial. Naming the base as canonical means `SearchRoot.path` points at the file a user would open to see their agents. On the reference machine this is literally true: the base is 76 KB with 18 agents, the overlay is 421 B with none (V1, V4).
2. **`.json` is the primary form.** It is the extension named first by both `plan-desarrollo-poc.md:156` and CA-5, and it is the strict form of which `.jsonc` is the permissive superset.
3. **Consistency with `resolve_opencode`.** That resolver puts the *more canonical / more expected* member on the `SearchRoot` and the other in `scan_paths` only. Choosing the overlay here would invert the precedent for no gain.

**Accepted cost, recorded.** On a machine where only `opencode.jsonc` exists, the root reports `path: …/opencode.json` with `status: Found` — a path that does not exist, marked as found. This is *exactly* the wart `resolve_opencode` already carries for `skills/` vs `skill/` (T4D §4), it is why `scan_paths` exists, and a UI that wants to show what was actually read must read `Location.path` per component (§6.4), which is always the real declaring file. Flagged for T9/T11 in §13, not fixed here, because fixing it means widening `SearchRoot` and forfeiting §2.

**Why the status rule matters more than it looks.** With `status` probed on the files (not on the directory), the report distinguishes three states that would otherwise blur into "empty":

| Machine state | root `status` | components | issues |
|---|---|---|---|
| Wrong path resolved / OpenCode not installed | `NotFound`, **path populated** | 0 | 0 |
| Correct path, config exists, no `agent` key | `Found` | 0 | 0 |
| Correct path, config exists, unparseable | `Found` | 0 (from that file) | 1 `Error` carrying the path |

Row 1 is the failure mode the proposal was most afraid of — a silently empty inventory from a wrong path. It now presents as *a named path that was not found*, which is a legible signal to a user and a fixture-testable one. `status` is deliberately probed on the two **files**, not on the `~/.config/opencode/` directory: a directory that exists with no config in it is not an OpenCode installation worth claiming, and probing the directory would report `Found` for it.

## 4. Decision: the merge chain is exactly two files — `config.json` does not participate

> **Decision: NO. The chain is `opencode.json` → `opencode.jsonc`, two files. `~/.config/opencode/config.json` is not read.** The proposal's default is confirmed, with reasons, not by omission.

| Argument | Weight |
|---|---|
| CA-5 (`alcance-poc-vertice.md:164`) and the T6 scope (`plan-desarrollo-poc.md:156`) both name exactly two files. Reading a third would be scope the acceptance criteria cannot check | Decisive |
| `config.json` is the **legacy** name in finding 2's chain. Its presence is unverified on the reference machine, so adding it would ship an unexercised code path — the same class of mistake §0/V4 warns about | Strong |
| Adding it later is **purely additive and contract-free**: one more path prepended to `scan_paths`, one more parse, one more merge input. No model change, no root-id change, no `Component` change, no spec rewrite. The merge (§6) is already an ordered fold over N inputs, not a hardcoded pair | Strong — this is what makes deferring safe rather than lazy |

**Accepted cost, stated plainly.** A user whose agents live only in a legacy `config.json` gets zero OpenCode agents and **no issue** — the root reports `Found` (if either newer file exists) with nothing behind it. That is invisible to every fixture, exactly like a wrong path would be. The **only** detector is T16's `opencode debug config` contrast, which prints the fully-merged config and would show agents Vertice does not report. §13 records it as a live open question with T16 as the target, not as a closed one.

**Implementation instruction that makes the deferral cheap**: the merge function MUST take an ordered slice of parsed inputs (`&[JsonValue]` or an iterator), never two named parameters. An implementer who writes `fn merge(base: JsonValue, overlay: JsonValue)` has hardcoded the arity and made §4's escape hatch a refactor instead of a one-line change.

## 5. Module and function surface

### 5.1 `roots.rs` — one new public resolver, nothing else touched

```rust
// crates/vertice-core/src/roots.rs

/// Resolve the OpenCode agent config root under `home`. `SearchRoot.path`
/// carries `~/.config/opencode/opencode.json` (the merge base, design §3);
/// `scan_paths` carries the base then the `.jsonc` overlay, in merge order.
/// `status` is `Found` if EITHER file exists. Root id is hardcoded, never
/// path-derived.
pub fn opencode_agent_root(home: &Path) -> ResolvedRoot;
```

Structurally a sibling of `resolve_opencode` (`roots.rs:132`): same two-`push` path construction, same `match (probe(a), probe(b))` status fold, same `scan_paths` vector. `probe` is **reused unchanged and stays private**; `resolve_single` is **not touched** (it takes a two-segment suffix and would not fit a three-segment path, and bending it would put T4's and T5's roots on this change's regression surface for no benefit).

**`scan_paths` order is load-bearing** and is the single source of merge order (§6) and of `Location` order (§6.4). It is not incidental iteration order that an implementer may reorder. `opencode_agent_root`'s doc comment says so; a unit test asserts index 0 ends with `opencode.json` and index 1 with `opencode.jsonc`.

Root id: `SearchRootId("opencode-agents")`, hardcoded, never derived from `home` (T4D §4's rule, inherited verbatim). Note it is distinct from T4's `opencode-skills` — same client, different `kind`, different root, per `location.rs:66-68`.

T4's and T5's existing `roots.rs` unit suite (`roots.rs:172-313`) is the regression guard and must stay green **with no edits at all**. This is a stricter guarantee than T5 could offer, since T5 had to add a parameter to `resolve_single`.

### 5.2 `jsonc.rs` — the dependency seam (Q2, Q3)

Placed as a **top-level module beside `yaml.rs`**, not as a private submodule of the adapter.

| Option | Consequence | Decision |
|---|---|---|
| Private `mod jsonc` inside `opencode_agents.rs` | Smaller surface, one fewer file. But a private submodule is **not a seam**: the parent module can `use jsonc_parser::…` directly and nothing detects it. The whole value of `yaml.rs` is that the invariant is *checkable* | **Rejected** |
| **Top-level `src/jsonc.rs`, mirroring `yaml.rs` verbatim** | One `pub mod` line. The "exactly one module imports the parsing crate" invariant becomes greppable and testable the same way `tests/yaml_behavior.rs` makes the YAML one testable. `yaml.rs` had exactly one consumer at T3 too — one-consumer is not an argument against a seam | **Chosen** |

**The seam owns its own value type.** This is the decision that makes it a real seam rather than a re-export:

```rust
// crates/vertice-core/src/jsonc.rs
//
// This is the ONLY module in `vertice-core` allowed to import the JSONC
// parsing crate. Every other module MUST go through `parse`. Swapping the
// crate later means changing this file and `Cargo.toml` only.

use std::collections::BTreeMap;

/// A parsed JSON/JSONC value, owned by the seam. The parsing crate's own
/// value type NEVER escapes this module — that is what makes the crate
/// swappable (design §5.2).
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    /// Source text, verbatim. The seam makes no numeric decision for data
    /// this crate does not consume (design §5.2).
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    /// `BTreeMap`, not `HashMap` and not the crate's map type: key order is
    /// sorted-by-construction, so determinism (§7) is a property of the type
    /// and not a convention an implementer can forget.
    Object(BTreeMap<String, JsonValue>),
}

#[derive(Debug, thiserror::Error)]
pub enum JsoncError {
    #[error("failed to parse JSON: {0}")]
    Parse(String),
}

/// Parse JSON or JSONC (comments and trailing commas) into a `JsonValue`.
/// Unquoted property names are NOT accepted — that is JSON5, not JSONC.
pub fn parse(input: &str) -> Result<JsonValue, JsoncError>;
```

Four sub-decisions, each with its reason:

- **The seam does not expose `from_str<T: DeserializeOwned>`.** `yaml.rs` could, because `serde` is already a crate-wide dependency and `DeserializeOwned` is not the YAML crate's type. There is no equivalent here: exposing `jsonc_parser::JsonValue` would leak the crate into every consumer and the seam would be decorative. Owning ~40 lines of enum is the price of the swap property. **Accepted cost**, recorded.
- **`Object(BTreeMap<String, JsonValue>)`.** See §7. It also fixes duplicate-key behavior inside a single file — a later duplicate key overwrites an earlier one, matching what mainstream JSON parsers do — instead of leaving it to the crate.
- **`Number(String)`.** This adapter reads strings and objects only. Parsing `1e400` into an `f64` would be a lossy decision made on behalf of data nobody consumes. Verbatim source text is both lossless and comparison-stable.
- **`JsoncError::Parse(String)`, not `#[from] <crate>::Error`.** `yaml.rs` uses `#[from] serde_norway::Error` and therefore leaks the crate's error type through its public API — a small hole in that seam. Not repeated here. The crate's error is formatted to `String` at the boundary. Reasons are developer diagnostics (T3D §6) and are never parsed or branched on, so nothing downstream loses anything.

#### Crate choice — **VERIFY-BEFORE-APPLY**

> **Recommended: `jsonc-parser`, used for BOTH files, with `serde_json` staying a dev-dependency.** This is the plan's own suggestion (`plan-desarrollo-poc.md:157`), and the two structural arguments below are independent of which crate wins. The crate's license, transitive tree and maintenance status **were not verified from this phase** (no shell) and are a **blocking first task in `sdd-tasks`**, held to the same evidence bar T1 applied to the YAML crate.

**Q3 — one parser for both files, resolved on behavior, not on dependency count.** JSON is a strict subset of JSONC, so a JSONC parser reads `opencode.json` correctly. Using `serde_json` for the base and a JSONC crate for the overlay would give the two files **two different error taxonomies and two different leniency levels**, which breaks the symmetry the whole change rests on: §8's isolation requirement is "malformed base + healthy overlay" and "malformed overlay + healthy base" behaving identically, and §10 tests them as a mirrored pair. With two parsers, that mirror is a fiction — the two arms would exercise different code. One parser also means `serde_json` stays in `[dev-dependencies]` (it is used by existing tests), so this change adds **exactly one** dependency, not two.

**Counter-argument, considered and accepted.** Parsing `opencode.json` leniently means Vertice accepts a `.json` file containing comments that a strict reader would reject — Vertice would be *more* permissive than the file extension implies. Accepted, because OpenCode ships one config loader for both extensions and it is JSONC-capable; matching the client's leniency is more correct for an inventory tool than matching the extension's pedantry. The failure mode of the alternative is worse: refusing to report agents the client itself loads.

**Parser options, to be set explicitly rather than left at defaults:** allow comments **yes**, allow trailing commas **yes**, allow unquoted/loose property names **no**. The third is JSON5, not JSONC; accepting it would make Vertice read agents from a file OpenCode rejects, which is the mirror of the failure above and equally wrong. *(The exact option identifiers must be confirmed against the crate's API at apply time; the intent above is normative, the spelling is not.)*

**What must be verified before the dependency is added, in order:**

1. License is on `deny.toml`'s allow-list (`MIT`, `Apache-2.0`, `BSD-*`, `ISC`, `Unicode-3.0`, `Zlib`, `CC0-1.0`, `MPL-2.0`, `Apache-2.0 WITH LLVM-exception`) — **for the crate and its entire transitive tree**, at the four targets in `deny.toml`'s `[graph].targets`. `cargo deny check licenses` is the gate and it is deterministic, so it either passes locally or it fails CI; there is no in-between.
2. Transitive tree is **empty or near-empty** with the default feature set (the `serde` feature, which pulls `serde_json`, is **not** enabled — the seam does not need it).
3. Maintenance is live at decision time — recent release, open issue triage. Verified *then*, not assumed from the plan's suggestion.
4. It compiles at the MSRV floor, **`rust-version = "1.88"`** (`Cargo.toml:8`), which is a fourth CI job (`msrv`) this change is newly exposed to. A crate whose own MSRV is above 1.88 fails that job even if every other gate is green.

**Fallbacks, ordered, so a failed gate is a substitution and not a redesign:**

| If | Then | Note |
|---|---|---|
| `jsonc-parser` fails gate 1, 3 or 4 | A serde_json-derived JSONC reader (e.g. `serde_jsonrc`) behind the identical `jsonc::parse` signature | Only `jsonc.rs` changes. Nothing else in the crate knows |
| Both fail | `jsonc-parser` **with** the `serde` feature + `serde_json` promoted to `[dependencies]` | The proposal's alternative. Costs one extra runtime dependency; keeps one parser for both files |
| All fail | **Stop and escalate.** Do **not** hand-roll a comment stripper | A regex or hand-rolled stripper breaks on `//` inside a string literal — finding 7's rule, applied to JSONC. Explicitly forbidden by a success criterion |

Because the seam owns its value type, every row above touches exactly one file. That is the property being bought.

### 5.3 `opencode_agents.rs` — the adapter

```rust
// crates/vertice-core/src/opencode_agents.rs

/// Owned result of one OpenCode agent scan. A distinct type from `SkillScan`
/// and `AgentScan` — not an alias, not a shared generic (design §5.5).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenCodeAgentScan {
    pub roots: Vec<SearchRoot>,      // always exactly 1 (§3)
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

/// Scan the OpenCode agent config under `home`. Infallible, mirroring
/// `skills::scan` and `agents::scan`. Read-only: `roots::probe`'s
/// `symlink_metadata` and `std::fs::read_to_string` are the COMPLETE disk
/// surface — no write of any kind, anywhere (CA-16).
pub fn scan(home: &Path) -> OpenCodeAgentScan;
```

`lib.rs` gains two lines, `pub mod jsonc;` and `pub mod opencode_agents;`, with no crate-root re-export — matching `lib.rs:7-12`.

Control flow, in order, so §4's arity escape hatch and §8's isolation are both structural:

1. `let resolved = roots::opencode_agent_root(home);`
2. For each path in `resolved.scan_paths`, **in order**: read → parse → extract the `agent` object. Each step can fail independently and produces `None` for that file plus at most one `ScanIssue` (§8). **A failure never returns early and never aborts the loop.**
3. Fold the surviving `agent` objects, in the same order, into one merged map (§6).
4. Emit one `Component` per merged key, in sorted key order (§7).

Step 2 producing `Option<(usize, BTreeMap<String, JsonValue>)>` per file — index retained — is what makes per-file provenance available in step 4 without a second pass (§6.4).

### 5.4 Decision: no DTO, no `serde` deserialization of an agent entry

This is a deliberate divergence from T5's `AgentFrontmatter` and it is the direct lesson of V2/V3.

> **Decision: agent entries are read at the value level. No `#[derive(Deserialize)]` struct describes an OpenCode agent. Exactly one field is extracted — `description`, and only if it is a `JsonValue::String`.**

| Option | Consequence | Decision |
|---|---|---|
| `#[derive(Deserialize)] struct OpenCodeAgent { description: Option<String>, mode: Option<String>, tools: Option<…>, … }` | Every field is a shape bet. `tools` is an object today (V2) — type it as `String` by symmetry with T5 and **all 18 real agents fail to deserialize and vanish**, with the blame landing on the merge. That is precisely the failure T5 avoided by verifying `tools` was a scalar, and V3 proves the two clients disagree | **Rejected** |
| `#[derive(Deserialize)] struct OpenCodeAgent { description: Option<String> }` with `#[serde(default)]` and unknown fields ignored | Safer, but still requires wiring `serde` deserialization through a seam that deliberately does not expose `DeserializeOwned` (§5.2), and still fails the whole entry if `description` is present with a non-string type | **Rejected** |
| **Value-level extraction: `entry.get("description")` matched against `JsonValue::String`; everything else ignored** | An unexpected type **anywhere** in the entry cannot make the agent disappear. The agent's existence depends only on the **key**, which is exactly the detection rule (§6.1). No `serde` involvement at all | **Chosen** |

**Why this is not laziness.** The identity of an OpenCode agent is the key, not the body. `ComponentId::derive(Agent, name)` consumes the key alone. Making the component's existence depend on successfully typing a body we do not display would couple inventory completeness to schema guesses about a client that is free to add fields — and V2 shows it already has six of them, one of which (`permission`) appears on 1/18 entries. **Presence of the key is the detection rule; the body is metadata, and unreadable metadata degrades a field, never a component** (§8).

`mode`, `prompt`, `tools`, `hidden` and `permission` are **not read**. Adding one later is one match arm plus one `Component` field plus one binding regeneration — a T11 decision, and the reason §2's "no binding regeneration" property must not be spent now.

### 5.5 Decision: no shared scanner abstraction — inherited, and now backed by a fact

T5D §5.4 rejected a shared `scan_root(root, walk_policy, parse)` and named T6 by name as the justifying case:

> *"T6's OpenCode agents are entries in a JSON object with no directory and no file per component, so a 'walk a directory, parse a file' abstraction is provably wrong for one third of its known future callers before it ships"* — T5D:195

**T6 is that case, and it confirms the prediction exactly.** This adapter has no directory walk, no per-component file, no recursion policy, no filename rule, no extension match, and no `walkdir`. There is nothing for that abstraction to abstract. The decision is inherited, not re-derived.

**What T6 adds is one fact that upgrades the argument for the *data* layer**, which T5D could only argue structurally:

> Claude Code's `tools` is a **scalar comma-separated string** (`agents.rs:55-58`, verified at T5). OpenCode's `tools` is an **object** mapping tool name → bool, 18/18 (V2/V3).

A shared agent DTO would have had to pick one, and either pick makes every real agent of the other client fail to deserialize and vanish from the inventory. The two clients agree on the *word* `tools` and on nothing else about it. This is recorded so a future reviewer reading three near-identical adapters sees a measured divergence rather than copy-paste debt.

Likewise `OpenCodeAgentScan` is a **third distinct type** with the same three fields as `SkillScan` and `AgentScan`, for T5D §5.4's reason unchanged: T9 destructures all three into `ScanReport` and never holds any of them generically. **Three near-identical adapters is the intended end state of Phase 1.** The extraction point, if ever, is T9 — when all adapters exist and their real common shape is observable rather than guessed.

### 5.6 Decision: T6 has **no** `escalate` function

T4 and T5 each carry a private `fn escalate(issue: ScanIssue) -> ScanIssue` (`skills.rs:151`, `agents.rs:226`) that raises whatever severity `frontmatter::read` returned to `Error`. T6 must not copy it, and the reason is worth a paragraph so its absence is not read as an omission.

`escalate` exists because T3's `frontmatter::read` is a **leaf reader that lacks caller context**: it cannot know whether a file that failed to parse was supposed to be a component, so it returns a conservative severity floor and the caller raises it. T6 has no such leaf. `jsonc::parse` returns a typed `JsoncError`, not a `ScanIssue`, so **T6 constructs every `ScanIssue` at the point where the caller context is already in hand**. There is no floor to raise.

The consequence is that T6 is the first adapter that can assign severity *by what was lost*, which is what makes §8's two-level taxonomy possible instead of T5's uniform `Error`.

## 6. The merge — the load-bearing algorithm

This is the section an implementer must not skim. A whole-object replacement here silently drops agents and the inventory still looks plausible.

### 6.1 What is merged

**Only the `agent` object.** Per file: parse the whole document (the parser has no choice), then take the value at the top-level key `agent`. `$schema`, `mcp`, `permission`, `share` and any future sibling (V5) are **read into memory and discarded, never merged and never inspected**. Nothing outside `agent` can produce a `Component`, a `ScanIssue`, or influence the merge — that is the out-of-scope-PoC-feature rule (`openspec/config.yaml`, proposal rule 3) enforced at the earliest possible point.

**Detection rule: presence of the key.** If the merged `agent` object has a key, it is an agent. **No name heuristics of any kind** — no prefix filter, no underscore rule, no "looks like a template", no `hidden` filter (§0). Guessing by name convention is how an inventory tool starts lying, and finding 6 already forced this discipline for `_shared`.

### 6.2 The algorithm, stated so replacement cannot be built by accident

> **Merge is a recursive deep merge of JSON objects, applied to the `agent` values in `scan_paths` order, last-wins at the leaf.**
>
> ```
> merge(base, overlay):
>     if base is Object(b) AND overlay is Object(o):
>         result = b
>         for (key, ov) in o:
>             result[key] = if result contains key then merge(result[key], ov) else ov
>         return Object(result)
>     else:
>         return overlay            # overlay replaces base wholesale
> ```
>
> Applied as an ordered fold over the surviving inputs: `inputs.fold(merge)`. A fold over **zero** inputs yields nothing (no agents, no issue); over **one** input yields that input unchanged — which is exactly what makes §8's isolation fall out with no special case.

**Depth: recursive, not one level and not two.**

| Option | Consequence | Decision |
|---|---|---|
| One level (merge the `agent` object's keys, replace each agent's value) | **Fails the discriminating fixture.** The same agent key in both files keeps only the overlay's fields; the base's `description` vanishes. This is the exact bug this change exists to prevent | **Rejected — this is the defect** |
| Exactly two levels (merge agent names, then merge each agent's fields, replace below) | Passes the fixture. But two is an arbitrary cutoff that is wrong the moment a nested field matters — `permission` is already a nested object on a real entry (V2) | **Rejected** |
| **Recursive at every depth; any non-object replaces** | One rule, statable in a sentence, correct at every depth, and the closest available model of the merge finding 2 was empirically verified against (which was verified on **MCP servers** — nested objects, i.e. depth > 2) | **Chosen** |

**The rules an implementer must not improvise:**

| Case | Behavior | Why |
|---|---|---|
| Object vs Object | Recurse per key. Keys present in only one side **survive unchanged** | The whole point |
| Array vs anything | Overlay **replaces**. Arrays are never concatenated, never element-merged | Element-wise array merge has no defensible identity rule, and finding 2 gives no evidence for concatenation |
| Scalar vs Object, Object vs Scalar, Scalar vs Scalar | Overlay **replaces** | The `else` branch. Type changes across files are legal and the later file wins |
| Overlay value is `Null` | Overlay **replaces** — the key survives with value `Null`. **`null` does NOT delete a key** | RFC 7386 JSON Merge Patch gives `null` delete semantics, and adopting it here would let an overlay **erase an agent**. That is unverified against OpenCode and it is the highest-consequence guess available. Not made. §13 / T16 |
| Key present in base only | Survives verbatim | Directly required by CA-5 for the base |
| Key present in overlay only | Survives verbatim | **This is the CA-5 assertion**: "an agent defined only in the `.jsonc` appears in the list" |
| Duplicate key within one file | Last occurrence wins, inside `jsonc::parse` | Handled by `BTreeMap` in the seam (§5.2), not by the merge |
| Keys differing only by case or Unicode form | **Both survive as two distinct keys.** The merge is byte-key based and does **NOT** normalize | §9 — normalizing before merging would fabricate a merge OpenCode does not perform |

**Keys are merged verbatim, never normalized.** Stated as its own rule because it is the tempting shortcut: an implementer who normalizes keys before merging would make `"Reviewer"` in the base and `"reviewer"` in the overlay merge into one agent, which OpenCode itself does not do. Normalization happens **only** inside `ComponentId::derive`, after the merge, at the identity layer where it belongs (`identity.rs:55-57`).

### 6.3 Why the merge operates on parsed values, not on text

The plan requires that "a malformed JSON produces a `ScanIssue` and does not prevent reading the other file" (`plan-desarrollo-poc.md:165`). That **forbids** the obvious implementation of concatenating or textually splicing the two files and parsing once: a syntax error anywhere would abort everything.

> **Parsing is per file. The merge consumes already-parsed `JsonValue`s. One file's failure removes one input from the fold; it never aborts it.**

This is why §5.3's control flow parses before merging, and why the fold must tolerate a zero- and one-element input list without a special case.

### 6.4 Component assembly

```rust
Component {
    id: ComponentId::derive(ComponentKind::Agent, key),   // from the merged KEY alone
    name: key.to_string(),                                 // verbatim, un-normalized
    kind: ComponentKind::Agent,
    description: /* Some(s) iff merged_entry["description"] is JsonValue::String(s) */,
    scope: Scope::User,                                    // CA-14; the only value ever constructed
    locations: /* one per DECLARING file, in scan_paths order — see below */,
    provenance_hint: None,
}
```

- **`id` derives from the key alone** — never from the file it came from, never from `description`, never from content. `ComponentId::derive` consumes a name, never a path (T4D §4's rule, inherited).
- **`name` is the raw key, un-normalized**, exactly as T5 kept the verbatim frontmatter name. Normalization is an identity concern, not a display one.
- **`description` is `None` unless the merged value is a string.** Absent → `None`, no issue. Present but not a string → `None` **plus a `Warning`** (§8).
- **`provenance_hint: None`**, per T4D §4: filling it would duplicate information already carried structurally, and `component.rs:26-31` forbids branching on it.
- **`scope: Scope::User` always.** No `Project` or `Local` is constructed anywhere; a contract test asserts it.

**One `Location` per declaring file**, `origin: LocationOrigin::File`, `path: Some(<that file>)`, `root: SearchRootId("opencode-agents")`, ordered by `scan_paths`.

| Option | Consequence | Decision |
|---|---|---|
| One `Location`, always the canonical `.json` | Simple. But an agent declared **only** in the overlay would report a path it is not in — the CA-5 case, mis-located. Actively misleading | **Rejected** |
| One `Location`, the file it was *last* declared in | Loses the fact that the base declared it, which is the fact the partial-override fixture exists to prove | **Rejected** |
| **One per declaring file, `scan_paths` order** | An agent in both files carries two `Location`s and is visibly "declared in both". `component.rs:9-12` already specifies "the same component under N search roots yields ONE `Component` with N `Location` entries" | **Chosen** |

**The honest stretch, recorded rather than hidden.** `component.rs`'s wording says N locations arise from N **search roots**; here two locations share **one** root id, because they are two files inside one root. The model permits it — `Location.root` is a reference, not a uniqueness constraint — but the prose did not anticipate it. It is **not amended**, because editing `model/` forfeits §2's property for a doc comment. Same class of wart as T5D §3's, flagged the same way, handed to T9 in §13. `opencode_agents.rs`'s module doc carries the reconciliation.

## 7. Determinism over an unordered map

A JSON object has no inherent key order, and a `HashMap` iterates in a per-process-random order in Rust. Left alone, component order would differ **between runs of the same binary**, not merely between platforms.

> **Determinism is bought at the type level, not by discipline: `jsonc::JsonValue::Object` is a `BTreeMap<String, JsonValue>` (§5.2). Iteration is sorted by key, by `Ord for String` = byte-wise over UTF-8, on every platform and every run.**

**Why the type and not a `sort_by_key` call at the end.** A trailing sort is a line an implementer, a refactor, or a merge conflict can delete, and the resulting flakiness surfaces as an intermittent CI failure on one leg only — the most expensive class of bug to diagnose. Making the container ordered means there is no unsorted intermediate state to forget about. Cost: original file key order is lost. **Nothing consumes it** (§6.2 shows order is irrelevant to the merge; §6.4 shows it is irrelevant to the component), so the cost is zero.

Two follow-on properties:

- **Sort is by the raw key, not the normalized name.** Two keys colliding after normalization (§9) still have a stable relative order, so the shadowing fixture's assertions are reproducible.
- **Byte-wise, never locale collation.** Locale-sensitive ordering would differ across the three CI legs, which is the exact failure T5D §6 bought its explicit `read_dir` sort to avoid.

**Issue order** is `scan_paths` order — base-file issues then overlay-file issues — because §5.3's loop is ordered and never reorders. **Root order** is trivially deterministic: there is exactly one.

Assertions in the test suite are still written **order-independently** wherever the assertion is about content, so correctness never depends on ordering; ordering has its own dedicated test (§12).

## 8. Error paths: the `ScanIssue` taxonomy

**No new `ScanIssue` variant, no new field, no `ScanIssueKind`.** T3D §6's policy stands: `reason` is a developer diagnostic, not localized copy, and MUST NOT be parsed or branched on. **T12 has zero T6-authored strings to translate.**

**The severity rule, stated once and applied uniformly** — this is T6's refinement over T5's uniform escalation, made possible by §5.6:

> **`Error` means one or more agents are MISSING from the inventory. `Warning` means an agent is PRESENT but some metadata could not be read. Absence of a file is neither — it is reported through root `status` and never as a `ScanIssue`.**

| Failure | root `status` | severity | `path` | `reason` shape | Other file still contributes? |
|---|---|---|---|---|---|
| Neither config file exists | `NotFound` | *no issue* | — | — | n/a — 0 components, **0 issues** |
| One file absent, the other present | `Found` | *no issue* for the absent one | — | — | **yes** — absence is not a failure |
| `read_to_string` fails on an existing file (permissions, I/O) | `Found` | `Error` | `Some(file)` | `could not read OpenCode config: {io}` | **yes** |
| File content is not valid UTF-8 | `Found` | `Error` | `Some(file)` | `could not read OpenCode config: {io}` | **yes** — `read_to_string` reports this as `InvalidData`; no separate arm needed |
| `jsonc::parse` fails (syntax error, unterminated comment, BOM) | `Found` | `Error` | `Some(file)` | `could not parse OpenCode config: {err}` | **yes — CA-12, and the defining requirement of this change** |
| Parsed document root is not a JSON object (array, scalar) | `Found` | `Error` | `Some(file)` | `OpenCode config is not a JSON object` | **yes** |
| `agent` key **absent** | `Found` | *no issue* | — | — | **yes**. Absence is not a failure. Zero components from that file |
| `agent` key present but **not an object** | `Found` | `Error` | `Some(file)` | `the "agent" key is not a JSON object` | **yes**. The user declared `agent`; we cannot read it; agents are missing ⇒ `Error` |
| `agent` object present and **empty** | `Found` | *no issue* | — | — | yes. Zero components, zero issues |
| A merged agent's value is **not an object** | `Found` | **`Warning`** | `Some(file)` | `agent "{key}" is not a JSON object; its metadata was not read` | yes. **The `Component` IS still emitted**, `description: None` |
| A merged agent's `description` is present but **not a string** | `Found` | **`Warning`** | `Some(file)` | `agent "{key}" has a non-string description` | yes. **The `Component` IS still emitted**, `description: None` |
| Two keys collide after normalization | `Found` | *no issue* | — | — | yes — **both** components emitted (§9) |
| Home directory unresolvable | — | *not a `ScanIssue`* | — | — | **no** — `ScanError`, T4D §7.2, unchanged and untouched |

**Why the last two `Warning` rows exist at all.** The alternative — emit the component with `description: None` and stay silent — was considered and rejected. Silence would make an implementer's shape assumption undetectable: if the extraction path were subtly wrong, every agent would arrive with no description and every test asserting only counts would still pass. A `Warning` is the cheapest possible tripwire, and it cannot be confused with a missing agent because the severity rule above says so explicitly. It is **not** an `Error`, because nothing is missing from the inventory — copying T5's uniform escalation here would report a healthy inventory as broken.

**Why an absent file is silent** (the CA-9 discipline, generalized from T4's empty `~/.config/opencode/skill/`): absence is a fact about the machine, and the model already expresses it through `SearchRootStatus`. Emitting an issue for it would produce an `Error` on every machine without OpenCode installed — noise that trains users to ignore the issue list.

**No non-UTF-8 *path* guard is needed, unlike T4/T5.** Both scan paths are `home` plus hardcoded ASCII segments, so a non-UTF-8 path is reachable only from a non-UTF-8 `home`, which `roots::resolve_home` (`roots.rs:43-47`) already rejects with a `ScanError` before any scan starts. T4D §7.1's `ensure_utf8_path` helper has no analogue here and **must not be copied in**; there is no discovered path to guard. Non-UTF-8 *content* is a different thing and is covered by row 4.

**UTF-8 BOM.** A BOM-prefixed config fails `jsonc::parse` and surfaces as an `Error` with its path. It is **not stripped**: whether OpenCode itself accepts one is unverified, and silently accepting input the client rejects is the §5.2 leniency mistake in another costume. Same posture T5 left it in (T5D §13). **T16.**

**Nothing here crosses IPC in T6** — no command exists. These strings reach the UI only after T9 aggregates and T10 serializes.

## 9. Identity collisions: two components, one `ComponentId`

Two independent collision sources, one decision.

**(a) Intra-adapter, after normalization.** `"Reviewer"` and `"reviewer"` are two distinct JSON keys — in the same file or across the two files, since §6.2 forbids normalizing before merging. Both derive `"agent:reviewer"` (`identity.rs:55-57`: trim → NFC → lowercase). Same for an NFC/NFD pair, which V6 confirms does not occur in real data but which a crafted config reaches trivially.

**(b) Cross-adapter.** An OpenCode agent and a Claude Code agent sharing a name collide the same way.

> **Decision, for both: emit both components, sharing one `ComponentId`. No `ScanIssue`, no filtering, no merging. Consolidation is T8's.**

This is T5D §9's precedent applied unchanged, and T4 already ships the shape: its reference fixture yields **69 components carrying 25 distinct ids** (`tests/skill_scanner.rs:260-275`). T6 introduces no new class of problem.

**Why no `ScanIssue` for case (a).** A duplicate identity is not a scan failure — nothing failed to be read and nothing is missing. `ScanIssue` is the channel for "something could not be read" (§8), and widening it to "something is worth reviewing" would put a policy judgment in the adapter that T8 is the phase equipped to make, with every adapter's output in hand. Emitting an issue would also fire on a perfectly legitimate cross-adapter duplicate, on every machine that runs both clients.

**Why T6 must not consolidate.** Consolidation is a whole-scan operation. T6 sees only OpenCode agents; merging locally would produce an inventory consolidated *within* one adapter and un-consolidated *across* adapters — a worse and less predictable state than uniformly deferring.

**What T8 inherits, stated so it is not a surprise.** Case (a) is the first instance where two components sharing an id also share a **root id** and differ only in `Location.path` and `name` casing. `component.rs:9-12` already specifies the target shape: one `Component`, one id, N `Location`s. T8 must additionally decide which `name` casing and which `description` survive. T6's contribution is that the `.jsonc`-declared entry's `description` may be `None`, so "prefer the non-`None` description" remains available as a rule rather than a coin flip. §10's `normalize-collision` fixture exists to hand T8 a pre-built, pre-asserted instance.

## 10. Fixture architecture

**The seam that makes this testable is `home` as a parameter**, inherited unchanged. `roots::home_dir()` is the only function that reads the environment and nothing in `opencode_agents` calls it. **No test reads the author's machine, and no test sets or reads an environment variable** — no `std::env::set_var`, which is unsound under parallel test execution anyway (T4D §8).

```
crates/vertice-core/tests/fixtures/roots/
├── <T4's nine skill homes>                  # untouched; no OpenCode-agent test points here
├── agents/<T5's ten homes>                  # untouched; likewise
└── opencode-agents/                         # NEW — grouping directory, never itself a home
    │   (every home below contains .config/opencode/)
    ├── absent-config/        .gitkeep only, no .config/ at all
    │                           → root NotFound (path populated), 0 components, 0 issues
    ├── empty-config-dir/     .config/opencode/.gitkeep, neither file
    │                           → root NotFound, 0 components, 0 issues  [TRIPWIRE]
    ├── json-only/            opencode.json with 2 agents
    ├── jsonc-only/           opencode.jsonc with 2 agents, no opencode.json
    │                           → root Found; CA-5's overlay half in isolation   [NON-NEGOTIABLE]
    ├── partial-override/     both files declare key "reviewer" with different sub-fields;
    │                         base carries description + a nested `permission` object,
    │                         overlay overrides only one nested leaf
    │                           → 1 component; base description SURVIVES; nested sibling
    │                             SURVIVES                                       [NON-NEGOTIABLE]
    ├── jsonc-syntax/         opencode.jsonc with // and /* */ comments AND a trailing comma
    │                           → parses; its agents are emitted                 [NON-NEGOTIABLE]
    ├── broken-json/          malformed opencode.json + healthy opencode.jsonc
    │                           → 1 Error carrying the .json path; ALL .jsonc agents emitted
    ├── broken-jsonc/         healthy opencode.json + malformed opencode.jsonc
    │                           → the exact mirror. Same shape, opposite file
    ├── no-agent-key/         opencode.json with $schema + mcp + share, no `agent`
    │                           → root Found, 0 components, 0 issues; the `mcp` key
    │                             produces nothing (V5)
    ├── empty-agent/          opencode.json with "agent": {}
    │                           → root Found, 0 components, 0 issues
    ├── normalize-collision/  "Reviewer" in .json, "reviewer" in .jsonc
    │                           → 2 components, both id "agent:reviewer", 0 issues (§9)
    ├── malformed-entry/      an agent whose value is a string; another whose
    │                         description is a number
    │                           → 2 components emitted, description None, 2 Warnings (§8)
    └── reference/            opencode.json: 5 agents (mirroring V2's shape — description,
                              mode, prompt, tools-as-OBJECT, hidden:true) + $schema + mcp
                              + permission + share;
                              opencode.jsonc: 3 agents, one of which shares a key with the base
                                → 7 components, 7 distinct ids, ≥1 sourced ONLY from .jsonc,
                                  ≥1 carrying two Locations                      [CA-5 PIN]
```

**Every directory under `opencode-agents/` is a synthetic home**, following T5D §10's chosen layout (`fixtures/roots/<suite>/<case>/`) so each suite's fixture namespace stays independently listable and growable. The restated invariant is unchanged: **nothing walks `fixtures/roots/`, `fixtures/roots/opencode-agents/`, or any grouping directory** — every test names its synthetic home explicitly. T4's and T5's homes are never reused and never referenced; the `broken-json` corrupt shape is a deliberate **copy**, never a pointer into another suite's tree.

**Three fixtures are marked NON-NEGOTIABLE, and §0/V4 is why.** The real `opencode.jsonc` has no `agent` key, no comments and no trailing commas. **The T16 oracle contrast will not exercise the `.jsonc`-agent path or the comment/trailing-comma path at all.** `jsonc-only`, `jsonc-syntax` and `partial-override` are the *sole* verification those three behaviors will ever receive. A green T16 must never be reported as covering them. This is stated in the fixture tree, in the spec, and here, because it is the one place where "verified against the real client" is actively misleading.

**`partial-override` is the single most load-bearing test in this change.** "An agent in each file" is **not** the discriminating fixture — it passes under a correct merge *and* under naive concatenation. The discriminating shape is the same key in both files with different sub-fields: under a correct per-key merge the base's non-overridden fields survive; under whole-object replacement they vanish. The nested `permission` object extends the same assertion one level deeper, which is what distinguishes §6.2's recursive choice from a two-level one. It must exist and **fail** before the merge function is written.

**The `.gitkeep` trap and its tripwire** (T4D §8, T5D §10, inherited). Git cannot track an empty directory, so `absent-config/` and `empty-config-dir/` both need a `.gitkeep`. If `empty-config-dir/`'s `.gitkeep` is lost, that directory vanishes and the "config dir present, files absent" test silently becomes the "nothing at all" test — **still passing**. A dedicated test named for its own failure, `empty_config_dir_fixture_still_exists_on_disk`, asserts the directory exists before any scanner code runs.

**`.gitattributes` needs no change**: line 2 already scopes `-text` to `crates/vertice-core/tests/fixtures/**`, which matters more here than for T4/T5 — a CRLF-normalizing checkout must not alter a fixture whose byte content is under test. Fixture paths are built from `env!("CARGO_MANIFEST_DIR")` with per-segment `push`, never `"/"`-joined literals (`tests/skill_scanner.rs:23-30` is the helper to copy).

## 11. Per-OS paths

Both scan paths are `home` plus hardcoded relative segments, pushed one at a time so separators are correct on all three CI legs. **No OS config-directory convention is consulted** — `$XDG_CONFIG_HOME`, `%APPDATA%` and `~/Library/Application Support` are all deliberately unused.

| Path | Windows (**verified**, this cycle — V1) | macOS (**unverified**) | Linux (**unverified**) |
|---|---|---|---|
| `opencode.json` (canonical, `SearchRoot.path`) | `C:\Users\<u>\.config\opencode\opencode.json` | `/Users/<u>/.config/opencode/opencode.json` | `/home/<u>/.config/opencode/opencode.json` |
| `opencode.jsonc` (overlay, `scan_paths[1]`) | `C:\Users\<u>\.config\opencode\opencode.jsonc` | `/Users/<u>/.config/opencode/opencode.jsonc` | `/home/<u>/.config/opencode/opencode.jsonc` |

**Windows is the interesting row, and it is the verified one.** OpenCode uses the XDG layout **on Windows too** — `~/.config/opencode/`, not `%APPDATA%\opencode\` — exactly as `alcance-poc-vertice.md:106` recorded and as V1 confirms by inspection of the reference machine. A `dirs::config_dir()`-style call would resolve to `%APPDATA%` there and find **zero** agents. This is the concrete reason the "never derive foreign paths from OS conventions" rule exists, and T6 is the change that demonstrates it.

macOS and Linux are unverified **by construction**: ground truth is one Windows machine. Revalidation on the other two platforms is T16's (`alcance-poc-vertice.md:71`), and because OpenCode uses one layout everywhere, T6 is *less* platform-fragile than T7 will be. `$XDG_CONFIG_HOME` being set to a non-default value on Linux would relocate the real config and Vertice would report `NotFound` at the default path — a known, legible limitation, not a silent one (§3's table, row 1), and it is out of scope by the same rule that forbids reading the variable.

## 12. File changes, testing, rollout

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/jsonc.rs` | **Create** | `JsonValue`, `JsoncError`, `parse` — the sole importer of the JSONC crate (§5.2) |
| `crates/vertice-core/src/opencode_agents.rs` | **Create** | `scan`, `OpenCodeAgentScan`, per-file read/parse/extract, `merge`, component assembly |
| `crates/vertice-core/src/roots.rs` | Modify (small) | `opencode_agent_root` + its unit tests. `probe`, `resolve_single`, `resolve_opencode` untouched |
| `crates/vertice-core/src/lib.rs` | Modify | two lines: `pub mod jsonc;`, `pub mod opencode_agents;` |
| `crates/vertice-core/tests/fixtures/roots/opencode-agents/**` | **Create** | §10, thirteen synthetic homes |
| `crates/vertice-core/tests/opencode_agent_scanner.rs` | **Create** | CA-driven integration suites |
| `crates/vertice-core/tests/jsonc_behavior.rs` | **Create** | Pinned seam behaviors, mirroring `tests/yaml_behavior.rs` |
| `crates/vertice-core/Cargo.toml` | Modify | one new `[dependencies]` entry. `serde_json` **stays** in `[dev-dependencies]` (§5.2) |
| `Cargo.lock` | Modify | first lockfile movement from a feature change since T2 |
| `deny.toml` | **In review scope** | Expected **unchanged**; if the crate's tree needs a new allow-list entry, that is a supply-chain review item, not a formality |
| `crates/vertice-core/src/{model/,frontmatter.rs,yaml.rs,skills.rs,agents.rs}` | **Unchanged** | §2, §5.5, §5.6 |
| `frontend/src/bindings/**` | **Unchanged** | no `TS` type added; drift gate green with **no regeneration** |
| `crates/vertice-app/**`, `frontend/src/**` | **Unchanged** | no IPC, no command, no capability change |
| `.github/workflows/**`, `.gitattributes`, `rust-toolchain.toml` | **Unchanged** | no MSRV change; the existing `msrv` job is the gate the new dependency must clear |

`strict_tdd: true`. Fixtures and RED tests land before implementation.

| Layer | What | How |
|---|---|---|
| Unit — seam | `jsonc::parse` accepts `//` and `/* */` comments and trailing commas; **rejects** unquoted property names; duplicate keys resolve last-wins; a syntax error yields `JsoncError` and never panics; the crate's own types do not appear in the signature | `tests/jsonc_behavior.rs`, in-memory, zero disk — the `tests/yaml_behavior.rs` pattern |
| Unit — merge | Base-only key survives; overlay-only key survives; **shared key deep-merges and the base's non-overridden fields survive** (incl. one level deeper); array replaces; scalar-vs-object replaces; `null` replaces and does **not** delete; fold over 0 inputs is empty; fold over 1 input is identity | `#[cfg(test)]` in `opencode_agents.rs`, on `JsonValue` literals, zero disk |
| Unit — roots | `opencode_agent_root` returns id `opencode-agents`, `kind: Agent`, path ending `opencode.json`, `scan_paths` of length 2 **in merge order**; ids stable across two different `home` values | `#[cfg(test)]` in `roots.rs` |
| Integration — CA-5 | `reference/` yields 7 components, 7 distinct ids, ≥1 sourced only from `.jsonc`; `jsonc-only/` yields its agents with no `.json` present | `tests/opencode_agent_scanner.rs` |
| Integration — CA-12 | `broken-json/` → exactly 1 `Error` carrying the `.json` path **and** every `.jsonc` agent emitted; `broken-jsonc/` → the exact mirror | idem |
| Integration — quiet paths | `absent-config/`, `empty-config-dir/`, `no-agent-key/`, `empty-agent/` each → 0 components **and 0 issues**; the first two additionally `status: NotFound` with a populated path | idem |
| Integration — scope discipline | `no-agent-key/`'s `mcp` key and `reference/`'s `mcp`/`permission`/`share`/`$schema` produce no component and no issue | idem |
| Integration — §9 | `normalize-collision/` → 2 components, one shared id, 0 issues | idem |
| Integration — §8 warnings | `malformed-entry/` → components still emitted, `description: None`, `Warning` (not `Error`) issues | idem |
| Contract | Every component has `kind: Agent`, `scope: Scope::User`, `id == ComponentId::derive(Agent, name)`; **no `Scope::Project`/`Local` is ever constructed**; `roots.len() == 1` for every home | assertions over `OpenCodeAgentScan` |
| Determinism | Two consecutive `scan` calls on `reference/` yield **byte-identical** component and issue vectors; the component id sequence is asserted against a literal expected order | idem — the §7 guarantee, tested rather than assumed |
| Regression | T4's `tests/skill_scanner.rs`, T5's `tests/agent_scanner.rs` and the `roots.rs` unit suite stay green **with no edits at all** | existing suites, unmodified |
| Tripwire | `empty-config-dir/.config/opencode/` still exists on disk | §10, named for its own failure |
| Read-only (CA-16) | A full scan leaves the `reference/` tree byte-for-byte unchanged | the `fixture_tree_bytes` before/after pattern (`tests/skill_scanner.rs:234-258`) |
| Invariant | The JSONC crate is imported in exactly one module; no `serde_norway` and no `regex` anywhere in the new modules; no `walkdir` | structural + the existing seam-invariant test pattern |
| Supply chain | `cargo deny check bans licenses` green; `cargo check` green at MSRV 1.88 | CI's `quality` and `msrv` jobs |

**Chained-PR seam** (proposal forecast ~475–740 lines, budget risk Medium-High). The split isolates the supply-chain review from the logic review, which is the point:

1. **PR 1 — dependency, seam, roots, fixtures (~220–330 lines).** The new `Cargo.toml` entry with its license evidence in the PR body, `Cargo.lock`, `cargo deny check bans licenses` green, `src/jsonc.rs`, `tests/jsonc_behavior.rs`, `roots::opencode_agent_root` and its unit tests, and the **whole** `fixtures/roots/opencode-agents/**` tree including the `.gitkeep` tripwire test. Self-contained, compiles and is green on merge. A reviewer evaluating a third-party crate is not simultaneously evaluating merge semantics.
2. **PR 2 — the adapter (~260–410 lines).** `src/opencode_agents.rs` and `tests/opencode_agent_scanner.rs`, with RED-before-GREEN preserved by commit order inside the PR — `partial-override` failing first.

Splitting at "tests then implementation" instead is **rejected** for T5D's reason: a test naming `vertice_core::opencode_agents` fails to *compile* rather than fails an assertion, which is a poor RED. The seam above compiles at every step. `sdd-tasks` owns the final slicing; this is the seam it should start from.

**Rollback.** Additive at the model layer; the supply-chain layer is the part that must not be forgotten. Delete both new modules, their tests and the fixture tree; revert two `lib.rs` lines and the `roots.rs` addition; **revert the `Cargo.toml` entry, regenerate `Cargo.lock`, and revert any `deny.toml` allow-list entry added.** Leaving an unused dependency behind keeps a license gate and an advisory surface for code that no longer exists. Nothing in `model/` or `frontend/src/bindings/` is touched, so no revert has to be atomic across layers. `vertice-app` and the frontend are untouched. **Migration: none** — no persisted data and no IPC contract depends on any of this.

## 13. Open questions

- [x] **Canonical file for `SearchRoot.path`** — `opencode.json`, the merge base. Settled in §3.
- [x] **Root shape** — exactly one `SearchRoot`, id `opencode-agents`, both paths in `scan_paths`, `Found` iff either file exists. Settled in §3.
- [x] **`config.json` does not participate** — two files only. Settled in §4, with the merge kept arity-free so adding it later is a one-line change.
- [x] **JSONC crate** — `jsonc-parser` recommended, seam owns its own value type so any substitute touches one file. Settled in §5.2, **conditional on the four VERIFY-BEFORE-APPLY gates**, with two ordered fallbacks and hand-rolling explicitly forbidden.
- [x] **`serde_json` is NOT promoted** — one parser for both files, for behavioral symmetry rather than dependency count. Settled in §5.2.
- [x] **No DTO** — value-level extraction of `description` only; an unexpected type cannot make an agent vanish. Settled in §5.4.
- [x] **Merge is a recursive deep merge**, arrays and scalars replace, `null` replaces and does not delete, keys are never normalized before merging. Settled in §6.2.
- [x] **Parsing is per file, merge consumes parsed values** — one failure removes one fold input. Settled in §6.3.
- [x] **One `Location` per declaring file**, `scan_paths` order. Settled in §6.4.
- [x] **Determinism via `BTreeMap` in the seam**, byte-wise on the raw key, not a trailing sort. Settled in §7.
- [x] **Severity rule** — `Error` = agents missing, `Warning` = agent present with unreadable metadata, absence = no issue. Settled in §8.
- [x] **No `escalate` function** — T6 has no leaf reader whose floor needs raising. Settled in §5.6.
- [x] **Normalization collisions emit both components with one id**, no issue; T8 consolidates. Settled in §9.
- [x] **`hidden: true` does not exclude an agent.** Product decision, restated in §0.
- [x] **No shared scanner abstraction, no shared DTO** — inherited from T5D §5.4 and strengthened by the `tools` scalar-vs-object fact. Settled in §5.5.
- [ ] **Whether a legacy `config.json` carries agents on any real machine** (§4) — the one gap no fixture can detect. **T16**, via the `opencode debug config` contrast, which prints the fully-merged config. If it does, the fix is one prepended path.
- [ ] **Whether an overlay `null` deletes a key in OpenCode's own merge** (§6.2) — RFC 7386 says yes, OpenCode is unverified. Not guessed. **T16.** If it deletes, the merge gains one arm and `null`-delete gains a fixture.
- [ ] **macOS and Linux config locations** (§11) — XDG expected everywhere, verified only on Windows. **T16**.
- [ ] **UTF-8 BOM in a config file** (§8) — currently an `Error`, not stripped. **T16**.
- [ ] **Whether OpenCode accepts unquoted property names** (§5.2) — currently rejected as JSON5-not-JSONC. **T16**. If it accepts them, the option flips and one fixture is added; the seam signature does not change.
- [ ] **`Component`'s "N locations arise from N search roots" wording vs. two locations under one root** (§6.4) — reconcile at **T9**, which aggregates every root and sees the full set. Not amended here because editing `model/` forfeits §2's property. Same class as T5D's open `SearchRoot` wording item, and the two should be resolved together.
- [ ] **`SearchRoot.path` may name a file that does not exist while `status` is `Found`** (§3) — inherent to the canonical+alias shape `resolve_opencode` established. A UI wanting the file actually read must use `Location.path`. **T9/T11** decide whether the report needs a wider vocabulary; T6 does not widen the model to find out.
- [ ] **Cross-client duplicate marking** (§9) — **T8**, by design and by inheritance from T5D §9.
