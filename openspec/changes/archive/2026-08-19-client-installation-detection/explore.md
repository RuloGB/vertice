# Exploration — T7: Client installation detection

- **Change name**: `2026-08-19-client-installation-detection`
- **Roadmap phase**: T7 (`internal-docs/plan-desarrollo-poc.md:171`)
- **Acceptance criteria**: CA-7 (two Claude Code installations detected separately, each with its version), CA-11 (absent client reported as "not detected")
- **Depends on**: T2 (domain model + type contract) — merged
- **Status**: exploration only. No proposal, no spec, no implementation.

## 1. Current state

**Model layer (T2, already merged — no gap):**

- `ClientInstallation { client, version, path }` — `crates/vertice-core/src/model/installation.rs:14`. Its doc comment already states each installation is counted separately and never merged, which is exactly CA-7.
- `ClientKind { ClaudeCode, OpenCode }` — `installation.rs:27`. Closed enum by design.
- `ScanReport.installations: Vec<ClientInstallation>` — `crates/vertice-core/src/model/report.rs:22`. The report field already exists; nothing to add there.
- `ScanIssue { severity, path, reason }` with `IssueSeverity { Warning, Error }` — the only per-item diagnostic channel in the model.

**Gap**: there is no representation of "client X was checked and is not present". Today a client with zero detected installations is simply absent from `installations` — indistinguishable from "never checked" and from an error. That ambiguity is precisely what CA-11 forbids.

**Root-resolution precedent (T5/T6, `crates/vertice-core/src/roots.rs`):**

- `home_dir()` is the only ambient-environment read in the crate; every other function takes `home: &Path`. This is what makes fixture-based, machine-independent tests possible.
- `probe(path) -> SearchRootStatus` treats `ErrorKind::NotFound` as `NotFound` and any other I/O error as `Found` (safer default). Directly reusable for installation-path existence checks.
- `ResolvedRoot { root, scan_paths }` merges N candidate paths into one logical root. T7's Claude Code npm-vs-desktop split does **not** fit this pattern: those are two separate installations to report distinctly (CA-7), not two paths merged into one root.
- `SearchRootKind` is `{ Skill, Agent }` — about *component* discovery roots, not client binaries. T7 most likely does not reuse `SearchRoot` at all.

**Adapter precedent (T4/T5/T6 — `skills.rs`, `agents.rs`, `opencode_agents.rs`):**

- Each adapter exposes an infallible `scan(home: &Path) -> XScan { roots, components, issues }`, accumulating `ScanIssue` instead of propagating `Result`. T7's output shape differs (`Vec<ClientInstallation>`, not `Vec<Component>`) but the calling convention is the idiom to mirror.
- `jsonc.rs` is the sealed seam for JSON/JSONC parsing. `package.json` is strict JSON, a subset of what that seam already accepts.
- `frontmatter.rs` is YAML/Markdown-specific (fence splitting) and is not reusable for `package.json`.

## 2. Detection surface (Windows, per plan)

| Client | Install kind | Path (relative to `home`) | Version source |
|---|---|---|---|
| Claude Code | npm | `AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/` | `package.json` → `"version"` |
| Claude Code | desktop | `AppData/Roaming/Claude/claude-code/<version>/` | the directory name itself is the version (path-segment read, not a file read) |
| OpenCode | npm | `AppData/Roaming/npm/node_modules/opencode-ai/` | `package.json` → `"version"` |

**Hard rule** (`plan-desarrollo-poc.md:179`): the scanner never infers foreign paths from OS conventions. OpenCode uses XDG even on Windows (`~/.config/opencode`, `~/.local/share/opencode`, `~/.cache/opencode`); Claude Code uses `~/.claude`. The `directories`/`dirs` crate is reserved exclusively for Vertice's own app-data directory and MUST NOT produce any of the three paths above. All three are composed from hardcoded `home`-relative segments, exactly as `roots.rs` does today (no `dirs`/`directories` dependency exists in `vertice-core`).

Subtlety worth recording in `design.md`: `AppData/Roaming/npm/...` *coincides* with an OS convention value, but the code path producing it must not run through a convention-resolving crate.

## 3. "Not detected" as an explicit state

Three candidate shapes, compared against the existing model:

1. **Reuse `ScanIssue` (`Warning`)** — e.g. `reason: "Claude Code (npm) not detected at <path>"`. Zero model changes; consistent with `report.rs:44` ("severity is a display/triage signal for later UI phases, not control flow") and with `SearchRootStatus`'s explicit refusal of a third value.
2. **New closed model type** — `ClientDetectionStatus { client, install_kind, status: Detected(ClientInstallation) | NotDetected }` on `ScanReport`. Strongest typed contract, but changes `ScanReport`'s shape (binding regeneration, frontend consumer contract) and introduces a state channel parallel to `ScanIssue`.
3. **Per-slot `Option<ClientInstallation>` inside the adapter (non-model)** — compile-time-closed set of slots inside the adapter; only `Some` entries reach `ScanReport.installations`, absence signaled via `ScanIssue`. Hybrid of 1 and 2 without touching `model/`.

