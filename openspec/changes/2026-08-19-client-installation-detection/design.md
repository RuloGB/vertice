# Design: Client Installation Detection

> Trace: **T7** (`internal-docs/plan-desarrollo-poc.md:171-187`) / closes **CA-7** and **CA-11**; bound by **CA-16** (read-only) and **CA-17** (versioned fixtures, three platforms). Carries the hard rule of `plan-desarrollo-poc.md:179` (never derive a foreign path from an OS convention).
> Proposal: `openspec/changes/2026-08-19-client-installation-detection/proposal.md`. Inherits T4's design (**T4D**), T5's (**T5D**) and T6's (`openspec/changes/archive/2026-08-18-opencode-agent-adapter/design.md`, **T6D**). T5D §5.4 / T6D §5.5 (**no shared scanner abstraction before T9**) and T6D §8 (**severity taxonomy**) are inherited as closed decisions; §4 below *diverges* from one clause of T6D §8 with a stated reason, which is a different act from re-opening it.
> `rules.design` coverage: core data model impact (§2), core/Tauri isolation for the CLI pathway (§1), per-OS paths (§11), `ScanIssue` taxonomy and error paths (§8), IPC contract surface (§2 — **none**, and that is load-bearing).
> **Environment note.** `cargo` was not available and this phase had no shell. **Nothing below was verified by compiling, and no installation path was re-inspected on the reference machine from here.** §0 separates what is verified-by-reading-the-repo from what is inherited-on-trust. Do not collapse that distinction.

## 0. What is verified, and what is inherited on trust

| # | Statement | Basis |
|---|---|---|
| V1 | `ClientInstallation { client, version: String, path: PathBuf }` and `ClientKind { ClaudeCode, OpenCode }` are merged and sufficient for the *detected* case; `ScanReport.installations` exists and is still empty in every code path | Read `model/installation.rs:14,27`, `model/report.rs:22` |
| V2 | `IssueSeverity` has exactly two values, `Warning` and `Error`, and `report.rs:44` declares severity a **display/triage signal, not control flow**. There is no `Info` and adding one is a model edit | Read `model/report.rs:46-52` |
| V3 | `SearchRootKind` is `{ Skill, Agent }`; `SearchRoot` is what `Location.root` points at. No variant describes a client binary | Read `model/location.rs`, `roots.rs:11` |
| V4 | `roots::probe` is **private** (`roots.rs:198`) and returns `SearchRootStatus`. `roots::home_dir` remains the crate's only ambient-environment read | Read `roots.rs:31,198` |
| V5 | `jsonc::parse(&str) -> Result<JsonValue, JsoncError>` accepts strict JSON as a subset, resolves duplicate keys last-wins, and returns an **empty `Object` for empty input** rather than erroring | Read `jsonc.rs:59-73` |
| V6 | The absence idiom already in the crate is *read once, branch on `ErrorKind::NotFound`* — never probe-then-read (`opencode_agents.rs:96-107`, `skills.rs:66-77`) | Read |
| **U1** | The three Windows installation paths in §11 | **Verified on 2026-08-19 by direct inspection of the reference machine — no longer merely inherited from `alcance-poc-vertice.md:71-102`.** All three probe paths exist exactly as §11 states: `AppData\Roaming\npm\node_modules\@anthropic-ai\claude-code\`, `AppData\Roaming\Claude\claude-code\<version>\`, `AppData\Roaming\npm\node_modules\opencode-ai\`. `AppData\Roaming\npm\node_modules\@anthropic-ai\claude-code\package.json` reads `"version": "2.1.140"` and `AppData\Roaming\npm\node_modules\opencode-ai\package.json` reads `"version": "1.17.20"`, matching the August 2026 record (`alcance-poc-vertice.md:71-102`, method: filesystem inspection, string extraction from the compiled OpenCode binary, and empirical contrast with `opencode debug skill`, `opencode debug config`, `opencode debug paths` and `claude agents`, `alcance-poc-vertice.md:73`) |
| **U2** | That exactly one versioned subdirectory ever exists under `AppData/Roaming/Claude/claude-code/` | **Refuted on 2026-08-19 by direct inspection of the reference machine.** This was never a filesystem observation — it was a user decision recorded 2026-08-19 that this phase pinned as an anomaly (§6, old text). Direct inspection the same day found `AppData\Roaming\Claude\claude-code\2.1.229\` and `AppData\Roaming\Claude\claude-code\2.1.234\` **coexisting** on the reference machine. The design now models **N desktop installations**, one per versioned subdirectory, never merged and never treated as an anomaly — the semantics `model/installation.rs:8-10` already states for a client installed twice. §6 |

**V5 has a sharp edge that this design must handle explicitly**: an *empty* `package.json` parses successfully into an empty object, so "empty file" reaches the same branch as "valid JSON with no `version` key". §8 routes both to the same `Error`, which is correct — but an implementer must not assume `jsonc::parse` guards emptiness.

## 1. Technical approach

One new module, **no** change to `roots.rs`, **no** change to `model/`, **no** new dependency.

```
                                     vertice-core
 frontend ──IPC──> vertice-app ──>   ├── model/           (pure data, zero I/O)  ← UNCHANGED, §2
                                     ├── roots            ← UNCHANGED, §5.3
 future vertice-cli ────────────>    ├── skills/agents/opencode_agents  ← UNCHANGED
                                     ├── jsonc            (JSON seam — reused, second caller)
                                     └── installations    (NEW)

 installations::scan(home)
        │
        ├─ HostPlatform::current()          ← the ONLY cfg point in the change (§5.2)
        ├─ windows_install_probes(home) ─> [InstallProbe; 3]   (path table, per OS)
        │
        └─ for each probe, INDEPENDENTLY (§8):
              npm slot     ──> version from package.json ──> 0..1 ClientInstallation
              desktop slot ──> one per versioned subdir  ──> 0..N ClientInstallation  (§6)
                            └─> absent  ──────> ScanIssue(Warning)   ← CA-11
                            └─> broken  ──────> ScanIssue(Error)
        │
        └──> InstallationScan { installations, issues }      ← NO `roots` field, §3
