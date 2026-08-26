# Design: Model the AI Client That Owns a Search Root

> Spec trace: closes the proposal's "Committed to resolving in `sdd-design`" list (`proposal.md:198-204`). Bound by the delta specs in `specs/` — `domain-model` and `inventory-ui` — which are approved and are **not** modified here. This document writes **no spec and no task**.
> Provenance: `internal-docs/pendientes-desarrollo.md:49` (P2); the MCP design §2 ("No client field anywhere") explicitly deferred this work to its own cycle, and this is that cycle.
> Invariants this design may not weaken: **CA-16** (read-only), **CA-17** (versioned fixtures only), core purity (no `tauri`), `model/`'s import allow-list, and the binding contract (regenerated bindings land in the same commit, never hand-edited).

## 0. What is verified

| # | Statement | Basis |
|---|---|---|
| **V1** | `Location` is constructed as a struct literal at **9** sites in `src/` (`skills.rs:134`, `agents.rs:187`, `agents.rs:211`, `codex_agents.rs:180`, `opencode_agents.rs:194`, `mcp_claude.rs:166`, `mcp_opencode.rs:166`, `mcp_codex.rs:441`, plus the test helper `consolidate.rs:154`) and **7** in `tests/model_contract.rs`. A new field breaks every one at compile time — loud, never silent | grep of `Location {` across `crates/` |
| **V2** | `SearchRoot` literals: **5** in `roots.rs` (102, 126, 167, 236, 270), **3** in `model/location.rs`'s inline tests, `sample_search_root` in `tests/model_contract.rs:13`, and **one in `crates/vertice-app/src/commands.rs:586`** — inside a `#[cfg(test)]` module, the only construction site outside `vertice-core` | grep of `SearchRoot {` |
| **V3** | Six of the eight `Location`-producing paths already receive the full `ResolvedRoot`: `agents::walk_agents_root` (`agents.rs:86-87`), `codex_agents::walk_agents_root` (`codex_agents.rs:68-69`), `opencode_agents::assemble_component` (`opencode_agents.rs:71-77`), and all three MCP `assemble_component`s (`mcp_claude.rs:138-144`). Only **two** helpers take minimal context: `skills::walk_one(scan_path, root_id: &SearchRootId, …)` (`skills.rs:60-64`) and `agents::emit_embedded_components(embedded_root_id: &SearchRootId, …)` (`agents.rs:203`) | Verbatim source |
| **V4** | `scan_for` never touches adapter output after the call: it only `extend`s roots, components and issues (`scan.rs:35-57`). There is **no** `scan.rs → adapter` hand-off point today | `scan.rs` |
| **V5** | `ClientKind` already derives `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS`, is camelCase (`"claudeCode" \| "openCode" \| "codex"`), and is already re-exported from `model/mod.rs` and bound at `bindings/ClientKind.ts` | `model/installation.rs:24-31` |
| **V6** | `domain-model` spec: "`SearchRoot` MUST NOT carry a client display name or label — client identification for UI purposes is derived elsewhere from `SearchRootKind`" (`specs/domain-model/spec.md:100-108`). A typed `client` field needs that requirement's wording adjusted by the delta spec — a **spec-level** reconciliation, flagged for `sdd-spec` | spec.md |
| **V7** | Verified product finding: skill roots are **read by more clients than they belong to** — OpenCode resolves skills from `~/.claude/skills/` and `~/.agents/skills/` (`internal-docs/alcance-poc-vertice.md:114-116`). `client` therefore means **owner**, not the full reader set | alcance doc |
| **V8** | Client display names are hardcoded proper nouns in `ClientsPage.svelte:25-47` (`name: "Claude Code"`), never i18n keys — the house precedent for this page type | ClientsPage.svelte |
| **V9** | `reference-volume`'s tree-snapshot equality snapshots the **fixture filesystem**, not the serialized report (`scan.rs:250-258`); the `complete` fixture pins roots **11**, components **15**, `issues.is_empty()` (`scan.rs:107-120`). Neither moves for an additive field | `scan.rs` |
| **V10** | `complete`'s two-location `shared` skill lives in `.claude/skills/shared/` **and** `.agents/skills/shared/` — i.e. one `Some(ClaudeCode)` location plus one `None` location, already committed. The `Some` + `None` pair needs **no new fixture** | fixture tree |

