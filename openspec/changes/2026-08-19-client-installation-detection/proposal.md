# Proposal: Client Installation Detection

> Plan trace: **T7** (Phase 1 — Reading) of `internal-docs/plan-desarrollo-poc.md:171-187`.
> Acceptance criteria: **CA-7** — "the two Claude Code installations are detected separately, each with its version" (`alcance-poc-vertice.md:166`) — and **CA-11** — "an absent client is reported as *not detected*, not as an error and not as an unexplained empty list" (`alcance-poc-vertice.md:170`). Bound by **CA-16** (read-only) and **CA-17** (versioned fixtures, three platforms). Carries forward the hard rule of `plan-desarrollo-poc.md:179` (the scanner never derives foreign paths from OS conventions).

## Intent

Every adapter shipped so far answers *what components are installed*. None answers *what clients are installed, and in which version*. `ClientInstallation` and `ClientKind` were merged in T2 (`crates/vertice-core/src/model/installation.rs:14`, `:27`) and `ScanReport.installations` exists (`model/report.rs:22`), but **no code has ever produced a single value of that type**. T7 is the phase that makes the client half of the inventory real.

T7 is also the first phase whose product question is **absence**, not presence. `alcance-poc-vertice.md:104` records why CA-7 matters: the two Claude Code installations share the same user roots and therefore see the same components — the *only* thing that distinguishes them in the PoC is the version, and a user running two different versions of the same client has the right to know. And CA-11 forbids the shape the crate produces today: a client with zero detected installations is simply absent from `installations`, indistinguishable from "never checked" and from "the probe failed". An inventory tool that cannot say *"Claude Code is not installed"* out loud is guessing on the user's behalf.

Verified this cycle: **the T2 model needs no change.** `ClientInstallation`, `ClientKind` and `ScanReport.installations` are merged and sufficient for the detected case. The only open modelling question is how *absence* is expressed — see Open Decisions.

## Scope

### In Scope

- A new `vertice-core` module (working name `installations`) mirroring the shape T4/T5/T6 settled: `scan(home: &Path) -> InstallationScan { installations, issues }`, infallible, `home` passed in so no test reads the author's machine.
- **Three Windows detection slots**, each probed and reported independently:
  | Client | Install kind | Path (relative to `home`) | Version source |
  |---|---|---|---|
  | Claude Code | npm | `AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/` | `package.json` → `"version"` |
  | Claude Code | desktop | `AppData/Roaming/Claude/claude-code/<version>/` | the directory name itself |
  | OpenCode | npm | `AppData/Roaming/npm/node_modules/opencode-ai/` | `package.json` → `"version"` |
- **The two Claude Code slots are never merged** into one entry, even though they share the same user component roots — that is the literal content of CA-7 and of `installation.rs:8-10`'s own doc comment.
- **An explicit "not detected" signal per slot** for a client that was probed and found absent, distinguishable from a probe error and from an unexplained omission (**CA-11**). Default representation: a `ScanIssue`; see Open Decisions.
- **Version extraction through the existing `jsonc.rs` seam** for `package.json` (strict JSON is a subset of what that seam already accepts) — no second JSON dependency.
- **A platform-dispatch seam left explicit for T16**: a per-OS candidate-probe table (e.g. `windows_install_probes(home)`) feeding a shared, OS-agnostic resolver. Only the path template is platform-specific; probing, parsing and version extraction are shared.
- Versioned fixtures under a **new** tree (`crates/vertice-core/tests/fixtures/installations/`), never reusing T4/T5/T6 homes: both Claude Code installs present with different versions (CA-7); OpenCode npm present; a home where a slot's path does not exist (CA-11); `package.json` with a missing `"version"`; `package.json` malformed; a desktop directory present but carrying **no** versioned subdirectory; and — per the resolved decision below — a desktop directory carrying more than one versioned subdirectory, yielding one installation per subdirectory, never merged and never an anomaly.
- Fixture-first TDD tests for CA-7, CA-11, and per-slot failure isolation.

### Out of Scope

