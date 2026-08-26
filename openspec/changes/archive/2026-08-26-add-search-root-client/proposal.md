# Proposal: Model the AI Client That Owns a Search Root

> Spec trace: this change is scoped from the living specs, not from the completed PoC roadmap. It extends `domain-model` (a new optional field on `SearchRoot`) and touches `inventory-ui` (the frontend already has three placeholder "AI Clients" sections waiting for data). It does NOT modify `scan-orchestration`, `duplicate-consolidation`, `mcp-scanner`, `skill-scanner`, `agent-scanner`, `opencode-agent-scanner`, `codex-agent-scanner`, `component-freshness`, `client-installation-detector`, `frontmatter-reader`, `frontend-i18n`, `desktop-shell`, `user-settings`, `application-logging`, or `ci-quality-gates`.
>
> Invariants this change is bound by and MUST NOT weaken: the **read-only invariant (CA-16)** — no write operations introduced; **versioned-fixture testing (CA-17)** — every assertion runs against fixtures, never the author's machine; the **core purity invariant** — `vertice-core` imports nothing from `tauri`; the **`model/` purity invariant** — no `std::fs`, `std::io`, `std::env` or clock read; the **type-contract invariant** — regenerated bindings land in the same commit.
>
> **Backend and frontend.** `crates/vertice-core` (the model field and root construction), `crates/vertice-app` (no change expected), and `frontend/src/` (consume the new field to populate the existing placeholder sections).
>
> **No new Rust dependency, no new Tauri command, no new IPC surface.**

## Intent

Vertice inventories AI components across AI clients. Today, every search root carries `id`, `path`, `kind` and `status` — and **deliberately no client field** (`model/location.rs:84`). The client that owns a root is recoverable only by inspecting the root id's string prefix (`claude-skills`, `opencode-agents`, `codex-mcp`, `agents-skills`) and recognizing the convention. No consumer can branch on this safely: it is a naming convention, not a contract.

Three concrete symptoms make this a well-shaped increment:

1. **The frontend already has the waiting surface.** `AgentDetail.svelte`, `SkillDetail.svelte` and `McpDetail.svelte` each carry a placeholder "AI Clients" section with empty-state text, wired and ready for data. The UI is blocked on the model, not on missing components.
2. **The type already exists.** `ClientKind` (`model/installation.rs:27`) is a closed enum with exactly the three variants needed — `ClaudeCode`, `OpenCode`, `Codex`. No new type is required.
3. **The convention is already embedded in root ids.** Every root id in `roots.rs` carries a client prefix (`claude-*`, `opencode-*`, `codex-*`) — except `agents-skills`, which is a shared root with no single owner. Making the client an explicit typed field converts convention into contract, and `Option<ClientKind>` handles the shared-root case honestly.

This is a small, low-risk model change. It does not alter scanning, consolidation, identity, or the IPC surface. It adds one field, populates it at root construction time, regenerates bindings, and the frontend consumes it.

## The decisions this proposal carries as fixed

These were settled by exploration and are **not** re-opened here.

### 1. `SearchRoot` gains `client: Option<ClientKind>`

A new field on the existing struct. `Option` because not every root has a single client owner: `agents-skills` (`~/.agents/skills/`) is a shared directory used by multiple AI clients and has no single owner. `None` is the truthful answer for that root, the same pattern `Location.path: Option<PathBuf>` uses for embedded components with no file on disk.

### 2. Reuse `ClientKind`, do not create a new type

`ClientKind` already exists in `model/installation.rs` with exactly the three variants the roots need. Creating a parallel enum would be two sources of truth for the same concept. Adding a variant to `ClientKind` in a future cycle (e.g. `Copilot`) automatically propagates to roots through the type system.

### 3. The mapping is populated at root construction, not derived lazily

Each root resolver in `roots.rs` sets `client` when it constructs the `SearchRoot`. The mapping from root id to client is hardcoded in `roots.rs`, exactly where the root id string is already hardcoded. This keeps the convention and the typed field in the same file, so they cannot drift.

