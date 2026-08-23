# Exploration: Codex client support

Status: exploration only — no proposal, spec, or design decisions are committed here.

## Context

Vertice currently scans two AI clients: Claude Code and OpenCode. This exploration maps
what it would take to add a third, **Codex (OpenAI)**, and surfaces the risks and open
questions before any proposal is written.

Prior decision on record: `openspec/changes/archive/2026-08-19-client-installation-detection/proposal.md:38`
states that detection of clients outside the closed `ClientKind` set (Copilot, Codex, …)
is outside the PoC. This exploration does not overturn that decision; it prepares the
ground for a change that would.

### Evidence gathered on the user's machine (Windows 10, real install, verified)

- **Skills root**: `~/.codex/skills/<name>/SKILL.md` — YAML frontmatter, shape compatible
  with the existing Claude Code skill contract. Extra keys observed beyond `name` and
  `description`: `disable-model-invocation`, `user-invocable`, `license`,
  `metadata.author`, `metadata.version`.
- **Agents root**: `~/.codex/agents/<name>.toml` — flat **TOML** files, one per agent, not
  Markdown-with-frontmatter. Observed top-level keys: `name`, `description`,
  `developer_instructions` (a multiline `"""…"""` string).
- **Installation path** (resolved during this exploration): the executable is
  `~/.codex/packages/standalone/releases/<version>-<target-triple>/bin/codex.exe`, e.g.
  `0.149.0-x86_64-pc-windows-msvc`, reached through a symlink chain:
  `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin` -> `~/.codex/packages/standalone/current/bin`,
  and `~/.codex/packages/standalone/current` -> `~/.codex/packages/standalone/releases/<version>-<triple>`.
  There is **no `package.json`** anywhere in the install tree.
- `~/.codex/version.json` contains `{"latest_version":"0.149.0","last_checked_at":"…","dismissed_version":null}`
  — an update-check cache, not an installed-version record.
- `%LOCALAPPDATA%\Codex\` exists but holds only `Logs\`.

## Current architecture (file:line evidence)

- `crates/vertice-core/src/model/installation.rs:24-30` — `ClientKind { ClaudeCode, OpenCode }`,
  documented as deliberately minimal but expected to grow, staying a closed enum
  (never `#[non_exhaustive]`).
- `crates/vertice-core/src/installations.rs:134-164` — private
  `InstallSlot { ClaudeCodeNpm, ClaudeCodeBundled, OpenCodeNpm }`, each mapped to
  `(ClientKind, label, VersionSource)`. `VersionSource { PackageJson, DirectoryName }`
  at `installations.rs:176-179`.
- `crates/vertice-core/src/model/component.rs:16-32` — `Component` has **no client field**:
  `id, name, kind, description, scope, locations, provenance_hint`.
- `crates/vertice-core/src/model/identity.rs:26-29` — `ComponentId::derive(kind, name)`;
  identity never derives from `Location` or client.
- `crates/vertice-core/src/model/location.rs:40-42` — "The scanner is modeled as
  'one root produces N components', not 'one client has N components'."
- `crates/vertice-core/src/consolidate.rs:107-119` — grouping strictly by `Component.id`.
  `ROOT_ORDER` (`consolidate.rs:18-25`) is a fixed 6-entry array pinned to `roots.rs` by
  `root_order_matches_the_roots_module_in_order` (`consolidate.rs:185-200`).
- `crates/vertice-core/src/roots.rs:53-54,61-77,82-106` — `skill_roots` returns
  `[ResolvedRoot; 3]`, a fixed-size array documented as "the CA-6/CA-14 guarantee,
  expressed in the type"; `agent_roots` returns 2; `opencode_agent_root` 1. Every path is
  `home` + hardcoded segments (no `dirs`/`directories`, per `plan-desarrollo-poc.md:179`).
- `crates/vertice-core/src/skills.rs:36-53,60-144` — a generic `SKILL.md` walker with
  **zero client-identity logic**.
- `crates/vertice-core/src/agents.rs:8-11,29-36,50-59,64-82` — Claude-Code-specific: flat
  `read_dir`, hardcoded `EMBEDDED_CLAUDE_AGENTS` list, typed `AgentFrontmatter` DTO.
  Module doc states `SkillScan`/`AgentScan` are "deliberately separate types, not a shared
  abstraction (design §5.4)".
- `crates/vertice-core/src/opencode_agents.rs:37-86,220-252,269-275` — no directory walk:
  reads two JSON(C) config files, deep-merges their `agent` object, extracts fields
  value-level with no typed DTO.
- `crates/vertice-core/src/scan.rs:24-55` — orchestrator concatenating the adapters, then
  `consolidate::consolidate(components)`.
