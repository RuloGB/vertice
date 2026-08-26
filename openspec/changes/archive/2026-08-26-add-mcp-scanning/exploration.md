# Exploration: MCP Server Scanning (backend only)

Scope: `crates/vertice-core` and `crates/vertice-app`. The frontend is handled in a
separate session; it appears here only as a binding-contract boundary note.

## Decisions taken during exploration

1. **Redaction: key names only, never values.** `env` and `headers` are captured as
   `Vec<String>` of key names. No value reaches the model, the report, the IPC payload or
   the log.
2. **Transport lives on `Location`, not on `Component`.** One server name configured in
   several clients is ONE component with N locations, each keeping its own transport.
3. **Clients in scope: Claude Code, OpenCode, Codex.** Copilot deferred until
   `ClientKind::Copilot` exists.

## Executive summary

Vertice's scan pipeline is a set of independent, infallible adapters (`skills.rs`,
`agents.rs`, `opencode_agents.rs`, `codex_agents.rs`) composed by `scan.rs`, consolidated
by `consolidate.rs`, and exposed by two thin `#[tauri::command]` wrappers in
`crates/vertice-app/src/commands.rs`. The architecture already has, as an established
pattern, everything an MCP scan needs: a format seam per parser (`yaml.rs`, `jsonc.rs`,
`toml.rs`), a `roots.rs` resolver whose only ambient-environment read is `home_dir()`, and
a non-aborting `ScanIssue` diagnostics contract.

The genuinely new problem MCP introduces is not architectural but a data-shape and
security one: MCP server definitions carry `command`/`args`/`env` (stdio) or
`url`/`headers` (remote) — exactly the surface where API keys and bearer tokens live —
and the model has never had to redact anything.

Recommendation: model MCP servers as `Component` with a new `ComponentKind::Mcp`,
reusing the existing report/consolidation/IPC machinery, and add an `McpTransport` value
type that carries **key names only, never values**, making redaction structural rather
than conventional. Reuse `jsonc.rs` for Claude Code / OpenCode / Copilot and `toml.rs`
for Codex — no new format seam is required.

## Current state (verified, file:line)

### Model layer — `crates/vertice-core/src/model/`, zero I/O, `ts_rs`-exported

- `Component` (`model/component.rs:16-32`) — `id`, `name`, `kind`,
  `description: Option<String>`, `scope`, `locations: Vec<Location>`,
  `provenance_hint: Option<String>`.
- `ComponentKind` (`model/component.rs:40-43`) is a **closed, non-`#[non_exhaustive]`**
  enum: `Skill`, `Agent`. Adding a variant is by design a reviewed breaking change
  (comment at `component.rs:34-36`).
- `ComponentId::derive(kind, name)` (`model/identity.rs:26-29`) —
  `"{kind}:{normalized name}"`, normalization trim -> NFC -> lowercase
  (`identity.rs:55-57`). Depends only on `(kind, name)`, never on `Location` or content.
- `Location` (`model/location.rs:14-20`) — `path: Option<PathBuf>`, `root: SearchRootId`,
  `origin: LocationOrigin::{File,Embedded}`.
- `SearchRoot` / `SearchRootKind` (`model/location.rs:43-75`) — `SearchRootKind` mirrors
  `ComponentKind` 1:1 today, "because clients organize search roots per component kind"
  (`location.rs:66-68`).
- `ScanReport` (`model/report.rs:21-34`) — `components`, `installations`, `roots_scanned`,
  `issues`, `client_presence`, `duration_ms` (caller-supplied, never measured in `model/`).
- `ClientKind` (`model/installation.rs:27-31`) — closed enum `ClaudeCode`, `OpenCode`,
  `Codex`. **No Copilot variant exists.**
- `Freshness` / `FreshnessSubject` (`model/freshness.rs:19-36`) — "outdated" detection
  applies only to `ClientInstallation` version strings, compared against an upstream
  reference fetched in `vertice-app/src/freshness/`. It is not a per-`Component` concept,
  and there is no obvious upstream version oracle for an individual MCP server.