- **macOS and Linux installation paths.** Unverified (`plan-desarrollo-poc.md:187`). T7 ships Windows with the structure prepared; **T16** closes the other two platforms and the `~/.agents/skills/` location question.
- ~~Modelling multiple simultaneous Claude Code desktop version directories as a supported case. Resolved as impossible.~~ **Superseded by machine evidence on 2026-08-19** (see Open Decisions): this is now a supported, in-scope case, covered by the "Three Windows detection slots" bullet above.
- **Any use of `directories`/`dirs` to produce these paths.** The hard rule at `plan-desarrollo-poc.md:179` reserves OS conventions for Vertice's own app-data directory. `AppData/Roaming/npm/...` *coincides* with a convention value, but must not be produced by a convention-resolving crate.
- **Linking installations to the components they can see.** The two Claude Code installs share roots (`alcance-poc-vertice.md:104`); an installation↔component relation is not in the PoC model and is not invented here.
- Detection of clients outside the closed `ClientKind` set (Copilot, Codex, …) — outside the PoC.
- Update status, upstream comparison, "is this version current" — explicitly outside the PoC (`alcance-poc-vertice.md:13`).
- Consolidation and duplicate marking — **T8**.
- `ScanReport` assembly, `duration_ms`, the "one bad adapter does not abort the scan" orchestration — **T9**. T7 returns installations and issues, exactly as the component adapters return components and issues.
- IPC exposure, Tauri commands, any frontend surface — **T10**.
- Project scope, MCP servers, and every write operation — outside the PoC.

## Capabilities

### New Capabilities

- `client-installation-detector`: per-slot probing of the Windows Claude Code (npm + desktop) and OpenCode (npm) installation paths, version extraction from `package.json` and from the desktop version directory name, separate reporting of every installation, the explicit "not detected" state, and the platform-dispatch seam reserved for T16.

### Modified Capabilities

None expected. `domain-model` is consumed exactly as merged. `skill-scanner`, `agent-scanner` and `opencode-agent-scanner` are untouched — T7 shares no code path with them beyond the `home: &Path` convention and, if reused, the `roots::probe` helper. **If the "not detected" representation resolves to a new model type (Option B below), `domain-model` becomes a Modified Capability and `sdd-spec` must be told.**

## Approach

**Approach A (with C as its refinement), per the exploration's recommendation.** A single new adapter module, absence signalled through `ScanIssue`, no change to the frozen T2 model contract, a Windows-only path table behind an explicit platform-dispatch seam. This is the lowest-risk path to CA-7 and CA-11, and it is consistent with how the crate already solves structurally identical "explicit absence, not silent omission" problems: `SearchRootStatus` refused a third value, and `report.rs:44` already records that severity is a display/triage signal, not control flow. Option C's refinement — a compile-time-closed set of per-slot `Option<ClientInstallation>` *inside* the adapter, so a forgotten slot fails to compile — costs internal plumbing only and changes nothing externally.

**Each slot is probed independently and fails independently.** This is the T6 isolation discipline transposed: a malformed `@anthropic-ai/claude-code/package.json` must yield one `ScanIssue` and **must not** prevent the desktop Claude Code install or the OpenCode install from being detected and reported. A slot that fails to parse is a client missing from the user's inventory, so it escalates to `IssueSeverity::Error`, mirroring `skills::escalate` and `agents::escalate`. A slot whose path simply **does not exist** is the CA-11 case, not a parse failure, and must be distinguishable from it — that distinction is a specified behavior with its own fixture, not an implementation detail.

**Two version sources, one shared resolver.** The npm slots read `"version"` from `package.json`; the desktop slot reads the version from the **directory name**, which is a path-segment read, not a file read. Both feed the same `ClientInstallation` assembly. Keeping extraction OS-agnostic and confining only the path table to per-OS dispatch is what makes T16 an additive change (`macos_install_probes`, `linux_install_probes`) rather than a rewrite.

**`package.json` goes through the existing `jsonc.rs` seam — one crate, one seam.** T6 already paid the supply-chain cost of a JSON/JSONC parser and confined it to a single module the way `yaml.rs` confines `serde_norway`. `package.json` is strict JSON, a subset of what that seam accepts. **T7 adds no dependency**: `Cargo.toml`, `Cargo.lock` and `deny.toml` are expected untouched, restoring the property T6 had to break.

**No path is derived from an OS convention.** Every probe is `home` plus hardcoded relative segments, exactly as `roots.rs` does today. `vertice-core` has no `dirs`/`directories` dependency and T7 does not add one. This is also what keeps fixture assertions machine-independent (CA-17).