A derive function (e.g. `fn client_for_root(id: &SearchRootId) -> Option<ClientKind>`) was considered and rejected: it would keep the convention hidden behind a function, could not cross the IPC boundary as structured data, and would force the frontend to call a function rather than read a field.

### 4. No mapping table; the field lives on `SearchRoot`

An external mapping table (root id → client) was considered and rejected because it creates a second source of truth for what root ids already encode. The field on `SearchRoot` is the single source of truth, populated at construction.

## The decisions this proposal settles

These were open after exploration. They are closed here so design can lock type shapes without guessing.

### 5. `agents-skills` carries `client: None`

`~/.agents/skills/` is a shared directory. It is not owned by Claude Code, OpenCode, or Codex exclusively. `None` honestly represents "no single client owner" — the same pattern `Location.path: None` uses for embedded components. The frontend MUST handle `None` gracefully, displaying "shared" or equivalent rather than omitting the root.

### 6. The frontend "AI Clients" sections consume this field

The three detail pages (`AgentDetail.svelte`, `SkillDetail.svelte`, `McpDetail.svelte`) currently show placeholder empty-state text. With this field available, they MUST group the component's locations by `client`, showing which AI clients can use this component. Locations with `client: None` are labeled "shared" or equivalent (i18n key required).

### 7. No change to `SearchRootId` naming convention

Root ids keep their current naming (`claude-skills`, `opencode-agents`, etc.). The `client` field is additive — it does not replace or rename the id. The id remains useful as a stable, human-readable identifier; the field provides the typed, branchable value.

## Approach

### Model: one field

- **`SearchRoot.client: Option<ClientKind>`** — `None` for shared roots (`agents-skills`), `Some(_)` for every client-specific root.

`model/` stays pure: this is a plain data field with zero I/O, no clock read, and no import outside the declared allow-list. `ClientKind` is already defined in `model/installation.rs` and already derives `Serialize`, `Deserialize`, and `TS`.

### Root construction: populated at the source

Every root constructor in `roots.rs` gains the `client` field:

- `resolve_single` and `resolve_pair` gain a `client: Option<ClientKind>` parameter.
- `skill_roots`: `claude-skills` → `Some(ClientKind::ClaudeCode)`, `agents-skills` → `None`, `opencode-skills` → `Some(ClientKind::OpenCode)`, `codex-skills` → `Some(ClientKind::Codex)`.
- `agent_roots`: `claude-agents` → `Some(ClientKind::ClaudeCode)`, `claude-embedded-agents` → `Some(ClientKind::ClaudeCode)`.
- `claude_mcp_root` → `Some(ClientKind::ClaudeCode)`.
- `opencode_mcp_root` → `Some(ClientKind::OpenCode)`.
- `codex_mcp_root` → `Some(ClientKind::Codex)`.

### Identity: unchanged

`ComponentId` derives from `(kind, name)` alone. This change does not touch `identity.rs`.

### Consolidation: unchanged

`consolidate.rs` merges by identity and concatenates locations. The `client` field lives on `SearchRoot`, not on `Location` or `Component`, so consolidation is unaffected.

### Frontend: populate the placeholder sections

The three detail pages group locations by `client`:

- Each location's `client` value is read from the `SearchRoot` it belongs to.
- Locations are grouped: one section per `Some(ClientKind)` variant present, plus a "shared" section for `None`.
- The empty-state placeholder text is replaced with actual data when locations are present.

**Wait — how does the frontend know which `SearchRoot` a `Location` belongs to?** This is the key design question. Today, `Location` does not carry a reference to its `SearchRoot`. The frontend receives `ScanReport.roots` and `ScanReport.components` as separate collections. The link between a location and its root is implicit: a location was produced by scanning a root, but that provenance is not carried on the `Location` type.

Two options:

- **Option A: Add `root_id: SearchRootId` to `Location`.** Each location carries the id of the root that produced it. The frontend joins `locations` with `roots` by `root_id`, then reads `root.client`. This is the most explicit approach but widens `Location` with a field that is only useful for this purpose.
- **Option B: The backend populates a `client` field on `Location` directly.** Instead of (or in addition to) carrying the root id, each `Location` carries `client: Option<ClientKind>` copied from the root that produced it. This is simpler for the frontend (no join needed) but duplicates data — every location from the same root carries the same client value.
- **Option C: The frontend infers client from the root id string.** The frontend parses the root id prefix (`claude-*` → `ClaudeCode`, etc.). This keeps the convention alive and is exactly what this proposal exists to eliminate. Rejected.

