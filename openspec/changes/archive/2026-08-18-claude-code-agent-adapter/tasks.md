# Tasks: Claude Code Agent Adapter

> Trace: **T5** (Phase 1 — Reading, `plan-desarrollo-poc.md:132-147`) / closes **CA-5 partial** (17 on-disk reference agents appear over equivalent fixtures), **CA-13 core half** (embedded components are marked and distinguishable by `origin`+`path` alone); contributes to **CA-12 partial** (corrupt file carries its path, scan continues); bound by **CA-16** (read-only) and **CA-17** (fixture-based, three-platform tests). Closes open decision 2 of `plan-desarrollo-poc.md:387`.
> Design: `openspec/changes/2026-08-18-claude-code-agent-adapter/design.md`. Inherits T3's fixture-per-case pattern and T4's PR-boundary precedent (`archive/2026-08-18-skill-scanner-user-roots/tasks.md`).
> Core-only — no IPC, no Tauri command, no frontend source change. `src/model/` and `frontend/src/bindings/` are **unchanged**; no `npm run` gate is exercised by this change beyond the existing regression build.
> `strict_tdd: true`. Fixtures and failing tests land before the implementation that turns them green, honoring design §12's rejection of a literal "tests first, implementation second" PR split: a test naming a not-yet-existing module fails to *compile*, which is a poor RED. TDD ordering applies **within** each PR (red/green per behavior), never as the PR boundary.
> Environment note: `cargo` may not be on PATH in this environment. Do not claim gates pass; only define them as tasks and report their real environment status.

## Work Units (design §12 chained-PR seam)

| Unit | Goal | PR | Base | Notes |
|------|------|----|------|-------|
| 1 | `roots::agent_roots`, the `kind` parameter on private `resolve_single`, its unit tests, and the whole `tests/fixtures/roots/agents/**` tree incl. the `.gitkeep` tripwire's disk-existence half | PR 1 (~180-260 lines) | `main` | No `agents.rs` yet. Every test in this unit compiles and passes standalone — it is the piece T4's regression suite is most exposed to. |
| 2 | `crates/vertice-core/src/agents.rs`, `lib.rs` wiring, `tests/agent_scanner.rs` full RED→GREEN suite | PR 2 (~320-460 lines) | PR 1 branch | Code and the tests that justify it travel together, per T3/T4 precedent. RED-before-GREEN preserved by commit order inside the PR, not by the PR boundary. |

## Phase 1: `roots.rs` and Fixture Tree (PR 1)