- `crates/vertice-core/src/frontmatter.rs:26-29,43-68` — `split()` requires an exact `---`
  first line; structurally inapplicable to a flat TOML file. `SkillFrontmatter` captures
  only `name`/`description` and does **not** use `deny_unknown_fields`.
- `crates/vertice-core/src/yaml.rs` (24 lines) and `crates/vertice-core/src/jsonc.rs` are
  the two existing "one module owns the parser" seams;
  `crates/vertice-core/tests/yaml_seam_invariant.rs:43-77` enforces the YAML one textually.
- `crates/vertice-core/Cargo.toml:8-15` — no TOML parser dependency today.
- `deny.toml:54-67` — license allow-list includes MIT and Apache-2.0; `[bans]` covers only
  `tauri`/`tauri-build`.
- `frontend/src/lib/pages/ScanPage.svelte:104-116` — `client_presence` rendered generically
  by `record.label`; no per-`ClientKind` branch anywhere in the Svelte sources.
- `frontend/src/bindings/ClientKind.ts:9` — `"claudeCode" | "openCode"` (generated).
- `openspec/specs/duplicate-consolidation/spec.md:19-21,35-39,41-49` — grouping key is
  `Component.id` alone; the only documented non-merge case is kind-based; name-convention
  filtering is deliberately absent.

## Findings

### 1. Extension points for a third `ClientKind`

Additive:

- `model/installation.rs`: add the `Codex` variant. Exactly **two** exhaustive matches on
  `ClientKind` exist in core (`InstallSlot::client()` at `installations.rs:141-146`, plus
  bindings regeneration), so the blast radius is small and compiler-enforced.
- `installations.rs`: new `InstallSlot` variant(s), a new branch in `windows_install_probes`,
  and (see finding 4) very likely a new `VersionSource` variant with its own resolver.
- New adapter module for Codex agents (finding 3).
- `roots.rs`: a new skill root entry and a new agent root.
- `consolidate.rs`: `ROOT_ORDER` grows; its pinning test must stay synchronized.
- Fixtures under `crates/vertice-core/tests/fixtures/`.
- New/extended openspec specs plus a change proposal.

Touches an existing invariant:

- `roots::skill_roots` must change from `[ResolvedRoot; 3]` to `[ResolvedRoot; 4]` — a real
  signature change, but the fixed-array pattern (the guarantee expressed in the type) is
  preserved.
- `ClientKind` stops being a two-variant set. Its own doc comment
  (`installation.rs:20-23`) anticipates exactly this, so it is expected evolution.
- CA-16 (read-only) and the no-OS-convention-crate rule apply unchanged; no exception is
  needed, and no shortcut is available for locating `~/.codex`.

**Frontend: no changes required.** The client-presence table renders generically
(`ScanPage.svelte:104-116`), and the skills/agents pages render `Component`, which carries
no client field. Bindings regenerate automatically; CI fails on drift. No new i18n keys
unless a future UX adds a client column or filter (out of scope).

### 2. Skills adapter — reusable as-is

`skills.rs::scan` (`skills.rs:36-53`) and its walker (`skills.rs:60-144`) are already
client-agnostic: they walk any given `scan_path` for files literally named `SKILL.md` and
emit `Component { kind: Skill, … }`. There is no `ClientKind` parameter or field in the file.

Reusing it for `~/.codex/skills` requires **zero changes to `skills.rs`** — only a fourth
entry in `roots::skill_roots` (`roots.rs:61-77`) following the existing `resolve_single`
pattern. This is the cleanest extension point in the whole exercise.

Caveat: Codex `SKILL.md` frontmatter carries extra keys. `SkillFrontmatter`
(`frontmatter.rs:26-29`) has no `deny_unknown_fields`, so unknown keys are silently ignored
today — confirm that remains the desired behavior (open question 5).

### 3. Agents adapter — the hard part (flat TOML)

| Approach | Pros | Cons |
| --- | --- | --- |
| **New `toml.rs` seam** mirroring `yaml.rs`/`jsonc.rs` | Matches the established one-module-owns-the-parser convention; a `toml_seam_invariant.rs` is a ~40-line copy of the YAML one; `toml`/`toml_edit` are dual MIT/Apache-2.0, already inside `deny.toml:54-66` — no license-gate change; correctly handles multiline `"""` strings, escapes, arrays, nested tables | One new dependency; MSRV of the toml crate must be checked against the workspace floor (not verified) |
| **Hand-rolled parser** for the observed 3-key subset | No new dependency | `developer_instructions` is a triple-quoted multiline string — precisely the class of bug `AGENTS.md` already warns about for YAML ("Frontmatter parsing must not use regex — multiline block scalars break it"). Requires reimplementing TOML string/escape rules correctly or silently mis-parsing. Every future Codex key addition becomes manual maintenance |
| Other | None identified — TOML tables and multiline strings are not line-delimited, so there is no cheap value-level shortcut analogous to `jsonc.rs` | — |

