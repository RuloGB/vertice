# Proposal: Add Codex Client Support

> Plan trace: primarily **T7** — "Detección de instalaciones de clientes" (`internal-docs/plan-desarrollo-poc.md:171-187`), the phase that owns `ClientKind`, the Windows probe table and the explicit not-detected state, and whose own risk note reserves other platforms for T16. The component half replays **T4** (skill roots, `plan-desarrollo-poc.md:110-128`) and the **T5/T6** per-client agent-adapter pattern (`:132-167`) for a third client. macOS and Linux Codex paths are deferred to **T16** (`:339-354`), exactly as the existing clients' paths are.
>
> Acceptance criteria addressed: **CA-11** (an absent client is reported as "not detected", never an error and never an unexplained empty list) and **CA-7** (every installation counted separately, never merged) extended to a third client; **CA-12** (a corrupt file is reported with its path and does not break the scan) for the new TOML dialect; **CA-8** (no name-convention filtering) re-affirmed with a third vendor in view. Must-not-regress: **CA-2/CA-3/CA-4** (the 69→25 reference-fixture pins), **CA-6** and **CA-14** (root scoping — no plugin and no project components). Bound by **CA-16** (no write outside the app data directory) and **CA-17** (versioned fixtures, three CI legs).
>
> **No out-of-scope PoC feature is introduced**: no MCP servers, no `Project`/`Local` scope, no write operation, no update-status or upstream-version comparison, no new IPC command. One new Rust dependency (a TOML parser) is introduced and is the only supply-chain cost.

## Intent

Vertice claims to inventory AI components "across AI clients". It scans two: Claude Code and OpenCode. A user with Codex installed — with real skills under `~/.codex/skills/` and real agents under `~/.codex/agents/` — is told nothing about them, and is not told that Vertice did not look. That is the exact failure mode CA-11 exists to forbid, one level up: not "a client we probe is absent", but "a client we never probe is invisible".

The exploration (`exploration.md`) verified the Codex install on the reference machine directly:

- **Skills**: `~/.codex/skills/<name>/SKILL.md` — YAML frontmatter, shape-compatible with the existing skill contract.
- **Agents**: `~/.codex/agents/<name>.toml` — flat TOML, one file per agent, with `developer_instructions` as a multiline `"""…"""` string.
- **Installation**: `~/.codex/packages/standalone/releases/<version>-<target-triple>/bin/codex.exe`, reached through a symlink chain from `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin`. There is no `package.json` anywhere in the tree.

Three properties make this a cheap, well-shaped increment rather than a rewrite:

1. **The skill scanner is already client-agnostic.** `skills.rs` (`skills.rs:36-53,60-144`) walks any path for files named `SKILL.md` and contains no `ClientKind` parameter and no client field. Codex skills cost one new entry in `roots::skill_roots` and **zero lines** in `skills.rs`.
2. **`ClientKind` was designed to grow.** Its own doc comment (`model/installation.rs:20-23`) records that growth "is expected as later adapter phases land, but it stays a closed enum, never `#[non_exhaustive]`". Exactly two exhaustive matches on it exist in core, so the blast radius is compiler-enforced.
3. **The frontend needs no source change.** The client-presence table renders generically from `record.label` (`ScanPage.svelte:104-116`); there is no per-`ClientKind` branch anywhere in the Svelte sources, and `Component` carries no client field. A fourth and fifth presence row appear on their own. Bindings still regenerate — see the binding contract below.

The genuinely new work is the agent side: Codex agents are flat TOML, and `frontmatter::split` structurally cannot read them (`frontmatter.rs:26-29` requires an exact `---` first line).

## The decision this change supersedes

`openspec/changes/archive/2026-08-19-client-installation-detection/proposal.md:38` lists, under Out of Scope, "Detection of clients outside the closed `ClientKind` set (Copilot, Codex, …) — outside the PoC."

**This proposal supersedes that exclusion for Codex, and only for Codex.** The exclusion is not being called a mistake. It was written while T7 was still establishing the probe table, the version-source seam and the absence contract for the *first* time; adding a third vendor in the same change would have conflated "does client detection work at all" with "does it generalize". Since then:

- T7 shipped the per-OS probe seam and the shared, OS-agnostic resolver it was explicitly designed to make additive.
- `2026-08-23-report-client-presence-as-status` replaced string-matched absence with the typed `ClientPresence` / `ClientPresenceStatus` record, so a new client is now a new **row in a typed table**, not three new English strings the frontend must learn to match.

The seam that made the exclusion cheap to hold is the same seam that now makes lifting it cheap. **Copilot and every other client remain out of scope**; this change lifts the exclusion for one named client on the strength of directly verified machine evidence, and does not open the set.

## Approach

### Skills: a fourth root, and nothing else

`roots::skill_roots` gains a `codex-skills` entry (`~/.codex/skills`) built with the existing `resolve_single` pattern — `home` plus hardcoded segments, no `dirs`/`directories`, no environment read (`plan-desarrollo-poc.md:179`). Its return type moves from `[ResolvedRoot; 3]` to `[ResolvedRoot; 4]`. This is a real signature change, but the fixed-array pattern — "the CA-6/CA-14 guarantee, expressed in the type rather than asserted in prose" (`roots.rs:52-56`) — is preserved, not weakened. `skills.rs` is untouched.

Codex `SKILL.md` files carry keys the existing DTO does not model (`disable-model-invocation`, `user-invocable`, `license`, `metadata.*`). `SkillFrontmatter` (`frontmatter.rs:26-29`) has no `deny_unknown_fields`, so they are silently ignored today. **This change keeps that permissive behavior** and states it as a decision rather than inheriting it — see Open Decisions.

### Agents: a new parser seam and a third standalone adapter

Codex agents get a **new `toml.rs` seam**, mirroring `yaml.rs` and `jsonc.rs`: one module owns the TOML crate, everything else goes through `toml::from_str`, and a `toml_seam_invariant.rs` test pins the containment textually the way `tests/yaml_seam_invariant.rs:43-77` already does for `serde_norway`.

**Hand-rolling a parser for the observed three-key subset is rejected.** `developer_instructions` is a triple-quoted multiline string — precisely the class of value `AGENTS.md` already warns about ("Frontmatter parsing must not use regex — multiline block scalars break it"). Repeating that mistake in a second format, in a project that already documented it, is not a saving.

The adapter is a **third standalone module, `codex_agents.rs`** — not an abstraction over the existing three. `agents.rs:8-11` records that `SkillScan`/`AgentScan` are "deliberately separate types, not a shared abstraction (design §5.4)", and the two existing agent adapters are structurally unlike each other: `agents.rs` walks a directory of Markdown+frontmatter files with a typed DTO; `opencode_agents.rs` never walks a directory at all, deep-merging two JSON(C) configs value-level. `codex_agents.rs` is closest to `agents.rs` (file-per-component, flat `read_dir`, typed DTO) but substitutes `toml::from_str::<CodexAgentFrontmatter>` for `frontmatter::read::<AgentFrontmatter>`. **This change does not attempt to unify the three adapters behind a trait**; a reviewer reading three near-parallel adapters is reading a recorded decision, not copy-paste debt.

`roots.rs` also gains a `codex-agents` root (`~/.codex/agents`), and `consolidate::ROOT_ORDER` grows from six entries to eight, with its pinning test (`consolidate.rs:185-200`) kept synchronized.

### Installation detection: a new `VersionSource`, and `version.json` is not one

A new `InstallSlot::CodexStandalone` joins the Windows probe table. Neither existing `VersionSource` fits:

- `PackageJson` expects a `package.json` with a `"version"` string. There is none anywhere in the Codex tree.
- `DirectoryName` expects the version directory's bare name to *be* the version. Codex's is `0.149.0-x86_64-pc-windows-msvc`. Reusing `DirectoryName` would require splitting on the first `-`, which is silently wrong the day a prerelease tag appears (`0.150.0-rc.1-x86_64-pc-windows-msvc`).

**A new `VersionSource` variant with its own resolver** is therefore added. The enum is private and closed, and it grows by variant addition exactly as `InstallSlot` does; overloading `DirectoryName`'s semantics would corrupt the version string on any future release-naming change, in a field the UI displays as fact.