## 1. Technical approach

Two additive `Option<ClientKind>` fields, populated at two well-defined sites, consumed by three pages:

```
 roots.rs constructors ──> SearchRoot.client ──> Location.client ──> frontend
 (hardcoded per root id,    (adapter copies it   (at the Location    groupLocationsByClient()
  same file as the id)       from the root it     literal — compiler  dedupe, fixed order,
                             already holds)       forced)             label + count per page
```

`scan.rs`, `identity.rs`, `consolidate.rs` logic, `installations.rs`, every parser seam, and the IPC command surface are **unchanged**.

## 2. Core data model changes

| Type | Change |
|---|---|
| `SearchRoot` | **`client: Option<ClientKind>` appended after `status`** (`location.rs:58-63`) |
| `Location` | **`client: Option<ClientKind>` appended after `mcp_transport`** (`location.rs:19-32`) — the second optional context field, mirroring the `mcp_transport` precedent the proposal already accepted |
| `ClientKind` | **Unchanged** — reused, no new variant, no new type (proposal decision 2) |
| `Component`, `ComponentId`, `Scope`, `ScanReport`, `ScanIssue`, `SearchRootId`, `SearchRootKind`, `SearchRootStatus`, `McpTransport`, `FreshnessSubject` | **Unchanged** — a diff in any of their bindings means something leaked |

```rust
pub struct SearchRoot {
    pub id: SearchRootId,
    pub path: PathBuf,
    pub kind: SearchRootKind,
    pub status: SearchRootStatus,
    /// The client that owns this search root. `None` for a shared root
    /// with no single owner (`agents-skills`) — the same honest-`None`
    /// pattern `Location.path` uses for embedded components. Ownership,
    /// NOT the set of clients able to read the root (V7).
    pub client: Option<ClientKind>,
}
```

`Location`'s new field, with its doc contract:

```rust
    /// The owning client, copied from the `SearchRoot` that produced this
    /// location. `None` means "shared root", never "unknown"; consumers
    /// MUST NOT treat it as an error and MUST NOT infer component kind
    /// from it.
    pub client: Option<ClientKind>,
```

Field placement is **append-last** in both structs: additive, serde field order is name-independent, and the diff stays one line per struct. `model/` purity holds: `ClientKind` comes from the sibling `model/installation.rs` — no import outside the declared allow-list, zero I/O.

Two stale texts this change must update, recorded so they are not missed: `model/location.rs:94-97`'s test doc ("the type carries no client-label field") and the MCP design's "no client field anywhere" note are both superseded — the first is rewritten in place, the second is history in an archived document and stays untouched.

## 3. Decision — how `client` reaches the adapters (open question 1)

**The question's premise does not hold, and that is the answer.** `scan.rs` does not pass roots to adapters: every adapter takes `home: &Path`, resolves its own roots via `roots::*`, and `ResolvedRoot` already embeds the `SearchRoot` (V3, V4). `client` therefore travels inside a structure every adapter already holds.

| Option | Consequence | Decision |
|---|---|---|
| `scan.rs` passes the root's `client` to adapters | No hand-off point exists (V4): adapters produce their roots, `scan.rs` only concatenates. Would invert ownership and force a signature change on all seven `scan` functions | **Rejected** |
| Every internal helper takes the full `ResolvedRoot`/`&SearchRoot` | Already free in six of eight paths (V3). For the two minimal-context helpers it would expose `path`/`status`/`kind` they must ignore | **Rejected as a change** — it already holds where it costs nothing |
| **Adapters read `resolved.root.client` where they already hold the root; the two minimal-context helpers gain one `client: Option<ClientKind>` parameter** | Follows the minimal-context precedent — `walk_one` already receives exactly what `Location` needs (`&root.id`); this adds the second thing it needs, a `Copy` enum. Public adapter signatures untouched | **Chosen** |

Exact signature changes — the complete list:

```rust
// roots.rs — both constructors gain the field, after `kind`
fn resolve_single(home: &Path, id: &str, kind: SearchRootKind,
                  client: Option<ClientKind>, suffix: &[&str]) -> ResolvedRoot;
fn resolve_pair(home: &Path, id: &str, kind: SearchRootKind,
                client: Option<ClientKind>, base: &[&str], overlay: &[&str]) -> ResolvedRoot;

// skills.rs — walk_one gains one parameter
fn walk_one(scan_path: &Path, root_id: &SearchRootId,
            client: Option<ClientKind>,
            components: &mut Vec<Component>, issues: &mut Vec<ScanIssue>);

// agents.rs — emit_embedded_components gains one parameter
fn emit_embedded_components(embedded_root_id: &SearchRootId,
                            client: Option<ClientKind>,
                            components: &mut Vec<Component>);
```

**Unchanged**: all seven public `pub fn scan(home: &Path) -> …Scan` entry points, `walk_agents_root` in `agents.rs`/`codex_agents.rs`, and all four `assemble_component`s — they read `resolved.root.client` in place. `resolve_opencode` and `opencode_agent_root` hardcode `Some(ClientKind::OpenCode)` internally, exactly as they already hardcode the root id.

The mapping, hardcoded where the root ids are hardcoded (proposal decision 3):

| Root id | `client` | Root id | `client` |
|---|---|---|---|
| `claude-skills` | `Some(ClaudeCode)` | `claude-agents` | `Some(ClaudeCode)` |
| `agents-skills` | **`None`** | `claude-embedded-agents` | `Some(ClaudeCode)` |
| `opencode-skills` | `Some(OpenCode)` | `opencode-agents` | `Some(OpenCode)` |
| `codex-skills` | `Some(Codex)` | `codex-agents` | `Some(Codex)` |
| `claude-mcp` | `Some(ClaudeCode)` | `opencode-mcp` | `Some(OpenCode)` |
| `codex-mcp` | `Some(Codex)` | | |

**Accepted limitation (V7):** `claude-skills` carries `ClaudeCode` even though OpenCode also resolves skills there. The field answers "which client owns this root" — the convention the root id already encodes, now typed — not "every client that can read it". The UI copy (§6) is written for the ownership reading.

## 4. Decision — where `Location.client` is populated (open question 2)