- `model/mod.rs:24-46` is the public surface; a new type must be re-exported there.
- Purity invariant: `model/`'s module doc bans `std::fs`, `std::io`, `std::env` and clock
  reads. Enforced by convention and review, not by a lint.

### Format seams — the pattern to imitate

Three seams, one file each, each the sole importer of its underlying crate:

- `yaml.rs:1-23` — `serde_norway`, used for skill/agent frontmatter.
- `jsonc.rs:1-97` — hand-rolled `JsonValue` enum over `jsonc-parser`; comments and
  trailing commas allowed, loose/unquoted keys explicitly rejected (`jsonc.rs:43-57`).
  Used today by `opencode_agents.rs`.
- `toml.rs:1-23` — `toml_seam` (renamed dependency alias so a containment test can grep
  for it); read-only by construction, no serialize function exposed. Added for Codex
  agents (`codex_agents.rs:173`).

### Adapters — three distinct precedents

- `opencode_agents.rs` — JSONC, entries under an object key inside 1-2 merged config
  files, no directory walk, no file-per-component. **This is the correct template for
  every JSON/JSONC MCP client.** It already solves per-file provenance without a second
  pass (`opencode_agents.rs:41-54`), deep-merge with last-wins-at-the-leaf semantics
  (`merge_two`, `opencode_agents.rs:231-252`), and wrong-type degradation to
  `None` + `Warning` instead of aborting (`opencode_agents.rs:166-189`).
- `codex_agents.rs` — TOML, one file per agent, flat `read_dir`, sorted for determinism.
  Relevant for the TOML seam usage, less so structurally (Codex MCP servers are table
  entries inside one file).
- `agents.rs` — Claude Code, `.md` per agent plus an embedded pseudo-root. Least relevant.

### Root resolution — `roots.rs`

`home_dir()` (`roots.rs:30-32`) is the crate's only ambient-environment read; every other
function takes `home: &Path`. Each client has a `resolve_*` function returning
`ResolvedRoot { root, scan_paths }`, where `scan_paths` may carry more than one path under
one logical root id (alias grouping: `opencode-agents` covers `opencode.json` +
`.jsonc` in merge order — `roots.rs:139-198`). `probe()` (`roots.rs:219-225`) is a single
`symlink_metadata` call, `NotFound` only on `ErrorKind::NotFound`.

### Orchestration and consolidation

`scan.rs:15-59` composes all adapters, concatenates roots/components/issues, calls
`consolidate::consolidate` once, and appends one `Warning` per distinct `NotFound` root
(`scan.rs:61-72`). `consolidate.rs` holds a hardcoded `ROOT_ORDER: [&str; 8]`
(`consolidate.rs:21-30`) pinned by `root_order_matches_the_roots_module_in_order`
(`consolidate.rs:190-206`). **Any new root must be appended there and in the pinning
test**, or the unknown-root fallback (`consolidate.rs:35-39`) silently ranks it last.

### App layer

`crates/vertice-app/src/commands.rs` exposes `scan`, `rescan`, `freshness`,
`user_settings`, `set_user_settings`, `log_file_path` (`lib.rs:57-64`), all thin
pass-throughs with zero business logic — stated as a design invariant at
`commands.rs:1-7`. Capabilities (`capabilities/default.json:1-7`) grant only
`core:default`; no fs/shell/dialog permission is needed because all I/O stays in the core.

`deny.toml:46-61` bans `tauri`/`tauri-build`/`reqwest` outside `vertice-app`. A new parsing
crate in `vertice-core` is acceptable if it is none of those and clears MSRV/license —
the precedent is the `toml` crate addition documented in
`openspec/changes/archive/2026-08-23-add-codex-client-support/design.md`.

### Testing

