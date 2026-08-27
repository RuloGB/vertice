# Design: Fix Duplicate Component Label

## Technical Approach

Keep consolidation, Rust model, generated bindings, IPC, and scan output unchanged. Redefine the frontend duplicate badge as a derived UI predicate: a component is duplicate only when the static product matrix says the same AI client can consume both a shared-root copy and that client's client-specific-root copy. The user explicitly accepts using Windows-verified `agents-skills` consumers OpenCode and Codex as the initial static matrix without runtime platform gating. macOS/Linux verification is out of scope for this SDD change and must happen in a later cycle.

## Shared-Root Consumer Matrix

| Kind | Shared root currently represented | Enabled consumers | Disabled / out of scope |
|---|---|---|---|
| Skill | `agents-skills` (`~/.agents/skills`, `Location.client: null`) | `openCode`, `codex`. Evidence: Windows OpenCode 1.18.22 loads `C:\Users\Raul\.agents\skills` and `C:\Users\Raul\.config\opencode\skills`/`skill`; Codex CLI 0.149.0 loads `C:\Users\Raul\.agents\skills` and `C:\Users\Raul\.codex\skills`. | `claudeCode` remains disabled: Claude Code 2.1.140 cannot run on this Windows host, so shared-skill consumption is unverified/unsupported here. macOS/Linux behavior is not claimed. |
| Agent | none evidenced | none | All shared-plus-client Agent pairs remain non-duplicate. Scanner roots are client-specific: Claude on-disk/embedded, OpenCode config, Codex directory. |
| MCP | none evidenced | none | All shared-plus-client MCP pairs remain non-duplicate. Scanner roots are client-specific: Claude, OpenCode, Codex MCP config roots. |

## Architecture Decisions

| Decision | Rejected | Rationale |
|---|---|---|
| Centralize the rule in `frontend/src/lib/inventory.ts`. | Per-surface badge logic. | `ComponentRow`, `AgentDetail`, `SkillDetail`, and `McpDetail` already depend on `isDuplicate`; one predicate prevents list/detail drift. |
| Use `Location.client` plus `Location.root` and an explicit static consumer matrix. | `locations.length > 1`, path prefixes, display labels, file hashes, or `provenanceHint`. | The approved rule is client consumability, not technical aggregation. `client:null` says shared ownership; `root` identifies which shared root the matrix covers. |
| Apply the Windows-verified OpenCode/Codex skill evidence as static product scope, with no runtime platform gate. | Platform-gated UI behavior; universal assumption for every client/platform. | The user accepted this tradeoff. It fixes the known false-positive class now while documenting that cross-platform confirmation is a later cycle, not a hidden claim. |

## Data Flow

```
Component.locations
  -> isDuplicate(component)
     -> shared locations: client === null and root is in the static matrix
     -> client locations: client is listed for that shared root and kind
     -> rendered badge in row/detail surfaces
```

## File Changes

| File | Action | Description |
|---|---|---|
| `frontend/src/lib/inventory.ts` | Modify | Replace `locations.length > 1` with matrix-backed shared/client overlap. Initial matrix: `skill -> agents-skills -> [openCode, codex]`. |
| `frontend/src/lib/inventory.test.ts` | Modify | Unit-test positives for `agents-skills` + OpenCode and `agents-skills` + Codex; negatives for distinct client-specific copies, Claude pair, unknown shared roots, shared-only components, nullable paths, and all Agent/MCP shared-plus-client pairs. |
| `frontend/src/lib/ComponentRow.svelte` | No logic change expected | Continues rendering through `isDuplicate`; deterministic tests must cover both compact and expanded branches for badge presence and absence. |
| `frontend/src/lib/pages/{AgentDetail,SkillDetail,McpDetail}.svelte` | No logic change expected | Existing `$derived(isDuplicate(component))` updates through the shared predicate; each detail surface must gain deterministic badge presence/absence tests. |
| `crates/vertice-core/**`, `frontend/src/bindings/**` | Unchanged | No model, scanner, consolidation, binding, or IPC changes. |

## Interfaces / Contracts

No public contract changes. Internal frontend-only data MAY be:

```ts
type SharedRootConsumers = Partial<Record<Component["kind"], Partial<Record<string, ClientKind[]>>>>;
```

Initial enabled entry: `skill -> "agents-skills" -> ["openCode", "codex"]`. It is static product scope, not cross-platform proof.

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Predicate | Badge semantics | `inventory.test.ts` covers skill positives and conservative negatives. |
| Rendered surfaces | Badge presence/absence in all current surfaces | Deterministic jsdom tests for compact `ComponentRow`, expanded `ComponentRow`, `AgentDetail`, `SkillDetail`, and `McpDetail`. |
| Boundaries | Missing evidence never enables badge | Tests for unknown shared `client:null` roots, shared-only components, Claude shared-skill pair, and every Agent/MCP shared-plus-client pair while their matrices are empty. |

## Migration / Rollout

No migration required. This is a frontend-only interpretation change over current scan reports.

## Open Questions

- [ ] Which, if any, supported Claude Code versions consume `~/.agents/skills` on a platform where Claude Code runs?
- [ ] Are OpenCode/Codex shared-skill roots consumed equivalently on macOS/Linux? Out of scope for this change.
- [ ] Do shared Agent or MCP roots exist in supported clients, or are those matrices intentionally empty?