**Recommendation**: add a `toml.rs` seam using the `toml` crate (read-only, so `toml_edit`'s
write-preservation is unnecessary). This follows two existing precedents in the crate and
avoids re-deriving multiline-string parsing by hand. Verify the crate's MSRV before
committing.

**Is there a per-client adapter pattern to follow?** Yes, and it is deliberately
un-abstracted. `agents.rs` walks a directory of Markdown+frontmatter files with a typed DTO;
`opencode_agents.rs` never walks a directory at all — it deep-merges two JSON(C) configs
value-level. `agents.rs:8-11` states the separation is a design decision (§5.4), not an
oversight. A Codex agent adapter is structurally closer to `agents.rs` (file-per-component)
but cannot reuse `frontmatter::read`. The recommended shape is a **third standalone module**
(`codex_agents.rs`) copying `agents.rs`'s flat `read_dir` structure with
`toml::from_str::<CodexAgentFrontmatter>` in place of `frontmatter::read::<AgentFrontmatter>`
— not an attempt to unify the three adapters behind a trait.

### 4. Installation detection

Neither existing `VersionSource` fits:

- `PackageJson` (`installations.rs:363-421`) expects a `package.json` with a `version`
  string — there is none anywhere in the Codex install tree.
- `DirectoryName` (`installations.rs:445-533`) expects the version directory's bare name to
  be the version. The Codex release directory is `0.149.0-x86_64-pc-windows-msvc`; parsing
  it as a version would require splitting on the first `-`, which is fragile against a
  prerelease tag (`0.150.0-rc.1-x86_64-pc-windows-msvc`).

A **new `VersionSource` variant** is the safer direction — the enum is closed and grows by
variant addition, and silently overloading `DirectoryName`'s semantics would corrupt the
version string on any future release-naming change. This is flagged for the design phase,
not decided here.

Additional concerns:

- **`version.json` is not a version source.** Its field is literally `latest_version`: an
  update-availability cache that diverges from the installed version whenever an update is
  known but not applied. The release directory name (or the `current` symlink target) is the
  trustworthy signal, mirroring how the bundled Claude slot trusts directory names over
  cached metadata.
- **Windows symlink/junction semantics are unverified.** `installations.rs:334-340` uses
  `symlink_metadata` deliberately so it does *not* follow links. Whether that behaves on a
  Windows directory symlink/junction chain the way the code assumes was not verified. Any
  resolution strategy must use only `std::fs` primitives already in use, and must respect
  CA-16.
- **Multiple releases under `releases/`** (e.g. after an update that left the old tree in
  place) map directly onto the existing bundled-slot precedent
  (`resolve_bundled_slot`, `installations.rs:445-533`): 1..N candidate roots, each resolved
  independently, never merged (CA-7). No new design needed — just follow that shape.

### 5. Component identity and consolidation

`ComponentId::derive(kind, name)` (`identity.rs:26-29`) and `Component`
(`component.rs:16-32`) carry no client discriminator. A Codex skill named identically to a
Claude Code skill would consolidate into one `Component` with multiple `Location` entries —
the same mechanism that already merges a skill present under both `.claude/skills/` and
`.agents/skills/` (`scan.rs:96-102`).

Two defensible readings:

- **Expected, no change**: identity is deliberately blind to provenance — a simple,
  consistent model that already handles the Claude Code / OpenCode case this way.
- **Worth revisiting**: Claude Code and OpenCode are close cousins, and a shared skill file
  between them is a plausible dotfiles setup. Codex is a separate vendor ecosystem with its
  own format; merging a same-named Codex skill conflates "the user copied one skill to both
  places" with "two unrelated tools coincidentally use the same name." The "No
  Name-Convention Filtering" decision (`spec.md:41-49`) was made without a third vendor in
  view.

**Decided (user, 2026-08-23): keep the current merge behavior.** Identity stays
`(kind, name)` with no client discriminator, and a same-named Codex component consolidates
with its Claude Code / OpenCode namesake into one `Component` carrying multiple `Location`
entries. Consequences to carry into the proposal:

- `identity.rs`, `component.rs`, and `consolidate.rs` need **no changes** for Codex support.
- `openspec/specs/duplicate-consolidation/spec.md` needs no delta, but the proposal should
  cite this decision so the "No Name-Convention Filtering" stance is understood to have been
  re-affirmed with a third, unrelated vendor in view — not merely inherited.
- Fixture coverage MUST include a same-named skill present in both a Codex root and a Claude
  Code root, asserting one `Component` with two `Location`s, mirroring `scan.rs:96-102`.