Option 1 is the most consistent with existing project philosophy. Its weakness: the frontend must cross-reference free-text `issues` to learn *which* client is missing. **This is a genuine open decision for `sdd-design`, not for exploration to settle.**

## 4. Multi-platform structure (forward-looking to T16)

`roots.rs` already establishes the shape to follow: hardcoded per-segment relative paths built from a passed-in `home`, with all OS-conditional logic isolated to path construction and never mixed into probing or parsing.

- A per-OS path-resolution function returning the candidate installation probes (e.g. `windows_install_probes(home)`), dispatched by target OS.
- Version extraction (parse `package.json`, or read a directory name) stays OS-agnostic and shared. Only the *path template* differs per platform.
- T16 then adds `macos_install_probes` / `linux_install_probes` without touching shared extraction code.
- Suggested internal (non-model) struct, analogous to `ResolvedRoot`: `InstallProbe { client, install_kind: {Npm, Desktop}, path, version_source: {PackageJson, DirName} }`, consumed by a shared resolver.

## 5. Fixture strategy

Existing adapters build synthetic `home` trees under `crates/vertice-core/tests/fixtures/` and pass them as the `home: &Path` parameter — never touching the real machine. T7 needs the equivalent (e.g. `tests/fixtures/installations/`):

- Two separate Claude Code installations: synthetic `AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/package.json` with a `"version"`, plus `AppData/Roaming/Claude/claude-code/<version>/` — proving CA-7.
- One OpenCode installation: synthetic `AppData/Roaming/npm/node_modules/opencode-ai/package.json`.
- One absent client: a `home` where a probe path does not exist — proving CA-11 without an I/O error and without a phantom empty-version entry.
- Edge fixtures to flag for `sdd-tasks`: `package.json` with missing/malformed `"version"` (parse-failure `ScanIssue`, mirroring the `frontmatter.rs` type-mismatch precedent), and a desktop directory present but carrying no versioned subdirectory.

## 6. Approaches

| Approach | Description | Pros | Cons | Effort |
|---|---|---|---|---|
| **A. Single `installations.rs`, `ScanIssue`-based absence** | `scan(home) -> InstallationScan { installations, issues }`, mirroring `agents.rs`. Windows path table now, platform-dispatch seam left explicit for T16. | No model or binding changes; matches existing `ScanIssue` philosophy; minimal review surface | Frontend must read free-text `issues` to know which client is missing | Low |
| **B. New `ClientDetectionStatus` model type** | Closed per-slot status added to the model and `ScanReport`. | Strongest typed contract for CA-11; scales to more clients | Model change → binding regen → frontend contract change; duplicates a channel `ScanIssue` already provides | Medium |
| **C. Per-slot `Option` inside the adapter** | Compile-time-closed slot set internally; `ScanIssue` for observability. | No model change; a missed slot fails to compile | More internal plumbing than A for the same external behavior | Low-Medium |

**Recommendation**: A (or its refinement C). Lowest-risk path to CA-7/CA-11 without touching the frozen T2 model contract, and consistent with how `SearchRootStatus` and `ScanIssue` already solve structurally identical "explicit absence, not silent omission" problems. B stays logged as an explicit open question for `sdd-design`; retrofitting it later is cheap.

## 7. Open questions and risks

1. **Unverified macOS/Linux install paths** (`plan-desarrollo-poc.md:388`, open decision #3). T7 ships Windows only, with structure prepared for T16. Risk: over-designing the platform seam for path shapes that are still unknown.
2. **`~/.agents/skills/` cross-platform location** is unconfirmed. Affects T4/T16 more than T7, but named in the same risk note.
3. **"Not detected" representation** (§3) — genuinely undecided; must be closed in `sdd-design`, not implicitly in `sdd-apply`.
4. **Multiple Claude Code desktop version directories** — ~~RESOLVED (user decision, 2026-08-19): multiple simultaneous version directories under `AppData/Roaming/Claude/claude-code/` are not possible; the desktop install keeps exactly one. The scanner treats the single versioned subdirectory as the desktop installation. If more than one is ever found on disk, that is an anomaly to report as a `ScanIssue`, not a case to model.~~ **SUPERSEDED (direct inspection of the reference machine, 2026-08-19)**: the earlier resolution was a user decision, not an observation, and it has been refuted. `AppData\Roaming\Claude\claude-code\2.1.229\` and `AppData\Roaming\Claude\claude-code\2.1.234\` were found coexisting on the reference machine. The scanner now models each versioned subdirectory as its own `ClientInstallation` — N subdirectories yield N installations, never merged, never an anomaly.
5. **`package.json` parsing seam** — reuse `jsonc.rs` (one crate, one seam) rather than adding a second JSON dependency. Confirm in `sdd-design`.
6. **Read-only invariant (CA-16)** — T7 only reads; no new risk beyond standing project discipline.
7. **`strict_tdd: true`** (`openspec/config.yaml:22`) — design and tasks must lead with fixtures-first tests, as in T3–T6.

## Ready for proposal

Yes. One item is explicitly deferred to `sdd-design`: the "not detected" representation (§3). The multiple-desktop-version edge case (§7.4) is resolved — multiple desktop version directories are a reachable, supported state, each its own installation, per the 2026-08-19 machine evidence. Neither changes T7's scope boundary.
