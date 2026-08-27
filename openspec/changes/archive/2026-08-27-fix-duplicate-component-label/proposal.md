# Proposal: Fix Duplicate Component Label

## Intent

Correct the `Duplicado`/duplicate badge so it reflects a real user-facing conflict, not merely that Vertice consolidated the same component from multiple roots. A component is duplicate only when the same AI client can consume both a shared-root copy and that client’s specific-root copy. Copies that exist only in distinct client-specific folders are not duplicates.

## Scope

### In Scope
- Redefine inventory duplicate-badge semantics for Agents, Skills, and MCPs using `Location.client` ownership.
- Preserve full location disclosure and existing consolidation behavior.
- Add/adjust frontend behavior tests for shared + client-specific overlap and distinct-client non-duplicates.

### Out of Scope
- Comparing file contents, hashes, or selecting a winning copy.
- Changing component identity, consolidation grouping, or generated model shape unless design proves current ownership data is insufficient.
- Writing to component folders or modifying installed AI-client components.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `inventory-ui`: duplicate badge changes from raw `locations.length > 1` to client-consumable shared/client-specific overlap.
- `duplicate-consolidation`: clarify that multi-location consolidation remains technical aggregation, not the UI duplicate-label contract.

## Approach

Use existing `Location.client` (`null` means shared, `Some(ClientKind)` means client-specific) as the primary signal. Derive duplicate status when a component has at least one shared location and at least one client-specific location for a client that consumes shared roots. Do not branch on `provenance_hint`, paths, display labels, or file contents. Keep all list/detail badge surfaces using one shared predicate.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `frontend/src/lib/inventory.ts` | Modified | Replace location-count duplicate predicate. |
| `frontend/src/lib/ComponentRow.svelte` | Modified | Continues rendering badge from shared predicate. |
| `frontend/src/lib/pages/*Detail.svelte` | Modified | Detail duplicate badges follow new semantics. |
| `openspec/specs/inventory-ui/spec.md` | Modified | Delta will redefine duplicate badge requirement. |
| `openspec/specs/duplicate-consolidation/spec.md` | Modified | Delta will preserve consolidation while narrowing UI meaning. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Shared-root consumption differs by client/kind | Medium | Specify the initial known client/root matrix before implementation. |
| Terminology conflict with existing specs | High | Delta specs must separate aggregation from UI duplicate meaning. |

## Rollback Plan

Revert the proposal/spec/design/tasks and frontend predicate/tests. No persisted data migration is expected; the model remains read-only and Tauri/core boundaries stay unchanged.

## Dependencies

- Confirm the client/root consumption matrix for shared skills, agents, and MCP roots.

## Success Criteria

- [ ] Distinct client-specific copies do not show `Duplicado`.
- [ ] Shared + consuming client-specific copy shows `Duplicado`.
- [ ] All locations remain visible.
- [ ] Core stays Tauri-free, `model/` stays I/O-free, and no write permissions are introduced.