# Design: Claude Code Agent Adapter

> Trace: **T5** (`internal-docs/plan-desarrollo-poc.md:132-147`) / addresses CA-5 (partial) and CA-13 (core half); contributes to CA-12; inherits CA-10; bound by CA-16 and CA-17. Closes open decision **2** of `plan-desarrollo-poc.md:387`.
> Proposal: `openspec/changes/2026-08-18-claude-code-agent-adapter/proposal.md`. Inherits T3's design (`openspec/changes/archive/2026-08-18-skill-frontmatter-reader/design.md`, hereafter **T3D**) and T4's (`openspec/changes/archive/2026-08-18-skill-scanner-user-roots/design.md`, hereafter **T4D**).
> `rules.design` coverage: core data model impact (§2), core/Tauri isolation for the CLI pathway (§1), per-OS paths (§11), `ScanIssue` taxonomy and error paths (§8).
> `rules.design` items **N/A in T5, with reason**: **IPC contract surface** — T5 registers no Tauri command and adds no `TS`/`Serialize` derive, so `frontend/src/bindings/` is byte-identical after this change (§2). That is the load-bearing property of this cycle, not an incidental one.
> **Spec coupling**: the `agent-scanner` capability spec was authored in parallel with this document. Where the two disagree on an observable behaviour, the spec wins and this design is amended; §13 lists the two places where I would have deferred to it.
> **Environment note**: `cargo` is not on PATH in the authoring environment. No claim below was verified by compiling. Nothing here depends on a new dependency, a toolchain feature, or an MSRV question, so the unverified surface is smaller than T4D §3's was.

## 1. Technical Approach

One new sibling module of `model/`, one parameter added to one private function, and **nothing else**.

```
                                     vertice-core
 frontend ──IPC──> vertice-app ──>   ├── model/        (pure data, zero I/O)  ← UNCHANGED, §2
                                     ├── roots         (+ agent_roots; resolve_single gains one param)
 future vertice-cli ────────────>    ├── skills        (T4, untouched — no shared abstraction, §5.4)
                                     ├── agents        (flat walk + embedded list — NEW)
                                     ├── frontmatter   (T3, consumed unchanged, byte-identical)
                                     └── yaml          (serde_norway seam, untouched)

 roots::agent_roots(home) ─> [ResolvedRoot; 2]
        ├── claude-agents           scan_paths = [~/.claude/agents]   ──> flat read_dir ──> *.md
        └── claude-embedded-agents  scan_paths = []                   ──> const list, no disk
                                             │
                       agents::scan(home) ───┴─> frontmatter::read::<AgentFrontmatter> ─> Component
                                                 └──────────────────────────────────────> ScanIssue
```

**The CLI pathway is preserved unchanged.** `agents::scan` takes a `&Path` (the home directory) and returns owned data. It performs no ambient-environment read at all: `roots::home_dir()` remains the single such call in the crate, one layer up, and every test bypasses it by passing a fixture path.

**`model/` purity survives trivially**, because `model/` is not edited (§2). All disk access lives in the new sibling, exactly as `frontmatter.rs` did in T3 (T3D §1) and `roots`/`skills` did in T4 (T4D §1).

## 2. Core data model impact: none, and what would break it

**None.** `Component`, `Location`, `LocationOrigin`, `Scope`, `SearchRoot`, `SearchRootKind`, `ScanIssue` and `IssueSeverity` are consumed exactly as merged in T2. Verified against source this cycle: `LocationOrigin::Embedded` (`model/location.rs:31`), `Location.path: Option<PathBuf>` (`location.rs:15`), `ComponentKind::Agent` (`component.rs:42`) and `SearchRootKind::Agent` (`location.rs:74`) all already exist. T5 is the first adapter to *construct* the first two; constructing a merged variant is not a model change.

Concrete, checkable consequence: CI's `git diff --exit-code -- frontend/src/bindings` step stays green with **no regeneration**. Unlike T4, this change cannot go red on binding drift, because it contains no `TS`-derived type. That is the mechanical proof T5 has no IPC surface.

**Exactly three things would break this property.** Stated so a reviewer can reject them on sight rather than discover them at CI:

| Temptation | Why it breaks the property | Verdict |
|---|---|---|
| Promote `AgentFrontmatter.tools` to a `Component` field | `Component` derives `TS`; a new field regenerates `Component.ts` and puts this change on the bindings drift gate | **Rejected — settled by the product owner: the PoC does not display an agent's tools.** `tools` is parsed and retained in `AgentFrontmatter`, and discarded at `Component` assembly |
| Promote `model` the same way | identical | Rejected, same reason. No UI shows it in the PoC |
| Add a third `LocationOrigin` variant (e.g. `Builtin`) to distinguish embedded agents from a future embedded skill | regenerates `LocationOrigin.ts`, and `component.rs:34-36` marks these enums as breaking-change surfaces | Rejected: `Embedded` already means exactly "reported by a client with no backing file" (`location.rs:29-31`) |

`tools` is retained rather than dropped from the struct because deleting the field would make the "`tools` is a scalar, not a sequence" contract untestable — the empirically-verified fact this cycle exists to pin (proposal, Approach). A parsed-and-unused field is the cheapest possible place to keep that assertion honest. It also means T11 gains the field by adding one `Component` field and one binding, with no re-parse.