**`~/.codex/version.json` MUST NOT be used as a version source.** Its field is literally `latest_version`, alongside `last_checked_at` and `dismissed_version` — an update-availability cache that diverges from the installed version the moment an update is known but not applied. Reporting it would make Vertice display a version the user does not have. The release directory name is the trustworthy signal, and `codex --version` (`codex-cli 0.149.0`) corroborates it exactly. This also keeps the change clear of the "update status / is this version current" exclusion (`alcance-poc-vertice.md:13`).

**Multiple releases under `releases/` are N installations, never merged.** This maps directly onto the bundled-Claude precedent (`resolve_bundled_slot`, `installations.rs:445-533`): 1..N candidate roots, each resolved independently. That is CA-7's literal content, applied to a third client with no new design.

### Identity and consolidation: unchanged, and deliberately so

**Decided by the user on 2026-08-23: keep the current merge behavior.** `ComponentId::derive(kind, name)` gains no client discriminator; a Codex skill named identically to a Claude Code skill consolidates into one `Component` with two `Location` entries, exactly as a skill present under both `.claude/skills/` and `.agents/skills/` does today (`scan.rs:96-102`).

This is recorded here, not merely inherited. The "No Name-Convention Filtering" requirement (`openspec/specs/duplicate-consolidation/spec.md:41-49`) and the identity design were settled with two closely-related clients in view. Codex is a **separate vendor ecosystem with its own agent format**, and the counter-reading is real: merging a same-named Codex skill conflates "the user copied one skill into both places" with "two unrelated tools happen to use the same name". The decision is to accept that, because identity being blind to provenance is the model's stated intent (`location.rs:40-42`: "one root produces N components, not one client has N components") and every `Location` remains individually visible, so nothing is hidden from the user.

Consequences: `identity.rs`, `component.rs` and `consolidate.rs` need **no logic change**, and `duplicate-consolidation` needs **no spec delta** — its canonical-root-order requirement is expressed by reference to `roots.rs`, not by enumerating root ids (`spec.md:67-71`), so growing `ROOT_ORDER` satisfies it rather than contradicting it. **Where Codex sits in `ROOT_ORDER` is nonetheless a product decision, not a formality**: canonical order drives first-non-empty field precedence (`spec.md:73-77`), so it decides whose `description` a merged component shows. Appending Codex last means Claude Code and OpenCode metadata win; that is the proposed default and is stated so it is auditable.

### The binding contract (explicit obligation)

Adding `ClientKind::Codex` changes `frontend/src/bindings/ClientKind.ts` (`"claudeCode" | "openCode"` becomes three variants). Bindings are regenerated **only** by `cargo test -p vertice-core` and MUST NEVER be hand-edited. CI regenerates them and fails on any diff, running `git add --intent-to-add` first. This change MUST land the regenerated binding in the same commit as the Rust enum. No **source** change is expected under `frontend/src/` outside `bindings/`.

## Scope

### In Scope

- `model/installation.rs`: the `ClientKind::Codex` variant, plus the regenerated `frontend/src/bindings/ClientKind.ts`.
- `roots.rs`: a `codex-skills` skill root (`~/.codex/skills`) — `skill_roots` becomes `[ResolvedRoot; 4]` — and a `codex-agents` root (`~/.codex/agents`).
- New `crates/vertice-core/src/toml.rs` seam plus `tests/toml_seam_invariant.rs`, mirroring the YAML seam and its containment test.
- New `crates/vertice-core/src/codex_agents.rs` adapter: flat `read_dir` over `*.toml`, typed DTO, per-file issue isolation, `Component { kind: Agent, scope: User, … }`.
- `installations.rs`: a new `InstallSlot` variant with its label, a new `VersionSource` variant with its resolver, and a new branch in `windows_install_probes` covering 1..N entries under `packages/standalone/releases/`.
- `consolidate.rs`: `ROOT_ORDER` grows to eight entries; the `root_order_matches_the_roots_module_in_order` pinning test stays synchronized.
- `scan.rs`: the Codex agent adapter joins the orchestrator concatenation; the Codex skill root flows through the existing skill scan.
- One new Rust dependency (a TOML parser), with the `Cargo.toml` / `Cargo.lock` movement and a `cargo deny check bans licenses` re-run. The candidate crate is dual MIT/Apache-2.0, already inside `deny.toml:54-66`'s allow-list, so **no `deny.toml` edit is expected**.
- New fixture trees: Codex skills (happy path, corrupt frontmatter, extra Codex-specific keys), Codex agents (including a genuine multiline `"""…"""` `developer_instructions`), a Codex installation tree, a same-name-across-Codex-and-Claude consolidation home, and Codex slots added to the orchestrator fixtures.
- Fixture-first failing tests for every behavior above (`strict_tdd: true`).