### 6. Fixtures and TDD

Fixtures needed, mirroring the existing per-adapter + orchestrator pattern:

- `.codex/skills/<name>/SKILL.md`: happy path, corrupt frontmatter, extra Codex-specific
  keys. These exercise the new `roots.rs` entry, not new code in `skills.rs`.
- `.codex/agents/<name>.toml`: including at least one genuine multiline `"""…"""`
  `developer_instructions` — the exact edge case the seam is meant to handle.
- Installation: a `packages/standalone/releases/<version>-<triple>/` tree, a `current`
  symlink (or Windows-junction equivalent — verify whether it is creatable and committable
  in a fixture without admin rights or special git handling), and a `version.json`.
- Orchestrator: extend `scan-orchestrator/complete/` and siblings with a Codex slot,
  mirroring `scan.rs:90-104`.

**Real-tool oracle**: partially established (verified by running the CLI during this
exploration). `claude agents` and `opencode debug` serve that role for the existing clients
(`agents.rs:25-26`). For Codex:

- `codex --version` prints `codex-cli 0.149.0` — an exact match for the `0.149.0` prefix of
  the release directory name. This is a usable oracle for the **version** and corroborates
  the directory-name signal over `version.json` (finding 4).
- `codex agents` is **not** an equivalent of `claude agents`: its help text reads "Browse all
  agent sessions on the shared local app-server daemon" — it lists *sessions*, not agent
  definitions.
- `codex debug` exposes only `models`, `app-server`, and `prompt-input`. Nothing lists
  installed skills or agent definitions.
- `codex doctor` ("Diagnose local Codex installation, config, auth, and runtime health") was
  not run; it is the remaining candidate for an **installation** oracle and is worth checking
  before the design phase.

Consequence: there is no component-listing oracle for Codex, so the fixture set carries more
weight than it does for Claude Code, and there is no cheap way to detect upstream drift in
the agent TOML key set (open question 6).

### 7. Scope boundary

In scope for a focused Codex change:

- Windows-only detection (matching T7, `installations.rs:19`).
- `Scope::User` only.
- Skills (near-zero-cost reuse) and Agents (new TOML seam + new adapter module).

Deferred:

- macOS/Linux Codex path tables — T16, same as the existing clients.
- Project/Local scope Codex components.
- Any UI work beyond what falls out of the generic `ScanPage.svelte` rendering.
- Any change to component identity or consolidation — settled by the decision in finding 5.

## Risks

- **Parser risk**: hand-rolling TOML would repeat a mistake the project already documented
  for YAML. Mitigated by the seam recommendation.
- **Version-source risk**: reusing `DirectoryName` for a triple-suffixed name silently
  produces wrong version strings on a naming change.
- **Platform risk**: Windows symlink/junction behavior under `symlink_metadata` is
  unverified, and fixtures containing symlinks may not be portable across the CI matrix.
- **MSRV risk**: the toml crate's floor must be checked against the workspace `rust-version`
  and the three places that must agree (`Cargo.toml`, CI `MSRV` env, `rust-toolchain.toml`).

## Open questions

1. ~~Should client provenance become part of identity for a third vendor?~~
   **Decided (user, 2026-08-23): no — keep the current merge behavior.** See finding 5 for
   the consequences carried into the proposal.
2. ~~Is there a `codex` subcommand equivalent to `claude agents` / `opencode debug`?~~
   **Answered (finding 6): no.** `codex agents` lists sessions, not definitions, and `codex
   debug` exposes only `models`/`app-server`/`prompt-input`. `codex --version` is a version
   oracle. Remaining sub-question: does `codex doctor` report the installation root and
   version in a machine-checkable form?
3. Does `symlink_metadata` on Windows report correctly through the
   `Programs\OpenAI\Codex\bin` -> `current` -> `releases\<version>-<triple>` chain, or does
   the adapter need explicit symlink-target resolution using `std::fs` primitives only?
4. Is `version.json`'s `latest_version` ever an acceptable fallback version source, or should
   the adapter treat it as untrustworthy for version purposes entirely?
5. Should `SkillFrontmatter` gain `#[serde(deny_unknown_fields)]`, or stay permissive now
   that a third, more feature-rich frontmatter dialect is in the mix? (Today: permissive.)
6. Is the observed Codex agent TOML key set (`name`, `description`, `developer_instructions`)
   complete and stable, or likely to grow — bearing on how much the seam should model?

## Unverified items

- Windows symlink/junction behavior under `std::fs::symlink_metadata`.
- The `toml` crate's MSRV against the workspace floor.
- Whether `codex doctor` reports the installation root/version in a machine-checkable form.