**Squelch note for apply**: an unused public struct field triggers no `dead_code` warning (it is `pub` on a `pub` struct), so no `#[allow]` is needed and none may be added.

## 3. Decision: what `Location.root` holds for an embedded component

An embedded agent has no file and no directory. `Location.root` is a non-optional `SearchRootId` (`location.rs:18`), so *something* must go there.

| Option | Consequence | Decision |
|---|---|---|
| Reuse the on-disk id `SearchRootId("claude-agents")` | Smallest diff, no second root. But it breaks a property T4D §2.1 stated as already-derivable: *a root has entries iff some `Component.locations[].root` equals its id*. A user with `~/.claude/` but no `~/.claude/agents/` would get a root reported `NotFound` with six locations pointing into it — a self-contradictory report handed to T9 and rendered by T11 as "6 agents inside a directory that does not exist" | **Rejected** |
| `Location.root: Option<SearchRootId>` | A `model/` change and a `Location.ts` regeneration, to express an absence the report can express otherwise | **Rejected** — violates §2 outright |
| `locations: vec![]` (no location at all) | No id needed. But `origin` lives on `Location`, so a component with zero locations carries no `Embedded` marker, and CA-13's "distinguishable by `origin` + `path` alone, with no name heuristic" becomes unsatisfiable | **Rejected** |
| **A second, distinct root: `SearchRootId("claude-embedded-agents")`, `path: <home>/.claude`, `kind: Agent`, `status` probed at that path** | One extra `SearchRoot` in `AgentScan.roots`. `Location.root` stays non-optional, the on-disk root's status stays truthful, and the derivable "has entries" property survives for both roots | **Chosen** |

`roots::agent_roots` therefore returns **two** `ResolvedRoot`s, the second with `scan_paths: vec![]` — a root that is resolved and reported but never walked. `ResolvedRoot` already expresses this: `scan_paths` is a `Vec`, and `agents::scan`'s walk loop iterates it, so an empty vector means "nothing to walk" with no special case in the caller.

**The honest wart, recorded rather than hidden.** `SearchRoot`'s doc comment says "a directory the scanner walked to produce zero or more components" (`location.rs:39-42`). `claude-embedded-agents` is a directory the scanner *probed* and did not walk. The wording is stretched. It is **not** amended, because editing a doc comment in `model/` is still an edit to `model/` and §2's property is worth more than the prose. `agents.rs`'s module doc carries the reconciliation instead, and §13 hands the wording question to T9, which is the phase that aggregates roots and will see all of them at once.

**Why `<home>/.claude` and not a synthetic path.** It is a real directory that really is where Claude Code keeps its state, and it is the path this design already probes for §4's gate. A fabricated path (`<embedded>`, an empty `PathBuf`) would be un-renderable text sitting in a `PathBuf` field, and T11 would need a special case to hide it. Accepted cost: a UI listing roots by path shows `~/.claude` alongside `~/.claude/agents`, which reads oddly. Same class of wart as T4D §4's alias-path issue, flagged the same way (§13).

## 4. Decision: the embedded list is emitted only when `~/.claude` is present

The proposal closed *whether* to emit the six embedded agents (yes). It did not close *when*.

> **Decision: `agents::scan` emits the six embedded components iff the `claude-embedded-agents` root's status is `Found` — that is, iff `<home>/.claude` exists on disk. Otherwise it emits none, and no `ScanIssue`.**

**Why not unconditionally.** A machine with no `~/.claude` at all has never run Claude Code, and reporting six of its agents is inventing inventory — the same failure the proposal rejected recursion for. It would also make the fully-empty case untestable as "empty": the `absent-root` fixture would yield six components, and every future "nothing installed" assertion in T9/T11 would have to carry a six-component exception.

**Why this is not T7's job being done early.** T7 owns *client detection* — versions, executables, install state. This is not detection: it is the same single `symlink_metadata` probe `roots::probe` already performs for every other root, expressed in the model's existing vocabulary (`SearchRootStatus`). No new primitive, no new heuristic, no new module. When T7 lands real detection, it replaces the *input* to this gate without touching the component contract, the const list, or `Location` assembly.

**Accepted cost, stated plainly.** A `~/.claude` directory left behind by an uninstall yields six phantom agents until T7. That is strictly better than yielding them on a machine that never had the client, and it is one probe away from being fixed by the phase that owns the question.

**Consequence for a success criterion, which must be read precisely.** The proposal's criterion "an absent and a present-but-empty `~/.claude/agents/` each produce no `ScanIssue` and **no component**" cannot be literally true for the present-but-empty case under *any* gate: if `~/.claude/agents/` exists, `~/.claude` exists, so the six embedded components are emitted. The criterion is about **file-backed** components, and §10's tests assert it that way — `components.iter().filter(|c| c.locations.iter().all(|l| l.origin == LocationOrigin::File)).count() == 0` — not by a bare `is_empty()`. Writing it as `is_empty()` would be a test that passes today and forbids CA-13 tomorrow.

## 5. Module and function surface