- [x] 1.1 [RED] In `crates/vertice-core/src/roots.rs`, add `#[cfg(test)]` unit tests for `agent_roots`: it returns exactly `[ResolvedRoot; 2]` for any `home`; the first `ResolvedRoot`'s id is the hardcoded `SearchRootId("claude-agents")` with `scan_paths` pointing at `<home>/.claude/agents`; the second's id is the hardcoded `SearchRootId("claude-embedded-agents")`, `path: <home>/.claude`, and `scan_paths: vec![]` (empty — probed, never walked); both ids are never path-derived. No disk access — in-memory assertions on the returned values. — *agent-scanner spec: "Agent Root Resolves Under The Home Directory"*
- [x] 1.2 [GREEN] Add `pub fn agent_roots(home: &Path) -> [ResolvedRoot; 2]` to `roots.rs`, calling the existing (still-private) `resolve_single` twice — once for `("claude-agents", SearchRootKind::Agent, [".claude", "agents"])`, once for `("claude-embedded-agents", SearchRootKind::Agent, [".claude"])` with an empty `scan_paths` override for the second. Pass 1.1.
- [x] 1.3 [GREEN] Give the still-private `resolve_single` a `kind: SearchRootKind` parameter (design §5.1: visibility is **unchanged**, still private — do **not** widen to `pub(crate)`). Update its two existing in-file call sites (the three skill roots) to pass `SearchRootKind::Skill` explicitly.
- [x] 1.4 [REFACTOR] Confirm `agent_roots` is the only new `pub` item in `roots.rs`; `resolve_single` and `probe` stay private; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [x] 1.5 Run `cargo test -p vertice-core --locked`; confirm T4's existing `roots.rs` unit suite (skill-root resolution) is still green, changed only by the added `SearchRootKind::Skill` arguments at its call sites — the regression guard design §5.1 names explicitly.
- [x] 1.6 Create the semantic fixture set under `crates/vertice-core/tests/fixtures/roots/agents/` (design §10), one synthetic home per directory, paths built via `env!("CARGO_MANIFEST_DIR")` + per-segment `push`, never `"/"`-joined literals:
  - `absent-root/` — `.gitkeep` only (no `.claude` at all)
  - `empty-root/` — `.claude/agents/.gitkeep`
  - `tools-scalar/` — `.claude/agents/reviewer.md` (`tools: Read, Grep, Glob, Bash`, `model: sonnet`)
  - `folded-description/` — `.claude/agents/summarizer.md` (`description: >`, multi-line)
  - `missing-optional/` — `.claude/agents/minimal.md` (`name` + `description` only, no `model`, no `tools`)
  - `broken-frontmatter/` — `.claude/agents/good.md` + `.claude/agents/broken.md` (a deliberate **copy** of a corrupt shape, never a reference into `fixtures/frontmatter/` or T4's `fixtures/roots/`)
  - `nested-decoy/` — `.claude/agents/flat.md` + `.claude/agents/group/nested.md`
  - `non-agent-entries/` — `.claude/agents/real.md` + `.claude/agents/notes.txt` + `.claude/agents/.DS_Store` + `.claude/agents/subdir/.gitkeep`
  - `shadowing/` — `.claude/agents/Plan.md` (`name: Plan`)
  - `reference/` — `.claude/agents/<17 files>.md`, content generated by one rule (`name`, one-line `description`, `model: sonnet`, `tools: Read, Grep, Glob, Bash`), no name colliding with any of the six embedded agents
- [x] 1.7 Write the disk-existence half of the `.gitkeep` tripwire: `empty_agent_root_fixture_directory_still_exists_on_disk`, asserting via `std::fs::metadata` that `empty-root/.claude/agents/` exists before any scanner code runs. This half needs no `agents.rs` and is GREEN in PR 1. (No status-assertion half exists for this fixture in PR 2, unlike T4's alias fixture — `empty-root`'s `SearchRootStatus::Found` is proven directly by 2.3's integration suite.)
- [x] 1.8 Confirm `.gitattributes` needs no change — the existing `-text` scope on `crates/vertice-core/tests/fixtures/**` already covers `roots/agents/`.
- [x] 1.9 **[Gate, PR 1]** `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked` (all green, including 1.1's `roots.rs` unit tests, 1.5's regression check, and 1.7's tripwire half). Confirm `git diff --exit-code -- frontend/src/bindings` is clean — PR 1 introduces zero `TS`-derived types. Report results; do not claim they ran if `cargo` is unavailable in the executing environment.

## Phase 2: `agents.rs` Module — TDD (RED → GREEN) (PR 2)

- [x] 2.1 [RED] In `crates/vertice-core/src/agents.rs` (new), write `#[cfg(test)]` unit tests: `escalate` maps every T3 `ScanIssue` severity class to `IssueSeverity::Error` uniformly, with `path`/`reason` untouched; the non-UTF-8 path→lossy-string conversion helper (`#[cfg(unix)]`-gated, per T4's precedent — no portable fixture exists for this case). No disk access.
- [x] 2.2 [GREEN] Implement `AgentFrontmatter { name: String, description: Option<String>, model: Option<String>, tools: Option<String> }` (`Deserialize`-only, no `Serialize`/`TS`) — `tools` MUST be typed `Option<String>`, never `Option<Vec<String>>`, matching the verified real-world comma-separated scalar (`tools: Read, Grep, Glob, Bash`). Implement `fn escalate(issue: ScanIssue) -> ScanIssue` and the UTF-8 path guard, structurally mirroring `skills::escalate` / `skills::ensure_utf8_path` (duplicated per design §5.4, not shared). Pass 2.1.
- [x] 2.3 [RED] Create `crates/vertice-core/tests/agent_scanner.rs`. One test (or tight group) per `agent-scanner` spec requirement, each pointed at its own synthetic-home fixture from Phase 1:
  - the agent root resolves to `<home>/.claude/agents/` with `kind: SearchRootKind::Agent`, never an OS config-dir path
  - a `.md` file directly under the root is discovered (`tools-scalar/`)
  - a `.md` file nested one level under the root is **not** discovered, and no `ScanIssue` references it (`nested-decoy/`)
  - a non-`.md` file directly under the root is silently ignored — no `Component`, no `ScanIssue` (`non-agent-entries/`)
  - `absent-root/` yields zero `origin == File` components and zero issues; `empty-root/` yields zero `origin == File` components and zero issues; the two roots' `SearchRootStatus` values are distinguishable from each other — assert by filtering `origin == File`, **never** by `components.is_empty()` (design §4, spec requirement "Absent and Empty Agent Roots...")
  - `tools-scalar/` deserializes `tools: Read, Grep, Glob, Bash` into one `String`, not a list
  - `missing-optional/` returns `Ok` with `model == None` and `tools == None`, and still produces a `Component`
  - `folded-description/`'s `description: >` block returns the complete, un-truncated description (CA-10 inherited)
  - a valid on-disk agent produces one `Component { kind: Agent, scope: User, locations: [Location { path: Some(_), origin: File, root: "claude-agents" }] }`
  - `empty-root/` yields exactly six components with `origin: Embedded, path: None` even though `.claude/agents/` exists and is empty (embedded gated on `<home>/.claude`, not on the agent root — design §4)
  - `absent-root/` yields **zero** components, embedded or on-disk, and zero issues (no `<home>/.claude` at all — design §4, spec scenario "No embedded agents are emitted when the client directory is absent")
  - embedded and on-disk components are distinguishable by `origin`+`path` alone, no name heuristic
  - every embedded component's `Location.root` holds a valid, well-formed `SearchRootId` (never omitted, never panic-inducing)
  - `shadowing/` yields two components both deriving `ComponentId` from `(Agent, "Plan")` — one `origin: Embedded, path: None`, one `origin: File, path: Some(_)` — neither replaces the other
  - `broken-frontmatter/` yields one `ScanIssue` at `IssueSeverity::Error` carrying `broken.md`'s path, and `good.md` is still discovered as a `Component` (CA-12 partial)
  - a non-UTF-8 discovered path yields one `ScanIssue { severity: Error, path: None, reason: <lossy> }` and the walk continues to sibling entries
  - a full scan over `reference/` leaves the fixture tree byte-for-byte unchanged (CA-16, read-only)
  - `reference/` yields exactly 17 on-disk (`origin: File`) components, and 23 total components (17 + 6 embedded) with 23 distinct `ComponentId`s (CA-5 partial)
  - component order is identical regardless of `read_dir`'s OS-dependent yield order — assert against a fixture with 3+ files, run against the sorted expectation, not incidental filesystem order
- [x] 2.4 [GREEN] In `crates/vertice-core/src/agents.rs`: `pub struct AgentScan { pub roots: Vec<SearchRoot>, pub components: Vec<Component>, pub issues: Vec<ScanIssue> }` (no `Serialize`/`TS`) and `pub fn scan(home: &Path) -> AgentScan`, calling `roots::agent_roots(home)`. **Collect `std::fs::read_dir` entries into a `Vec` and sort by file name before parsing** — `read_dir` yields OS-dependent order, unlike `walkdir`'s `sort_by_file_name()` (design §6); without this, component order diverges between the Linux and Windows CI legs. Filter to `entry.file_type()?.is_file() && path.extension() == Some("md")` (exact, lowercase, case-sensitive), skip everything else silently, never recurse into subdirectories. Emit file-backed components first (sorted), then the six embedded in const declaration order. Pass 2.3.
- [x] 2.5 [GREEN] Add `const EMBEDDED_CLAUDE_AGENTS: [&str; 6] = ["Explore", "Plan", "general-purpose", "statusline-setup", "claude", "claude-code-guide"];` with a doc comment stating its provenance (`claude agents` on the reference machine) and verification date (2026-08-18). Gate emission on the `claude-embedded-agents` root's probed `SearchRootStatus == Found` — i.e., iff `<home>/.claude` exists — never unconditionally. Each embedded `Component` gets `description: None`, `provenance_hint: None`, one `Location { path: None, origin: Embedded, root: "claude-embedded-agents" }`.
- [x] 2.6 [GREEN] Wire the `ScanIssue` taxonomy per design §8's table for the two rows that diverge from T4's `walkdir`-based one: `read_dir` itself failing after a successful root probe (`path: Some(root)`, `reason: "could not read search root: {io}"`) and a failing iterator item (`path: Some(root)` — **not** `None`; a bare `io::Error` from `read_dir` carries no `DirEntry` and therefore no path, so `None` stays reserved for the non-UTF-8-path case). Root probe `NotFound` → no issue, `status: NotFound`; any other root-probe `io::Error` or a root that exists but is not a directory → `Error` issue with `path: Some(root)`.
- [x] 2.7 Wire `pub mod agents;` in `crates/vertice-core/src/lib.rs` — one line, no crate-root re-export, matching `pub mod model; pub mod roots; pub mod skills;`.
- [x] 2.8 [REFACTOR] Confirm `escalate`, the UTF-8 guard, and any walk-internal helpers stay private; only `AgentScan`, `scan`, `AgentFrontmatter` are `pub`; `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Phase 3: Verification (local, pre-commit gates)

- [x] 3.1 `cargo fmt --all --check`.
- [x] 3.2 `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 3.3 `cargo test --workspace --locked` — Phase 1 and Phase 2 suites, in-module units, all green, including T4's `tests/skill_scanner.rs` and `roots.rs` unit suite staying green untouched (design §12 regression row).
- [x] 3.4 `cargo deny check bans licenses` — expected unaffected; no new dependency (`std::fs::read_dir`, no `walkdir`), so this is a straight regression check, not a contingency.
- [x] 3.5 **Read-only grep (CA-16, `rules.apply`)**: confirm no `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, or `remove_*` anywhere in `agents.rs`, the `roots.rs` diff, or `tests/agent_scanner.rs`.
- [x] 3.6 **Domain-model and bindings invariant** (this change's load-bearing property, design §2): confirm `git diff --exit-code -- crates/vertice-core/src/model` and `git diff --exit-code -- frontend/src/bindings` are **both clean** — zero lines changed in either. Do not treat "no diff expected" as an assumption; run the diff and record its output.
- [x] 3.7 **YAML seam invariant**: re-run `tests/yaml_seam_invariant.rs` (T3-authored) and confirm `agents.rs` contains no `use serde_norway` / `serde_norway::` — the module consumes `frontmatter::read<T>` only.
- [x] 3.8 Confirm no regular expression and no `walkdir` import appear anywhere in `agents.rs` (structural checks — grep for `walkdir::` and `regex::`/`Regex::`).
- [x] 3.9 **Platform note**: fixtures run on all three CI platforms via the existing matrix automatically. Windows is the only platform this cycle's reasoning is verified against (per-OS path table, design §11); macOS/Linux path revalidation, case-sensitive `.md` matching, and the shadowing-precedence question against the real client are explicitly deferred to **T16** — no manual system verification is required here beyond noting that deferral.
- [x] 3.10 From `frontend/`: `npm run lint && npm run check && npm run test && npm run build` — regression check only; this change adds no consumer of any binding and no new binding exists to consume.