`scan.rs:74-423` is the pattern: versioned fixture homes under
`tests/fixtures/scan-orchestrator/<case>/`, never the real machine. The `complete`
fixture holds one directory per client under a synthetic `home`
(`.claude/agents/reviewer.md`, `.codex/agents/codex-agent.toml`,
`.config/opencode/opencode.json`). A `reference-volume` fixture proves read-only-ness via
before/after tree hashing (`scan.rs:213-246`) and enforces a <2s budget (CA-15).

### Redaction

There is **no existing secret/redaction handling anywhere** in `vertice-core` or
`vertice-app`. Grepping `redact|secret|token|apiKey` yields only unrelated HTTP plumbing
in `logging.rs` / `freshness/fetch.rs`. This is a new concern for this codebase.

## MCP configuration sources per client

**All rows below are web-sourced (August 2026) and ASSUMED.** They were deliberately not
verified against the user's real `~/.claude.json`, `~/.codex/config.toml`, etc., to avoid
reading live credentials into a persisted document. Re-verify with sanitized fixtures
during design.

| Client | Location(s) | Format | Root key | Notes |
|---|---|---|---|---|
| Claude Code | `~/.claude.json` (user), `.mcp.json` (project, git-shared), `~/.claude/settings.json` / `.claude/settings.json` | JSON | `mcpServers` | Project overrides user. `type` (`stdio`/`http`/`streamable-http`) plus `command`/`args`/`env` or `url`/`headers`. Plugin-provided MCPs are a further source. |
| Codex CLI | `~/.codex/config.toml`, optionally `.codex/config.toml` for trusted projects | TOML | `[mcp_servers.<name>]` | Underscore, not camelCase. Nested `[mcp_servers.<name>.env]`. Extra `startup_timeout_sec` / `tool_timeout_sec` fields with no analog elsewhere. |
| OpenCode | `~/.config/opencode/opencode.json` (global), `opencode.json` at project root (highest precedence) | JSON/JSONC | `mcp` | `type: "local"` (command) vs `type: "remote"` (`url` + `headers`). Remote OAuth tokens live in a separate file, `~/.local/share/opencode/mcp-auth.json` — a second sensitive source if ever read. |
| GitHub Copilot (VS Code) | `.vscode/mcp.json` (workspace), user-profile `mcp.json`, or `~/.copilot/mcp-config.json` | JSON | **`servers`** | Root key differs from every other client — a parsing gotcha, not just a path difference. |

Repo-verified: none of these paths or formats appear in `roots.rs` or any adapter today.
This is entirely new surface with no partial implementation to reconcile.

## Affected areas

- `model/component.rs` — add `ComponentKind::Mcp`. Closed enum; exhaustive matches must be
  updated. Blast radius looks small (`identity.rs:41-46`; adapters and `consolidate.rs`
  construct `Component` rather than matching on kind).
- `model/location.rs` — decide whether `SearchRootKind` gains a mirroring `Mcp` variant.
- `model/mod.rs` — export any new type.
- `roots.rs` — new MCP root resolver per client; JSON clients follow
  `opencode_agent_root`'s multi-file merge-order shape, Codex follows the single-file shape.
- New adapter module(s) `src/mcp_*.rs` (or one `mcp.rs` dispatching per client),
  respecting the no-abort `ScanIssue` contract.
- `jsonc.rs` / `toml.rs` — reused, not replaced. No new seam expected.
- `consolidate.rs` — append every new root id to `ROOT_ORDER` and its pinning test.
- `scan.rs` — wire the new adapters into `scan_for`; `append_missing_root_issues` is
  already generic over `roots_scanned` and needs no change.
- `Cargo.toml` / `deny.toml` — no change expected (no new dependency).
- `crates/vertice-app/src/commands.rs` — nothing, if MCP scanning is folded into the
  existing `scan`/`rescan`.
- `capabilities/default.json` — no change; all I/O stays in the core.
- **Frontend boundary (not planned here):** new `ts_rs`-exported types regenerate into
  `frontend/src/bindings/*.ts` via `cargo test -p vertice-core`, and CI's drift gate fails
  if the regeneration is not committed alongside the Rust change.