**Detection rule: presence of the path, no heuristics.** If the probe path exists and yields a version, the client is installed. No inference from `PATH`, no shelling out to `claude --version` or `opencode --version`, no registry read — those are machine-dependent oracles reserved for T16's manual verification (`alcance-poc-vertice.md:132`), never automated tests.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/installations.rs` (name TBD) | New | Per-OS probe table, per-slot probing, version extraction, `ClientInstallation` assembly, absence signalling |
| `crates/vertice-core/src/lib.rs` | Modified | Declare the new module |
| `crates/vertice-core/src/roots.rs` | Possibly modified (small) | `probe` may be reused / made `pub(crate)`; no change to any resolved root's id, path, kind or status |
| `crates/vertice-core/src/jsonc.rs` | Unchanged | Reused as-is for `package.json`; second caller of an existing seam |
| `crates/vertice-core/src/skills.rs`, `agents.rs`, `opencode_agents.rs`, `frontmatter.rs`, `yaml.rs` | Unchanged | No shared abstraction extracted; T7 produces installations, not components |
| `crates/vertice-core/src/model/` | **Expected unchanged** | `ClientInstallation`, `ClientKind`, `ScanIssue` merged in T2. **Changes only if Open Decision 1 resolves to Option B** |
| `frontend/src/bindings/*.ts` | **Expected unchanged** | No model edit ⇒ no regeneration; the CI drift gate should stay green untouched |
| `crates/vertice-core/tests/fixtures/installations/` | New | ~8 synthetic homes, new tree, no reuse of T4/T5/T6 |
| `crates/vertice-core/tests/` | New | Probing, version-extraction, isolation, CA-7 and CA-11 suites |
| `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Unchanged** | No new dependency; `jsonc.rs` seam reused |
| `vertice-app`, `frontend/` source | Unchanged | No IPC, no command, no capability change |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| A Windows install path is wrong ⇒ the client is silently reported "not detected" instead of "found in the wrong place" | **Med — the defining risk of this change** | Every probe reports its **concrete probed path** in the absence signal, so a wrong path presents as *a named path not found*, not as an unexplained empty result. Same structural mitigation T6 adopted for its config root. Paths pinned against the reference installation recorded at `alcance-poc-vertice.md:71` |
| The two Claude Code installs get merged into one entry (same client, "obviously" one install) | Med | A CA-7 fixture with **two different versions** written before the assembly code; it fails under any merging implementation |
| "Not detected" implemented as free-text only ⇒ the frontend must string-match to learn *which* client is missing | **Med — unresolved, see Open Decisions** | Representation closed in `sdd-design` before any code; if `ScanIssue` is kept, its `reason` format is specified, not improvised |
| macOS/Linux path shapes are unknown ⇒ the platform seam is over-designed for shapes that never arrive | Med | The seam is one function per OS returning a probe list — the smallest structure that makes T16 additive. No trait, no registry, no abstraction over unknown shapes |
| A parse failure and a missing path collapse into the same signal | Med | Distinct fixtures and distinct severities; CA-11 explicitly requires "not detected" ≠ "error" |
| Version string typed or validated as semver | Low | Carried verbatim as the `String` the model already declares; the PoC reports, it does not interpret (update status is out of scope) |
| Desktop directory present with zero versioned subdirectories | Low-Med | Fixture required; yields no installation plus one `ScanIssue`, never a phantom entry with an empty version |
| More than one desktop version directory found on disk | **Confirmed — observed on the reference machine on 2026-08-19** | Each versioned subdirectory is its own `ClientInstallation`, never merged and never an anomaly (see Open Decisions, superseded); a fixture pins the behavior |
| Reviewers read a fourth near-identical adapter as copy-paste debt | Med | Deliberate, per T5's merged decision to defer any shared abstraction to T9 at the earliest; T7's output shape (`ClientInstallation`, not `Component`) makes it the *least* extractable of the four |

## Open Decisions

**Closed in this proposal:**

- **~~Multiple Claude Code desktop version directories are not a supported case~~ (user decision, 2026-08-19) — SUPERSEDED by machine evidence on 2026-08-19.** The original entry read: "The desktop install keeps exactly one versioned subdirectory under `AppData/Roaming/Claude/claude-code/`. That single subdirectory **is** the desktop installation. If more than one is ever found on disk, it is reported as a `ScanIssue` anomaly — never modelled as N installations, never silently resolved by picking the highest version." Direct inspection of the reference machine the same day found `AppData\Roaming\Claude\claude-code\2.1.229\` and `AppData\Roaming\Claude\claude-code\2.1.234\` **coexisting**, refuting the "exactly one" premise. The corrected decision: **each versioned subdirectory is its own `ClientInstallation`**, with the directory name as its version and its own path — N subdirectories yield N installations, never merged, never an anomaly. This mirrors the contract `crates/vertice-core/src/model/installation.rs:8-10` already states for a client installed twice; the earlier decision had narrowed T7 below what the merged T2 model already supported.
- **Approach A/C, not B by default.** No change to the T2 model contract; absence via `ScanIssue`; per-slot internal closure optional.
- **No new dependency.** `package.json` reuses the `jsonc.rs` seam.
- **Windows only in T7.** macOS/Linux path tables land in T16, behind the seam this change ships.
- **No external binary is invoked.** `claude --version` / `opencode --version` are T16 manual oracles, never automated tests.
- **The probed path is reported in the absence signal**, so "looked in the wrong place" stays distinguishable from "nothing installed".

**Committed to resolving in `sdd-design` — do not guess:**

- **The exact representation of "not detected".** The proposal's default is **reuse `ScanIssue`** (exploration §3 option 1), because it costs no model change, no binding regeneration and no frontend contract change. Its weakness is real and must be weighed rather than inherited: the frontend has to cross-reference free-text `issues` to learn *which* client is missing. Option B (a closed `ClientDetectionStatus` on `ScanReport`) gives the strongest typed contract for CA-11 at the cost of a model edit, a binding regeneration and a consumer contract change; option C closes the slot set internally without touching `model/`. **`sdd-design` must close this explicitly. If it selects B, `domain-model` becomes a Modified Capability and this proposal's "bindings unchanged" property is void.**
- **The `ScanIssue` severity for a not-detected client.** `Warning` (nothing is broken; the client simply is not installed) versus `Error`. These are different product statements and must not be decided by habit.
- **The `reason` string format**, if `ScanIssue` is kept — it becomes a de facto contract the T10/T11 UI will parse or display.
- **Whether T7 emits any `SearchRoot`.** `SearchRootKind` is `{ Skill, Agent }` — about component discovery, not client binaries. The exploration's reading is that T7 reuses none of it; confirm rather than assume.
- **Module, type and function names**; whether `InstallationScan` is a distinct type or mirrors the existing `*Scan` shapes.
- **Whether a desktop version directory whose name is not a plausible version string** is reported, skipped, or accepted verbatim.

**Deferred, with target:**

- **macOS/Linux path validation and the `~/.agents/skills/` location** — **T16**.
- **Oracle contrast against the real machine** (`claude`/`opencode` version output) — **T16**, manual, never automated (`alcance-poc-vertice.md:132`).
- **Surfacing "not detected" in the UI** — **T10/T11**.

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. Fixtures and failing tests land before implementation. Specifically: the two-Claude-Code-installs fixture carrying **two different versions** must exist and fail before the assembly function is written, and the absent-slot fixture must exist before the absence signal is chosen.

## Changed-Line Forecast

| Bucket | Est. lines |
|---|---|
| `installations` module implementation | 130–200 |
| Per-OS probe table + platform-dispatch seam + doc comments | 40–70 |
| Version extraction (`package.json` + directory name) | 40–60 |
| Tests (probing, extraction, isolation, CA-7, CA-11) | 180–260 |
| Fixtures (~8 homes, small JSON files) | 50–80 |
| `lib.rs` / possible `roots.rs` visibility change | 5–15 |
| **Total** | **~445–685** |

**Decision needed before apply: Yes. Chained PRs recommended: Yes. 400-line budget risk: Medium-High.** Slightly smaller than T6 (no dependency, no supply-chain review, no merge semantics). Natural slice, matching the T3–T6 precedent: (1) fixtures, the probe table and RED tests; (2) version extraction and `ClientInstallation` assembly turning them GREEN. Final slicing is `sdd-tasks`'s call.

## Rollback Plan

Additive at every layer, and — unlike T6 — **free at the supply-chain layer**.

- **Core**: delete `installations.rs`, its tests and `tests/fixtures/installations/`; revert one `pub mod` line in `lib.rs` and any `roots.rs` visibility change.
- **`roots.rs`**: revert the visibility change, if any. The T4/T5/T6 suites are the regression guard and must stay green untouched.
- **Model + bindings**: nothing to revert **under Approach A/C**. If `sdd-design` selects Option B, rollback additionally requires reverting the `model/` edit and regenerating `frontend/src/bindings/*.ts` — recorded here so the cost is visible at decision time, not discovered at revert time.
- **CI / supply chain**: nothing to revert — `Cargo.toml`, `Cargo.lock` and `deny.toml` are untouched.
- **App (`vertice-app`)**: zero impact — no command registered, `capabilities/default.json` untouched.
- **Frontend source**: zero impact — no IPC surface, no consumer.

Reverting the branch restores the exact pre-T7 state. No persisted data and no IPC contract depend on any of it.

## Dependencies

- **T2** (`ClientInstallation`, `ClientKind`, `ScanReport.installations`, `ScanIssue`/`IssueSeverity`) — complete and archived; verified sufficient for the detected case with no change required.
- **T4** (`roots::home_dir`, `probe`, `SearchRootStatus`, escalation patterns) — complete and archived; reused by pattern, and possibly by one helper.
- **T6** (`jsonc.rs` seam) — complete and archived; T7 is its second caller, for `package.json`.
- **Blocks**: T9 (`ScanReport` assembly, which populates `installations`), the CA-7 and CA-11 claims, and the T10/T11 client view. **Independent of T8**; may run in parallel with it.

## Success Criteria

- [ ] A fixture home carrying **both** Claude Code installations with **different versions** yields exactly **two** `ClientInstallation` values with `client: ClaudeCode`, each with its own `version` and its own `path`, never merged (**CA-7**).
- [ ] A fixture home carrying the OpenCode npm installation yields one `ClientInstallation` with `client: OpenCode` and its version read from `package.json`.
- [ ] The Claude Code desktop installation's `version` is taken from the **directory name** and its `path` points at that versioned directory.
- [ ] A fixture home where a slot's path does not exist yields **no** `ClientInstallation` for that slot and an explicit **"not detected"** signal naming the client, the install kind and the **probed path** — distinguishable from a parse error and never a silent omission (**CA-11**).
- [ ] A malformed `package.json` in one slot yields exactly one `ScanIssue` at `IssueSeverity::Error` carrying its path, while every other slot is still detected and reported (per-slot isolation).
- [ ] A `package.json` with **no** `"version"` key yields no installation and one `ScanIssue` — never an entry with an empty version string.
- [ ] A desktop directory present with **no** versioned subdirectory yields no installation and one `ScanIssue`.
- [ ] A desktop directory carrying **more than one** versioned subdirectory yields one `ClientInstallation` per subdirectory, never merged and never a `ScanIssue` anomaly on account of the count — pinned by a fixture (CA-7).
- [ ] Every probed path is composed from the passed-in `home` plus hardcoded relative segments; the new module imports no `dirs`/`directories` crate and reads no environment variable (`plan-desarrollo-poc.md:179`).
- [ ] Per-OS path resolution is confined to a single dispatch point; version extraction and `ClientInstallation` assembly contain no `cfg(target_os)` branch, so T16 adds platforms without touching them.
- [ ] `package.json` is parsed through the existing `jsonc.rs` seam; no second JSON crate and no regular expression is introduced.
- [ ] Installation and issue ordering is deterministic across runs and platforms.
- [ ] `Cargo.toml`, `Cargo.lock` and `deny.toml` are byte-identical to their pre-change state; `cargo deny check bans licenses` passes and `vertice-core` still imports nothing from `tauri`.
- [ ] `crates/vertice-core/src/model/` and `frontend/src/bindings/` are byte-identical to their pre-change state **unless** `sdd-design` selects Option B, in which case the binding regeneration is an explicit, reviewed part of the change.
- [ ] No `File::create`, `OpenOptions::write`, or equivalent anywhere in the new module (**CA-16**).
- [ ] All tests read from `crates/vertice-core/tests/fixtures/`; no test reads the author's machine, sets an environment variable, invokes `claude` or `opencode`, or reuses T4/T5/T6 fixture homes (**CA-17**).
- [ ] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` and `cargo deny check bans licenses` pass on the three-platform CI matrix; the T4/T5/T6 suites stay green.

## Proposal question round

The interactive question round could not be run from this phase. These are the product questions whose answers would change the proposal, with the assumption currently written into it. Answer, correct, or skip — a second round is available.

| # | Question | Assumption currently written in |
|---|---|---|
| 1 | Does the user need to know *which* client is not detected in a structured way (filterable, per-client UI slot), or is a human-readable diagnostic line enough for the PoC? | Human-readable `ScanIssue` is enough; the typed alternative is flagged as an open design decision with its full cost stated |
| 2 | Is "Claude Code is not installed" a **warning** or just neutral information? The chosen `IssueSeverity` is what the UI will colour. | Undecided on purpose; listed for `sdd-design` so it is not settled by habit |
| 3 | If a slot's `package.json` is unreadable, should the client show as "not detected" or as "detected, version unknown"? | Neither — no installation is emitted and an `Error` issue is raised; a phantom entry with an empty version is explicitly rejected |
| 4 | Should the two Claude Code installations be visually linked as "one client, two installs", or presented as two independent rows? | Two independent `ClientInstallation` values; grouping, if any, is a T11 presentation decision, not a core one |
| 5 | Is anything expected from a detected installation beyond client, version and path (install kind, "is this the active one")? | No — `ClientInstallation` is consumed exactly as T2 merged it; adding a field would break the no-model-edit property |
