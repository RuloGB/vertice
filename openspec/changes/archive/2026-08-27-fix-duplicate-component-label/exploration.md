## Exploration: P16 — Duplicate label semantics

### Current State
The current implementation marks a component as duplicated when `component.locations.length > 1` via `frontend/src/lib/inventory.ts::isDuplicate`, and the badge is rendered in `frontend/src/lib/ComponentRow.svelte` and the detail pages. The core model already consolidates same-identity discoveries into one `Component` with multiple `Location` entries, so the UI is using location count as the only signal.

The living spec in `openspec/specs/inventory-ui/spec.md` already says the UI MUST mark a component as duplicated iff `locations.length > 1`, and MUST NOT regroup components by name or compare file contents. That means P16 is not a missing implementation detail so much as a semantic conflict between the observed symptom in `internal-docs/pendientes-desarrollo.md` and the current spec/model.

### Affected Areas
- `frontend/src/lib/inventory.ts` — owns the duplicate predicate used by all UI surfaces.
- `frontend/src/lib/ComponentRow.svelte` — renders the duplicate badge in the list view.
- `frontend/src/lib/pages/AgentDetail.svelte` — renders the duplicate badge in the agent detail view.
- `frontend/src/lib/pages/McpDetail.svelte` — renders the duplicate badge in the MCP detail view.
- `frontend/src/lib/pages/SkillDetail.svelte` — renders the duplicate badge in the skill detail view.
- `frontend/src/lib/clientGroups.ts` — shows that locations are already grouped by client for other UI purposes, which is relevant to the proposed semantics change.
- `openspec/specs/inventory-ui/spec.md` — current authoritative behavior for duplicate marking.
- `internal-docs/pendientes-desarrollo.md` — records the reported false-positive symptom and the desired business meaning.

### Approaches
1. **Keep current location-count semantics** — Treat any multi-location component as duplicated, because the core already consolidates identities and the spec already defines duplication that way.
   - Pros: no code change, matches current spec, preserves simple and testable rule.
   - Cons: does not address the reported false-positive symptom in P16; likely still shows "Duplicado" where users expect a different meaning.
   - Effort: Low

2. **Refine duplication to client-scoped overlap** — Redefine duplicate to mean overlap between shared and client-specific roots, or more generally a collision within the same client namespace, rather than any multi-location consolidation.
   - Pros: aligns better with the P16 note that different clients read separate folders; can eliminate the reported false positive.
   - Cons: requires a new business rule, spec update, and likely richer location classification in the predicate than a raw count.
   - Effort: Medium/High

3. **Split the concept into two badges** — Keep `Duplicado` for true namespace collision and add a separate informational marker for "found in multiple locations / clients".
   - Pros: preserves existing technical truth while making user intent explicit; lowest risk of confusing data with business meaning.
   - Cons: increases UI complexity and requires copy/spec work; may be overkill if only one label is needed.
   - Effort: Medium

### Recommendation
Move to Approach 2 only if product agrees that "duplicado" must mean namespace collision rather than consolidation multiplicity. Right now the code and spec are internally consistent, so the real work is deciding the business definition first; otherwise implementation changes will just fight the current model.

### Risks
- The reported symptom may be a terminology problem, not a code bug, so changing logic without redefining the label could create a worse mismatch.
- Any semantic shift from `locations.length > 1` will ripple through list badges, detail pages, and tests.
- If the intended rule depends on client-specific roots, the current UI predicate may not have enough information and the spec will need to be updated first.

### Ready for Proposal
Yes — but only after confirming the intended business meaning of "Duplicado" for shared vs. client-specific locations.
