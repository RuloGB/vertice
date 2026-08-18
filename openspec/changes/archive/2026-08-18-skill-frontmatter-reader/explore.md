# Exploration — Frontmatter and `SKILL.md` Reader (T3)

**Change**: `skill-frontmatter-reader`
**Phase**: T3 of `internal-docs/plan-desarrollo-poc.md`
**Artifact store**: openspec
**Status**: complete — ready for `sdd-propose`

---

## 1. Current State (post-T2)

**`vertice-core` crate layout** (`crates/vertice-core/src/`):

- `lib.rs:1-26` — `pub mod model; pub mod yaml;` plus one smoke test. No third module exists yet. No `std::fs`/`std::io` usage anywhere in the crate today — T2's model purity invariant (`model/mod.rs:8-15`) forbids it there, and nothing else touches the filesystem yet. T3 will be the **first** module in `vertice-core` to perform disk I/O.
- `yaml.rs:1-24` — the YAML deserialization seam: `pub fn from_str<T: DeserializeOwned>(input: &str) -> Result<T, YamlError>`, wrapping `serde_norway::from_str`. `YamlError::Parse(#[from] serde_norway::Error)` via `thiserror`. The doc comment (`yaml.rs:1-9`) states an explicit invariant: **"This is the ONLY module in `vertice-core` allowed to import the YAML parsing crate (`serde_norway`) directly."** Any new module MUST route YAML parsing through `yaml::from_str` and MUST NOT import `serde_norway` itself.
- `tests/yaml_behavior.rs:1-93` — pins seam behavior: folded (`>`) and literal (`|`) block scalars, bareword `no`/`yes` staying `String` not `bool`, CRLF normalization in literal scalars, duplicate-key rejection as a hard error. **No test exists yet for a type-mismatch case** (a scalar-typed field fed a YAML list or mapping) — a real gap for T3 to close, not an assumption to inherit.
- `model/` (9 types across `component.rs`, `location.rs`, `installation.rs`, `report.rs`, `identity.rs`, `error.rs`) — no `SkillFrontmatter`-shaped type exists. `Component { id, name, kind, description: Option<String>, scope, locations, provenance_hint }` (`model/component.rs:16-32`) is the eventual consumer of T3's output, assembled by **T4**, not T3.
- **No fixtures directory exists** anywhere in the repo. T3 is the first task to introduce `tests/fixtures/`.
- **No `.gitattributes` file exists.** Relevant to the non-UTF-8 fixture requirement (see §8).
- `deny.toml:1-67` — license allow-list already covers MIT/Apache-2.0/BSD/ISC/Unicode-3.0/Zlib/CC0-1.0/MPL-2.0; `cargo deny check bans licenses` runs in CI's `quality` job (`.github/workflows/ci.yml:74-75`). If T3 needs zero new dependencies (see Q1), this gate needs no changes.
- `Cargo.toml:12-17` (workspace deps): `serde`, `thiserror`, `ts-rs`, `unicode-normalization`. `crates/vertice-core/Cargo.toml:8-16` adds `serde_norway = "0.9"` directly (not workspace-level) plus `serde_json` as a dev-dependency.

**T2's explicit carry-forward contract for T3** (`openspec/changes/archive/2026-08-17-domain-model-type-contract/archive-report.md:126-132`, `design.md §6`):

> From T3 onward, an adapter that meets a **non-UTF-8 path** MUST emit a `ScanIssue` (`path: None`, `reason` carrying a lossy rendering) and MUST NOT construct a `Location` holding it.
> Seam-internal parse failure (`YamlError`) is converted to a `ScanIssue.reason` string **by T3**.

This is a binding design decision, and it concerns non-UTF-8 **paths** — a distinct case from non-UTF-8 **file content** (see Q3).

**`ScanIssue` shape** (`model/report.rs:36-42`): `{ severity: IssueSeverity (Warning|Error), path: Option<PathBuf>, reason: String }` — a single `reason: String`, not a list. This shapes how many issues one file read can plausibly report (see Q2).

## 2. What T3 Requires

