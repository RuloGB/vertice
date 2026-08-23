# Design: Add Codex Client Support

> Trace: **T7** (client detection), replaying **T4** (skill roots) and **T5/T6** (per-client agent adapter). Addresses **CA-11**, **CA-7**, **CA-12**, **CA-8**. Must-not-regress **CA-2/CA-3/CA-4**, **CA-6**, **CA-14**. Bounded by **CA-16**, **CA-17**.
> Inherits, unchanged: `archive/2026-08-19-client-installation-detection/design.md` (**T7D**) §5.2 (`cfg!` platform seam), §6 (verbatim version directories), §9 (never merge); `archive/2026-08-23-fix-windows-claude-desktop-probe/design.md` (**P30D**) decision 1 (slot vocabulary, `InstallSlot` stays private); `archive/2026-08-23-report-client-presence-as-status/design.md` (**P32D**) §2 (`ClientPresence`), §3 (`flatten_presence` is the only producer), §7 (`Detected` ≠ "usable").
> `rules.design` coverage: core data model (§2), core/Tauri isolation and the CLI pathway (§1), IPC contract surface (§9), per-OS paths (§8), `ScanIssue` taxonomy and error paths (§7).
> Scope guard: this design closes the proposal's "Committed to resolving in `sdd-design`" list. It writes **no spec and no task**; `specs/` is owned by the parallel `sdd-spec` phase and is not touched here.

## 0. What is verified, and what is inherited on trust

| # | Statement | Basis |
|---|---|---|
| **V1** | The `toml` crate is `1.1.4+spec-1.1.0`, `MIT OR Apache-2.0`, `rust-version = 1.85`. The workspace floor is **1.88** (`Cargo.toml:8`, `.github/workflows/ci.yml:44` `MSRV`, `rust-toolchain.toml:2` pinning 1.97.1). 1.85 ≤ 1.88 ⇒ **no MSRV change, no `deny.toml` change** | `cargo info toml` run on the reference machine, 2026-08-23 |
| **V1b** | The **whole** dependency tree clears the floor and the licence allow-list: `toml` 1.85, `toml_datetime` 1.85, `serde_spanned` 1.85, `toml_parser` 1.85 — all `MIT OR Apache-2.0` — and `winnow` 1.65.0 / MIT. `serde_core` arrives via `serde`, which the workspace already depends on. `default-features = false, features = ["parse", "serde"]` resolves cleanly to **11 locked packages** | `cargo tree` + per-crate `cargo info` against crates.io, 2026-08-23 |
| **V1c** | **`toml_parser` is a real crate and a transitive dependency of `toml`** (`toml` → `toml_parser` 1.1.3 → `winnow`). It is therefore unusable as a rename alias for the façade | Same `cargo tree` |
| **V2** | `~/.codex/packages/standalone/releases/` is a **real directory**, and its children (e.g. `0.149.0-x86_64-pc-windows-msvc`) are **real directories** — `symlink_metadata` reports `is_dir = true, is_symlink = false`, and `read_dir` reports the same through `DirEntry::file_type()` | Compiled `std::fs` probe run against the real install, 2026-08-23 |
| **V3** | `AppData/Local/Programs/OpenAI/Codex/bin` and `~/.codex/packages/standalone/current` are **symlinks** (`symlink_metadata`: `is_symlink = true`; `metadata`: `is_dir = true`; `read_link`/`canonicalize` resolve into `releases/<version>-<triple>`) | Same probe. **Weaker for the `AppData` half** — that shell may see an MSIX-redirected view. The `~/.codex/...` half is unaffected, and §3 depends only on that half |
| **V4** | `codex --version` prints `codex-cli 0.149.0`, matching the release directory's version prefix exactly. `codex agents` lists **sessions**, `codex debug` exposes only `models`/`app-server`/`prompt-input`. **There is no component-listing oracle for Codex** | CLI run during exploration and verification |
| **V5** | `skills::walk_one` returns silently on a `NotFound` scan path (`skills.rs:66-68`) — an absent root produces **zero** `SkillScan` issues. The `NotFound` *Warning* comes from `scan::append_missing_root_issues` (`scan.rs:57-68`), never from the adapter | Read `skills.rs`, `scan.rs` |
| **V6** | `consolidate::root_order_matches_the_roots_module_in_order` (`consolidate.rs:184-200`) builds its expectation as `skill_roots ++ agent_roots ++ opencode_agent_root`, in that concatenation order | Read `consolidate.rs` |
| **V7** | The 69/25 pins are computed as `skills::scan(reference).components.len() + .issues.len()` (`skill_scanner.rs:260-275`), i.e. from the **fixture tree contents**, not from the root count | Read `skill_scanner.rs` |
| **V8** | `scan.rs` orchestrator tests pin `roots_scanned.len() == 6` twice, `installations.len() == 3`, `components.len() == 10`, `client_presence.len() == 3`, and `report.issues.is_empty()` on `complete` | Read `scan.rs:90-163` |
| **U1** | Codex's on-disk layout (`~/.codex/skills/<n>/SKILL.md`, `~/.codex/agents/<n>.toml`, the extra `SKILL.md` keys, `version.json`'s `latest_version`) | Inherited from `exploration.md`, verified there on the reference machine. Not re-probed here |
| ~~U2~~ | The `toml` crate's **transitive** MSRV | **Closed by V1b.** The caution was methodologically right — V1 alone reads the façade only — but the answer does not change: no crate in the tree exceeds 1.85 |

**Correction to the proposal, carried through this design.** The proposal states "Codex root ids are **appended last** in `ROOT_ORDER`". Given V6, appending `codex-skills` to `skill_roots` places it at **index 3 of 8**, *before* `claude-agents` — not last. The precedence guarantee survives anyway, but for a different and stronger reason than "it is last"; see §6.

## 1. Technical approach

Three independent additive slices over one closed enum. Nothing existing is refactored.

