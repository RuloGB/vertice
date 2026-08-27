# Tasks: Fix Duplicate Component Label

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 120–220 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Frontend predicate + surface updates | PR 1 | Single slice; keep list/detail badge behavior aligned. |

## Phase 1: Test Red — Predicate Semantics

- [x] 1.1 Add failing cases in `frontend/src/lib/inventory.test.ts` for spec `inventory-ui: Duplicate Rows and Complete Paths` — shared + consuming client-specific skill copies, distinct client-specific non-duplicates, and shared-only/unknown-root negatives.
- [x] 1.2 Add failing rendering cases in `frontend/src/lib/ComponentRow.test.ts` and `frontend/src/lib/pages/{SkillDetail,AgentDetail,McpDetail}.test.ts` for the same spec, covering duplicate badge presence/absence.

## Phase 2: Green — Shared Duplicate Predicate

- [x] 2.1 Replace `isDuplicate` in `frontend/src/lib/inventory.ts` with the static `Location.client` + shared-root consumer matrix from `openspec/changes/fix-duplicate-component-label/design.md`, satisfying `inventory-ui`.
- [x] 2.2 Wire `ComponentRow.svelte`, `SkillDetail.svelte`, `AgentDetail.svelte`, and `McpDetail.svelte` to the shared predicate only; keep full location disclosure unchanged for `inventory-ui`.

## Phase 3: Green — Detail and List Surface Coverage

- [x] 3.1 Update or add fixtures in `frontend/src/lib/*test.ts` so duplicate-positive and duplicate-negative components reflect the design matrix without regrouping by name or comparing file contents (`inventory-ui`, `duplicate-consolidation`).
- [x] 3.2 Verify nullable `location.path` still renders safely in `frontend/src/lib/pages/McpDetail.svelte` and the corresponding test fixture (`inventory-ui` null-path scenario).

## Phase 4: Refactor — Consistency and Verification

- [x] 4.1 Normalize duplicate-badge copy and `title={components.duplicateTitle}` usage across list/detail surfaces so all badge surfaces stay in sync.
- [x] 4.2 Run the frontend test slice for `frontend/src/lib/inventory.test.ts` plus the affected component/page tests; confirm no core, IPC, or generated binding files changed.