**This proposal recommends Option B**: `Location` gains `client: Option<ClientKind>`, populated from the root at scan time. The rationale is the same as `mcp_transport: Option<McpTransport>`: it is the only placement that survives consolidation without losing information, and it avoids a cross-collection join in the frontend. The duplication is accepted — it is small (one enum variant per location) and the alternative (a root id join) adds complexity to every consumer.

**Consequence**: `Location` gains a second kind-conditional optional field (`client` alongside `mcp_transport`). The pattern is established and documented.

### The binding contract (explicit obligation)

Adding `SearchRoot.client` and `Location.client` regenerates files under `frontend/src/bindings/`. Bindings are produced **only** by `cargo test -p vertice-core` and MUST NEVER be hand-edited. CI regenerates them and fails on any diff. **The regenerated bindings MUST land in the same commit as the Rust change.**

## Scope

### In Scope

- `model/location.rs`: `SearchRoot.client: Option<ClientKind>`.
- `model/location.rs`: `Location.client: Option<ClientKind>`.
- `roots.rs`: every root constructor populates the `client` field.
- `scan.rs` / adapter layer: each adapter copies the root's `client` onto the `Location`s it constructs.
- `frontend/src/`: the three detail pages consume `Location.client` to populate the "AI Clients" sections.
- `frontend/src/bindings/`: regenerated.
- New i18n keys for "shared" client label (en/es).
- Tests: fixture-first, covering `Some(_)` and `None` cases.

### Out of Scope

- **Any change to `ClientKind` itself.** No new variant. `Copilot` remains a future cycle.
- **Any change to `SearchRootId` naming.** Root ids keep their current convention.
- **Any change to identity, consolidation, or the IPC surface.**
- **Any change to `scan-orchestration`'s adapter list or error handling.**
- **Any change to `component-freshness`.**
- **Any change to `client-installation-detector`.**
- **Any write operation.** CA-16 is unchanged.
- **Any new Rust dependency.**
- **Any new Tauri command or capability.**
- **Project-scope or Local-scope roots.** `Scope::User` only, unchanged.

## Capabilities

### New Capabilities

None. This change does not introduce a new capability — it enriches existing types with a field that was always derivable by convention.

### Modified Capabilities

- **`domain-model`** — `SearchRoot` gains `client: Option<ClientKind>`; `Location` gains `client: Option<ClientKind>`; the generated-TypeScript-contract requirement must reflect the regenerated bindings.
- **`inventory-ui`** — the three detail pages consume `Location.client` to populate the "AI Clients" sections, replacing placeholder empty-state text with actual per-client grouping.

