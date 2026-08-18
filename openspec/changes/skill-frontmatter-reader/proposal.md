# Proposal: Frontmatter and `SKILL.md` Reader

> Plan trace: **T3** (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md:92-107`.
> Acceptance criteria: closes **CA-10** (multi-line `description: >` returns complete and correct) and **CA-12 (partial)** (a corrupt file yields a `ScanIssue` carrying its path and interrupts nothing). All tests run on repository fixtures, never machine-dependent paths.

## Intent

T4–T7 all need to turn one file on disk into typed fields, and none of them can start without that primitive. T3 builds it once, at leaf level: `&Path` in → `SkillFrontmatter` or `ScanIssue` out. Today `vertice-core` has never touched the filesystem — `model/` is I/O-free by invariant and `yaml.rs` only accepts an already-isolated `&str`. Nothing splits a `---` fence, nothing converts a `YamlError` into a `ScanIssue`, and no fixture exists in the repository at all. The plan forbids solving the split with a regular expression: a folded block scalar (`description: >`) is exactly what a regex truncates (finding 7). T3 is also where the leaf-level non-panic guarantee is established — every documented failure class becomes a value, never an abort.

## Scope

### In Scope

- A new `vertice-core` module owning `---`-fence splitting: hand-rolled, line-based, regex-free, dependency-free. It delegates YAML deserialization to `yaml::from_str` and MUST NOT import `serde_norway`, preserving that module's stated single-owner invariant (`crates/vertice-core/src/yaml.rs:1-9`).
- A **generic** reader, `read_frontmatter<T: DeserializeOwned>(path: &Path) -> Result<T, ScanIssue>` (naming to be fixed in design): read bytes → validate UTF-8 content → split fence → deserialize → convert `YamlError` into `ScanIssue.reason`.
- `SkillFrontmatter { name: String, description: Option<String> }` as T3's only instantiation — a core-internal DTO, no `TS`/IPC derives, not in `model/`. Field optionality mirrors the merged T2 model (see Approach).
- Ten versioned fixtures under `crates/vertice-core/tests/fixtures/` (first fixtures in the repository): the plan's six, plus type-mismatch, unterminated-opening-fence, absent-`name`, and absent-`description` (the last one proving a *successful* read carrying `None`, not a failure). The I/O-failure class deliberately has no fixture — it is exercised through a repository-relative path that does not exist on disk.
- A `.gitattributes` entry (the repository has none) pinning the byte content of the non-UTF-8 fixture across the three-platform CI matrix.
- Fixture-first TDD tests proving CA-10, CA-12-partial, and the non-panic guarantee per failure class.

### Out of Scope

- Directory walking, root discovery, plugin/project exclusion, `_shared` and alias handling — **T4**.
- Duplicate consolidation across roots — **T8**.
- `ScanReport` aggregation and the "one bad adapter does not abort the scan" guarantee — **T9**. T3's contribution is the leaf-level non-panic guarantee only.
- `Component` assembly (`id`, `kind`, `scope`, `locations`, `provenance_hint`) — **T4**.
- Agent-specific fields (`model`, `tools`) — **T5**, which plugs its own type into the generic reader.
- IPC exposure and Tauri commands — **T10**. No frontend surface in T3.
- Any write operation, MCP support, or project scope. No new dependency, so no `deny.toml` or license-gate delta.

## Capabilities

### New Capabilities

- `frontmatter-reader`: single-file frontmatter reading — fence splitting, UTF-8 content validation, YAML delegation, failure-class-to-`ScanIssue` conversion, and the non-panic guarantee.

### Modified Capabilities

None. T3 consumes `ScanIssue`/`IssueSeverity` as merged in T2; it changes no existing requirement.

## Approach

**A new module, not an extension of `yaml.rs`.** Fence splitting is a text operation that never touches `serde_norway`. Putting it in `yaml.rs` would dilute that module's role as the crate-swap seam and blur which module is allowed to import the YAML crate. Splitting also happens *before* parsing, so an indented block scalar's content can never be mistaken for a fence.

**Hand-rolled splitter, no third-party crate.** ~15-20 lines of line-based logic against re-triggering T1-style crate governance (maintenance, license, MSRV) for a supply-chain cost out of all proportion. `str::lines()` already normalizes CRLF, matching the precedent pinned in `tests/yaml_behavior.rs:69-75`.

**Generic from day one.** `plan-desarrollo-poc.md:141` states T5 reuses T3's frontmatter reader. The read/split/error-conversion logic does not depend on the target type, so the generic form costs nothing now and spares T5 either duplicating the plumbing or refactoring an already-shipped, already-tested public API.

**`Result<T, ScanIssue>`, one outcome per file** — not a partial result plus an issue list. It matches the plan's singular phrasing ("a corrupt file returns **a** `ScanIssue`") and `ScanIssue`'s own single `reason: String` shape. No criterion in scope needs two independent diagnostics from one file.

**Required versus optional fields follow the merged T2 model, not symmetry.** `name` is mandatory and `description` is optional, because `Component.name` is `String` while `Component.description` is `Option<String>` (`crates/vertice-core/src/model/component.rs:21-23`). T2 already ruled that a component with no description is a legitimate component; identity, by contrast, is derived from the name (`identity::normalize_name`), so a component without one cannot exist. Making `description` mandatory in `SkillFrontmatter` would contradict that merged decision and make a skill whose `description` key is absent or misspelled vanish from the inventory entirely rather than appear with `None`. The blast radius is not local: **CA-2** requires consolidating to exactly 25 skills, so one real skill lacking a description would break the count and the failure would be attributed to T8's consolidation rather than to T3's reader. Accordingly: absent `name` → `ScanIssue`; absent `description` → successful read carrying `None`; a wrong *type* on either field (a list or mapping where a scalar belongs) → `ScanIssue`, which is what the plan means by "unexpected types produce a `ScanIssue`, not a panic".

**Non-UTF-8 *content* is not non-UTF-8 *path*.** T2's carried-forward contract (`path: None` plus a lossy rendering) covers a path that cannot be represented as UTF-8 — a T4 concern, since T3 receives an already-valid `&Path`. When the file's *bytes* fail to decode, the path is perfectly good and the issue MUST carry `path: Some(path)`. Conflating the two would null out a valid path and degrade CA-12's "carrying its path" requirement.

**Two coverage gaps closed beyond the plan's literal six fixtures.** `serde_norway`'s behavior on a scalar field fed a YAML list is asserted by the plan's text ("unexpected types produce a `ScanIssue`, not a panic") but pinned by no test in `tests/yaml_behavior.rs`. An unterminated opening fence is absent from the plan's fixture list yet is exactly where a hand-rolled splitter panics on an out-of-bounds slice. Both are non-panic guarantees the plan already implies; asserting them is cheaper than discovering them in T4.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/lib.rs` | Modified | Declare the new module |
| `crates/vertice-core/src/<new module>` | New | Fence splitter, reader, `SkillFrontmatter` |
| `crates/vertice-core/src/yaml.rs` | Unchanged | Consumed as-is; invariant preserved |
| `crates/vertice-core/src/model/` | Unchanged | `ScanIssue`/`IssueSeverity` consumed, not modified |
| `crates/vertice-core/tests/` | New | Fixture-driven reader tests |
| `crates/vertice-core/tests/fixtures/` | New | First fixtures in the repository (10 cases) |
| `.gitattributes` | New | Byte-integrity pin for the non-UTF-8 fixture |
| `openspec/specs/frontmatter-reader/` | New | Capability spec |
| `vertice-app`, `frontend/`, `deny.toml`, CI | Unchanged | No IPC, no dependency, no gate change |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Non-UTF-8 fixture silently mangled by `core.autocrlf` on the Windows CI leg | Med-High | Explicit `.gitattributes` entry decided in design; a test asserting the fixture's exact byte length fails loudly if Git rewrote it |
| `serde_norway` type-mismatch behavior assumed rather than verified | Low-Med | Fixture #7 pins it empirically; if it panics rather than erroring, the reader must guard before the seam — discovered at RED, not at T4 |
| Hand-rolled splitter panics on an unterminated fence or zero-length input | Low if TDD'd | Fixtures #5 and #8 are written before the splitter; strict TDD is enabled for this project |
| `yaml.rs`'s single-importer invariant is documentation-only, easy to break with a sibling module beside it | Low | Design records the constraint; apply keeps `serde_norway` out of the new module's imports |
| Fixture layout chosen for T3 alone forces a reshuffle when T4 needs overlapping fixtures | Low-Med | Settle the convention once in design, with T4's directory walking in view |

## Open Questions

Deferred to `sdd-design`, deliberately, not silently: exact module and function names; `ScanIssue.severity` per failure class (I/O error, non-UTF-8 content, absent frontmatter, corrupt YAML), which the plan leaves unspecified; whether `ScanIssue.reason` is user-facing copy subject to the i18n-from-first-commit principle or a developer diagnostic that T11 re-renders, which decides whether raw `serde_norway` error text may be embedded; the exact `.gitattributes` pattern; and the fixture directory convention (T3-scoped versus a layout anticipating T4 reuse).

Resolved here, not deferred: whether an absent `description` invalidates the file. It does not — see Approach. That question is closed because the merged T2 model already answers it.

## Rollback Plan

Purely additive; no consumer exists yet.

- **Core**: delete the new module, its tests, and `tests/fixtures/`; revert one `pub mod` line in `lib.rs`. `yaml.rs` and `model/` are read-only inputs and are untouched by the revert.
- **App (`vertice-app`)**: zero impact — no command registered, no type imported.
- **Frontend**: zero impact — no generated binding changes, no IPC surface.
- **CI / supply chain**: no dependency added, so `deny.toml` and the license gate are unaffected. Only `.gitattributes` survives as a repository-level artifact; removing it is independently safe once the fixture is gone.

Reverting the branch restores the exact post-T2 state. If the generic signature proves wrong under T5, the blast radius is one function signature plus its call sites inside the crate — no public IPC contract and no persisted data depend on it.

## Dependencies

- **T1** (workspace, CI, `serde_norway` seam) — complete and archived.
- **T2** (`ScanIssue`, `IssueSeverity`) — complete and archived; `domain-model` spec merged.
- **Blocks**: T4 (skill scanner), T5 (Claude Code agents, which reuses this reader).

## Success Criteria

- [x] A fixture with a folded `description: >` returns the complete, correct description (**CA-10**).
- [x] A corrupt-YAML fixture returns a `ScanIssue` whose `path` is `Some(fixture path)` and whose `reason` carries the parse failure (**CA-12 partial**).
- [x] Every failing fixture — absent frontmatter, empty file, non-UTF-8 content, type mismatch, unterminated fence, absent `name` — returns a `ScanIssue`; no input panics.
- [x] A fixture whose frontmatter has `name` but no `description` returns `Ok`, with `description == None`, not a `ScanIssue` — matching `Component.description: Option<String>` (`crates/vertice-core/src/model/component.rs:23`) so a description-less skill still reaches the T8 consolidation count required by **CA-2**.
- [x] A non-UTF-8-**content** failure carries `path: Some(path)`, never `None`; the distinction from T2's non-UTF-8-**path** contract is written in `design.md`.
- [x] The reader is instantiated in a test with a second, non-skill target type, proving T5's reuse path without a refactor.
- [x] The new module contains no `serde_norway` import and no regular expression.
- [x] All tests use repository fixtures; no test reads a path outside the repository.
- [ ] `cargo test` passes on all three CI platforms with no dependency, `deny.toml`, or CI-workflow change. **Pending**: verified locally on Windows only (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p vertice-core --locked` — all green, 60 tests). The macOS and Linux legs cannot be confirmed until CI runs on the pushed branch. The no-dependency and no-CI-change half of this criterion IS confirmed: `Cargo.toml`, `deny.toml`, and `.github/workflows/ci.yml` are untouched.