**At the `Location` construction sites, in the adapter layer.** This is structurally forced, which is precisely why it is the clean answer: a required struct field must appear in every literal (V1's sixteen sites), so "populate later" is not even compilable as written. Beyond the compiler argument:

| Option | Consequence | Decision |
|---|---|---|
| Copy in `scan.rs` after adapters return | Requires a mutation pass over every component that exists nowhere today and contradicts the house shape "one root produces N components; the adapter owns its output". `scan.rs` has never touched adapter output (V4) | **Rejected** |
| **Adapter sets it at construction, from `resolved.root.client` (or the passed parameter)** | One expression per literal (`client: resolved.root.client` / `client`), no second pass, and the referential rule "a location's client equals its root's client" holds by construction | **Chosen** |

**Delta against the proposal, recorded:** the proposal's Affected Areas table lists `scan.rs` (15–25 lines) and `crates/vertice-app/` as "Unchanged". Verified truth is the mirror image: **`scan.rs` needs zero changes** (no wiring exists to add), while **one `vertice-app` test literal must change** — `commands.rs:586` constructs a `SearchRoot` and gains `client: Some(ClientKind::ClaudeCode)` to compile (it is a `claude-skills` root). `vertice-app`'s **production source and `capabilities/default.json` remain byte-identical**; the success criterion reads accordingly.

## 5. Decision — i18n keys (open question 3)

Two classes of string, treated differently:

- **Client display names are proper nouns and stay hardcoded**, following `ClientsPage.svelte`'s precedent (V8): `CLIENT_LABEL: Record<ClientKind, string> = { claudeCode: "Claude Code", openCode: "OpenCode", codex: "Codex" }` in the new frontend module (§6). Not i18n keys — "Claude Code" does not translate.
- **"Shared" is a common noun and gets one i18n key.** New catalog section, shared by all three pages:

```ts
aiClients: { shared: string };   // Catalog type; en and es catalogs both grow it
```

| Locale | Key | Value |
|---|---|---|
| en | `aiClients.shared` | `"Shared"` |
| es | `aiClients.shared` | `"Compartido"` |

One key, not three per-page keys: all three pages render the identical concept, and tripling the translation surface for one word is exactly the drift this catalog's typed shape exists to prevent. The section name mirrors the section heading already on the three pages (`skillDetail.aiClients` etc.). The existing `aiClientsEmpty` keys are **kept** for the zero-locations case.

## 6. Decision — grouping and display logic (open question 4)

**A deduplicated client summary in fixed order, one row per distinct client with its location count** — replacing the dashed placeholder box on all three detail pages.

| Option | Consequence | Decision |
|---|---|---|
| One section per client, listing location paths under each | Duplicates the Locations section already rendered on every detail page — same paths twice, two sources of display truth | **Rejected** |
| Flat list of one label per location | A skill in three Claude roots shows "Claude Code" three times; answers "where" (already answered) instead of "who" | **Rejected** |
| **Deduplicated groups, fixed order, with counts** | Answers the section's actual question — "which AI clients can use this component" — in ≤ 4 rows; counts add provenance without repeating paths | **Chosen** |

New pure module `frontend/src/lib/clientGroups.ts` (house pattern: logic in `lib/*.ts`, consumed by pages — same as `inventory.ts`/`isDuplicate`):

```ts
import type { ClientKind } from "../bindings/ClientKind";
import type { Location } from "../bindings/Location";

export interface ClientGroup {
  /** Owning client of the root that produced these locations; null = shared root. */
  client: ClientKind | null;
  count: number;
}

/** Deduplicate locations by `client`. Order is fixed and total:
 *  claudeCode → openCode → codex → shared(null) last, regardless of
 *  location order. Groups with count 0 are never emitted. */
export function groupLocationsByClient(locations: Location[]): ClientGroup[];

/** Hardcoded proper nouns (V8) — never i18n keys. */
export const CLIENT_LABEL: Record<ClientKind, string>;
```

Group order follows the `ClientKind` declaration order and `ClientsPage`'s `clients` array; `shared` is always last. Rendering is identical across the three pages: `{#each groups as group}` — one row per group, label from `CLIENT_LABEL` or `i18n.t("aiClients.shared")`, plus the count; rows reuse the existing location-row visual family (`rounded-control bg-canvas/35 px-3 py-2.5`). If `component.locations.length === 0`, the existing `aiClientsEmpty` placeholder renders instead.

## 7. IPC contract

**No new command, no new event, no capability change.** `crates/vertice-app/` production source and `capabilities/default.json` stay byte-identical; `scan`/`rescan` remain thin pass-throughs. The contract change is entirely inside the existing payload:

| Binding file | Action |
|---|---|
| `SearchRoot.ts` | Modified — gains `client: ClientKind \| null` |
| `Location.ts` | Modified — gains `client: ClientKind \| null` |
| `ClientKind.ts` | Unchanged (already exported, V5) |
| every other `bindings/*.ts` | Unchanged — a diff there means something leaked into `model/` |

Regenerated **only** by `cargo test -p vertice-core`, in the same commit, never hand-edited. Rollback has **no orphan binding to delete** — both files pre-exist, unlike the MCP cycle's `McpTransport.ts`.

## 8. Error paths

**None new — by construction.** `client` is a hardcoded constant assigned at root construction; there is no runtime path that can fail to produce it. No new `ScanIssue`, no new severity (`IssueSeverity` stays at two variants), `append_missing_root_issues` untouched, no new `unwrap`/`expect`/`panic` surface. The only failure mode this change introduces is a **compile error**: a future root constructor that omits `client` does not build — which is the proposal's risk-table mitigation, delivered by the type system rather than a review promise.

## 9. File changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/model/location.rs` | Modify | Two fields + doc contracts (§2); inline-test literals gain the field; stale test doc at :94-97 rewritten |
| `crates/vertice-core/src/roots.rs` | Modify | `resolve_single`/`resolve_pair` signatures; every constructor populates `client` (§3 table); new mapping-pin test |
| `crates/vertice-core/src/skills.rs` | Modify | `walk_one` gains `client`; one literal |
| `crates/vertice-core/src/agents.rs` | Modify | `emit_embedded_components` gains `client`; two literals read it |
| `crates/vertice-core/src/codex_agents.rs`, `opencode_agents.rs`, `mcp_claude.rs`, `mcp_opencode.rs`, `mcp_codex.rs` | Modify | One literal each: `client: resolved.root.client` — no signature change |
| `crates/vertice-core/src/consolidate.rs` | Modify | **Test helper only** (`location()` at :154). `ROOT_ORDER` and merge logic untouched |
| `crates/vertice-core/src/scan.rs`, `identity.rs`, `installations.rs`, all parser seams | **Unchanged** | §4 delta: `scan.rs` needs no wiring |
| `crates/vertice-core/tests/model_contract.rs` | Modify | 7 literals + `sample_search_root`; new round-trip and referential tests (§10) |
| `crates/vertice-app/src/commands.rs` | Modify | **One test literal** at :586 (V2). Production source byte-identical |
| `frontend/src/bindings/{SearchRoot,Location}.ts` | Regenerated | Never hand-edited |
| `frontend/src/lib/clientGroups.ts` + `clientGroups.test.ts` | **Create** | §6 helper + unit tests |
| `frontend/src/lib/pages/{AgentDetail,SkillDetail,McpDetail}.svelte` | Modify | Placeholder box → client-group rows (§6) |
| `frontend/src/lib/i18n/catalogs.ts` | Modify | `Catalog` type + en/es `aiClients.shared` (§5) |
| `frontend/src/lib/pages/*.test.ts` (new, 3 files) | **Create** | One wiring test per detail page, ClientsPage.test.ts pattern |
| `Cargo.toml`, `Cargo.lock`, `deny.toml`, `capabilities/default.json` | **Byte-identical** | No dependency, no capability |

**CA-16 structurally.** No disk surface added at all — not even a new read.

## 10. Testing strategy (`strict_tdd: true` — RED first)

**No new fixture directories.** Every root already exists in committed fixture trees (V10); `client` is a constant per root, so assertions layer onto existing fixtures. The load-bearing failing tests, in order:

1. `search_root_with_client_round_trips_through_json` — `Some(ClaudeCode)` survives serde round-trip.
2. `shared_search_root_serializes_client_as_json_null` — `agents-skills` shape: `"client": null` in JSON, round-trips to `None`.
3. `every_root_id_carries_its_client_mapping` — pins the §3 table for all 11 roots against a nonexistent home (the `root_order_matches_the_roots_module_in_order` pattern — no fixture, no disk).
4. `skill_location_carries_its_roots_client` — `skills::scan` over existing fixtures: a `.claude/skills` location carries `Some(ClaudeCode)` (CA-17).
5. `shared_skill_locations_carry_no_client` — the `complete` fixture's `shared` component (V10): one location `Some(ClaudeCode)`, one `None` — the load-bearing Some+None pair, already on disk.
6. `every_location_client_matches_its_root_client` — orchestrator-level referential integrity over the full `complete` report, mirroring `location_root_resolves_to_a_scanned_search_root` (`tests/model_contract.rs:180`).
7. Frontend RED: `clientGroups.test.ts` — dedupe, fixed order with shared last, counts, empty input, unknown-order input; then the three page tests asserting client rows replace the placeholder (2 clients + 1 shared fixture each).

| Layer | What stays green / what is pinned |
|---|---|
| Regression | `complete` 11/15/`issues.is_empty()` and `missing-root-client` 11 warnings (V9); reference-fixture counts and tree-snapshot (V9); all consolidation pins — `location_key` untouched |
| Contract | `ClientKind` binding unchanged; every non-`SearchRoot`/`Location` binding unchanged; every skill/agent/MCP location's `client` matches its root's |
| Frontend | `npm run lint && npm run check && npm run test && npm run build`; existing suites green against regenerated bindings |

Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, bindings-in-sync, frontend quartet — full CI matrix.

## 11. Slicing and rollback

Two slices, each independently green and revertible:

1. **Core + bindings.** Model fields, `roots.rs` mapping, adapter literals, all sixteen construction sites, tests 1–6, regenerated bindings. Frontend still compiles (new fields are additive `| null`).
2. **Frontend.** `clientGroups.ts`, three detail pages, i18n keys, page tests.

Rollback is the proposal's two-layer revert in dependency order, with nothing extra: no moved functions to un-move, no orphan bindings to delete, no dependency to remove. **Migration: none** — nothing is persisted; `ScanReport` is rebuilt on every scan.

## Open Questions

None. All four items the proposal committed to `sdd-design` are closed (§3, §4, §5, §6), and the one spec-level reconciliation (V6 — the "no client display name or label" wording) is a delta-spec concern already flagged for `sdd-spec`, not a design blocker.