From `internal-docs/plan-desarrollo-poc.md:92-107`:

- A tested function that, given the path of a `SKILL.md`, returns name, description, and parse errors.
- Frontmatter parsed with the T1-chosen YAML crate (`serde_norway`). **Regex is explicitly forbidden** — multi-line `description: >` breaks under regex (finding 7).
- Fields read: `name`, `description`. Missing fields and unexpected types produce a `ScanIssue`, never a panic.
- Versioned fixtures: normal frontmatter, multi-line frontmatter, absent frontmatter, corrupt YAML, empty file, non-UTF-8 file.
- Depends on T2 (consumes `ScanIssue`/`IssueSeverity`).

Acceptance criteria this phase must close:

- **CA-10**: a skill with multi-line `description: >` returns the complete, correct description.
- **CA-12 (partial)**: a corrupt file returns a `ScanIssue` carrying its path, without interrupting anything.
- All tests run on repository fixtures (no machine-dependent paths).

**T5's explicit dependency note** (`plan-desarrollo-poc.md:141`): *"T5 ... Depende de: T2 (y reutiliza el lector de frontmatter de T3)."* T5 (Claude Code agents) reuses T3's reader for a differently-shaped frontmatter (`name`, `description`, `model`, `tools`). This is a real forward-compatibility constraint on T3's API design.

## 3. The Problem T3 Solves, and Its Boundary

T3 is a **leaf-level, single-file** operation: `Path in → (SkillFrontmatter | ScanIssue) out`, with zero directory walking and zero knowledge of the three root paths. Concretely, T3 owns:

1. Reading file bytes from a given path (first `std::fs` usage in the crate).
2. Detecting and rejecting non-UTF-8 **content** (distinct from T2's non-UTF-8 **path** contract — see Q3).
3. Splitting the `---`-fenced frontmatter block from the Markdown body (a text-slicing concern, not YAML parsing).
4. Deserializing the extracted block via `yaml::from_str`, converting any `YamlError` into `ScanIssue.reason`.
5. Producing a `ScanIssue` for every documented failure class without ever panicking.

**Explicitly NOT T3's job**: walking `~/.claude/skills/` et al. and discovering *which* files to read (T4); deciding whether a discovered path belongs to a plugin or a project scope (T4); aggregating multiple `ScanIssue`s from many files into one `ScanReport` and guaranteeing that one bad adapter does not abort the rest of the scan (T9). T3's contribution to "does not interrupt anything" is only the leaf-level non-panic guarantee, not the orchestration-level continuation guarantee.

## 4. The YAML Seam Question

`yaml.rs`'s current surface — `from_str<T: DeserializeOwned>(input: &str) -> Result<T, YamlError>` — is **sufficient but incomplete** for T3:

- **Sufficient** for step 4: given an already-isolated YAML string, it deserializes into a target struct and returns a typed error. No change needed to `yaml.rs` itself.
- **Insufficient** for step 3: `yaml.rs` has no concept of Markdown or frontmatter fences. `---`-fence splitting is a plain string operation that never touches `serde_norway`. Adding it to `yaml.rs` would dilute that module's stated scope as a crate-swap seam.

**Recommendation**: frontmatter extraction (the `---`/`---` split) belongs in a **new module separate from `yaml.rs`**, e.g. `crates/vertice-core/src/frontmatter.rs`. That module calls `yaml::from_str` internally for the YAML half and owns none of the deserialization logic. This keeps `yaml.rs` exactly what its doc comment says it is, and gives the splitting logic its own focused, regex-free test surface.

## 5. Approach Options

### Q1 — Frontmatter fence splitting: hand-rolled vs third-party crate

| Approach | Pros | Cons | Effort |
| --- | --- | --- | --- |
| **A. Hand-rolled line-based splitter** (`content.lines()`, first line must be exactly `---`, find the next line that is exactly `---`, block is everything between, body is everything after) | Zero new dependency; deterministic; regex-free by construction; `str::lines()` already normalizes CRLF, matching the precedent in `tests/yaml_behavior.rs:69-75`; splitting on fence markers happens *before* YAML parsing, so an indented block scalar's content cannot be mistaken for a fence | Edge cases must be handled by hand: no opening fence, opening fence with no closing fence before EOF, empty file, leading blank lines | Low |
| **B. Third-party frontmatter crate** (`gray_matter`, `frontmatter`) | Less hand-written logic | Re-triggers the T1-style crate governance (maintenance activity, license, MSRV) for ~15-20 lines of logic; disproportionate supply-chain cost | Medium (evaluation overhead) |
| **C. Parse the whole file as a multi-document YAML stream, take the first document** | Reuses the existing seam entirely | Semantically wrong — the body after the frontmatter is **Markdown**, not YAML; either fails uncontrollably or silently misbehaves | Rejected |

**Recommendation**: **A**. Hand-rolled, dependency-free, in a new module separate from `yaml.rs`. No crate evaluation, no MSRV or license delta.

### Q2 — Result representation: `Result<T, E>` vs partial result plus issue list

| Approach | Pros | Cons | Effort |
| --- | --- | --- | --- |
| **A. `Result<SkillFrontmatter, ScanIssue>`** — one outcome per file | Matches the plan's singular phrasing ("a corrupt file returns **a** `ScanIssue`"); matches `ScanIssue`'s own shape (one `reason: String`); simplest to implement and test; `?`-friendly | Cannot report two independent problems from one file in one call (e.g. "name missing" *and* "description wrong type") — not required by any CA in scope | Low |
| **B. `(Option<SkillFrontmatter>, Vec<ScanIssue>)`** | More diagnostic headroom | Not motivated by any CA in scope; `ScanIssue`'s single-`reason` shape does not batch naturally; adds surface area with no current consumer | Medium |

**Recommendation**: **A**. It matches CA-10/CA-12's literal framing and `ScanIssue`'s existing shape. If a later phase needs richer per-file diagnostics, this is a low-cost signature change — nothing here is an expensive precedent to revisit.

### Q3 — Non-UTF-8 path vs non-UTF-8 content: these are NOT the same case

T2's carried-forward contract (`archive-report.md:126-132`) concerns a **path** that cannot be represented as UTF-8 (Windows WTF-8 surrogates, Linux arbitrary path bytes): the rule is `path: None` plus a lossy-rendered reason, because the `PathBuf` itself is unsafe to embed in a `Location`.

T3's "non-UTF-8 file" fixture is a **different failure**: the path is a valid, readable `PathBuf` (a fixture path checked into the repo); it is the file's **byte content** that fails UTF-8 decoding. Here `path: Some(path.clone())` is correct — T3 already holds a valid path from its input argument; only the content is unreadable as text. Conflating the two would incorrectly null out a perfectly good path.

**Recommendation**: state the distinction explicitly in `design.md`. `ScanIssue.path` for a non-UTF-8-**content** failure is `Some(path)`; `ScanIssue.path` for a non-UTF-8-**path** failure (T4's concern — T3 receives an already-valid `&Path`) is `None`.

### Q4 — Dedicated struct vs reusing a domain type

| Approach | Pros | Cons | Effort |
| --- | --- | --- | --- |
| **A. Dedicated `SkillFrontmatter { name: String, description: String }`** (no `TS`/`Serialize` derives — never crosses IPC directly) | Scoped exactly to what T3 knows; leaves `kind`, `scope`, `locations`, `id`, `provenance_hint` assembly to T4 where that knowledge lives; avoids polluting `model/`, reserved for IPC-crossing zero-I/O types (`model/mod.rs:8-15`) | One more small type in the crate | Low |
| **B. Reuse `model::Component` as T3's return type** | Nothing new to define | Forces T3 to fabricate placeholders for fields it has no authority over (`id`, `scope`, `locations`) or make them `Option`, degrading `model/`'s pure IPC contract; breaks the "T4 assembles the `Component`" boundary | — |

**Recommendation**: **A**. A small T3-local DTO, living alongside the frontmatter module rather than in `model/` — it is a core-internal reader artifact, not a domain or IPC type.

### Q5 — Skill-specific function vs generic reusable reader (forward-looking for T5)

- **A. `read_skill_frontmatter(path: &Path) -> Result<SkillFrontmatter, ScanIssue>`** — skill-specific from day one; T5 must either duplicate the read/split plumbing for its agent-shaped struct or refactor an already-shipped API.
- **B. `read_frontmatter<T: DeserializeOwned>(path: &Path) -> Result<T, ScanIssue>`** — generic over the deserialization target, with `SkillFrontmatter` as T3's only current instantiation; T5 supplies its own `AgentFrontmatter { name, description, model, tools }` and reuses the read/split/error-conversion logic unchanged.

**Recommendation**: **B**. The generic form costs nothing extra now — the split/read/error-conversion logic does not depend on the target type — and it directly satisfies T5's stated reuse dependency instead of deferring a refactor. Flagged as Open Question 2 since it changes the public API shape T3 ships.

## 6. Risks

| # | Risk | Impact / Likelihood | Evidence |
| --- | --- | --- | --- |
| 1 | Committing genuinely non-UTF-8 bytes to git is fragile without an explicit `.gitattributes` entry; none exists in this repo today | Medium / Medium-High (Windows dev machines commonly default `core.autocrlf=true`; CI runs a Windows leg) | No `.gitattributes` in the repo. Silent CRLF or byte mangling of this one fixture would surface as a Windows-only test flake that is hard to diagnose. |
| 2 | `serde_norway`'s behavior on a genuine type mismatch (`name` given as a YAML list, not a scalar) is asserted by the plan text but not empirically pinned by any existing test | Medium / Low-Medium | `crates/vertice-core/tests/yaml_behavior.rs:1-93` pins bareword and duplicate-key behavior; no test exercises a scalar field fed a non-scalar. Must be verified by a T3 test, not assumed. |
| 3 | A hand-rolled splitter must reject an unterminated opening fence without panicking on an out-of-bounds slice or silently treating the rest of the file as YAML | Medium / Low if TDD'd deliberately | The plan's fixture list (`plan-desarrollo-poc.md:99`) names 6 cases; "unterminated fence" is not one, so it could be missed without deliberate test authorship. |
| 4 | If T3 ships a skill-specific-only function (Q5 Option A), T5 either duplicates plumbing or forces a refactor of an already-shipped, already-tested public API | Low-Medium / Medium if unaddressed | `plan-desarrollo-poc.md:141` states T5 reuses T3's frontmatter reader explicitly. |
| 5 | `yaml.rs`'s "only module allowed to import `serde_norway`" invariant is documentation-only, not mechanically enforced; a sibling module could violate it by accident | Low / Low, but cheap to break with both modules side by side | `crates/vertice-core/src/yaml.rs:1-9` — invariant stated in a doc comment; no CI check or `deny.toml` rule enforces it at module level. |
| 6 | Introducing a new dependency (Q1 Option B) would re-trigger T1/T2-style MSRV and license verification | Low / Low, given Option A is recommended | `deny.toml:1-67`, `openspec/specs/workspace-architecture/spec.md:59-74` — the same governance pattern T1 applied to the YAML crate choice. |

## 7. Recommended Scope Boundary

**In T3**:

- A new module (e.g. `frontmatter.rs`) housing the hand-rolled, regex-free `---` fence splitter, kept structurally separate from `yaml.rs`.
- A reader function that reads file bytes → validates UTF-8 content → splits frontmatter → delegates YAML parsing to `yaml::from_str` → converts any `YamlError` into `ScanIssue.reason` → returns `Result<SkillFrontmatter, ScanIssue>` (or its generic equivalent per Q5) for a single given path.
- A dedicated `SkillFrontmatter { name: String, description: String }` DTO, not derived for TS/IPC, not living in `model/`.
- Six versioned fixtures matching the plan's list, plus two recommended additions (§8) closing Risks 2 and 3.
- Fixture-first TDD tests proving CA-10 and CA-12-partial, plus the non-UTF-8-content distinction from Q3.

**Deferred to T4+**:

- Directory walking and root discovery (`~/.claude/skills/`, `~/.agents/skills/`, `~/.config/opencode/skills/`) — T4.
- Plugin-skill exclusion, project-scope exclusion — T4.
- `_shared` handling, alias path handling (`opencode/skill/` singular) — T4.
- Duplicate consolidation across roots — T8.
- `ScanReport`/`ScanIssue` aggregation across adapters and the "one bad adapter does not abort the scan" guarantee — T9.
- IPC exposure and Tauri commands — T10.
- Agent-specific frontmatter fields (`model`, `tools`) — T5, though T3's design should leave the generalization door open per Q5.

## 8. Fixture Inventory

Proposed layout: `crates/vertice-core/tests/fixtures/skill-frontmatter/<case-name>/SKILL.md` — one directory per case, mirroring the real skill-folder shape T4 will later walk.

| # | Fixture | Proves | CA |
| --- | --- | --- | --- |
| 1 | Normal frontmatter (`name`, single-line `description`) | Baseline happy path | Foundational (comparison basis for CA-10) |
| 2 | Multi-line frontmatter (`description: >` folded block scalar) | Complete, correct multi-line description | **CA-10** |
| 3 | Absent frontmatter (Markdown body, no `---` fence at all) | `ScanIssue`, not panic, on missing frontmatter | CA-12 ("missing fields") |
| 4 | Corrupt YAML (malformed content inside the fence) | `ScanIssue` with path, no interruption | **CA-12 (partial)** |
| 5 | Empty file (zero bytes) | Splitter handles zero-length input without panicking — distinct from #3 (no content at all vs content with no fence) | Non-panic guarantee |
| 6 | Non-UTF-8 file content | `ScanIssue` distinct from T2's non-UTF-8-*path* contract (`path: Some(path)`, not `None`) | Q3 distinction; needs `.gitattributes` handling (Risk 1) |
| 7 *(recommended addition)* | Unexpected type (`name` as a YAML list) | Closes Risk 2 — pins `serde_norway`'s type-mismatch behavior | Plan's "tipos inesperados" text |
| 8 *(recommended addition)* | Unterminated opening fence (`---` opens, EOF before the closing `---`) | Closes Risk 3 | Non-panic guarantee |

**Fixture #6 needs explicit care**: add a `.gitattributes` entry (the repo has none) marking that file `-text binary`, or `-text` scoped narrowly, so `core.autocrlf` and Git's binary-content heuristics do not silently mutate the byte sequence across the Windows/macOS/Linux CI matrix. This must be a deliberate decision recorded in `design.md`, not implicit reliance on Git defaults.

## 9. Open Questions for `sdd-propose` / `sdd-design`

1. Exact module and function names (`frontmatter.rs` vs `reader.rs`; names for the split and read functions).
2. Whether T3 ships the generic `read_frontmatter<T: DeserializeOwned>(path) -> Result<T, ScanIssue>` now (Q5 Option B) or a skill-specific function only, deferring generalization to T5.
3. `ScanIssue.severity` (`Warning` vs `Error`) per failure class — I/O error, non-UTF-8 content, absent frontmatter, corrupt YAML — is unspecified by the plan and needs an explicit decision; `IssueSeverity` is documented as a display/triage signal, not a technical constraint (`openspec/specs/domain-model/spec.md:83-97`).
4. Whether to add fixtures #7 and #8 beyond the plan's literal six, given the plan's acceptance text implies type-mismatch coverage.
5. `.gitattributes` strategy for fixture #6 — must be settled before or during apply.
6. Whether an absent `description` (name present, description missing) differs in severity from a fully corrupt file, or whether the single-`Result`/single-`ScanIssue` model treats both identically as "this file failed to parse".
7. Exact fixture directory convention (`tests/fixtures/skill-frontmatter/...` vs a shared `tests/fixtures/skills/...` layout anticipating T4 reuse) — worth settling once, since T4 will want overlapping or adjacent fixtures.

## 10. Ready for Proposal

**Yes.** The seven open questions above are implementation-shape decisions for `sdd-design` to resolve explicitly; none of them is scope-changing, and none blocks writing the proposal.