### Out of Scope

- **macOS and Linux Codex paths** — **T16**, exactly as for the existing clients. `HostPlatform::Unsupported` behaviour is unchanged and MUST NOT be rewritten into per-slot `notDetected` rows.
- **Any change to component identity or consolidation logic** — settled above. No client discriminator, no name-convention rule, no content comparison.
- **`Project` / `Local` scope Codex components.** Codex components are emitted as `Scope::User` only.
- **Any frontend source change**: no client column, no per-client filter, no new i18n key. Only `frontend/src/bindings/` moves.
- **Codex MCP servers, config, auth, sessions, or prompts.** `~/.codex/` holds much more than skills and agents; only those two trees are read.
- **`~/.codex/version.json` as a version source**, and any update-status or upstream-comparison feature (`alcance-poc-vertice.md:13`).
- **Invoking any external binary.** `codex --version` and `codex doctor` are T16 manual oracles, never automated tests (`alcance-poc-vertice.md:132`).
- **Any refactor unifying `agents.rs`, `opencode_agents.rs` and `codex_agents.rs`** behind a shared trait.
- **Adding any client beyond Codex** to `ClientKind`. Copilot and the rest stay excluded.

## Capabilities

### New Capabilities

- `codex-agent-scanner`: reading `~/.codex/agents/<name>.toml` through the new TOML seam — input path, the DTO's field mapping onto `Component`, per-file error isolation, and the `ScanIssue` taxonomy for a malformed or unreadable TOML file.

### Modified Capabilities

- `skill-scanner`: the capability is written throughout in terms of "the three roots" (`spec.md:11,83,87-99,125-130`), including the plugin-exclusion argument ("the scanner only ever walks the three fixed roots") and the 69-entry reference pin. A fourth root changes the count in the prose without changing the rule. The **root-scoping argument for CA-6/CA-14 MUST be restated for four roots, not silently left at three.**
- `client-installation-detector`: "The scanner MUST probe three slots under `home`" (`spec.md:11`) and the scenario "A machine with no clients yields three notDetected records and zero issues" (`spec.md:70-74`, "exactly three records") both become four. A new requirement governs the Codex version source and the explicit rejection of `version.json`.
- `domain-model`: `ClientKind`'s closed two-variant set becomes three; the generated-TypeScript-contract requirement must reflect the regenerated binding.
- `workspace-architecture`: the "one module owns the parser" seam inventory grows from two seams (`yaml.rs`, `jsonc.rs`) to three, and the new dependency is subject to the same containment MUST.
- `scan-orchestration`: the orchestrator's adapter list gains a fourth component adapter; the "one bad adapter does not abort the scan" property must hold for it.