```
                                    vertice-core                      (no tauri; one new dep, core-only)
 frontend ──IPC──> vertice-app ──>  ├── model/installation  + ClientKind::Codex        (§2, the ONLY model edit)
 future vertice-cli ───────────>    ├── roots        + codex-skills, + codex_agent_root (§6)
                                    ├── skills       UNCHANGED — already client-agnostic
                                    ├── toml         NEW seam, sole importer of the parser (§5)
                                    ├── codex_agents NEW adapter, flat read_dir over *.toml (§4)
                                    ├── installations + CodexStandalone slot + ReleaseDirectoryName (§3)
                                    ├── consolidate  ROOT_ORDER 6 -> 8; merge logic UNCHANGED (§6)
                                    └── scan         + one adapter in the concatenation

 installations::scan_for(home, Windows)
   slot CodexStandalone -> candidate: <home>/.codex/packages/standalone/releases
        -> read_dir, keep directories, sort byte-wise
        -> per directory: "<version>-<triple>" -> version            (§3.2)
        -> ClientPresence { label, probed_paths, status, installations: 0..N }   (never merged, CA-7)
```

**CLI isolation is unchanged and is the reason for the `toml.rs` seam's shape.** `vertice-core` gains one dependency (`toml`, aliased `toml_seam` — §5.2) and still imports nothing from `tauri`; `deny.toml`'s `[bans]` (parent-allow-listed to `vertice-app`) keeps that mechanical. Every new entry point takes `home: &Path` explicitly and reads no environment, so a future `vertice-cli` calls exactly what `vertice-app` calls.

## 2. Core data model changes

| Type | Change |
|---|---|
| `ClientKind` | **`Codex` variant added.** The enum stays closed, never `#[non_exhaustive]` — its own doc (`installation.rs:20-23`) anticipates exactly this growth. `installation.rs`'s doc comment updates from "(Claude Code, OpenCode)" to name three |
| `Component`, `ComponentId`, `Location`, `Scope`, `SearchRoot`, `SearchRootId`, `SearchRootKind`, `SearchRootStatus`, `ScanReport`, `ScanIssue`, `IssueSeverity`, `ClientInstallation`, `ClientPresence`, `ClientPresenceStatus` | **Unchanged.** A diff in any of their bindings means something leaked |

Consequences worth stating rather than inheriting:

- **No client discriminator on `Component`.** `ComponentId::derive(kind, name)` is untouched, so a Codex skill named `shared` merges with a Claude Code skill named `shared` into one `Component` with two `Location`s (user decision, 2026-08-23). Provenance stays fully visible per `Location.root`.
- **`model/`'s import allow-list is untouched.** No new file under `model/`, therefore no new import to audit.
- **`SearchRootKind` needs no `Codex` notion.** The Codex agent root is `SearchRootKind::Agent` and the Codex skill root is `SearchRootKind::Skill`, because the kind describes *what is found*, not *who ships it* (`location.rs:40-42`: "one root produces N components, not one client has N components").

## 3. Decision 1 — the version source

### 3.1 The variant

`VersionSource` gains **`ReleaseDirectoryName`**. `InstallSlot::CodexStandalone.version_source()` returns it; `resolve_slot` dispatches it to a new `resolve_codex_slot`.

| Option | Consequence | Decision |
|---|---|---|
| Reuse `VersionSource::DirectoryName` | Its contract is "the directory's bare name **is** the version" (`install_from_version_dir:542-563`). Codex's name is `<version>-<triple>`. Reusing it either corrupts the Claude semantics or forces a per-slot `if` inside a shared resolver — a version field displayed as fact, decided by a hidden branch | **Rejected** |
| Reuse `PackageJson` | There is no `package.json` anywhere in the Codex tree (U1) | **Rejected** |
| Read `~/.codex/version.json` | The field is `latest_version` — an update-availability cache. Reporting it displays a version the user does not have, and drags the PoC into the excluded update-status feature (`alcance-poc-vertice.md:13`) | **Rejected, and pinned** (§10) |
| **New `ReleaseDirectoryName` variant + its own resolver** | One variant, one resolver, zero behaviour change for the two existing slots. `VersionSource` is private and closed and grows by variant addition exactly as `InstallSlot` does | **Chosen** |

`resolve_codex_slot` is a **structural sibling** of `resolve_bundled_slot`, not a parameterization of it. Rejected alternative: pass a `fn(&str) -> Option<String>` extractor into a shared resolver. It would save ~35 lines and would make the two slots' `ScanIssue` reasons and failure classes co-vary — Codex has a failure class (unparseable release name) the bundled slot does not have, so the shared function would grow a slot-dependent branch anyway. `agents.rs:8-11` / design §5.4's "deliberately separate, not a shared abstraction" is the house precedent, and this is the same trade.

### 3.2 The extraction rule — decisive, and prerelease-proof

```
release directory name := "<version>-<target-triple>"
```

**Rule: strip the longest suffix that is `-` followed by an exact member of a closed `CODEX_TARGET_TRIPLES` table. What remains, if non-empty, is the version.**

```rust
/// Target triples Codex publishes standalone releases for. MANUAL
/// MAINTENANCE, exactly like `agents::EMBEDDED_CLAUDE_AGENTS`: a triple
/// OpenAI adds is invisible to Vertice until this table is extended.
/// Windows-only for T7; T16 adds the macOS/Linux triples here and nowhere
/// else.
const CODEX_TARGET_TRIPLES: [&str; 2] = [
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
];
```

| Input | Output | Why |
|---|---|---|
| `0.149.0-x86_64-pc-windows-msvc` | `0.149.0` | The observed shape (V2) |
| `0.150.0-rc.1-x86_64-pc-windows-msvc` | `0.150.0-rc.1` | **The case that kills "split on the first `-`"**, which would yield `0.150.0` and silently report a prerelease as a release |
| `0.151.0-riscv64-unknown-linux-gnu` | *no match* → §3.3 | Unknown triple; never guessed |
| `x86_64-pc-windows-msvc` | *no match* → §3.3 | Suffix strip leaves an **empty** remainder, which is rejected explicitly |
| `nightly` | *no match* → §3.3 | No `-`, no triple |

Rejected alternatives: **split on the first `-`** (wrong on any prerelease tag, and wrong *silently*, in a field the UI shows as fact); **take the last N dash-separated fields as the triple** (N is 4 for `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` but 3 for `aarch64-apple-darwin`, so it breaks the moment T16 lands); **any regex** (forbidden by `AGENTS.md`, and it would not disambiguate `-rc.1` from `-x86_64` any better than the table does). The table costs one line per new triple and fails **loudly** (§3.3) instead of producing a wrong string.

The rule is a pure `fn split_release_dir_name(name: &str) -> Option<&str>` with no I/O, unit-testable without a fixture, mirroring `install_from_version_dir`'s "factored out so the branch is directly testable" precedent.