### 5.1 `roots.rs` — the minimal change, and why it is smaller than the proposal predicted

```rust
// crates/vertice-core/src/roots.rs

/// Resolve the Claude Code agent roots under `home`. Two entries: the
/// on-disk root that is walked, and the embedded pseudo-root that is only
/// probed (design §3).
pub fn agent_roots(home: &Path) -> [ResolvedRoot; 2];

// CHANGED: `kind` becomes a parameter. Visibility is UNCHANGED — still private.
fn resolve_single(home: &Path, id: &str, kind: SearchRootKind, suffix: [&str; 2]) -> ResolvedRoot;
```

The proposal predicted `resolve_single` would have to become `pub(crate)`. **It does not**, and the reason is a design choice this document makes: `agent_roots` lives in `roots.rs`, not in `agents.rs`. T3D §2 fixed the crate's rule — *modules are named after the thing, never the role* — and "search roots" is the thing `roots.rs` names. Root resolution for a second kind belongs there for the same reason skill root resolution does. Keeping it there means the only edit is **one added parameter to a private function**, with two call sites inside the same file passing `SearchRootKind::Skill`. No visibility widening, no cross-module coupling at all.

This is the smallest possible change because the two things `agents` needs from `roots` — `ResolvedRoot` and a public resolver — are already `pub` or newly `pub` by the same pattern `skill_roots` established. `probe` stays private. `resolve_home` stays private. T4's existing `roots.rs` unit suite (`roots.rs:133-224`) is the regression guard and must stay green with **only** the two `SearchRootKind::Skill` arguments added at its call sites.