- Testing — new fixture trees per client under `crates/vertice-core/tests/fixtures/`, plus
  updates to the pinned counts in `scan-orchestrator/complete` if MCP scanning joins the
  main orchestrator suite immediately.

## Approach options

### Option 1 — Reuse `Component` with `ComponentKind::Mcp`, fold into the existing pipeline

- **Pros:** reuses `ScanReport`, `ScanIssue`, `consolidate`, identity and all IPC wiring
  for free; smallest diff against a codebase whose whole design is "N adapters, one
  report".
- **Cons:** `Component`'s shape does not carry `command`/`args`/`env`/`url`/`headers`/
  transport/enabled state. Either those fields are bolted onto the shared `Component`
  (all `Option`, polluting skills and agents), or MCP detail lives in a separate sibling
  type keyed by `ComponentId`, reintroducing a cross-payload join the codebase explicitly
  warns against (`model/freshness.rs:38-41`).
- **Effort:** medium.

### Option 2 — Parallel `McpServer` model with its own scan operation and report

- **Pros:** MCP-specific fields live in a purpose-built type with no `Option` pollution;
  freshness never gets forced onto MCP servers; ships without touching the skills/agents
  pipeline's stability guarantees (CA-15 budget, `ROOT_ORDER` pin).
- **Cons:** duplicates the roots -> adapters -> consolidate -> report shape for one more
  entity; the frontend later has to merge two report shapes; loses the "one Component, N
  Locations across clients" consolidation unless reimplemented.
- **Effort:** medium-high.

### Option 3 — Hybrid: `Component` with `kind: Mcp`, connection detail on `Location`

`Component` carries identity/listing/consolidation as usual; the connection detail is an
`Option<McpTransport>` field on **`Location`**, populated only for MCP locations.

- **Pros:** keeps Option 1's reuse for the inventory use case (the app's actual purpose)
  while isolating the MCP-only, secret-bearing fields; mirrors the existing "not every
  kind uses every field" tolerance already present as `description: Option<String>`; and
  it is the only placement that survives consolidation without losing information — one
  `github` server configured in three clients with three different commands stays one
  component with three locations, each keeping its own transport.
- **Cons:** widens `Location` (and its TS binding) with a nullable field that is `None`
  for every skill and agent; `Location` previously answered only "where is the file", so
  carrying connection detail is a new responsibility for the type and must be documented.
- **Effort:** medium.

## Recommendation

**Option 3**, with one firm sub-decision: redaction happens **inside the parsing/adapter
layer**, never in the model and never at the IPC boundary.

Concretely:

- Add `ComponentKind::Mcp` to the existing closed enum — a small, reviewed diff of exactly
  the kind `component.rs:34-36` anticipates.
- Add `McpTransport`:
  `Stdio { command, args, env_keys: Vec<String> }` /
  `Remote { url, header_keys: Vec<String> }`. **Key names only, never values.** This makes
  the redaction decision structural rather than a "remember not to log this" convention,
  the same way `Freshness::Unknown { reason }` makes degraded state a value rather than an
  error path. **`Location.mcp_transport: Option<McpTransport>`** — the transport lives on
  the location, not on the component, so a name configured in several clients keeps one
  transport per client.
- Identity: `ComponentId::derive(ComponentKind::Mcp, key)` where `key` is the config's
  server-name object key. This holds against "identity is `(kind, name)` alone", exactly
  as `opencode_agents.rs:159-210` already assembles components from config keys rather
  than file names.
- Parsing: `jsonc.rs` for Claude Code / OpenCode / Copilot (three root keys, three
  resolvers, one parser); `toml.rs` for Codex. No new seam.
- Fold into the existing `scan()` / `rescan()` commands — no new Tauri command, matching
  the "no business logic in `commands.rs`" convention.

This keeps the diff smallest where the codebase's precedent is strongest and introduces
exactly one new pattern (kind-conditional field plus structural redaction) where the
domain genuinely differs.

## Open questions, ranked