**Explicitly NOT modified**: `duplicate-consolidation` (canonical root order is defined by reference to `roots.rs`, and the merge rule is unchanged by user decision), `frontmatter-reader`, `agent-scanner`, `opencode-agent-scanner`, `inventory-ui`, `frontend-i18n`, `desktop-shell`.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/model/installation.rs` | Modified | `ClientKind::Codex` |
| `crates/vertice-core/src/toml.rs` | **New** | The TOML seam; sole importer of the TOML crate |
| `crates/vertice-core/src/codex_agents.rs` | **New** | Flat `read_dir` over `*.toml`, typed DTO, per-file isolation |
| `crates/vertice-core/src/roots.rs` | Modified | `codex-skills` (array 3→4) and `codex-agents` roots |
| `crates/vertice-core/src/installations.rs` | Modified | New `InstallSlot`, new `VersionSource` + resolver, new probe branch |
| `crates/vertice-core/src/consolidate.rs` | Modified | `ROOT_ORDER` 6→8 + pinning test; **no logic change** |
| `crates/vertice-core/src/scan.rs` | Modified | Fourth adapter wired into the orchestrator |
| `crates/vertice-core/src/lib.rs` | Modified | Two `pub mod` lines |
| `crates/vertice-core/src/skills.rs` | **Unchanged** | Already client-agnostic — the whole point |
| `crates/vertice-core/src/agents.rs`, `opencode_agents.rs`, `frontmatter.rs`, `yaml.rs`, `jsonc.rs` | **Unchanged** | No shared abstraction extracted |
| `crates/vertice-core/src/model/identity.rs`, `component.rs` | **Unchanged** | Identity decision above |
| `crates/vertice-core/tests/fixtures/` | New trees | Codex skills, agents, installation, consolidation, orchestrator slots |
| `crates/vertice-core/tests/` | New + Modified | New suites; existing root-count and slot-count assertions updated |
| `frontend/src/bindings/ClientKind.ts` | Regenerated | Three variants; never hand-edited |
| `frontend/src/` (source) | **Unchanged** | Presence renders from `record.label`; `Component` has no client field |
| `crates/vertice-app/` | **Unchanged** | No new command, no capability change |
| `Cargo.toml`, `Cargo.lock` | Modified | One new dependency |
| `deny.toml` | **Expected unchanged** | Dual MIT/Apache-2.0 is already allow-listed |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| The reference fixture's 69→25 pins (CA-2/CA-3/CA-4) shift because Codex components leak into the reference tree | **Med — the sharpest regression risk** | Codex fixtures live in **new homes**; `tests/fixtures/roots/reference/` is byte-identical and its 69/25/22/3 assertions stay green untouched, as the tripwire |
| Windows symlink/junction behaviour under `symlink_metadata` (`installations.rs:334-340`, deliberately non-following) does not resolve the `Programs\OpenAI\Codex\bin` → `current` → `releases\<version>-<triple>` chain as assumed | **Med — unverified** | Must be closed in `sdd-design` before any resolver code. The probe targets `packages/standalone/releases/` directly rather than following the chain, so the symlink is a corroborating signal, not a dependency |
| Fixtures containing a symlink or junction are not portable across the three CI legs, or need admin rights on Windows | Med | Fixtures MUST NOT contain symlinks; the release-directory tree is plain directories. If the design concludes a symlink is load-bearing, it must be constructed at test time, not committed |
| The new TOML crate's MSRV is above the workspace floor, breaking the `msrv` CI job | **Med — unverified** | Verify before pinning; the floor is declared in three places that must agree (`Cargo.toml` `rust-version`, CI `MSRV` env, `rust-toolchain.toml`) and a CI step fails on drift. If the crate's floor is higher, the dependency choice changes — the floor does not |
| The version string is parsed out of `0.149.0-x86_64-pc-windows-msvc` and silently corrupts on a prerelease tag | Med | The new `VersionSource` is designed for this shape explicitly; a `0.150.0-rc.1-<triple>` fixture must exist and fail before the resolver is written |
| Multiple `releases/` directories collapse to one installation, or a "highest wins" reduction sneaks in | Med | Follow `resolve_bundled_slot`'s 1..N shape; a two-release fixture asserts two `ClientInstallation` values with distinct versions and paths (CA-7) |
| A malformed Codex agent TOML aborts the Codex agent scan or the whole scan | Med | Per-file isolation mirroring `agents.rs::escalate`: one `Error` `ScanIssue` with the file path, every other agent still emitted (CA-12) |
| `ROOT_ORDER` grows but its pinning test is not updated, or Codex is inserted mid-order and silently changes which `description` wins for existing merged components | Med | The pinning test is the guard; Codex entries are **appended**, so precedence for existing components is provably unchanged, asserted by the untouched reference-fixture pins |
| A same-named Codex and Claude skill merging is read at review time as a bug | Med | Recorded as a user decision above, and covered by a dedicated fixture asserting one `Component` with two `Location`s — the behaviour is pinned, not incidental |
| No component-listing oracle exists for Codex (`codex agents` lists *sessions*; `codex debug` exposes only `models`/`app-server`/`prompt-input`), so upstream key-set drift is undetectable | Med | Accepted. The fixture set carries more weight than for Claude Code; the DTO models only the verified keys and ignores the rest, so drift adds fields rather than breaking parsing |
| The new dependency trips `cargo deny` licenses or bans | Low | Verified dual MIT/Apache-2.0 against `deny.toml:54-67`; `[bans]` covers only `tauri`/`tauri-build`. `vertice-core` must still import nothing from `tauri` |
| The regenerated `ClientKind.ts` is forgotten and CI's drift gate fails late | Low | Regenerate via `cargo test -p vertice-core` in the same commit as the enum; never hand-edit |
| Lifting an archived Out-of-Scope line sets a precedent that any client can now be added | Low | The supersession is scoped to Codex by name, justified by verified machine evidence and by T7's seam having shipped; the bar stays "the paths are verified on a real machine and the seam is additive" |

## Open Decisions

**Closed in this proposal:**

- **Identity and consolidation are unchanged** (user, 2026-08-23). No client discriminator; same-named components merge.
- **A `toml.rs` seam, not a hand-rolled parser.** Two existing precedents; the multiline-string class of bug is already documented in `AGENTS.md`.
- **A third standalone adapter, not a shared abstraction.** Per `agents.rs:8-11` / design §5.4.
- **A new `VersionSource` variant; `version.json` is never a version source.**
- **Windows only, `Scope::User` only, skills + agents only.** macOS/Linux at T16.
- **No frontend source change**; bindings regenerate.
- **`SkillFrontmatter` stays permissive** — no `deny_unknown_fields`. Codex's extra keys are ignored as they are today. Adding strictness would turn every upstream key addition into a scan failure across *all* clients, which is a worse product outcome than ignoring a field. Recorded rather than inherited, and reversible later.
- **Codex root ids are appended last in `ROOT_ORDER`**, so field precedence for existing components is unchanged.

**Committed to resolving in `sdd-design` — do not guess:**

- **Windows symlink/junction resolution.** Does `std::fs::symlink_metadata` report as assumed through the `bin` → `current` → `releases\<version>-<triple>` chain, and does the resolver need explicit target resolution using `std::fs` primitives only, under CA-16?
- **The TOML crate's MSRV** against the workspace floor, and which crate is pinned as a result.
- **The exact version-extraction rule** for `<version>-<target-triple>`: split on the first `-`, strip a known triple suffix, or match a leading version pattern — and what happens to a directory name that fits none of them (reported, skipped, or carried verbatim).
- **The `CodexAgentFrontmatter` field mapping**: which TOML keys map to `Component.name` / `description` / `provenance_hint`, and what a file missing `name` yields.
- **Whether the Codex agent root emits a `SearchRoot`**, and its `SearchRootKind`, mirroring how `opencode_agent_root` is treated.
- **Whether `codex doctor`** reports the installation root and version in a machine-checkable form — worth checking, but only as a T16 manual oracle, never as an automated test.

**Deferred, with target:**

- **macOS/Linux Codex path tables and their verification** — **T16**.
- **Oracle contrast against a real Codex install** (`codex --version`) — **T16**, manual.
- **Any per-client UI affordance** (client column, filter, grouping) — post-PoC.

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. Fixtures and failing tests land before implementation. Specifically, these MUST exist and fail first:

- A Codex agent fixture with a genuine multiline `"""…"""` `developer_instructions`, asserting the full value — the exact case a hand-rolled parser would break.
- A two-release installation fixture with different versions, asserting two un-merged `ClientInstallation` values (CA-7).
- A prerelease-shaped release directory name, asserting the extracted version.
- A same-named skill in a Codex root and a Claude Code root, asserting one `Component` with two `Location`s.
- A malformed `.toml` fixture, asserting one `Error` `ScanIssue` with its path and every other Codex agent still emitted (CA-12).
- A home with no `~/.codex` at all, asserting a `NotDetected` presence record and **zero** issues (CA-11).

## Changed-Line Forecast

| Bucket | Est. lines |
|---|---|
| `toml.rs` seam + `toml_seam_invariant.rs` | 60–100 |
| `codex_agents.rs` adapter + doc comments | 130–190 |
| `installations.rs`: slot, `VersionSource`, resolver, probe branch | 120–180 |
| `roots.rs` two roots + array widening | 30–50 |
| `consolidate.rs` `ROOT_ORDER` + pinning test | 10–20 |
| `scan.rs` / `lib.rs` / `model` wiring + binding | 25–50 |
| Fixtures (skills, agents, installation, consolidation, orchestrator) | 90–150 |
| Tests (adapter, installation, roots, consolidation, orchestrator, updated counts) | 250–360 |
| `Cargo.toml` / `Cargo.lock` | 5–15 |
| **Total** | **~720–1115** |

**Decision needed before apply: Yes. Chained PRs recommended: Yes. 400-line budget risk: High.** Natural slices, each independently green and independently revertible: (1) `ClientKind::Codex` + installation detection + presence row + fixtures; (2) the `codex-skills` root + its fixtures (near-zero implementation, meaningful test surface); (3) the `toml.rs` seam + `codex_agents.rs` + orchestrator wiring + `ROOT_ORDER`. Final slicing is `sdd-tasks`'s call.

## Rollback Plan

Additive at every layer. Three-layer revert, in dependency order.

1. **Core (`vertice-core`)** — delete `toml.rs`, `codex_agents.rs`, their tests and all Codex fixture trees; revert the two `lib.rs` `pub mod` lines; revert `roots.rs` to `[ResolvedRoot; 3]` and drop the `codex-agents` root; revert the `installations.rs` slot, `VersionSource` variant and probe branch; revert `ROOT_ORDER` to six entries and its pinning test; revert the `scan.rs` wiring; remove `ClientKind::Codex`. `identity.rs`, `component.rs`, `consolidate.rs`'s merge logic, `skills.rs` and the three existing parser seams have nothing to revert — they were never edited.
2. **Bindings** — `cargo test -p vertice-core` regenerates `ClientKind.ts` from the reverted enum. **Never hand-edited, in either direction.** The `--intent-to-add` gate confirms the revert is complete.
3. **Frontend source** — nothing to revert. The presence table renders whatever rows it receives; removing the Codex row removes it from the UI with no source change.
4. **Supply chain** — remove the TOML dependency from `Cargo.toml` and regenerate `Cargo.lock`. `deny.toml` is expected untouched, so `cargo deny check bans licenses` returns to its pre-change state automatically. This is the only layer whose revert is not free.

**`vertice-app` is untouched**, so the IPC surface and `capabilities/default.json` need no revert. **Migration: none** — the PoC persists nothing; `ScanReport` is rebuilt on every scan, so an old and a new report never coexist. A partial rollback (core reverted, binding not) fails at TypeScript compile time or at the CI drift gate, not silently at runtime. Reverting the branch restores the exact pre-change state.

## Dependencies

- **T7 / `client-installation-detection`** (archived) — complete. It shipped the per-OS probe seam, the shared resolver and the `VersionSource` enum this change extends. Its Codex exclusion is the line this proposal supersedes.
- **`report-client-presence-as-status`** (PR #32, archived 2026-08-23) — merged. It replaced string-matched absence with the typed `ClientPresence` record, which is what makes a new client a data row rather than a frontend change.
- **T4** (`roots.rs`, `skills.rs`) and **T5/T6** (the per-client adapter pattern) — complete and archived; extended by pattern.
- **T8** (`consolidate.rs`) — complete and archived; extended only by `ROOT_ORDER`.
- No blocking external dependency. **T16 gains scope** from this change (Codex macOS/Linux paths join the platform-validation list) but does not block it.

## Success Criteria

- [ ] A fixture home with `~/.codex/skills/<name>/SKILL.md` yields the Codex skills as `Component { kind: Skill, scope: User }`, with **zero lines changed in `skills.rs`**.
- [ ] A Codex `SKILL.md` carrying `disable-model-invocation`, `user-invocable`, `license` and `metadata.*` parses successfully; the unmodelled keys are ignored, not an error.
- [ ] A Codex agent `.toml` whose `developer_instructions` is a multiline `"""…"""` string yields the **complete, correct** value — no truncation at the first quote or newline.
- [ ] A malformed Codex agent `.toml` yields exactly one `ScanIssue` at `IssueSeverity::Error` carrying its path, while every other Codex agent in the same directory is still emitted (**CA-12**).
- [ ] A fixture home with a Codex installation yields a `ClientPresence` record with `status: Detected`, a `ClientInstallation` with `client: Codex`, the version taken from the **release directory name**, and its path pointing at that directory.
- [ ] A fixture home with **two** release directories at different versions yields **two** `ClientInstallation` values, never merged and never reduced to a highest-version winner (**CA-7**).
- [ ] A fixture home with **no** `~/.codex` yields a `NotDetected` Codex presence record and **zero** `ScanIssue`s — never an error and never a silent omission (**CA-11**).
- [ ] `~/.codex/version.json` is never read for a version; no code path consults `latest_version`.
- [ ] A same-named skill present in a Codex root and a Claude Code root consolidates into **one** `Component` with **two** `Location`s, both visible (**CA-8**, identity decision).
- [ ] `crates/vertice-core/tests/fixtures/roots/reference/` is byte-identical and its 69 / 25 / 22-with-3-locations / 3-with-1-location assertions stay green untouched (**CA-2/CA-3/CA-4**).
- [ ] No Codex project-scope or plugin-shaped tree appears in any result; the four-root scoping argument is restated in the `skill-scanner` delta (**CA-6**, **CA-14**).
- [ ] Every Codex path is composed from the passed-in `home` plus hardcoded relative segments; no `dirs`/`directories` import and no environment read is introduced (`plan-desarrollo-poc.md:179`).
- [ ] The TOML crate is imported by `toml.rs` and by no other module, pinned textually by `tests/toml_seam_invariant.rs`, mirroring the YAML seam invariant.
- [ ] No regular expression is used to parse any Codex file.
- [ ] `ROOT_ORDER` has eight entries in `roots.rs` order, and `root_order_matches_the_roots_module_in_order` passes.
- [ ] `frontend/src/` outside `bindings/` is byte-identical; `ClientKind.ts` is regenerated by `cargo test -p vertice-core`, committed, and never hand-edited.
- [ ] `crates/vertice-app/` and `capabilities/default.json` are byte-identical.
- [ ] `deny.toml` is byte-identical; `cargo deny check bans licenses` passes; `vertice-core` imports nothing from `tauri`.
- [ ] The workspace MSRV floor is unchanged, and `Cargo.toml` `rust-version`, the CI `MSRV` env and `rust-toolchain.toml` still agree.
- [ ] No `File::create`, `OpenOptions::write` or equivalent is introduced anywhere; nothing is written outside the app data directory (**CA-16**).
- [ ] Every assertion runs against `crates/vertice-core/tests/fixtures/`; no test reads the author's machine, sets an environment variable, or invokes `codex` (**CA-17**).
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, and `npm run lint && npm run check && npm run test && npm run build` all pass on the three-platform CI matrix.

## Proposal question round

The interactive question round could not be run from this phase. These are the product questions whose answers would change the proposal, with the assumption currently written into it. Answer, correct, or skip — a second round is available.

| # | Question | Assumption currently written in |
|---|---|---|
| 1 | Is lifting the archived Codex exclusion the intended product move now, or is the real want "make adding *any* client cheap" — a generalization this proposal deliberately does not do? | One named client, on verified evidence; the set stays closed and Copilot stays out |
| 2 | If a user has a Codex skill and a Claude Code skill with the same name but **different content**, is showing one merged entry with two paths the right product answer, or does that read as Vertice hiding a difference? | Merged, per the 2026-08-23 decision; both paths stay visible and content is never compared (that is post-PoC drift detection) |
| 3 | When Codex is installed but has no skills and no agents, should the UI say "Codex detected, no components" or is an empty list alongside a `Detected` presence row enough? | The typed presence row carries it; no new empty-state affordance is added |
| 4 | Is a Codex install whose release directory name cannot be parsed into a version better reported as "detected, version unknown" or as not detected at all? | Neither is settled — `sdd-design` must close it; the T7 precedent rejects a phantom entry with an empty version |
| 5 | Codex's `~/.codex/` also holds MCP config, auth and sessions. Is reading only `skills/` and `agents/` the right boundary, or does the user expect Vertice to notice more? | Skills and agents only; MCPs are an explicit PoC exclusion |
| 6 | Does one new Rust dependency for a third parser format cross any supply-chain line the project cares about, given the PoC ships unsigned? | Acceptable — same trade `jsonc.rs` already made, contained by the same seam discipline |