Root ids, hardcoded and never path-derived (T4D §4's rule, inherited verbatim): `SearchRootId("claude-agents")` and `SearchRootId("claude-embedded-agents")`.

### 5.2 `agents.rs` — the new module

```rust
// crates/vertice-core/src/agents.rs

/// Owned result of one agent scan. A distinct type from `SkillScan`, not an
/// alias and not a shared generic — see §5.4.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentScan {
    pub roots: Vec<SearchRoot>,       // always 2 (§3)
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

pub fn scan(home: &Path) -> AgentScan;   // infallible, mirroring skills::scan

/// Frontmatter contract for a Claude Code agent `*.md`. `Deserialize`-only:
/// no `Serialize`, no `TS`, so it emits no binding (§2).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AgentFrontmatter {
    pub name: String,                    // required — identity derives from it
    pub description: Option<String>,
    pub model: Option<String>,
    pub tools: Option<String>,           // comma-separated scalar, NOT Vec<String>
}

/// The six agents Claude Code ships with no file behind them. Provenance:
/// `claude agents` on the reference machine, verified 2026-08-18
/// (`alcance-poc-vertice.md:118`, finding 4). MANUAL MAINTENANCE: if
/// Anthropic adds, removes or renames one, Vertice is silently wrong until
/// the T16 oracle contrast is re-run. One named const, never scattered.
const EMBEDDED_CLAUDE_AGENTS: [&str; 6] = [
    "Explore", "Plan", "general-purpose", "statusline-setup", "claude", "claude-code-guide",
];
```

`lib.rs` gains one line, `pub mod agents;`, with no crate-root re-export — matching `lib.rs:7-11`.

**On-disk component assembly**, reusing T2/T3 unchanged:

```rust
let fm: AgentFrontmatter = frontmatter::read(&path).map_err(escalate)?;   // §7
Component {
    id: ComponentId::derive(ComponentKind::Agent, &fm.name),
    name: fm.name,
    kind: ComponentKind::Agent,
    description: fm.description,
    scope: Scope::User,                       // CA-14; the only value ever constructed
    locations: vec![Location { path: Some(path), root: agent_root_id.clone(), origin: LocationOrigin::File }],
    provenance_hint: None,
}
// fm.model and fm.tools are deliberately dropped here (§2).
```

**Embedded component assembly:**

```rust
Component {
    id: ComponentId::derive(ComponentKind::Agent, name),   // "agent:plan", "agent:explore", ...
    name: name.to_string(),                                 // verbatim from the const, un-normalized
    kind: ComponentKind::Agent,
    description: None,
    scope: Scope::User,
    locations: vec![Location { path: None, root: embedded_root_id.clone(), origin: LocationOrigin::Embedded }],
    provenance_hint: None,
}
```

`description: None` is deliberate: the descriptions of the embedded agents are not knowable without running the oracle, and `Option` exists precisely so absence is representable rather than fabricated. `provenance_hint: None` follows T4D §4 — filling it with "ships with Claude Code" would duplicate `LocationOrigin::Embedded` as un-actionable display text, and `component.rs:26-31` forbids branching on it anyway. **The frontmatter `name` wins over the filename** for on-disk agents, exactly as it won over the directory name in T4D §4; `ComponentId::derive` consumes a name, never a path.

### 5.3 Decision: `AgentFrontmatter` lives in `agents.rs`, not `frontmatter.rs`

The precedent points the other way — `SkillFrontmatter` sits in `frontmatter.rs` (`frontmatter.rs:25-29`) — so this needs a real argument, not a preference.

| Option | Consequence | Decision |
|---|---|---|
| `frontmatter.rs`, beside `SkillFrontmatter` | All frontmatter DTOs in one place. But it edits T3's module, which the proposal's rollback plan and success criteria both require to stay untouched; and it makes reverting T5 a two-file surgery instead of deleting one file plus one `pub mod` line | **Rejected** |
| **`agents.rs`, beside its only consumer** | The struct sits next to the walk that fills it, the const list it deliberately does not apply to, and the assembly that drops two of its four fields. T3's module stays byte-identical | **Chosen** |

**Against the naming rule, explicitly.** The rule is *modules are named after the thing, never the role* (T3D §2). It constrains module **names**, not type placement, and both candidates satisfy it: `frontmatter` and `agents` are both things. The rule is therefore silent here, and the tie is broken by T3's own recorded intent: `read<T>` was made generic "so a future caller (T5) supplies its own frontmatter shape **without modifying this function**" (`frontmatter.rs:72-74`). *Supplies its own* is the operative phrase — the caller owns the shape. `SkillFrontmatter` lives in `frontmatter.rs` for a historical reason, not a normative one: T3 shipped before any walker existed and needed a concrete `T` to test the generic against.

**Cost, recorded.** A reader looking for "every frontmatter shape in the crate" must now look in two places. `agents.rs`'s module doc names the split and its reason. The pressure to consolidate is low and shrinking: T6's OpenCode agents are entries inside a JSON object, not frontmatter documents, so there is no third DTO coming.

### 5.4 Decision: no shared skills+agents scanner — the deferral, recorded

Following the precedent of T4D §2.2, where the `RootScan` wrapper was rejected rather than quietly omitted.

| Option | Consequence | Decision |
|---|---|---|
| A shared `scan_root(root, walk_policy, parse) -> (Vec<Component>, Vec<ScanIssue>)` | Must own recursion policy **and** filename rule — the two things the two callers disagree on. Worse, T6's OpenCode agents are entries in a JSON object with no directory and no file per component, so a "walk a directory, parse a file" abstraction is **provably wrong for one third of its known future callers before it ships** | **Rejected** |
| A shared `Scan { roots, components, issues }` type used by both | Cheaper — ~10 lines saved — but it is the first step of the same abstraction, and it asserts that a skill scan and an agent scan are one concept at exactly the moment this design argues they are not. T9 destructures both into `ScanReport` and never holds either generically | **Rejected**; `AgentScan` is a distinct type with an identical shape |
| **Duplicate the shape; extract nothing** | ~10 duplicated struct lines and one duplicated `escalate` function. Every divergence (§6, §7) stays local and visible | **Chosen** |

The right moment to extract, if ever, is **T9**, when all adapters exist and their real common shape is observable rather than guessed. Recorded here so T9 inherits the reasoning and not just the duplication.

## 6. Walk policy

| Question | Decision | Why |
|---|---|---|
| Depth | **Flat.** `std::fs::read_dir` on `~/.claude/agents/`, subdirectories are never descended into | Claude Code agents are `~/.claude/agents/<name>.md`, one level (`plan-desarrollo-poc.md:137`). T4 chose recursion because OpenCode's *own documented glob* is `{skill,skills}/**/SKILL.md`; no equivalent evidence exists for agents. A nested `.md` is not a documented agent, and discovering it invents an inventory entry |
| Dependency | **`std::fs::read_dir` only.** No `walkdir` | A flat read needs no work stack, no cycle policy, no depth tracking — the four silent-defect classes T4D §3 bought `walkdir` to avoid do not exist here. `walkdir` stays a direct dependency of the crate for `skills`; `agents` simply does not import it |
| Symlinks | **No `follow_links` decision to make.** `read_dir` does not traverse; a symlink to a directory is one entry that is not a file and is skipped, a symlink to a file is read through by `fs::read` | Recorded so the absence of T4D §6's explicit `follow_links(false)` is understood as *not applicable*, not as an oversight. The Windows-junction question T4D §11 left open does not arise |
| Detection | `entry.file_type()?.is_file()` **and** `path.extension() == Some("md")` — exact, lowercase, case-sensitive | The plan's rule is `.md` files under the agent root. Case-sensitivity is the conservative choice: a `.MD` agent is unobserved on the reference machine, Claude Code's docs use `.md`, and a case-insensitive match would behave differently on the case-sensitive Linux CI leg than on Windows/macOS. Flagged for T16 (§13) |
| Non-`.md` files, subdirectories, dotfiles | **Silently skipped. Never an issue** | Verbatim T4D §6's rule. A `.DS_Store` or a `notes/` directory is not a missing agent |
| Ordering | **Collect, then sort by file name before parsing** | `read_dir` yields OS-dependent order — unlike `walkdir`, which T4 configured with `sort_by_file_name()`. Without an explicit sort, component order differs between the Linux and Windows CI legs, and every downstream ordering (T9's report, T11's list) becomes non-reproducible. ~17 entries; the allocation is irrelevant. Assertions are still written order-independently, so correctness does not depend on it |
| Emission order | File-backed components first (sorted), then the six embedded in const declaration order | Deterministic and greppable. No semantic weight — T8 consolidates, T11 sorts for display |

## 7. Severity escalation, and the cost that comes with a weaker detection rule

> **T5 escalates every `ScanIssue` returned by `frontmatter::read` for a discovered `*.md` under the agent root to `IssueSeverity::Error`, uniformly. `path` and `reason` are untouched.**

One private `fn escalate(issue: ScanIssue) -> ScanIssue`, structurally identical to `skills::escalate` (`skills.rs:151-156`), one invariant, directly unit-testable. Duplicated rather than shared, per §5.4.

**Rationale, inherited from T4D §5**: the severity rule must agree with the detection rule. A `.md` file directly under `~/.claude/agents/` is an agent by the same "presence is the detection rule" logic; failing to parse one means the user has an agent on disk that is missing from their inventory. That is the caller-context knowledge T3D §5 said the leaf reader lacked and T3D §111 wrote the forward contract for.

**The cost T4 did not have, named rather than discovered later.** T4's detection rule is a filename match (`SKILL.md`), so a `README.md` beneath a skills root is invisible to it. T5's rule is an *extension* match, so a `README.md` placed in `~/.claude/agents/` is picked up, fails at "no opening fence", and — after escalation — surfaces as an **`Error`** for a file that was never an agent. Uniform escalation makes T5 noisier than T4 on exactly this shape.

**Considered and rejected: conditional escalation** — pass T3's severity through verbatim, since T3D §5's rule (`Error` iff the file got past the opening fence) already encodes "this file declared itself a frontmatter document". It is tempting precisely because it would need *no* `escalate` function at all. Rejected for this cycle: it makes T5's severity contract differ from T4's for no observed benefit, and the case is **unobserved** — all 17 files under `~/.claude/agents/` on the reference machine are agents, with no README and no non-agent `.md`. Fitting the severity rule to a hypothetical is the mirror image of the mistake the flat-walk decision avoids.

**Revisit trigger, explicit**: if T16's real-machine pass finds a non-agent `.md` under a real agent root, the fix is to drop `escalate` and inherit T3's floor — a one-function deletion. It is **not** a `README.md` filename allowlist; that would be the name-convention heuristic CA-8 and T4D §6 forbid everywhere else.

**Inherited consequence**: a BOM-prefixed agent file falls into `NoOpeningFence` (T3D §15), a `Warning` at the leaf, and surfaces here as an `Error`. Still skipped, still no component, still deferred to T16 — but louder, which improves T16's chance of catching it. Same state T4 left it in.

## 8. Error paths: `ScanIssue` taxonomy

**No new `ScanIssue` variant, no new field, no `ScanIssueKind`.** T3D §6's policy stands unchanged: `reason` is a developer diagnostic, not localized copy, and MUST NOT be parsed or branched on. **T12 has zero T5-authored strings to translate.**

| Failure | Root `status` | `severity` | `path` | `reason` shape | Scan continues? |
|---|---|---|---|---|---|
| `~/.claude/agents/` probe → `ErrorKind::NotFound` | `NotFound` | *no issue* | — | — | yes |
| `~/.claude/agents/` probe → any other `io::Error` | `Found` | `Error` | `Some(root)` | `could not inspect search root: {io}` | yes |
| Root path exists but is not a directory | `Found` | `Error` | `Some(root)` | `search root is not a directory` | yes |
| **`read_dir` itself fails after a successful probe** | `Found` | `Error` | `Some(root)` | `could not read search root: {io}` | yes — embedded list still emitted |
| **A `read_dir` iterator item is `Err`** | `Found` | `Error` | `Some(root)` | `could not read directory entry: {io}` | **yes**, same root, next entry |
| `entry.file_type()` fails | `Found` | `Error` | `Some(entry)` | `could not read directory entry: {io}` | yes |
| Discovered path not representable as UTF-8 | `Found` | `Error` | **`None`** | `skipped a file whose path is not valid UTF-8: {lossy}` | yes |
| Any `frontmatter::read` failure on a discovered `*.md` | `Found` | `Error` (escalated, §7) | `Some(file)` | verbatim from T3D §7 | **yes** — CA-12 |
| `<home>/.claude` probe → `NotFound` | embedded root `NotFound` | *no issue* | — | — | yes; zero embedded components (§4) |
| Home directory unresolvable | — | *not a `ScanIssue`* | — | — | **no** — `ScanError`, T4D §7.2, unchanged |

**Two rows differ from T4 and must not be copied blind.** `walkdir` folded directory-opening and per-entry iteration into a single `Result` stream and attached a path to entry errors (`skills.rs:96-110`). `std::fs::read_dir` splits them: opening returns `io::Result<ReadDir>` — a separate arm — and a failing iterator item is a bare `io::Error` with **no `DirEntry` and therefore no path**. `path: Some(root)` is the best available attribution, and it is deliberately not `None`: `None` is reserved for the non-UTF-8-path case, where nulling the path is the *point* (T4D §7.1). Attributing a nameless entry error to its root is strictly more useful than dropping it.

**Non-UTF-8 paths (T4D §7.1, inherited verbatim).** Skip the file, emit `ScanIssue { severity: Error, path: None, reason: "... {to_string_lossy()}" }`, emit no `Component`. Emitting `Location { path: None, origin: File }` is forbidden by `location.rs:27-28` — and T5 is the module where that temptation is strongest, because it constructs `path: None` legitimately three lines away for embedded components. **The distinction, stated so it cannot be blurred**: `path: None` with `origin: Embedded` is a component the client reports and no file backs; `path: None` on a `ScanIssue` is a file that exists and cannot be named. They are unrelated, and conflating them would put an un-serializable `PathBuf` one field away from the report. Testability is unchanged from T4D §7.1: a `#[cfg(unix)]` unit test on the conversion helper, no portable fixture.

**Nothing here crosses IPC in T5** — no command exists. These strings reach the UI only after T9 aggregates and T10 serializes.

## 9. Shadowing: two components, one `ComponentId` — intended, and T8's problem

A user-authored `~/.claude/agents/Plan.md` declaring `name: Plan` collides with the embedded `Plan`. `ComponentId::derive` normalizes to lowercase (`identity.rs:55-57`), so both derive `"agent:plan"`.

> **Decision: T5 emits both, as two separate `Component` values sharing one `ComponentId`. Consolidation is T8's.**

This is the same contract T4 shipped, where the reference fixture yields **69** components carrying only **25** distinct ids (`tests/skill_scanner.rs:260-275`). T5 introduces no new class of problem; it introduces the first *cross-origin* instance of one.

**Why T5 must not consolidate.** Consolidation is a whole-scan operation: it needs every adapter's output at once, and T5 sees only Claude Code agents. Merging locally would produce a component consolidated within one adapter and un-consolidated across adapters — a worse and less predictable state than uniformly deferring.

**What T8 inherits, stated so it is not a surprise.** Merging these two is precisely the case `Component` was shaped for: one `Component`, one id, **two** `Location`s — `{ path: Some(...), root: "claude-agents", origin: File }` and `{ path: None, root: "claude-embedded-agents", origin: Embedded }`. `component.rs:9-12` already specifies this ("discovering the same component under N search roots yields ONE `Component` with N `Location` entries"). T8 must additionally decide which `description` and `name` casing survive; T5's contribution is that the embedded entry always carries `description: None`, so "prefer the non-`None` description" is available as a rule and not a coin flip. §10's `shadowing` fixture exists to hand T8 a pre-built, pre-asserted instance of the case.

**Which shadows which is not decided here.** Whether a user-authored `Plan.md` *overrides* the embedded `Plan` inside Claude Code, or coexists with it, is unverified against the client. T5 reports what it observes and asserts nothing about precedence (§13, T16).

## 10. Fixture architecture

**The seam that makes this testable is `home` as a parameter**, inherited unchanged: `roots::home_dir()` is the only function that reads the environment and nothing in `agents` calls it. **No test reads the author's machine, and no test sets or reads an environment variable** — no `std::env::set_var`, which is unsound under parallel test execution anyway (T4D §8).

```
crates/vertice-core/tests/fixtures/roots/
├── <T4's nine skill cases>                     # untouched; no agent test ever points here
└── agents/                                     # NEW — grouping directory, never itself a home
    ├── absent-root/         .gitkeep only
    │                          → both roots NotFound, 0 components, 0 issues (§4)
    ├── empty-root/          .claude/agents/.gitkeep
    │                          → agent root Found, 0 file-backed components, 6 embedded, 0 issues
    ├── tools-scalar/        .claude/agents/reviewer.md  (`tools: Read, Grep, Glob, Bash`, `model: sonnet`)
    ├── folded-description/  .claude/agents/summarizer.md (`description: >`, multi-line)
    ├── missing-optional/    .claude/agents/minimal.md    (name + description only)
    ├── broken-frontmatter/  .claude/agents/{good,broken}.md
    │                          → 1 component + 1 Error issue carrying `broken.md` (CA-12 partial)
    ├── nested-decoy/        .claude/agents/flat.md + .claude/agents/group/nested.md
    │                          → exactly 1 file-backed component; the nested file is never seen
    ├── non-agent-entries/   .claude/agents/{real.md, notes.txt, .DS_Store, subdir/.gitkeep}
    │                          → 1 component, 0 issues (§6's silent-skip rule)
    ├── shadowing/           .claude/agents/Plan.md (`name: Plan`)
    │                          → 7 components, two sharing id "agent:plan" (§9)
    └── reference/           .claude/agents/<17 files>.md
                               → 17 file-backed + 6 embedded = 23 components, 23 distinct ids
```

**Every directory under `agents/` is a synthetic home.** This is T3D §9's `fixtures/<case>/` pattern at T4D §8's level, grouped one deeper.

| Layout option | Consequence | Decision |
|---|---|---|
| `fixtures/agents/<case>/` (a third top-level tree) | Breaks T3D §9's two-way split — *addressed files* under `frontmatter/`, *walked trees* under `roots/*` — for trees that are unambiguously walked | **Rejected** |
| `fixtures/roots/agents-<case>/` (prefixed siblings) | Keeps "every directory under `roots/` is a home" as one flat rule. But T4's nine cases and T5's ten interleave alphabetically, and T6/T7 add more | Rejected, narrowly |
| **`fixtures/roots/agents/<case>/`** | Each suite's fixture namespace stays independently listable and independently growable. Costs one restated invariant: **nothing walks `fixtures/roots/` or `fixtures/roots/agents/` itself** — every test names its synthetic home explicitly, which is already true of every existing test | **Chosen** |

**`fixtures/frontmatter/` is never walked**, and no agent test points at T4's nine skill homes. T3D §9's rule is what keeps one suite's fixture count from breaking another suite's assertions; the `broken-frontmatter` case is a deliberate **copy** of a corrupt shape, never a reference into T3's or T4's fixtures.

**The `reference/` tree, and why 23 is the number that matters.** Seventeen files, content generated by rule — `name`, a one-line `description`, `model: sonnet`, `tools: Read, Grep, Glob, Bash` — so a reviewer verifies *the rule and the count*, not 17 diffs. None of the 17 names collides with the six embedded, so the scan yields **23 components with 23 distinct `ComponentId`s**. That figure is the direct match to `claude agents`'s "23 active, 6 embedded" (`alcance-poc-vertice.md:150`) and is the strongest fixture-level statement of CA-5 available without the real machine. The 17-file count is pinned to the same recorded figure and verified on disk this cycle.

**The `.gitkeep` trap and its tripwire** (T4D §8, inherited). Git cannot track an empty directory, so `absent-root/` and `empty-root/` both need a `.gitkeep`. Detection is `*.md`, so `.gitkeep` is invisible to the walk — but if `empty-root/`'s `.gitkeep` is lost, that directory vanishes and the "present and empty" test silently becomes the "absent" test, still passing. A dedicated test named for its own failure, `empty_agent_root_fixture_directory_still_exists_on_disk`, asserts the directory exists before any scanner code runs — the same discipline, the same naming convention.

**`.gitattributes` needs no change**: line 2 already scopes `-text` to `crates/vertice-core/tests/fixtures/**`. Fixture paths are built from `env!("CARGO_MANIFEST_DIR")` with per-segment `push`, never `"/"`-joined literals (`tests/skill_scanner.rs:23-30` is the helper to copy).

## 11. Per-OS paths

Both roots are `home` plus a hardcoded relative suffix. **No OS config-dir convention is consulted** — `%APPDATA%`, `~/Library/Application Support` and `$XDG_CONFIG_HOME` are all deliberately unused, for the reason T4D §9 records: Claude Code uses `~/.claude` on every platform, and a `config_dir()`-style call would find zero agents on Windows.

| Root | Windows (**verified**, Aug 2026) | macOS (**unverified**) | Linux (**unverified**) |
|---|---|---|---|
| `claude-agents` | `C:\Users\<u>\.claude\agents\` | `/Users/<u>/.claude/agents/` | `/home/<u>/.claude/agents/` |
| `claude-embedded-agents` | `C:\Users\<u>\.claude\` | `/Users/<u>/.claude/` | `/home/<u>/.claude/` |

macOS and Linux are unverified by construction: ground truth is one Windows machine, and revalidation on the other two platforms is T16's (`alcance-poc-vertice.md:71`). Suffixes are built with per-segment `PathBuf::push`, so they are separator-correct on all three CI legs.

## 12. File changes, testing, rollout

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/agents.rs` | Create | `scan`, `AgentScan`, `AgentFrontmatter`, `EMBEDDED_CLAUDE_AGENTS`, flat walk, `escalate`, `ensure_utf8_path` |
| `crates/vertice-core/src/roots.rs` | Modify (small) | `agent_roots`; `resolve_single` gains a `kind` parameter, stays private |
| `crates/vertice-core/src/lib.rs` | Modify | one line: `pub mod agents;` |
| `crates/vertice-core/tests/fixtures/roots/agents/**` | Create | §10, ten synthetic homes |
| `crates/vertice-core/tests/agent_scanner.rs` | Create | CA-driven suites |
| `crates/vertice-core/src/{model/,frontmatter.rs,skills.rs,yaml.rs}` | **Unchanged** | §2, §5.3, §5.4 |
| `frontend/src/bindings/**` | **Unchanged** | no `TS` type added; drift gate green with no regeneration |
| `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/workflows/**`, `.gitattributes` | **Unchanged** | no new dependency — `std::fs::read_dir`, no `walkdir` |
| `vertice-app`, `frontend/src/**` | **Unchanged** | no IPC, no command, no capability change |

`strict_tdd: true`. Fixtures and RED tests land before implementation. The 17-file `reference/` set exists before any assertion counts it; the `tools: Read, Grep, Glob, Bash` fixture exists before `AgentFrontmatter` is written.

| Layer | What | How |
|---|---|---|
| Unit | `escalate` maps every T3 severity to `Error`; path→UTF-8 conversion (`#[cfg(unix)]`); `agent_roots` returns exactly 2 with stable, never-path-derived ids; the embedded pseudo-root's `scan_paths` is empty | `#[cfg(test)]` in `agents.rs`/`roots.rs`, in-memory, zero disk |
| Integration | CA-5-partial (23 components, 23 ids over `reference/`); CA-13 (6 components with `origin: Embedded` **and** `path: None`, identified by that pair alone with no name matching); CA-12-partial; folded description (CA-10 inherited); `tools` scalar; missing optionals → `Ok` with `None`; flat walk skips `nested-decoy`'s nested file; non-agent entries produce no issue; absent vs. empty root distinguishable by `status` | `tests/agent_scanner.rs` over `fixtures/roots/agents/`, one synthetic home per case |
| Contract | No `Scope::Project`/`Local` is ever constructed; `AgentScan.roots.len() == 2` for every home; `frontmatter.rs` is byte-identical (asserted by review, not code) | assertions over `AgentScan` |
| Regression | T4's `tests/skill_scanner.rs` and `roots.rs` unit suite stay green with only the two `SearchRootKind::Skill` arguments added | existing suites, unmodified otherwise |
| Tripwire | `empty-root/.claude/agents/` still exists on disk | §10, named for its own failure |
| Read-only (CA-16) | A full scan leaves the `reference/` tree byte-for-byte unchanged | the `fixture_tree_bytes` before/after pattern (`tests/skill_scanner.rs:234-258`) |
| Invariant | No `serde_norway` import in `agents.rs`; no regex; no `walkdir` import | `tests/yaml_seam_invariant.rs` already covers the first; the other two are structural |

**Read-only (CA-16), structurally.** The complete disk surface of the new module is `std::fs::symlink_metadata` (via `roots::probe`), `std::fs::read_dir`, and T3's `std::fs::read`. No `File::create`, `OpenOptions`, `fs::write`, `create_dir*` or `remove_*` — including in the tests, which read committed fixtures and never materialize a temp tree. `rules.apply`'s grep finds nothing.

**Chained-PR seam** (proposal forecast: ~500–720 lines, budget risk Medium-High). The natural split is **not** "tests then implementation", because a test naming `vertice_core::agents` fails to *compile* rather than fails an assertion, which is a poor RED. The seam that compiles at every step:

1. **PR 1 — roots and fixtures (~180–260 lines).** `roots::agent_roots`, the `resolve_single` `kind` parameter, its unit tests, and the whole `fixtures/roots/agents/**` tree including the `.gitkeep` tripwire test. Self-contained, green on merge, and it is the piece T4's regression suite is most exposed to.
2. **PR 2 — the agents module (~320–460 lines).** `agents.rs` and `tests/agent_scanner.rs`, with RED-before-GREEN preserved by commit order inside the PR.

`sdd-tasks` owns the final slicing; this is the seam it should start from.

**Migration**: none. Rollback is the proposal's plan, and its load-bearing property is that **nothing in `model/` or `frontend/src/bindings/` is touched**, so no revert has to be atomic across layers — the failure mode that made T4's rollback delicate does not exist here.

## 13. Open questions

- [x] **`Location.root` for an embedded component** — a second root, `claude-embedded-agents`, at `<home>/.claude`. Settled in §3.
- [x] **When the embedded list is emitted** — iff `<home>/.claude` is `Found`. Settled in §4.
- [x] **`AgentFrontmatter` lives in `agents.rs`**, not `frontmatter.rs`. Settled in §5.3.
- [x] **`AgentScan` is a distinct type**, not shared with `SkillScan`; no scanner abstraction before T9. Settled in §5.4.
- [x] **Non-`.md` files and subdirectories are silently skipped**, never reported. Settled in §6.
- [x] **`read_dir` output is sorted before parsing** so component order is identical on all three CI legs. Settled in §6.
- [x] **Shadowing emits two components with one id**; T8 consolidates into one `Component` with two `Location`s. Settled in §9.
- [x] **`resolve_single` stays private** — only a `kind` parameter is added, because `agent_roots` lives in `roots.rs`. Settled in §5.1; smaller than the proposal predicted.
- [ ] **`SearchRoot`'s doc wording vs. a probed-but-unwalked root** (§3) — reconcile at **T9**, which aggregates every root and can see the full set. Not amended here because editing `model/` would forfeit §2's property.
- [ ] **`~/.claude` as a client-presence signal** (§4) — replaced by real detection at **T7**. A stale directory from an uninstall yields six phantom agents until then.
- [ ] **Case-sensitive `.md` matching** (§6) — whether Claude Code itself accepts `Plan.MD` is unverified. **T16**, on a real machine.
- [ ] **Whether a user-authored agent overrides or coexists with its embedded namesake** (§9) — unverified against the client. **T16**; T8 needs the answer before it picks a winner.
- [ ] **A non-agent `.md` under the agent root surfaces as an `Error`** (§7) — unobserved on the reference machine. If **T16** finds one in the wild, drop `escalate` and inherit T3's floor; never add a filename allowlist.
- [ ] **UTF-8 BOM** — unchanged from T4: skipped, no component, reported as `Error`. **T16**.
- [ ] **Embedded list drift** — the accepted known limitation. The only detector is T16's manual `claude agents` contrast. Provenance and verification date live in the const's doc comment (§5.2).
- [ ] **Deferred to the parallel spec** — two behaviours this design fixes but the `agent-scanner` spec is the authority on: (1) the exact `reason` prefixes in §8's table, and (2) whether "no component" in the empty-root requirement is written as file-backed-only (§4). If the spec words either differently, the spec wins and §4/§8 are amended.