```

**The CLI pathway is preserved unchanged.** `scan` takes `home: &Path` and returns owned data; it performs **no ambient-environment read**, consults no `dirs`/`directories` crate (`vertice-core` has none and T7 adds none), and reads no environment variable. Every path is `home` plus hardcoded relative segments pushed one at a time. `AppData/Roaming/npm/...` *coincides* with a convention value; the code path producing it must never run through a convention resolver, because a resolver honours `%APPDATA%` redirection and would make fixture assertions machine-dependent, destroying CA-17.

**Core purity and `model/` purity survive trivially**: no dependency is added (`Cargo.toml`, `Cargo.lock`, `deny.toml` byte-identical), and `model/` is not opened. **CA-16, structurally**: the complete disk surface of this change is `std::fs::symlink_metadata`, `std::fs::read_to_string` and `std::fs::read_dir`. No `File::create`, no `OpenOptions`, no `fs::write`, no `create_dir*`, no `remove_*` — in the module or in its tests, which read committed fixtures and never materialize a temp tree.

## 2. Decision: the representation of "not detected" — **Option A refined by C**. Core data model impact: none

> **Decision: absence is signalled through `ScanIssue`. `model/` is not edited, no `ClientDetectionStatus` type is introduced, and `frontend/src/bindings/*.ts` is byte-identical after this change. Option C's refinement is adopted internally: the slot set is a fixed-size array (`[InstallProbe; 3]`), so a forgotten slot fails to compile.**
>
> **Option B is REJECTED.** `domain-model` therefore stays out of Modified Capabilities, the proposal's "bindings unchanged" property **holds**, and `sdd-spec` (running in parallel against option A) needs **no revision**.

| Option | Consequence | Decision |
|---|---|---|
| **A — `ScanIssue`** | Zero model edit, zero binding regeneration, zero consumer contract change. Absence is a free-text diagnostic plus a typed `path`. The UI cannot filter "which client is missing" without reading strings | **Chosen** |
| C — per-slot `Option` closed *inside* the adapter | Externally identical to A. Buys compile-time slot closure for internal plumbing only | **Chosen as a refinement of A** (§5.1) |
| B — `ClientDetectionStatus { client, install_kind, status }` on `ScanReport` | Strongest typed CA-11 contract. Costs a `model/` edit, three new `TS` types, a binding regeneration, a `ScanReport` shape change, and a **second state channel parallel to `ScanIssue`** for the same fact | **Rejected** |

**Why B loses, in the terms this codebase already argued.** Twice now the model has been asked for a third state and twice it refused: `SearchRootStatus` stayed two-valued and "present but unreadable" became `Found` **plus** a `ScanIssue` (`location.rs:53-57`, restated in T6D §2); `IssueSeverity` stayed two-valued and severity was declared a display signal, not control flow (V2). B is the same request in a third costume. It would also freeze a shape T9 has not yet seen: `ScanReport` is assembled at T9 and T10 serializes it — introducing a parallel status channel *before* the aggregator exists is guessing at the consumer's needs one phase early, and every guess costs a binding regeneration to undo.

**The cost of A, stated plainly rather than buried.** The T10/T11 UI cannot answer "is Claude Code installed?" from a typed field. It must either display the issue list verbatim or match on `reason` text — and §4 forbids the latter. **If T10/T11 concludes it needs a structured answer, B is the retrofit**, and it is cheap: `installations` already computes the per-slot outcome as a closed value internally (§5.1), so B becomes "publish the internal enum through `model/`", not a redesign. Recorded in §13 with T10/T11 as the target.

Mechanical consequences a reviewer can check without running anything:

- **No `TS`-derived type is introduced.** `InstallationScan`, `InstallProbe`, `InstallKind`, `VersionSource` and `HostPlatform` derive neither `Serialize` nor `TS`. Precedent: `ResolvedRoot` and `jsonc::JsonValue` are public non-model types today.
- **`frontend/src/bindings/*.ts` is byte-identical.** No regeneration is performed or required; CI's `git diff --exit-code -- frontend/src/bindings` step stays green untouched.
- **No IPC contract surface.** No Tauri command, `crates/vertice-app/capabilities/default.json` untouched, no frontend source file changed. `rules.design`'s "detail IPC contract surface" is satisfied by the **empty set**, with the mechanical proof above. IPC exposure is T10.

| Temptation | Why it breaks the property | Verdict |
|---|---|---|
| Add `install_kind` to `ClientInstallation` so the UI can tell npm from desktop | `ClientInstallation` derives `TS`; one field regenerates `ClientInstallation.ts` and puts T7 on the bindings drift gate. Proposal Q5 answered this: the type is consumed exactly as T2 merged it | **Rejected** |
| Add a `ClientKind` variant "so the enum is ready" | Same gate, for a client with no adapter | **Rejected** |
| Add `IssueSeverity::Info` for the not-detected case | Same gate, and V2 says two levels is a deliberate design, not an oversight | **Rejected** — see §4 for what is done instead |

## 3. Decision: T7 emits **no** `SearchRoot`

> **Decision: confirmed — zero `SearchRoot` values. `InstallationScan` has no `roots` field, unlike `SkillScan`, `AgentScan` and `OpenCodeAgentScan`. The asymmetry is deliberate and is the point.**

`SearchRootKind` is `{ Skill, Agent }` (V3): a `SearchRoot` is a place *components* are discovered, and `Location.root` is a reference into that set. An installation directory discovers no component. Emitting one would force either a new `SearchRootKind` variant — a model edit, a binding regeneration, forfeiting §2 — or mislabelling `AppData/Roaming/npm/node_modules/opencode-ai` as a `Skill`/`Agent` root, which would corrupt `ScanReport.roots_scanned` at T9 and any T11 view that groups components by root. Neither is acceptable for a field nothing consumes.

**The consequence that drives §4**, and it must not be skimmed: the other three adapters express *absence* through `SearchRootStatus::NotFound` on a root they emit anyway, which is exactly why T6D §8 could rule that an absent file produces **no `ScanIssue`**. T7 has no such structural channel. Absence therefore has nowhere to live except `ScanIssue` — or nowhere at all, which is the shape CA-11 exists to forbid.

## 4. Decision: severity `Warning`, and the exact `reason` grammar

> **Decision: a probed-and-absent slot yields exactly one `ScanIssue` with `severity: IssueSeverity::Warning`, `path: Some(<the probe path>)`, and `reason: "{Client} ({kind}) not detected"`.**

**This is a deliberate, narrow divergence from T6D §8's "absence is silent" clause**, licensed by §3: T6's absence had a typed home and T7's does not. The severity *rule* itself is inherited verbatim and extended by one row:

> `Error` = an installation is **missing from the inventory because something failed**. `Warning` = a slot was checked and the client is **legitimately not installed**. Nothing that failed is ever a `Warning`; nothing that succeeded is ever an issue.

**Why `Warning` and not `Error`** (decision 2 — a product statement, made on purpose):

| Argument | Weight |
|---|---|
| CA-11's own words: an absent client is "not an error". Emitting `IssueSeverity::Error` would contradict the acceptance criterion in the one field the UI colours | Decisive |
| Vertice ships adapters for two clients and probes three slots. A user with **only** Claude Code installed — the common case — would see a permanent red `Error` on every scan for a machine that is perfectly healthy. That is T6D §8's "noise that trains users to ignore the issue list", reached by a different road | Decisive |
| `Error` is already load-bearing for "an installation exists and we could not read it" (§8). Reusing it for "no installation exists" makes the two indistinguishable at exactly the severity a triage UI filters on | Strong |

**Accepted cost, recorded.** `Warning` is the lowest level the model offers and the not-detected case is arguably *neutral information*, not a warning. `IssueSeverity::Info` is not added (§2's table). T10/T11 MUST NOT badge a not-detected `Warning` identically to a metadata `Warning`; the discriminator available to it is the pair (`severity`, whether the slot appears in `installations`), not the string. §13.

**The `reason` grammar** (decision 3), specified rather than improvised:

| Case | `severity` | `path` | `reason` |
|---|---|---|---|
| Slot probed, nothing there | `Warning` | `Some(<probe path>)` | `{Client} ({kind}) not detected` |
| Every failure case (§8) | `Error` | `Some(<the failing path>)` | `could not …` / `expected …` — always a verb phrase about the failure |

with `{Client}` ∈ `{"Claude Code", "OpenCode"}` and `{kind}` ∈ `{"npm", "desktop"}`. Concretely: `Claude Code (npm) not detected`, `Claude Code (desktop) not detected`, `OpenCode (npm) not detected`.

Three rules an implementer must not improvise:

1. **The probed path is carried in `ScanIssue.path`, never interpolated into `reason`.** This satisfies the closed decision "the probed path MUST appear in the absence signal" through a **typed field** rather than free text, so "we looked in the wrong place" is distinguishable from "nothing is installed" without any string handling. It also mirrors T6 exactly (`opencode_agents.rs:100-106` puts the path in `path` and keeps `reason` path-free).
2. **`reason` remains a developer diagnostic** (T3D §6 policy, unchanged): it MUST NOT be parsed or branched on by any consumer, and T12 has zero T7-authored strings to translate. The grammar above is a **stability commitment for display**, not a parsing contract. It is specified only so three near-identical messages do not drift into three different shapes.
3. **The grammar makes the two classes visually disjoint**: exactly the not-detected issues end in `not detected`; every failure reason opens with a verb about the failure. A human reading a log can separate them at a glance, which is the whole benefit that was actually available under option A.

## 5. Module, type and function surface (decision 5)

### 5.1 `installations.rs`

```rust
// crates/vertice-core/src/installations.rs

/// Owned result of one client-installation scan. A distinct type from
/// `SkillScan`, `AgentScan` and `OpenCodeAgentScan`, and deliberately
/// WITHOUT a `roots` field (design §3).
#[derive(Debug, Clone, PartialEq)]
pub struct InstallationScan {
    pub installations: Vec<ClientInstallation>,
    pub issues: Vec<ScanIssue>,
}

/// Scan for installed clients under `home`, dispatching on the compiled
/// target (design §5.2). Infallible, mirroring the component adapters.
pub fn scan(home: &Path) -> InstallationScan;

/// Same scan against an explicit platform's path table. Public because it
/// is what makes the Windows table testable on the Linux and macOS CI legs
/// (design §5.2) — not a general-purpose knob.
pub fn scan_for(home: &Path, platform: HostPlatform) -> InstallationScan;

/// Which OS path table to use. NOT a model type: no `Serialize`, no `TS`.
/// T16 replaces `Unsupported` with `MacOs` and `Linux`.
pub enum HostPlatform { Windows, Unsupported }

// --- private ---
struct InstallProbe { client: ClientKind, kind: InstallKind, path: PathBuf, version: VersionSource }
enum InstallKind   { Npm, Desktop }
enum VersionSource { PackageJson, DirectoryName }

fn windows_install_probes(home: &Path) -> [InstallProbe; 3];   // §11
fn resolve(probe: &InstallProbe, issues: &mut Vec<ScanIssue>) -> Option<ClientInstallation>;   // npm slots — §6 note below
```

`lib.rs` gains exactly one line, `pub mod installations;`, with no crate-root re-export (matching `lib.rs:7-12`).

- **`InstallationScan` is a fourth distinct type**, not an alias and not a shared generic — T5D §5.4 / T6D §5.5 inherited unchanged. T7 is the *least* extractable of the four: it produces `ClientInstallation`, not `Component`, walks no directory in the component sense, and emits no root. **Four near-identical adapter entry points is the intended end state of Phase 1**; the extraction point, if ever, is T9.
- **`[InstallProbe; 3]` is option C's compile-time closure**, and it is the exact precedent `skill_roots -> [ResolvedRoot; 3]` set (`roots.rs:52-56`: "the array length is the CA-6/CA-14 guarantee, expressed in the type rather than asserted in prose"). A slot dropped during a refactor is a compile error, not a silently shrunk inventory.
- **For the two npm slots, `resolve` returns `Option<ClientInstallation>` and pushes at most one issue.** `None` + one issue is the *only* shape for a slot that yields nothing; `Some` + zero issues the only shape for one that yields something. A phantom `ClientInstallation` with an empty `version` is unrepresentable by construction, which is proposal Q3's answer made structural. **The desktop slot's own resolution shape is §6's**, not this one: it yields zero-to-N `ClientInstallation` values (one per versioned subdirectory) and at most one issue (only the 0-candidate case), reflecting the U2 refutation recorded in §0. The three-slot loop in §1's diagram folds this in by iterating candidates within the desktop slot; it is not a fourth `Option`-shaped probe.
- **`InstallKind` and `VersionSource` stay private.** Making `InstallKind` public and `TS`-derived would be §2's rejected model change entering through the back door. It reaches the outside world only as the `"npm"` / `"desktop"` label in §4's grammar, via a private `fn label(&self) -> &'static str`.

### 5.2 Decision: the platform-dispatch seam (decision 7)

> **Decision: one function per OS returning a fixed-size probe table, plus a single `cfg!` expression in `HostPlatform::current()`. No trait, no registry, no abstraction over path shapes that are still unverified.**

```rust
impl HostPlatform {
    /// The ONLY compile-target branch in this change. `cfg!` (an expression,
    /// not an attribute) so every arm compiles on every target — see below.
    fn current() -> Self {
        if cfg!(target_os = "windows") { HostPlatform::Windows } else { HostPlatform::Unsupported }
    }
}
```

**Why `cfg!` and not `#[cfg(target_os = ...)]`, which is the obvious spelling.** Under `#[cfg]`, `windows_install_probes` would not be compiled on the Linux and macOS CI legs. Two consequences, both fatal:

1. Every fixture test of the Windows table would have to be `#[cfg(windows)]`-gated, so **two thirds of the CI matrix would verify nothing** — the exact opposite of CA-17 (the matrix is `[ubuntu-24.04, windows-2022, macos-14]`, `.github/workflows/ci.yml:124`).
2. On Linux/macOS `windows_install_probes` would not exist at all — it would be compiled out entirely — so the unconditional call to it from `scan`/`scan_for` would reference an unresolved item. That fails to **compile**, not lint: it is a hard build break on those two legs, not a `cargo clippy -- -D warnings` finding, and it breaks before any test on that leg can even run.

With `cfg!`, `HostPlatform::Windows` is constructible everywhere, `windows_install_probes` is live everywhere, and the fixture suite calls `scan_for(&home, HostPlatform::Windows)` on all three legs. **The Windows path table is verified on Linux and macOS**, because a synthetic `home` containing `AppData/Roaming/...` is just directory names to any filesystem. This is the single most valuable structural property in the change.

**`Unsupported` is not "not detected".** On macOS/Linux, `scan` returns zero installations and **one** `ScanIssue` at `Warning`, `path: None`, `reason: "client installation detection is not implemented on this platform"`. It MUST NOT emit three "not detected" warnings — that would tell a macOS user their clients are absent when the truth is that Vertice did not look. That is the same class of lie CA-11 exists to forbid, and it is reachable today, not hypothetically.

**T16 is then purely additive**: add `MacOs` and `Linux` variants, add `macos_install_probes` / `linux_install_probes`, delete `Unsupported`. `resolve`, version extraction, `ClientInstallation` assembly and every issue string contain **no** target branch and are not touched.

### 5.3 Decision: `roots::probe` stays private; a 3-line local helper is used instead

> **Decision: `roots.rs` is NOT modified at all — not even a visibility change. `installations.rs` carries its own private `fn exists(path: &Path) -> bool` over `std::fs::symlink_metadata`.**

| Option | Consequence | Decision |
|---|---|---|
| Make `roots::probe` `pub(crate)` and reuse it | Returns `SearchRootStatus` — a **model type whose doc comment is about search roots** — inside a module that deliberately emits no `SearchRoot` (§3). The caller would translate it back to a bool anyway. It also puts T4/T5/T6's shared resolver on this change's regression surface for three lines | **Rejected** |
| **Local private `exists`, `NotFound` ⇒ `false`, any other error ⇒ `true`** | Same conservative semantics as `probe` (`roots.rs:193-204`: `NotFound` is a positive claim; anything else means "we found something, or something went wrong looking") without importing search-root vocabulary. `roots.rs` and its unit suite stay green **with no edits at all** | **Chosen** |

This is **deliberate, recorded duplication**, not oversight. Its only use is the one place where a bool is genuinely needed: distinguishing "the npm package directory is absent" (→ not detected) from "the directory exists but its `package.json` does not" (→ `Error`). Everywhere else the crate's read-once idiom applies (V6): `read_to_string`/`read_dir` is called directly and `ErrorKind::NotFound` *is* the absence signal — no probe-then-read, which would be both a double syscall and a TOCTOU window.

### 5.4 Version extraction — two sources, one assembly point

| Slot | `InstallProbe.path` | Version | `ClientInstallation.path` |
|---|---|---|---|
| Claude Code npm | `…/node_modules/@anthropic-ai/claude-code` | `jsonc::parse(<path>/package.json)` → top-level `"version"`, **only if `JsonValue::String`** | the package directory |
| OpenCode npm | `…/node_modules/opencode-ai` | idem | the package directory |
| Claude Code desktop | `…/Claude/claude-code` | the **name of each versioned subdirectory** (§6), one per candidate | the **versioned subdirectory** itself, not the parent |

`package.json` goes through the existing `jsonc.rs` seam — second caller of a sealed seam, zero new dependency, and **no regular expression anywhere**. Extraction is value-level, exactly as T6D §5.4 chose: no `#[derive(Deserialize)]` DTO describes `package.json`. `package.json` has dozens of fields Vertice does not consume and a DTO would make an unexpected type in any of them able to delete an installation from the inventory. Only `"version"` is read, and only as a string.

## 6. Decision: the desktop version directory (decision 6)

> **Decision: the directory name is accepted VERBATIM as the version. No plausibility predicate, no semver validation, no name heuristics. Each candidate subdirectory is its own `ClientInstallation` — N candidates yield N installations, never merged, never "highest wins".**

| Option | Consequence | Decision |
|---|---|---|
| Validate the name against a version shape and skip non-matching entries | There is **no independent oracle** — the directory name is the only version source that exists. Any predicate silently drops a real installation the day Anthropic ships `2.0.0-rc.1`, `nightly`, or a date-stamped directory. Silently dropping an installed client is the CA-11 failure with extra steps | **Rejected** |
| Validate and report a non-matching name as an issue | Same predicate, same fragility, plus a false `Error` on a healthy machine | **Rejected** |
| **Accept verbatim** | The PoC *reports*, it does not interpret — the same rule that already keeps `version` an unvalidated `String` (proposal risk table) and that T6D §6.1 applied to agent keys ("presence is the detection rule; no name heuristics") | **Chosen** |

**Candidate rule, stated so it cannot be improvised.** Enumerate `…/Claude/claude-code` with `read_dir`; a candidate is an entry that **is a directory** (`file_type().is_dir()`); everything else is not a candidate and produces no issue by itself. Candidates are collected and **sorted by file name, byte-wise, never locale collation** (§7), so the resulting installation vector is deterministic across the three CI legs. Then, by candidate count:

| Candidates | Behavior | Severity |
|---|---|---|
| 0 (including "directory exists but holds only files") | no installation, **one issue**: `expected at least one Claude Code desktop version directory, found none`, `path: Some(<claude-code dir>)` | `Error` |
| N ≥ 1 | **N `ClientInstallation` values**, one per candidate, in sorted order — `version` = that candidate's directory name verbatim, `path` = that candidate directory. No merging, no "highest wins", no anomaly | — |
| any, name not valid UTF-8 | that candidate is not usable as a `String` version: one issue, `path: None`, name rendered lossily in `reason` (the `skills.rs:118-125` precedent); the candidate itself contributes no installation, and every other valid candidate is still resolved | `Error` |

**N ≥ 1 yields N installations, never merged and never "the highest".** This is what U2's refutation makes structural: the reference machine has `2.1.229` and `2.1.234` coexisting under `Claude/claude-code/`, so "exactly one" was never the invariant — it was an unverified user decision that direct inspection on 2026-08-19 disproved. The rule now applied is the one `model/installation.rs:8-10` already states for a client installed twice: "each installation is counted separately … reported as two `ClientInstallation` values, never merged." The desktop slot was narrower than the model it feeds; this correction removes that gap.

**Accepted cost, recorded — and it gets worse under this rule, said honestly.** A stray leftover directory (a partial download, a `.tmp` staging dir) is indistinguishable from a version directory under this rule. Under the old ≥2-is-an-anomaly rule, a stray directory alongside a real one at least surfaced as a visible `Error`. Under the corrected N-installations rule, that same stray directory becomes a **phantom installation** silently mixed into the inventory — a worse failure mode, because nothing about it is visible without knowing the machine's real install state. This is accepted because there remains no independent oracle for a directory name, and the alternative (validating the name) was already rejected above for the same reason. The detector is still T16's `claude --version` contrast — it is what makes a phantom entry from a stray directory noticeable, and it is now doing more work than before. §13.

## 7. Determinism

- **Slot order is the probe table's order**, which is a fixed-size array literal — deterministic by construction, on every platform and every run. `installations` and `issues` are both emitted in that order, so no trailing sort exists to be deleted by a refactor (T6D §7's reasoning).
- **`read_dir` order is OS-dependent and is never trusted.** Desktop candidates are collected and **sorted by file name (byte-wise, never locale collation)** before use, so the emitted `ClientInstallation` order and the non-UTF-8 arm's `reason` text are identical across the three CI legs — ordering the emitted installations deterministically, not only stabilising an error string (§6).
- A slot contributes **at most one** entry to `installations` and **at most one** to `issues`, so the two vectors' lengths are a direct function of the probe table.

## 8. Error paths: the `ScanIssue` taxonomy

**No new `ScanIssue` variant, no new field.** Every slot is resolved independently and a failure in one **never** returns early and **never** skips another — the T6 isolation discipline transposed, and a required fixture (§10).

| Case | installation | severity | `path` | `reason` |
|---|---|---|---|---|
| npm package dir absent (its `package.json` too) | — | **`Warning`** | `Some(<package dir>)` | `{Client} (npm) not detected` |
| desktop `claude-code` dir absent | — | **`Warning`** | `Some(<claude-code dir>)` | `Claude Code (desktop) not detected` |
| npm package dir **exists**, `package.json` absent | — | `Error` | `Some(<package.json>)` | `could not read package.json: {io}` |
| `read_to_string` fails otherwise (permissions, non-UTF-8 content) | — | `Error` | `Some(<package.json>)` | `could not read package.json: {io}` |
| `jsonc::parse` fails (syntax error, BOM) | — | `Error` | `Some(<package.json>)` | `could not parse package.json: {err}` |
| parsed root is not an object | — | `Error` | `Some(<package.json>)` | `package.json is not a JSON object` |
| `"version"` absent, **or the file was empty** (V5) | — | `Error` | `Some(<package.json>)` | `package.json has no "version" string` |
| `"version"` present but not a string | — | `Error` | `Some(<package.json>)` | `package.json has no "version" string` |
| `"version"` present as an **empty** string | — | `Error` | `Some(<package.json>)` | `package.json has no "version" string` |
| desktop: 0 candidates | — | `Error` | §6 | §6 |
| desktop: a candidate whose name is not valid UTF-8 | — | `Error` | §6 | §6 |
| desktop `read_dir` fails for any other reason | — | `Error` | `Some(<claude-code dir>)` | `could not read the Claude Code desktop directory: {io}` |
| platform is `Unsupported` | — | `Warning` | `None` | `client installation detection is not implemented on this platform` |
| home directory unresolvable | — | *not a `ScanIssue`* | — | `ScanError`, T4D §7.2, unchanged and untouched |

**The three collapsed rows are deliberate.** Missing key, wrong type and empty string all produce the same `Error` and the same reason, because the product statement is identical — *no usable version was found in this file* — and three distinct strings would suggest to a consumer that the distinction matters. It does not; an installation with an empty `version` is explicitly rejected (proposal Q3).

**No `escalate` function** — T6D §5.6's reasoning applies unchanged: T7 has no leaf reader returning a conservative severity floor. Every `ScanIssue` is constructed where the caller context is already in hand.

## 9. CA-7: two Claude Code installations, never merged

The two Claude Code slots share `ClientKind::ClaudeCode` and see the same user component roots (`alcance-poc-vertice.md:104`). The **only** thing that distinguishes them in the PoC is `version` and `path`.

> **Decision: they are two independent probe-table entries resolved by the same code path, and nothing anywhere de-duplicates by `client`. `ClientInstallation` has no identity and no `Eq`-based collapsing is performed.**

This is structural, not a rule to remember: a `for` loop over `[InstallProbe; 3]` pushing at most one `ClientInstallation` per npm probe, and one per desktop candidate (§6), **cannot** merge two entries. There is no map keyed by `ClientKind` anywhere in the module — an implementer who introduces one (e.g. `BTreeMap<ClientKind, _>` "for determinism") breaks CA-7, and the two-different-versions fixture (§10) fails immediately. Grouping "one client, two installs" for presentation is a **T11** decision (proposal Q4); the core reports every row independently.

**The reference machine now demonstrates CA-7 with three Claude Code installations, not two**, strengthening this decision from a hypothetical to an observed fact. Direct inspection on 2026-08-19 (§0, U1/U2) found: `Claude Code (npm)` at `2.1.140`, `Claude Code (desktop)` at `2.1.229`, and `Claude Code (desktop)` at `2.1.234` — three independently reported `ClientInstallation` values, all `ClientKind::ClaudeCode`, none merged. The two desktop entries alone are the case U2 had declared impossible; they are simultaneously present on the machine CA-7 was written against.

## 10. Fixture architecture

New tree, **no reuse of T4/T5/T6 homes** — the seam is `home` as a parameter, inherited unchanged. No test reads the author's machine, sets or reads an environment variable, or invokes `claude`/`opencode`.

```
crates/vertice-core/tests/fixtures/installations/          # NEW; grouping dir, never itself a home
  (every home below is a synthetic %USERPROFILE%, i.e. contains AppData/Roaming/…)
├── nothing/                    .gitkeep only
│                                 → 0 installations, 3 Warnings, each with its probe path  [CA-11 PIN]
├── two-claude/                 npm @anthropic-ai/claude-code/package.json "1.0.100"
│                               + Claude/claude-code/2.5.3/
│                                 → exactly 2 installations, both ClaudeCode, DIFFERENT versions,
│                                   different paths, 0 issues                    [CA-7 PIN, NON-NEGOTIABLE]
├── opencode-npm/               opencode-ai/package.json "0.4.2"
│                                 → 1 installation, client OpenCode
├── isolation/                  MALFORMED claude-code/package.json
│                               + healthy opencode-ai/package.json + healthy desktop dir
│                                 → 1 Error on the malformed slot; the OTHER TWO still
│                                   detected and reported                        [NON-NEGOTIABLE]
├── no-version-key/             package.json valid JSON, no "version"      → 0 for that slot, 1 Error
├── version-not-a-string/       "version": 3                                → 0 for that slot, 1 Error
├── package-json-empty/         package.json is a zero-byte file            → 0 for that slot, 1 Error
│                                 → same branch as no-version-key (V5's empty-input edge, §8's collapsed row)
├── package-json-unreadable/    package.json contains non-UTF-8 bytes
│                                 → 1 Error (`could not read package.json: {io}`), distinct from the
│                                   parse-failure fixture above — `read_to_string` fails before `jsonc::parse` runs
├── npm-dir-no-package-json/    package dir present, no package.json
│                                 → 1 ERROR, NOT a not-detected Warning          [NON-NEGOTIABLE]
├── desktop-empty/              Claude/claude-code/.gitkeep, no subdirectory → 0, 1 Error (§6)
├── desktop-two-versions/       Claude/claude-code/{1.0.0,2.0.0}/
│                                 → 2 installations, both ClaudeCode desktop, DIFFERENT versions,
│                                   distinct paths, 0 issues                    [CA-7 PIN, §6]
└── reference/                  npm Claude Code + two desktop versions + OpenCode npm, realistic
                                 package.json shapes, mirroring the verified reference machine (§0)
                                  → 4 installations, 0 issues
```

**`npm-dir-no-package-json` and `isolation` are non-negotiable** because they pin the two claims that make CA-11 worth anything: a *broken* install must not read as *absent*, and one broken slot must not silence the other two.

**`two-claude` must carry two different versions and must exist and FAIL before the assembly code is written** (`strict_tdd: true`). A fixture with two identical versions passes under a merging implementation and proves nothing.

**The `.gitkeep` trap** (T4D §8, inherited). `nothing/` and `desktop-empty/Claude/claude-code/` are empty directories git cannot track. Losing `desktop-empty`'s `.gitkeep` turns an "empty desktop directory" test into a "desktop absent" test — here the severities differ (`Error` vs `Warning`) so it would fail loudly rather than pass silently, but a dedicated existence test named for its own failure is added anyway, per precedent. Note the interlock: that `.gitkeep` is a **file**, so §6's "a candidate must be a directory" rule is what makes the fixture assert what it claims to.

`.gitattributes` line 2 already scopes `-text` to `crates/vertice-core/tests/fixtures/**`; no change. `package-json-unreadable/`'s `package.json` is binary content, the same situation as `crates/vertice-core/tests/fixtures/frontmatter/non-utf8-content/SKILL.md`, which line 2's blanket `-text` does not fully cover — that file gets its own explicit `binary` line (`.gitattributes:5`) to also suppress diff/merge attempts. `sdd-apply` MUST add the equivalent dedicated line for `crates/vertice-core/tests/fixtures/installations/package-json-unreadable/AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/package.json` when the fixture is created. Fixture paths are built from `env!("CARGO_MANIFEST_DIR")` with per-segment `push`, never `"/"`-joined literals (`tests/skill_scanner.rs:23-30`).

## 11. Per-OS paths

Every path is `home` plus hardcoded segments pushed one at a time, so separators are correct on all three CI legs. **No OS convention is consulted** — `%APPDATA%`, `dirs::config_dir()`, `$XDG_*` are all deliberately unused (`plan-desarrollo-poc.md:179`).

| Slot | Windows (**U1 — verified 2026-08-19**, §0) | macOS | Linux |
|---|---|---|---|
| Claude Code npm | `<home>\AppData\Roaming\npm\node_modules\@anthropic-ai\claude-code` | **T16** | **T16** |
| Claude Code desktop | `<home>\AppData\Roaming\Claude\claude-code\<version>` | **T16** | **T16** |
| OpenCode npm | `<home>\AppData\Roaming\npm\node_modules\opencode-ai` | **T16** | **T16** |

**T7 is more platform-fragile than T6 was**, and the table says why: T6's OpenCode config uses the XDG layout on *every* OS, so one table served all three. These are `%APPDATA%`-shaped paths with no reason to exist anywhere else, so macOS and Linux need genuinely different tables — which is precisely why §5.2 keeps `Unsupported` honest instead of pretending the Windows table generalizes.

**U1 is the defining risk of this change** and §4 is its mitigation: a wrong path presents as *a named path that was not found*, carried in a typed `ScanIssue.path`, not as an unexplained empty list. `@anthropic-ai` contains an `@` and `claude-code` a hyphen; both are ordinary path segments, but a fixture whose directory name is mistyped fails as "not detected" and looks like a passing CA-11 test — so the `two-claude` and `reference` fixtures assert **installation counts**, never merely "no crash".

## 12. File changes, testing, rollout

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/installations.rs` | **Create** | §5 — probe table, platform seam, per-slot resolution, version extraction, assembly |
| `crates/vertice-core/src/lib.rs` | Modify | one line: `pub mod installations;` |
| `crates/vertice-core/tests/fixtures/installations/**` | **Create** | §10, twelve synthetic homes |
| `crates/vertice-core/tests/client_installations.rs` | **Create** | CA-driven integration suites |
| `crates/vertice-core/src/roots.rs` | **Unchanged** | §5.3 — not even a visibility change |
| `crates/vertice-core/src/{model/,jsonc.rs,skills.rs,agents.rs,opencode_agents.rs,frontmatter.rs,yaml.rs}` | **Unchanged** | §2, §5.1 |
| `frontend/src/bindings/**` | **Unchanged** | no `TS` type added; drift gate green with **no regeneration** |
| `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Unchanged** | no new dependency; `jsonc.rs` reused |
| `crates/vertice-app/**`, `frontend/src/**`, `.github/workflows/**` | **Unchanged** | no IPC, no command, no capability, no MSRV change |

`strict_tdd: true`. Fixtures and RED tests land before implementation.

| Layer | What | How |
|---|---|---|
| Unit — probe table | `windows_install_probes` returns exactly 3 entries, in a fixed order, with the §11 suffixes and the right `VersionSource`; identical structure for two different `home` values | `#[cfg(test)]` in `installations.rs`, no disk |
| Unit — version extraction | `"version"` string → `Some`; absent / non-string / empty / empty document → `None`; no regex, no DTO | `#[cfg(test)]`, on `JsonValue` literals |
| Integration — CA-7 | `two-claude/` → exactly 2 `ClientInstallation`, both `ClaudeCode`, versions `1.0.100` and `2.5.3`, distinct paths, desktop path = the **versioned subdirectory** | `tests/client_installations.rs`, via `scan_for(home, Windows)` |
| Integration — CA-11 | `nothing/` → 0 installations and exactly 3 `Warning` issues, each carrying its probe path; **no `Error`** | idem |
| Integration — isolation | `isolation/` → exactly 1 `Error` on the malformed slot **and** the other two installations present | idem |
| Integration — broken ≠ absent | `npm-dir-no-package-json/` → an `Error`, and **zero** `Warning` | idem |
| Integration — empty package.json | `package-json-empty/` → 0 for that slot, 1 `Error`, same reason string as `no-version-key/` (§8's collapsed row, V5's empty-input edge) | idem |
| Integration — unreadable package.json | `package-json-unreadable/` → 1 `Error` (`could not read package.json: {io}`), and **zero** `Warning`; distinct fixture from the parse-failure case in `isolation/` — this one fails at `read_to_string`, before `jsonc::parse` ever runs | idem |
| Integration — desktop empty | `desktop-empty/` → 0 installations, 1 `Error`; never a phantom entry | idem |
| Integration — desktop N installations | `desktop-two-versions/` → exactly 2 `ClientInstallation`, both `ClaudeCode` desktop, DIFFERENT versions, distinct paths, **0 issues** — never merged, never an anomaly (§6, CA-7 pin) | idem |
| Integration — happy path | `reference/` → 4 installations (npm Claude Code, two desktop Claude Code versions, npm OpenCode), **0 issues** | idem |
| Platform seam | `scan_for(home, Unsupported)` → 0 installations, exactly 1 `Warning`, `path: None`; every fixture suite runs on **all three CI legs** | idem — §5.2's payoff |
| Entry point — `scan` dispatch | `scan(home)` (not `scan_for`) on `reference/`, exercising `HostPlatform::current()`'s `cfg!(target_os = "windows")` check directly: on the **Windows** CI runner it MUST match `scan_for(home, Windows)` — 4 installations, 0 issues; on the **Linux/macOS** legs, where `current()` evaluates to `Unsupported`, it MUST match `scan_for(home, Unsupported)` — 0 installations, exactly 1 `Warning`, `path: None` | `tests/client_installations.rs`, via `scan(home)` (unconditional call); the assertion branches on `cfg!(target_os = "windows")`, mirroring §5.2's own dispatch |
| Contract | No `ClientInstallation` ever carries an empty `version`; installations + issues counts are a function of the probe table | assertions over `InstallationScan` |
| Determinism | Two consecutive scans of `reference/` and of `desktop-two-versions/` yield byte-identical vectors | idem |
| Regression | T4/T5/T6 suites and the `roots.rs` unit suite stay green **with no edits at all** | existing suites, unmodified |
| Tripwire | `desktop-empty/AppData/Roaming/Claude/claude-code/` still exists on disk | §10 |
| Read-only (CA-16) | A full scan leaves `reference/` byte-for-byte unchanged | `fixture_tree_bytes` before/after (`tests/skill_scanner.rs:234-258`) |
| Invariant | No `dirs`/`directories`, no `std::env`, no `regex`, no second JSON crate, no `tauri` in the new module | structural + `cargo deny check bans licenses` |

**Chained-PR seam** (forecast ~445–685 lines, budget risk Medium-High). Split so the path table is reviewable independently of the extraction logic:

1. **PR 1 — fixtures, probe table, platform seam, RED tests (~200–300 lines).** The whole `tests/fixtures/installations/**` tree with its `.gitkeep` tripwire, `HostPlatform`, `windows_install_probes`, `InstallationScan` with `scan_for` returning an empty scan, and the unit suite for the probe table. Compiles and is green on merge; the CA-7/CA-11 integration tests land RED in PR 2's first commit.
2. **PR 2 — resolution, version extraction, assembly (~245–385 lines).** `resolve`, both version sources, the §8 taxonomy, and the integration suites, with RED-before-GREEN preserved by commit order — `two-claude` failing first.

Splitting at "tests then implementation" is rejected for T5D's reason: a test naming `vertice_core::installations` fails to *compile* rather than fails an assertion, which is a poor RED. `sdd-tasks` owns the final slicing.

**Rollback.** Delete `installations.rs`, `tests/client_installations.rs` and `tests/fixtures/installations/`; revert one `lib.rs` line. Nothing else — no dependency, no lockfile movement, no `deny.toml` entry, no `model/` edit, no binding regeneration, no `roots.rs` change, no IPC surface. **Migration: none.**

## 13. Open questions

- [x] **"Not detected" representation** — `ScanIssue` (option A), with option C's fixed-size slot array internally. **Option B rejected**; `model/` and `frontend/src/bindings/` untouched, `domain-model` is **not** a Modified Capability, `sdd-spec` needs no revision. §2.
- [x] **Severity for a not-detected client** — `Warning`, because CA-11 says "not an error" and because `Error` would paint every single-client machine red. §4.
- [x] **`reason` grammar** — `{Client} ({kind}) not detected`; the probed path lives in the typed `ScanIssue.path`, never in the string; `reason` stays a non-parsable developer diagnostic. §4.
- [x] **T7 emits no `SearchRoot`** — confirmed; `InstallationScan` has no `roots` field, and that absence is what forces §4. §3.
- [x] **Names** — module `installations`, `InstallationScan` as a fourth distinct type, `scan` / `scan_for`, private `InstallProbe` / `InstallKind` / `VersionSource`. §5.1.
- [x] **`roots::probe` stays private** — `roots.rs` is not modified at all; a 3-line local `exists` avoids importing search-root vocabulary. §5.3.
- [x] **Implausible desktop directory name** — accepted verbatim; no plausibility predicate, no name heuristics. §6.
- [x] **Platform seam** — one probe-table function per OS, `cfg!` (expression, not attribute) as the single dispatch point so the Windows table is exercised on all three CI legs. §5.2.
- [x] **Desktop version directory cardinality** — **N candidates yield N `ClientInstallation` values, never merged, never an anomaly.** Refuted on 2026-08-19 by direct inspection of the reference machine, which found `2.1.229` and `2.1.234` coexisting; the earlier "≥2 is impossible" reading (a user decision, not an observation) is superseded. §0 (U2), §6.
- [x] **The three Windows paths themselves (U1)** — **verified on 2026-08-19 by direct inspection of the reference machine**: all three paths exist, and `AppData\Roaming\npm\node_modules\@anthropic-ai\claude-code\package.json` / `AppData\Roaming\npm\node_modules\opencode-ai\package.json` report `2.1.140` / `1.17.20`, matching `alcance-poc-vertice.md:71-102`. §0.
- [ ] **macOS and Linux path tables** (§11) — **T16**. `Unsupported` is the honest placeholder until then.
- [ ] **Oracle contrast** (`claude --version`, `opencode --version`) — **T16**, manual, never automated. It is also the only detector for §6's stray-directory cost, which is now worse under the N-installations rule (§6's accepted-cost paragraph).
- [ ] **Whether a stray non-version directory exists under `Claude/claude-code/` in practice** (§6) — no fixture can settle this. **T16.**
- [ ] **Whether T10/T11 needs a typed absence channel** (§2) — if it does, option B is the retrofit and it is cheap, because `resolve` already computes the per-slot outcome as a closed value. **T10/T11.**
- [ ] **How the UI distinguishes a not-detected `Warning` from a metadata `Warning`** (§4) — the discriminator is (`severity`, absence from `installations`), never the string. **T10/T11.**
- [ ] **Grouping "one client, two (or more) installs" in the UI** (§9) — **T11**. The core reports every row independently and will not change.