1. ~~**Redaction scope and mechanism**~~ — **RESOLVED (user decision).** Key names only,
   never values. `env` and `headers` are captured as `Vec<String>` of key names; no value
   ever enters the model, the report, the IPC payload or the log. A `url` is still a raw
   value, so the credential-in-query-parameter case must be handled explicitly at design
   time under this same rule rather than being treated as an exception to it.
2. **Does `ComponentKind::Mcp` get freshness support at all?** There is no natural
   "latest version" oracle for an arbitrary MCP server. Recommend declaring it explicitly
   out of scope for this cycle rather than leaving it silently absent.
3. ~~**Cross-client identity collisions**~~ — **RESOLVED (user decision).** The transport
   lives on `Location`. One name configured in several clients is ONE component with N
   locations, each carrying its own transport; nothing is discarded at merge time.
   Verified safe against the existing merge: `merge_into` concatenates locations with
   `target.locations.extend(other.locations)` (`consolidate.rs:104`) and never
   deduplicates them, a behavior already pinned by
   `total_location_count_is_conserved` (`tests/consolidation.rs:113`).

   Follow-up for design, not blocking: `location_key` (`consolidate.rs:48`) sorts by
   `(root_rank, root_id, path)`. Every MCP server from one client shares the same config
   file path, so that key is no longer unique *within a root* if a client ever exposes two
   entries for one name. Confirm the key stays total for MCP locations, or extend it.
4. **`SearchRootKind` symmetry** — does it need a mirroring `Mcp` variant?
   `location.rs:66-68`'s stated rationale argues for symmetry; confirm during design.
5. **Enabled/disabled state** — several clients allow disabling a server without removing
   its entry. New field, or dropped for v1? It cannot go in `provenance_hint`, which the
   domain-model spec requires to stay opaque.
6. ~~**Copilot needs `ClientKind::Copilot` first**~~ — **RESOLVED (user decision).** This
   cycle is scoped to Claude Code, OpenCode and Codex. Copilot is deferred to a later
   cycle that first adds `ClientKind::Copilot` to `model/installation.rs`, so this change
   touches one capability instead of also pulling in `client-installation-detector`.
7. **Fixture sourcing** — the four config-source claims are web-sourced only. Design needs
   sanitized real samples or authoritative upstream schema docs before fixtures can be
   trusted as accurate rather than merely self-consistent.

## Risks

- **Secret leakage is the primary risk of this feature, not an edge case.** `env` (stdio)
  and `headers` (remote) routinely carry API keys. The read-only invariant (CA-16) does
  nothing to prevent an in-memory `ScanReport` — which crosses IPC and can be logged, as
  `log_scan_report` / `log_freshness_report` already do by design — from carrying a raw
  credential. This must be enforced at the type level, because the codebase has zero
  existing redaction tooling to lean on.
- **Format-seam purity.** Adding JSON/TOML parsing is safe only through `jsonc.rs` /
  `toml.rs`. A contributor reaching for `serde_json` or a raw `toml::Value` inside an MCP
  adapter would violate the sole-importer containment the project already tests for.
- **`ROOT_ORDER` pinning test** (`consolidate.rs:184-206`) breaks the moment new roots are
  added without updating it. Low risk (loud failure), but guaranteed — sequence it in the
  task list.
- **`ComponentKind` is closed and non-`#[non_exhaustive]` by design.** Adding `Mcp` is a
  real breaking change for every downstream exhaustive match, including in the frontend
  once that session starts. The frontend session should be told this in advance.
- **No Copilot client-kind support exists.** Scoping Copilot in silently expands the change
  into `installation.rs` / `ClientKind`, a different capability
  (`client-installation-detector`) than the one this feature nominally touches.

## Suggested next step

`sdd-propose`, scoped to **Claude Code + OpenCode + Codex** MCP scanning, carrying the
three decisions above as fixed constraints. Open questions 2, 4, 5 and 7 (freshness scope,
`SearchRootKind` symmetry, enabled/disabled state, fixture sourcing) remain for the
proposal to settle before design locks the type shapes.