### 3.3 An unparseable release directory name

| Option | Consequence | Decision |
|---|---|---|
| Carry the composite name verbatim as the version | The UI would print `0.151.0-riscv64-unknown-linux-gnu` in a version cell. Not *false*, but it silently normalizes a parse failure into data, and there is no oracle (V4) that would ever catch it | **Rejected** |
| `ClientInstallation` with an empty version | A phantom entry; rejected by T7 and by the existing "no `ClientInstallation` with an empty version" contract test | **Rejected** |
| Report the slot as `NotDetected` | A lie: the directory is there. The exact class of error CA-11 forbids | **Rejected** |
| **`Detected`, no installation for that directory, one `Error` `ScanIssue` carrying its path** | Truthful on both axes: present *and* not understood. Structurally identical to the settled `npm-dir-no-package-json` row (P32D §7: `Detected` + 0 installations + 1 `Error`). Sibling release directories in the same `releases/` still resolve | **Chosen** |

This closes the proposal's question 4 (*"detected, version unknown" or "not detected"*): **detected, version unknown, plus a visible `Error`**. Given V4 (no oracle), this failure mode is the only drift signal Vertice will ever get for Codex's release naming — making it silent would remove the one alarm available.

### 3.4 Where the probe points

The slot's single candidate is `<home>/.codex/packages/standalone/releases`, enumerated one level deep. **No symlink is ever followed, created, or depended on.**

| Option | Consequence | Decision |
|---|---|---|
| Probe `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin` and follow the chain | It is a symlink (V3), and it is the *weakest* observation in §0. Following it needs `read_link`/`canonicalize`, whose Windows junction-vs-symlink semantics differ, whose `\\?\` verbatim prefix leaks into `ClientInstallation.path`, and which cannot be exercised by a committed fixture (§10) | **Rejected** |
| Resolve `~/.codex/packages/standalone/current` | Also a symlink (V3), and it names **one** release — structurally incapable of expressing the 1..N that CA-7 requires | **Rejected** |
| **Enumerate `~/.codex/packages/standalone/releases/` directly** | Real directory, real directory children (V2). Works verbatim with the module's existing non-following `symlink_metadata`/`read_dir`/`DirEntry::file_type` helpers. Maps 1:1 onto `resolve_bundled_slot`'s 1..N candidate shape (CA-7). Needs **no symlink in any fixture** (§10) | **Chosen** |

`exists()` (`installations.rs:334-340`) treating non-`NotFound` errors as "present" is noted but **not relied upon**: every path this design probes is a plain directory.

**Recorded limitation, not scope.** A Codex installed some other way (e.g. an npm-distributed `@openai/codex`) has no `packages/standalone/releases` tree and yields `NotDetected` for this slot — while its `~/.codex/skills` and `~/.codex/agents` components are still inventoried. That asymmetry is honest (the slot is *a place we look for installations*) but it is a real gap; probing `~/.codex` itself instead was rejected because a stale config directory would then report `Detected` + `Error` for a user who has no Codex at all.

## 4. Decision 2 — the slot and its label

`InstallSlot` gains **`CodexStandalone`**, appended after `OpenCodeNpm` so probe order and therefore `ClientPresence` record order stay stable for the existing three.

```rust
InstallSlot::CodexStandalone => "Codex CLI (standalone)",
```

House grammar is `{product}[ CLI] ({distribution})`: `"Claude Code CLI (npm)"`, `"Claude Code (bundled in Claude Desktop)"`, `"OpenCode (npm)"`. `"Codex CLI (standalone)"` follows it exactly — `CLI` because the shipped binary identifies itself as `codex-cli` (V4), `(standalone)` because that is OpenAI's own name for this distribution channel (`packages/standalone/`). Rejected: `"Codex (OpenAI)"` (vendor, not distribution — breaks the grammar), `"OpenAI Codex CLI"` (no distribution parenthetical, so a future npm slot would have no way to differ). The label is core-owned, never localized, and rendered verbatim in both locales (P32D §6).

`InstallSlot::client()` maps `CodexStandalone -> ClientKind::Codex`. That match and the `ClientKind` binding are the only two exhaustive-match sites in core, so the blast radius stays compiler-enforced.

## 5. Decision 3 — the TOML seam

### 5.1 The crate

**`toml` 1.1.4, declared exactly as:**

```toml
toml = { version = "1", default-features = false, features = ["parse", "serde"] }
```

`default-features = false` drops the crate's `display`/serializer half, so the write API is not even linked — read-only expressed in the manifest, not merely in convention. The resolved tree is 11 locked packages and every crate in it clears the 1.88 floor and the `deny.toml` allow-list (V1b), **transitively, not just at the façade**. So: no MSRV edit, no `deny.toml` edit, and **no special MSRV tripwire** — the existing `msrv` CI job and `cargo deny check bans licenses` cover this like any other dependency.

Rejected: `toml_edit` (its value proposition is write-preserving round-trips; this crate never writes), and a hand-rolled parser (`developer_instructions` is a triple-quoted multiline string — the exact class of bug `AGENTS.md` already documents for YAML block scalars).

The one enforceable guarantee that survives any future feature-flag change is structural, not manifest-level: **the seam exposes no serialization function** (§5.3).

### 5.2 The naming problem, and the fix

The seam module wants to be `src/toml.rs` (house pattern: `yaml.rs`, `jsonc.rs` are named for the format). But the `yaml`/`jsonc` seam invariants work *because the crate name differs from the module name* — `tests/yaml_seam_invariant.rs:66` greps for the literal `serde_norway::`. With a crate named `toml` and a module named `toml`, every legitimate call site (`crate::toml::from_str`) contains the substring `toml::`, and the textual invariant becomes unwritable.

**Decision: rename the dependency in `crates/vertice-core/Cargo.toml` to `toml_seam`.**

```toml
toml_seam = { package = "toml", version = "1", default-features = false, features = ["parse", "serde"] }
```

The seam is `src/toml.rs` and is the only file that may name `toml_seam`. `tests/toml_seam_invariant.rs` is then a line-for-line analogue of the YAML one, grepping `use toml_seam` / `toml_seam::` and excluding only `src/toml.rs` (parent == `src/` **and** file name == `toml.rs`, mirroring `is_yaml_module`).

**The alias name is load-bearing and was chosen against two rejected candidates.**

| Alias | Consequence | Decision |
|---|---|---|
| `toml_parser` | **`toml_parser` is a genuinely different crate, and it is already in this graph** — `toml` depends on it (V1c). The alias would make one name mean the façade inside `vertice-core` while the real `toml_parser` sits one level down, and the invariant test would be grepping a name that legitimately belongs to something else. It defeats the alias's own purpose | **Rejected** |
| `codex_toml` | Collision-free, but it names the seam after one consumer. The same reason the module is not `codex_toml.rs`: a format seam named for a client invites a second, per-client TOML seam, which is exactly the containment this test exists to prevent | **Rejected** |
| **`toml_seam`** | Cannot collide — no crate in the resolved tree (or plausibly on crates.io) carries it, and it reads as **project vocabulary**, not as a crate name. "Seam" is this codebase's own established word for the pattern (`yaml.rs`, `jsonc.rs`, `tests/yaml_seam_invariant.rs`), so a reader meeting `toml_seam::de::Error` inside `src/toml.rs` immediately knows it is the aliased dependency and not a sibling module | **Chosen** |

Also rejected: dropping the invariant entirely — it is the only mechanical guarantee that the parser crate stays swappable.

Same house caveat, restated: the check is **textual**. Any module documenting this constraint MUST do so in prose only, never writing `use toml_seam` or `toml_seam::` in a doc comment.

### 5.3 Public surface — a mirror of `yaml.rs`

```rust
//! TOML deserialization seam.
//!
//! The ONLY module in `vertice-core` allowed to import the TOML parsing
//! crate directly (declared under a renamed dependency alias so the
//! containment test can be textual). Every other module MUST go through
//! [`from_str`]. Read-only by construction: no serialization function is
//! exposed, so no caller can acquire a write capability through this seam.