**Expected NOT modified**: `scan-orchestration`, `duplicate-consolidation`, `mcp-scanner`, `skill-scanner`, `agent-scanner`, `opencode-agent-scanner`, `codex-agent-scanner`, `component-freshness`, `client-installation-detector`, `frontmatter-reader`, `frontend-i18n` (only new keys, no structural change), `desktop-shell`, `user-settings`, `application-logging`, `ci-quality-gates`, `workspace-architecture`.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/model/location.rs` | Modified | `SearchRoot.client: Option<ClientKind>`; `Location.client: Option<ClientKind>` |
| `crates/vertice-core/src/roots.rs` | Modified | Every root constructor populates `client` |
| `crates/vertice-core/src/scan.rs` | Modified | Adapters copy root's `client` onto constructed `Location`s |
| `crates/vertice-core/src/skills.rs` | Modified | Pass `client` through when constructing locations |
| `crates/vertice-core/src/agents.rs` | Modified | Pass `client` through when constructing locations |
| `crates/vertice-core/src/opencode_agents.rs` | Modified | Pass `client` through when constructing locations |
| `crates/vertice-core/src/codex_agents.rs` | Modified | Pass `client` through when constructing locations |
| `crates/vertice-core/src/mcp*.rs` | Modified | Pass `client` through when constructing locations |
| `crates/vertice-core/tests/` | Modified | Existing tests updated for new field; new tests for `Some(_)` and `None` cases |
| `frontend/src/bindings/` | Regenerated | `SearchRoot.ts`, `Location.ts` gain `client` field; never hand-edited |
| `frontend/src/lib/pages/AgentDetail.svelte` | Modified | "AI Clients" section consumes `Location.client` |
| `frontend/src/lib/pages/SkillDetail.svelte` | Modified | "AI Clients" section consumes `Location.client` |
| `frontend/src/lib/pages/McpDetail.svelte` | Modified | "AI Clients" section consumes `Location.client` |
| `frontend/src/lib/i18n/en.ts`, `es.ts` | Modified | New keys for "shared" client label |
| `frontend/src/` (tests) | Modified | Fixtures and assertions updated |
| `crates/vertice-app/` | **Unchanged** | No new command; `commands.rs` stays logic-free |
| `crates/vertice-app/capabilities/default.json` | **Unchanged** | `core:default` only |
| `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Expected unchanged** | No new dependency |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| **`Location` widening pollutes every location with a `null` field in the TS binding** | Med | Accepted and documented on the type. It mirrors the existing `mcp_transport: Option<McpTransport>` pattern — kind-conditional optional fields on `Location` are an established shape |
| **Forgetting to populate `client` on a new root in the future** | Low | The field is non-optional (`Option<ClientKind>` is still a required field — it must be explicitly `Some(_)` or `None`). A constructor that omits it fails to compile |
| **The frontend join between locations and roots is implicit and fragile** | Med | Option B (client on Location) eliminates the join entirely. The frontend reads `location.client` directly — no root lookup needed |
| **`agents-skills` with `None` confuses the frontend** | Low | The frontend MUST handle `None` as "shared", with an i18n key. The empty-state placeholder already establishes the pattern |
| **The regenerated bindings are forgotten and CI's drift gate fails late** | Low | Regenerate via `cargo test -p vertice-core` in the same commit; never hand-edit |
| **A future `ClientKind::Copilot` variant requires updating every root constructor** | Low | Compiler-enforced: adding a variant breaks every `match` that does not cover it. The root constructors use `Some(ClientKind::X)` directly, so the compiler flags every site |

## Open Decisions

**Closed in this proposal:**

- **`SearchRoot` gains `client: Option<ClientKind>`** (decision 1).
- **`Location` gains `client: Option<ClientKind>`**, populated from the root at scan time (Option B from the design question).
- **`agents-skills` carries `client: None`** (decision 5).
- **Reuse `ClientKind`**, no new type (decision 2).
- **Populated at root construction**, not derived lazily (decision 3).
- **No mapping table** (decision 4).
- **Root ids keep their current naming** (decision 7).
- **The frontend consumes the field** to populate the "AI Clients" sections (decision 6).

**Committed to resolving in `sdd-design` — do not guess:**

- **How `scan.rs` passes the root's `client` to adapters.** The current adapter signatures take `&Path` or similar; design must decide whether the client is passed as an additional parameter, or whether adapters receive the full `ResolvedRoot` (which already carries the `SearchRoot`). The project's precedent (adapters receive minimal context) argues for an additional parameter.
- **Whether `Location.client` is populated in the adapter layer or in `scan.rs` after the adapter returns.** The adapter knows which root it is scanning; `scan.rs` could also copy the field. Design must decide which is cleaner given the existing code structure.
- **The i18n key names and translated strings** for "shared" (en/es).
- **The exact grouping and display logic** in the three detail pages — one section per client present, or a flat list with client labels?

**Deferred, with target:**

- **`ClientKind::Copilot`** — a later cycle, gated on adding the variant and its adapter.
- **Project-scope and Local-scope roots** — post-cycle, gated on project-root discovery existing at all.
- **Enabled/disabled state for MCP servers** — a follow-up cycle (P3 in `pendientes-desarrollo.md`), gated on verified consistent semantics.

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. Fixtures and failing tests land before implementation. These MUST exist and fail first:

- A root constructed with `client: Some(ClientKind::ClaudeCode)` serializes and deserializes with the field intact.
- A root constructed with `client: None` (the `agents-skills` case) serializes and deserializes with `client: null` in JSON.
- A `Location` produced by the Claude Code skill adapter carries `client: Some(ClientKind::ClaudeCode)`.
- A `Location` produced by the `agents-skills` root carries `client: None`.
- The frontend detail pages group locations by client, with `None` labeled "shared".
- The existing reference-fixture pins stay green.
- The regenerated bindings include the `client` field on both `SearchRoot` and `Location`.

## Changed-Line Forecast

| Bucket | Est. lines |
|---|---|
| `model/location.rs`: two new fields, docs | 20–30 |
| `roots.rs`: populate `client` at every constructor | 30–50 |
| Adapter layer (`skills.rs`, `agents.rs`, `opencode_agents.rs`, `codex_agents.rs`, `mcp*.rs`): pass `client` through | 40–70 |
| `scan.rs`: wire `client` into location construction | 15–25 |
| Tests (model, roots, adapters, updated counts) | 80–120 |
| Regenerated bindings | 10–20 |
| Frontend detail pages (3 files) | 60–100 |
| Frontend i18n keys | 10–15 |
| Frontend tests | 40–60 |
| **Total** | **~305–490** |

**Decision needed before apply: No — the proposal is fully closed. Chained PRs: not required (under 400 lines).**

## Rollback Plan

Additive at every layer. Two-layer revert (core + frontend), in dependency order.

1. **Core (`vertice-core`)** — remove `client` from `SearchRoot` and `Location`; revert `roots.rs` constructors; revert adapter signatures; revert `scan.rs` wiring. `identity.rs`, `consolidate.rs`, `installations.rs`, and all parser seams have nothing to revert.
2. **Bindings** — `cargo test -p vertice-core` regenerates them from the reverted types.
3. **Frontend source** — revert the three detail pages and i18n keys to their placeholder state.
4. **Supply chain** — nothing to revert. No dependency added.

**`vertice-app` is untouched**, so the IPC surface and `capabilities/default.json` need no revert. **Migration: none** — nothing is persisted; `ScanReport` is rebuilt on every scan. Reverting the branch restores the exact pre-change state.

## Dependencies

- **`domain-model`** (living spec) — complete. This change extends `SearchRoot` and `Location` by one field each.
- **`inventory-ui`** (living spec) — complete. This change populates the placeholder sections it already defines.
- **`add-mcp-scanning`** (archived 2026-08-26) — complete. Decision 9 in its proposal explicitly deferred this work to its own cycle. This proposal fulfills that deferral.
- **No blocking external dependency.**

## Success Criteria

- [ ] `SearchRoot` carries `client: Option<ClientKind>`, populated at construction in `roots.rs`.
- [ ] `Location` carries `client: Option<ClientKind>`, populated from the root at scan time.
- [ ] `agents-skills` carries `client: None`; every other root carries `Some(_)`.
- [ ] `ClientKind` is reused — no new type is introduced.
- [ ] Root ids keep their current naming convention.
- [ ] The three frontend detail pages group locations by `client`, with `None` labeled "shared" (i18n).
- [ ] The placeholder "AI Clients" empty-state text is replaced with actual data when locations are present.
- [ ] `identity.rs` is **unchanged**.
- [ ] `consolidate.rs` merge logic is **unchanged**.
- [ ] `crates/vertice-app/` and `capabilities/default.json` are byte-identical; no new IPC command exists.
- [ ] Bindings are regenerated by `cargo test -p vertice-core`, committed in the same change, and never hand-edited.
- [ ] Every assertion runs against versioned fixtures; no test reads the author's real configuration (CA-17).
- [ ] No `File::create`, `OpenOptions::write` or equivalent is introduced (CA-16).
- [ ] `Cargo.toml`, `Cargo.lock` and `deny.toml` are byte-identical; `cargo deny check bans licenses` passes; `vertice-core` imports nothing from `tauri`.
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, and `npm run lint && npm run check && npm run test && npm run build` all pass on the three-platform CI matrix.