/// Error returned when TOML input cannot be parsed or deserialized.
#[derive(Debug, thiserror::Error)]
pub enum TomlError {
    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml_seam::de::Error),
}

/// Deserialize a value of type `T` from a TOML string.
pub fn from_str<T: DeserializeOwned>(input: &str) -> Result<T, TomlError>;
```

Exactly `yaml.rs`'s shape: one error enum with one `#[from]` variant, one generic function, no options struct (`jsonc.rs` needs `OPTIONS` because JSONC has dialect choices; TOML does not). The parser crate's own value/error types never escape the module — that is what keeps it swappable. `tests/toml_behavior.rs` pins what the seam guarantees, mirroring `tests/yaml_behavior.rs`: multiline `"""…"""` preserved verbatim, escapes, a missing required field surfacing as an error, an unknown key ignored.

### 5.4 The DTO — and a rename

```rust
/// Contract for a Codex agent `*.toml`. `Deserialize`-only: no `Serialize`,
/// no `TS`, so it emits no binding. Permissive by design (§8): unmodelled
/// Codex keys are ignored, never an error.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CodexAgentDocument {
    /// Required. Absent or non-string => the whole file is an `Error` (§7).
    pub name: String,
    pub description: Option<String>,
    /// Parsed and pinned by tests — the multiline `"""…"""` case the seam
    /// exists for — but deliberately NOT mapped onto `Component` (§4.1).
    pub developer_instructions: Option<String>,
}
```

**Renamed from the proposal's `CodexAgentFrontmatter`.** `frontmatter` is load-bearing vocabulary here: `frontmatter.rs`, the `frontmatter-reader` capability, `AgentFrontmatter`, `SkillFrontmatter` all mean "the `---`-fenced YAML block at the head of a Markdown file". A Codex agent file has no fence and no body — `frontmatter::split` (`frontmatter.rs:47-68`) is structurally inapplicable to it. Calling the DTO `…Frontmatter` would tell a future reader the file has a frontmatter block. `Document` is what it is.

## 6. Decision 4 & 5 — the adapter, the roots, and `ROOT_ORDER`

### 6.1 `roots.rs`

| Addition | Shape |
|---|---|
| `skill_roots` gains a **4th and last** entry: `resolve_single(home, "codex-skills", SearchRootKind::Skill, [".codex", "skills"])`. Return type `[ResolvedRoot; 3] -> [ResolvedRoot; 4]` | The fixed-array pattern — "the CA-6/CA-14 guarantee expressed in the type" (`roots.rs:52-56`) — is preserved, not weakened. The doc comment's "three" becomes "four" everywhere, including the CA-6/CA-14 scoping argument |
| New `pub fn codex_agent_root(home: &Path) -> ResolvedRoot` = `resolve_single(home, "codex-agents", SearchRootKind::Agent, [".codex", "agents"])` | Public single-root shape mirroring `opencode_agent_root`, but built from `resolve_single` because it is one plain directory with one scan path — no alias, no merge order |

**The Codex agent root DOES emit a `SearchRoot`** (closing an open decision), with `kind: Agent`, `status` from the existing `probe`, and one `scan_path`. It is a walked on-disk directory exactly like `claude-agents`; emitting it keeps `roots_scanned` honest, gives `scan::append_missing_root_issues` its `Warning` when absent, and gives every emitted `Location.root` a referent that actually appears in the report. Rejected: probe-only (that is `claude-embedded-agents`' shape, and it exists only because those agents have no files).

Both roots are `home` + hardcoded segments, no `dirs`/`directories`, no environment read (`plan-desarrollo-poc.md:179`).

### 6.2 `consolidate::ROOT_ORDER` — 6 → 8, and why precedence is provably unchanged

```rust
const ROOT_ORDER: [&str; 8] = [
    "claude-skills", "agents-skills", "opencode-skills", "codex-skills",   // skill_roots
    "claude-agents", "claude-embedded-agents",                             // agent_roots
    "opencode-agents", "codex-agents",                                     // single roots
];
```

The pinning test (`consolidate.rs:184-200`) stays green by adding one line — `expected.push(crate::roots::codex_agent_root(&home).root.id.0.clone());` after the `opencode_agent_root` push — and by nothing else: the `skill_roots` loop already iterates whatever the array holds, so the fourth skill root flows in automatically. **The test is the guard, and it must be updated in the same commit as `roots.rs`, never after.** No merge logic, no `root_rank`, no `location_key`, no `merge_into` changes.

**Correcting the proposal's "appended last".** `codex-skills` lands at index **3**, not 7 (V6). Field precedence for existing components is still provably unchanged, for a stronger reason than position: `ComponentId` includes `ComponentKind`, so a `Skill` component's locations can only ever come from skill roots and an `Agent` component's only from agent roots. Precedence is therefore decided by *relative* order inside one family, and inside both families the pre-existing ids keep their exact relative order (`claude-skills < agents-skills < opencode-skills`, `claude-agents < claude-embedded-agents < opencode-agents`). The absolute ranks of the agent ids shift by one; nothing compares an agent rank against a skill rank. The untouched reference-fixture pins (§10) are the empirical half of that argument.

Consequence, stated so it is auditable: a Codex skill sharing a name with a Claude Code or OpenCode skill contributes its `Location` but **loses** the `description`/`provenance_hint` race to them — Codex is last within the skill family.

### 6.3 `codex_agents.rs`

Structurally `agents.rs`'s walk with three deliberate differences.

```rust
pub struct CodexAgentScan {
    /// Always exactly one root.
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

pub fn scan(home: &Path) -> CodexAgentScan;
```

Walk shape, in order: resolve the single root → `symlink_metadata` on its scan path (`NotFound` ⇒ silent return) → assert it is a directory → `read_dir` → collect entries, `sort_by_key(DirEntry::file_name)` so ordering is identical on all three CI legs → per entry: `file_type().is_file()` and `extension() == Some("toml")`, else skip silently → UTF-8 path check → `std::fs::read_to_string` → `crate::toml::from_str::<CodexAgentDocument>` → `Component`.

The three differences from `agents.rs`, each with a reason:

1. **No embedded pseudo-root.** There is no verified list of agents Codex ships with no file (V4: no oracle), and inventing one would be worse than omitting it.
2. **No `escalate` function.** `agents.rs` needs one because `frontmatter::read` hands back a pre-built `ScanIssue` with a graded severity. `crate::toml::from_str` returns a `TomlError`, so every issue here is constructed at the failure site with the severity already correct — `opencode_agents.rs:6-10`'s reasoning, applied.
3. **Flat, never recursive**, same as `agents.rs`: `read_dir`, not `walkdir`. A `SKILL.md`-style nested layout is not Codex's agent shape.

Field mapping:

| `CodexAgentDocument` | `Component` |
|---|---|
| `name` | `name`, and `id: ComponentId::derive(ComponentKind::Agent, &name)` |
| `description` | `description` |
| `developer_instructions` | **dropped** |
| — | `kind: Agent`, `scope: Scope::User`, `provenance_hint: None`, `locations: vec![Location { path: Some(file), root: "codex-agents", origin: LocationOrigin::File }]` |

`developer_instructions` is dropped from the `Component` deliberately: `provenance_hint` answers *where this came from*, not *what its prompt says*, and the value is an unbounded multiline blob that would cross IPC into a UI with nowhere to render it. It stays **in the DTO** so the seam's multiline-string behaviour is exercised and asserted — that assertion is the whole point of choosing a real parser over a hand-rolled one.

A file missing `name` is an `Error` for the whole file, not a component with an invented name. Rejected: falling back to the file stem — `agents.rs` requires `name` in the frontmatter and errors otherwise, and inventing a name the user never wrote is a worse inventory than a visible error. Inherited without change: a present-but-blank `name` produces a component with a blank name and the id `agent:` — the same behaviour `agents.rs` has today; this change does not introduce a new validation rule for one client only.

### 6.4 `scan.rs`

One adapter call, three `extend`s, appended after `opencode_agents` so record order is stable:

```rust
let codex_agents = crate::codex_agents::scan(home);
roots_scanned.extend(codex_agents.roots);   // 6 roots -> 8 (with the 4th skill root)
components.extend(codex_agents.components);
issues.extend(codex_agents.issues);
```

`lib.rs` gains `pub mod codex_agents;` and `pub mod toml;`. The "one bad adapter never aborts the scan" property holds structurally: `codex_agents::scan` is infallible and returns owned buffers, exactly like the other three.

## 7. Error paths — the `ScanIssue` taxonomy

**No new severity, no new field on `ScanIssue`.** `IssueSeverity` stays at exactly two variants; that is a review check.

### 7.1 `codex_agents.rs`

| Condition | Severity | `path` | Reason |
|---|---|---|---|
| Root absent (`NotFound`) | **no issue** | — | Absence is `SearchRootStatus::NotFound`; the single `Warning` comes from `scan::append_missing_root_issues` (V5). **CA-11** |
| Root present but not inspectable | `Error` | scan path | `could not inspect search root: {err}` |
| Root exists but is not a directory | `Error` | scan path | `search root is not a directory` |
| `read_dir` fails | `Error` | scan path | `could not read search root: {err}` |
| `DirEntry` iteration error | `Error` | scan path | `could not read directory entry: {err}` (a bare `io::Error` carries no path) |
| `entry.file_type()` fails | `Error` | entry path | `could not read directory entry: {err}` |
| Non-`.toml` file, or a subdirectory | **no issue** | — | Not a Codex agent; silent, like `agents.rs` with non-`.md` |
| Non-UTF-8 path | `Error` | `None` | `skipped a file whose path is not valid UTF-8: {lossy}` |
| File unreadable **or** not valid UTF-8 | `Error` | file path | `could not read Codex agent file: {err}` — `read_to_string` collapses both into one `io::Error` (`InvalidData`), as in `opencode_agents.rs` |
| TOML parse error, type error, or missing `name` | `Error` | file path | `could not parse Codex agent file: {err}` |

Every row is `Error`, uniformly, for `agents.rs:220-225`'s reason: under an agents root, "if there is a file of the right extension, it is an agent", so every failure to read one is a component missing from the user's inventory. **Isolation is per file**: each arm `continue`s, so one corrupt `.toml` never costs a sibling agent — **CA-12**.

### 7.2 `installations.rs`, slot `CodexStandalone`

| Condition | Status | Installations | Issue |
|---|---|---|---|
| `releases/` absent | `NotDetected` | 0 | **none** — **CA-11** |
| `releases/` present, `read_dir` fails | `Detected` | 0 | `Error`, path `releases/`: `could not read the Codex CLI (standalone) directory: {err}` |
| `DirEntry` iteration error | `Detected` | partial | `Error`, path `releases/`, same reason; siblings still resolve |
| `releases/` present, zero child directories | `Detected` | 0 | `Error`, path `releases/`: `expected at least one Codex CLI (standalone) release directory, found none` |
| Child is not a directory | — | ignored | none |
| Child directory name is not valid UTF-8 | `Detected` | 0 for it | `Error`, `path: None`, lossy name in the reason (mirrors `install_from_version_dir:553-561`) |
| Child directory name matches no known triple, or strips to an empty version | `Detected` | 0 for it | `Error`, path = that directory: `could not read a version from the Codex CLI (standalone) release directory name: {name}` — §3.3 |
| Child directory name parses | `Detected` | +1 | none. `ClientInstallation { client: Codex, version, path: <releases>/<name> }` |

Reason strings are built with `slot.label()`, exactly like the bundled slot's four `Error` reasons, so `InstallSlot::label()`'s dual role (presence label + issue reason, P32D V3) is unchanged. **Never merged**: N parseable directories yield N `ClientInstallation`s, never reduced to a highest-version winner — **CA-7**. Child directories are sorted byte-wise on `file_name()` before conversion, never by locale, so ordering is identical on all three CI legs.

`ClientInstallation.path` points at the **release directory** (`…/releases/0.149.0-x86_64-pc-windows-msvc`), not at `bin/codex.exe` and not at a canonicalized target: it is the directory whose name the version came from, so path and version can never disagree.

## 8. Decision 6 — `SkillFrontmatter` stays permissive

**No `#[serde(deny_unknown_fields)]`.** Recorded as a decision, not inherited.

| Option | Consequence | Decision |
|---|---|---|
| Add `deny_unknown_fields` | Every Codex `SKILL.md` observed carries `disable-model-invocation`, `user-invocable`, `license`, `metadata.*` (U1) — so the very components this change adds would **all** fail to parse. The DTO is shared by all clients, so any upstream key addition by Anthropic or OpenCode would turn working skills into `Error` rows overnight, with no oracle (V4) to warn us first. That converts a forward-compatible reader into a brittle one, across three vendors, to catch nothing the user cares about | **Rejected** |
| **Stay permissive** | Unmodelled keys are ignored. Codex skills parse with **zero lines changed in `skills.rs` and `frontmatter.rs`** | **Chosen** |

The burden of proof was on changing it, and adding a third, more feature-rich dialect makes the case *against* strictness stronger, not weaker. Accepted cost, stated plainly: a typo'd key (`descriptoin:`) is silently ignored and the component shows no description. Detecting that is a lint feature about *content quality*, not an inventory concern, and is out of scope for the PoC. Reversible later, in one line, if a lint capability ever lands.

## 9. IPC contract surface and per-OS paths

### 9.1 IPC

**No new command, no new event, no capability change.** `crates/vertice-app/` and `capabilities/default.json` are byte-identical; `scan`/`rescan` stay thin pass-throughs. The contract change is entirely inside the existing payload: `ScanReport.clientPresence` gains a fourth record and `ScanReport.installations` may gain entries whose `client` is the new `"codex"`.

| Binding file | Action |
|---|---|
| `frontend/src/bindings/ClientKind.ts` | **Modified** — `"claudeCode" \| "openCode"` becomes three variants |
| every other `bindings/*.ts` | **Unchanged** — a diff there means something leaked into `model/` |

Regenerated **only** by `cargo test -p vertice-core`, never hand-edited, landing in the same commit as the enum. CI runs `git add --intent-to-add` first, so a new binding is caught too. `frontend/src/` outside `bindings/` is byte-identical: the presence table renders from `record.label` and `Component` carries no client field, so the fourth row and the new components appear with **zero** frontend source changes and **zero** new i18n keys.

### 9.2 Paths, by OS

| Purpose | Windows (this change) | macOS / Linux |
|---|---|---|
| Codex skills | `<home>\.codex\skills` | **same shape, and already resolved today** |
| Codex agents | `<home>\.codex\agents` | **same shape, and already resolved today** |
| Codex installation | `<home>\.codex\packages\standalone\releases\<version>-<triple>\` | **T16** — a triple table entry plus verification |
| Rejected | `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin` (symlink shim, §3.4); `<home>\.codex\packages\standalone\current` (symlink, one release only); `<home>\.codex\version.json` (update cache, §3.1); `%LOCALAPPDATA%\Codex\` (logs only) | |

**An asymmetry worth stating, because "Windows only" is easy to over-read.** `roots.rs` has no platform branch and never has: `.claude`, `.config/opencode` and now `.codex` are dotfile directories under the user's home on *every* OS. So the two new **component roots are resolved and walked on all three platforms from day one**, exactly like the existing roots — and are exercised on all three CI legs by fixture homes. Only **installation detection** is Windows-gated, via `HostPlatform` (`cfg!` as an expression, never an attribute — T7D §5.2), and on `Unsupported` the whole `client_presence` field stays `None` (P32D §4), so nothing about Codex is claimed on macOS or Linux. `CODEX_TARGET_TRIPLES` being Windows-only for now is a consequence of that gate, not an independent limitation.

## 10. Fixtures — and the reference-tree tripwire

### 10.1 The hard guard

**`crates/vertice-core/tests/fixtures/roots/reference/` MUST stay byte-identical.** The 69/25 pins are `components.len() + issues.len()` over that tree (V7), so *any* file added under it moves them, and a `.codex/` directory added under it would move them silently.

Three tripwires, in increasing strength:

1. `reference_fixture_tree_yields_69_entries` and its 25-id corroborator stay in the file **unmodified**, and the CA-3/CA-4 (22-with-3-locations / 3-with-1-location) assertions likewise. A diff to any of those four numbers in the PR is a stop-the-line signal, not a fixture update.
2. The fourth root resolves to `reference/.codex/skills`, which does not exist, so `skills::walk_one` returns silently with **zero** issues (V5) — the count is structurally unaffected, not merely observed to be unaffected.
3. A new **negative-existence** assertion in `skill_scanner.rs`: `reference/.codex` MUST NOT exist on disk. Absent this, a future contributor "helpfully" adding Codex coverage to the reference home would break CA-2/CA-3/CA-4 with a confusing count diff instead of a named failure.

`tests/fixtures/scan-orchestrator/reference-volume/` is likewise untouched (it backs the read-only/CA-16 snapshot and the duration bound).

Two `skill_scanner.rs` assertions **do** change and are expected to: `scan.roots.len() == 3 -> 4` (line 149) and the three-id list in `roots.rs`'s `root_ids_are_stable_and_never_path_derived` / `skill_roots_always_returns_exactly_three_entries` (renamed to `..._four_entries`).

### 10.2 No symlink, anywhere — confirmed

The §3.4 decision to enumerate `releases/` **removes the need for a symlink fixture entirely**. Every path this design reads is a plain directory or a plain file. This matters concretely: `std::os::windows::fs::symlink_dir` needs Developer Mode or elevation, git stores symlinks as text files without `core.symlinks`, and the three CI legs do not agree on any of it. **Rule: no committed fixture contains a symlink or a junction, and none is constructed at test time.** If some future change genuinely needs to resolve the `current` chain, it must build the link at test time inside a temp directory — and must first re-argue CA-16, since creating a link is a write.

### 10.3 The fixture set

New homes, all outside the reference trees:

| Home | Proves |
|---|---|
| `codex-installations/single-release` | `Detected`, one `ClientInstallation`, version `0.149.0` from the directory name, path = the release directory |
| `codex-installations/two-releases` | **CA-7**: two directories at different versions ⇒ **two** installations, never merged, never reduced to a winner |
| `codex-installations/prerelease` | `0.150.0-rc.1-x86_64-pc-windows-msvc` ⇒ `0.150.0-rc.1`. The RED test that kills "split on the first `-`" |
| `codex-installations/unknown-triple` | `Detected` + 0 installations + 1 `Error` with the directory's path (§3.3) |
| `codex-installations/empty-releases` | `releases/` exists with no children ⇒ `Detected` + 0 installations + 1 `Error` |
| `codex-installations/stale-version-json` | `version.json` says `9.9.9`, the release directory says `0.149.0` ⇒ the report says **`0.149.0`** |
| `codex-installations/nothing` | No `~/.codex` at all ⇒ `NotDetected` + **zero** issues — **CA-11** |
| `codex-agents/complete` | A happy-path agent, and one whose `developer_instructions` is a genuine multiline `"""…"""` asserted **in full** — no truncation at the first quote or newline |
| `codex-agents/extra-keys` | Unmodelled top-level keys and a nested table are ignored, not an error |
| `codex-agents/corrupt` | One malformed `.toml` and one missing `name` ⇒ two `Error` issues carrying their paths, **every other agent in the directory still emitted** — **CA-12** |
| `codex-agents/not-a-directory`, `codex-agents/empty` | The root-shape error arms and the zero-agents-no-issue arm |
| `roots/codex-skills` | A Codex `SKILL.md` carrying `disable-model-invocation`, `user-invocable`, `license`, `metadata.*` parses; the extra keys are ignored (§8) |
| `roots/codex-and-claude-same-name` | A skill named `shared` under both `.claude/skills` and `.codex/skills` ⇒ **one** `Component` with **two** `Location`s, both visible |

**Every fixture directory MUST contain at least one file** — git does not track empty directories, so a bare `releases/<version>-<triple>/` would vanish on a fresh clone and the test would pass or fail for the wrong reason. Use `.gitkeep`, and assert the directory's on-disk existence in a dedicated test, exactly as `skill_scanner.rs:36-52` already does for the `empty-alias` fixture. This applies to `empty-releases` and to every release directory whose contents the resolver does not read.

Existing fixtures that **must** change, with their assertions (V8):

| Fixture / test | Change |
|---|---|
| `scan-orchestrator/complete` | Gains `.codex/skills/<n>/SKILL.md`, `.codex/agents/<n>.toml`, and a `packages/standalone/releases/<version>-<triple>/` tree — otherwise the two new roots resolve `NotFound` and `report.issues.is_empty()` fails on two new `Warning`s. Counts move: roots `6 -> 8`, installations `3 -> 4`, components `10 -> 12` |
| `scan-orchestrator/missing-root-client` | No fixture change. Assertions move: roots `6 -> 8`, `path.is_none()` warnings `6 -> 8`, `client_presence.len() 3 -> 4`, still all `NotDetected` |
| `scan-orchestrator/corrupt-skill` | No change; `installations.len() == 1` holds (no Codex tree there) |
| `tests/model_contract.rs` | `ClientKind` exhaustive-match test gains `Codex` |

### 10.4 Two textual invariants

1. `tests/toml_seam_invariant.rs` — `src/toml.rs` is the only file naming `toml_seam` (§5.2).
2. `tests/codex_version_source_invariant.rs` — **no file under `src/` contains `version.json` or `latest_version`.** With no oracle (V4), this is the only mechanical guarantee that the update-availability cache never becomes a version source; a prose "MUST NOT" would not survive a future contributor who finds the file and thinks it helpful.

## 11. File changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/model/installation.rs` | Modify | `ClientKind::Codex`; doc comment names three clients |
| `crates/vertice-core/src/toml.rs` | **Create** | §5.3 — the seam; sole namer of `toml_seam` |
| `crates/vertice-core/src/codex_agents.rs` | **Create** | §6.3 — flat `read_dir`, `CodexAgentDocument`, per-file isolation |
| `crates/vertice-core/src/roots.rs` | Modify | `codex-skills` (array 3→4) + `codex_agent_root`; doc "three"→"four" |
| `crates/vertice-core/src/installations.rs` | Modify | `CodexStandalone` slot + label + client mapping, `ReleaseDirectoryName`, `resolve_codex_slot`, `split_release_dir_name`, `CODEX_TARGET_TRIPLES`, probe branch; module doc "three slots"→"four" |
| `crates/vertice-core/src/consolidate.rs` | Modify | `ROOT_ORDER` 6→8 + one line in the pinning test; **no logic change** |
| `crates/vertice-core/src/scan.rs` | Modify | Fourth adapter wired in; three orchestrator tests' counts |
| `crates/vertice-core/src/lib.rs` | Modify | `pub mod codex_agents;`, `pub mod toml;` |
| `crates/vertice-core/src/skills.rs`, `agents.rs`, `opencode_agents.rs`, `frontmatter.rs`, `yaml.rs`, `jsonc.rs`, `model/identity.rs`, `model/component.rs` | **Unchanged** | No shared abstraction extracted; no identity change; `SkillFrontmatter` stays permissive |
| `crates/vertice-core/tests/toml_seam_invariant.rs`, `toml_behavior.rs`, `codex_version_source_invariant.rs`, `codex_agent_scanner.rs` | **Create** | §5.3, §10.4 |
| `crates/vertice-core/tests/skill_scanner.rs`, `client_installations.rs`, `consolidation.rs`, `model_contract.rs` | Modify | Root count 3→4, new Codex cases, `ClientKind` match |
| `crates/vertice-core/tests/fixtures/roots/reference/`, `fixtures/scan-orchestrator/reference-volume/` | **Byte-identical** | §10.1 — the tripwire |
| `frontend/src/bindings/ClientKind.ts` | Regenerated | Three variants; never hand-edited |
| `frontend/src/` (source), `crates/vertice-app/`, `capabilities/default.json`, `deny.toml` | **Unchanged** | §9.1 |
| `Cargo.toml`, `Cargo.lock` | Modify | `toml_seam = { package = "toml", version = "1", default-features = false, features = ["parse", "serde"] }`, core only; 11 locked packages |

**CA-16 structurally.** The disk surface added is `symlink_metadata`, `read_dir`, `DirEntry::file_type`, `read_to_string`. No `File::create`, `OpenOptions`, `fs::write`, `create_dir*`, `remove_*`, `symlink*` — in source **or** tests (§10.2). The TOML seam exposes no serializer (§5.1).

## 12. Testing strategy (`strict_tdd: true` — RED first)

The load-bearing failing tests, in this order, before any implementation:

1. `codex_agent_with_multiline_developer_instructions_yields_the_complete_value` — the seam's reason for existing. Must fail to **assert**, not to compile.
2. `prerelease_release_directory_name_yields_the_full_prerelease_version` — `0.150.0-rc.1`. A "split on the first `-`" implementation fails here.
3. `two_release_directories_yield_two_unmerged_installations` — **CA-7**.
4. `home_without_codex_yields_not_detected_and_zero_issues` — **CA-11**.
5. `malformed_codex_agent_is_reported_without_losing_siblings` — **CA-12**.
6. `same_named_skill_in_codex_and_claude_roots_yields_one_component_with_two_locations`.

| Layer | What |
|---|---|
| Unit, no I/O | `split_release_dir_name` over the §3.2 table including both no-match rows; `InstallSlot::CodexStandalone` label/client/version-source; the non-UTF-8 directory-name arm (synthetic `OsString`, per-platform as `install_from_version_dir`'s tests already are) |
| Seam | `tests/toml_behavior.rs` — multiline `"""…"""`, escapes, missing required field, unknown key ignored |
| Invariant | `tests/toml_seam_invariant.rs`; `tests/codex_version_source_invariant.rs`; `root_order_matches_the_roots_module_in_order` |
| Integration | The §10.3 fixture table, one test per row, via `skills::scan` / `codex_agents::scan` / `installations::scan_for(home, HostPlatform::Windows)` — all three CI legs (**CA-17**) |
| Regression | The untouched 69/25/22/3 reference pins plus the new negative-existence assertion (§10.1) |
| Read-only | The existing tree-snapshot equality tests, extended over the new Codex fixture homes (**CA-16**) |
| Contract | No `ClientInstallation` with an empty version; `IssueSeverity` still exactly two variants; `ClientKind` exhaustive match |
| Frontend | **No new test and no source change expected.** Existing Vitest suites must stay green against the regenerated `ClientKind.ts` |

Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, the `msrv` job at 1.88, bindings-in-sync, and `npm run lint && npm run check && npm run test && npm run build`.

## 13. Slicing and rollback

Three slices, each independently green and independently revertible — matching the proposal's forecast and its High 400-line budget risk:

1. **`ClientKind::Codex` + the `CodexStandalone` slot + `ReleaseDirectoryName` + installation fixtures + the regenerated binding.** Self-contained; adds a presence row and nothing else.
2. **The `codex-skills` root** (`skill_roots` 3→4) **+ `ROOT_ORDER` + fixtures.** Near-zero implementation, meaningful test surface, and it is the slice that carries the §10.1 tripwire.
3. **`toml.rs` + `codex_agents.rs` + `codex_agent_root` + the orchestrator wiring + `ROOT_ORDER`'s eighth entry.** The only slice with a new dependency.

`ROOT_ORDER` is touched by slices 2 and 3; whichever lands second updates the array and the pinning test together. Final slicing is `sdd-tasks`'s call.

Rollback is the proposal's ordered three layers, unchanged. The only layer whose revert is not free is the dependency (`Cargo.toml` + `Cargo.lock`); `deny.toml` is expected untouched in both directions. A partial rollback (core reverted, binding not) fails at the CI drift gate or at TypeScript compile time, never silently at runtime. No migration: nothing is persisted.

## 14. Open questions

- [x] Windows symlink/junction resolution — **moot by design**: `releases/` and its children are real directories (V2) and are enumerated directly; no link is ever followed. §3.4
- [x] TOML crate, MSRV and features — `toml` 1.1.4 with `default-features = false, features = ["parse", "serde"]`; the **whole** tree is ≤ 1.85 against the 1.88 floor and already licence-allow-listed (V1b). No MSRV tripwire, no `deny.toml` edit. §5.1
- [x] The dependency alias — `toml_seam`, because `toml_parser` is a real crate already in the graph (V1c). §5.2
- [x] The version-extraction rule — longest known-triple-suffix strip, prerelease-safe. §3.2
- [x] An unparseable release directory name — `Detected`, no installation, one `Error` with its path (closes proposal question 4). §3.3
- [x] The DTO field mapping, and a missing `name` — §5.4, §6.3
- [x] Does the Codex agent root emit a `SearchRoot`? — **Yes**, `kind: Agent`, one scan path. §6.1
- [x] `SkillFrontmatter` strictness — stays permissive. §8
- [x] Fixture symlink portability — **no symlink is needed anywhere**. §10.2
- [ ] `codex doctor` as an installation oracle — **T16, manual only.** Never an automated test (`alcance-poc-vertice.md:132`)
- [ ] `CODEX_TARGET_TRIPLES` drift: with no component or install oracle (V4), a triple OpenAI adds surfaces as an `Error` row, not as a wrong version. Accepted; the table is a one-line fix and §10.4's invariant keeps the fallback honest
- [ ] Non-standalone Codex installations (e.g. npm) report `NotDetected` for this slot while their components are still inventoried. Recorded limitation, §3.4 — out of scope here
- [ ] macOS and Linux Codex **installation** path tables and their triples — **T16**. The component roots already work on all three platforms (§9.2)
