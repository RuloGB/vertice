# Apply Progress: Fix Duplicate Component Label

## Status

All implementation tasks are complete in strict TDD mode.

## Completed Tasks

- [x] 1.1 Add failing cases in `frontend/src/lib/inventory.test.ts` for spec `inventory-ui: Duplicate Rows and Complete Paths` — shared + consuming client-specific skill copies, distinct client-specific non-duplicates, and shared-only/unknown-root negatives.
- [x] 1.2 Add failing rendering cases in `frontend/src/lib/ComponentRow.test.ts` and `frontend/src/lib/pages/{SkillDetail,AgentDetail,McpDetail}.test.ts` for the same spec, covering duplicate badge presence/absence.
- [x] 2.1 Replace `isDuplicate` in `frontend/src/lib/inventory.ts` with the static `Location.client` + shared-root consumer matrix from `openspec/changes/fix-duplicate-component-label/design.md`, satisfying `inventory-ui`.
- [x] 2.2 Wire `ComponentRow.svelte`, `SkillDetail.svelte`, `AgentDetail.svelte`, and `McpDetail.svelte` to the shared predicate only; keep full location disclosure unchanged for `inventory-ui`.
- [x] 3.1 Update or add fixtures in `frontend/src/lib/*test.ts` so duplicate-positive and duplicate-negative components reflect the design matrix without regrouping by name or comparing file contents (`inventory-ui`, `duplicate-consolidation`).
- [x] 3.2 Verify nullable `location.path` still renders safely in `frontend/src/lib/pages/McpDetail.svelte` and the corresponding test fixture (`inventory-ui` null-path scenario).
- [x] 4.1 Normalize duplicate-badge copy and `title={components.duplicateTitle}` usage across list/detail surfaces so all badge surfaces stay in sync.
- [x] 4.2 Run the frontend test slice for `frontend/src/lib/inventory.test.ts` plus the affected component/page tests; confirm no core, IPC, or generated binding files changed.

## TDD Cycle Evidence

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| 1.1 / 2.1 / 3.1 | `frontend/src/lib/inventory.test.ts` | Unit | ✅ 15/15 existing tests passed in affected slice before edits | ✅ New predicate tests failed under `locations.length > 1` | ✅ 12/12 inventory tests passed after matrix implementation | ✅ OpenCode positive, Codex positive, client-specific negative, shared-only negative, unknown-root negative, Claude negative, Agent/MCP disabled, nullable-path positive | ✅ Extracted typed static matrix and kept predicate pure |
| 1.2 / 2.2 / 3.1 / 4.1 | `frontend/src/lib/ComponentRow.test.ts`, `frontend/src/lib/ComponentRowHarness.svelte` | Component | N/A (new test/harness) | ✅ Expanded/compact row tests failed for distinct client-specific copies under old predicate | ✅ 3/3 ComponentRow tests passed through shared predicate | ✅ Expanded positive, compact positive, expanded negative with full paths | ✅ Test harness isolates i18n context without changing production component |
| 1.2 / 2.2 / 3.1 / 4.1 | `frontend/src/lib/pages/SkillDetail.test.ts` | Component | ✅ 15/15 existing tests passed in affected slice before edits | ✅ Distinct client-specific detail test failed under old predicate | ✅ 5/5 SkillDetail tests passed through shared predicate | ✅ Shared+Codex positive and OpenCode+Codex negative while preserving locations | ✅ Shared render helper reduced repeated mount setup |
| 1.2 / 2.2 / 3.1 / 4.1 | `frontend/src/lib/pages/AgentDetail.test.ts` | Component | ✅ 15/15 existing tests passed in affected slice before edits | ✅ Shared+client Agent detail test failed under old predicate | ✅ 4/4 AgentDetail tests passed with disabled Agent matrix | ✅ Existing grouping plus shared+client negative with full paths | ✅ Shared render helper reduced repeated mount setup |
| 1.2 / 2.2 / 3.1 / 3.2 / 4.1 | `frontend/src/lib/pages/McpDetail.test.ts` | Component | ✅ 15/15 existing tests passed in affected slice before edits | ✅ Shared+client MCP and nullable-path tests failed under old predicate | ✅ 5/5 McpDetail tests passed with disabled MCP matrix | ✅ Existing grouping, shared+client negative, nullable-path negative and safe placeholder | ✅ Shared render helper reduced repeated mount setup |
| 4.2 | affected frontend slice | Verification | ✅ Baseline captured before edits | ✅ RED run: 5 files failed, 11 assertions failed as expected | ✅ Final slice: 5 files / 29 tests passed | ✅ Conservative matrix boundaries covered across unit and rendered surfaces | ✅ `npm run check` and `npm run lint` pass |

## Test Summary

- **Total tests written/updated**: 14 behavior cases added or changed for duplicate semantics.
- **Total tests passing**: 29/29 in affected frontend slice.
- **Layers used**: Unit (12), Component/jsdom (17), E2E (0).
- **Approval tests**: None — behavior was intentionally changed, not refactored-only.
- **Pure functions created**: 0 new exported functions; `isDuplicate` remains a pure function.

## Commands Run

- `npm run test -- src/lib/inventory.test.ts src/lib/pages/SkillDetail.test.ts src/lib/pages/AgentDetail.test.ts src/lib/pages/McpDetail.test.ts` → 4 files / 15 tests passed (safety net).
- `npm run test -- src/lib/inventory.test.ts src/lib/ComponentRow.test.ts src/lib/pages/SkillDetail.test.ts src/lib/pages/AgentDetail.test.ts src/lib/pages/McpDetail.test.ts` → RED failed as expected: 5 files failed, 11 assertions failed.
- `npm run test -- src/lib/inventory.test.ts src/lib/ComponentRow.test.ts src/lib/pages/SkillDetail.test.ts src/lib/pages/AgentDetail.test.ts src/lib/pages/McpDetail.test.ts` → GREEN passed: 5 files / 29 tests.
- `npx @sveltejs/mcp svelte-autofixer ./src/lib/ComponentRowHarness.svelte --svelte-version 5` → reported no Svelte issues/suggestions, then exited non-zero due certificate verification while fetching remote sections (`UNABLE_TO_VERIFY_LEAF_SIGNATURE`).
- `npm run check` → passed, 0 errors / 0 warnings.
- `npm run lint` → passed.

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `frontend/src/lib/inventory.ts` | Modified | Replaced raw location-count logic with typed static shared-root consumer matrix. |
| `frontend/src/lib/inventory.test.ts` | Modified | Added matrix-backed positive and conservative negative predicate coverage. |
| `frontend/src/lib/ComponentRowHarness.svelte` | Created | Added test-only i18n harness for `ComponentRow`. |
| `frontend/src/lib/ComponentRow.test.ts` | Created | Covered duplicate badge presence/absence in compact and expanded rows. |
| `frontend/src/lib/pages/SkillDetail.test.ts` | Modified | Added duplicate-positive and client-specific-negative detail coverage. |
| `frontend/src/lib/pages/AgentDetail.test.ts` | Modified | Added disabled Agent matrix badge-negative detail coverage. |
| `frontend/src/lib/pages/McpDetail.test.ts` | Modified | Added disabled MCP matrix and nullable-path safe rendering coverage. |
| `openspec/changes/fix-duplicate-component-label/tasks.md` | Modified | Marked all implementation tasks complete. |
| `openspec/changes/fix-duplicate-component-label/apply-progress.md` | Created | Persisted strict TDD evidence and command results. |

## Deviations from Design

None — implementation matches the design. `ComponentRow.svelte` and detail components did not require production changes because they already consume `isDuplicate`.

## Issues Found

- Svelte MCP autofixer reported no issues for the new harness, but the CLI exited non-zero after that due local certificate verification while fetching remote documentation sections.
- `openspec/changes/fix-duplicate-component-label/` is untracked in Git because this OpenSpec change folder was not previously tracked in this worktree.

## Workload / PR Boundary

- Mode: single PR.
- Current work unit: Frontend predicate + surface tests.
- Boundary: frontend-only duplicate badge semantics; no Rust core, IPC, generated bindings, living specs, or `state.yaml` changes.
- Estimated review budget impact: low, within the 120–220 line forecast band.
## Review Finding Correction

- Added the missing deterministic compact-mode negative assertion for `ComponentRow`: distinct OpenCode/Codex client-specific skill copies MUST NOT render the duplicate badge when the row is compact.
- No production code changed for this correction.

### Correction TDD Evidence

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| Review correction | `frontend/src/lib/ComponentRow.test.ts` | Component | ✅ 1 file / 3 tests passed before edit | ⚠️ Added missing assertion after production predicate was already green from prior cycle; no failing production change was expected | ✅ `ComponentRow.test.ts`: 1 file / 4 tests passed | ✅ Negative coverage now exercises both expanded and compact modes | ✅ Reused existing parameterized row fixture without production changes |

### Correction Commands

- `npm run test -- src/lib/ComponentRow.test.ts` → safety net passed: 1 file / 3 tests.
- `npm run test -- src/lib/ComponentRow.test.ts` → correction passed: 1 file / 4 tests.
- `npm run check` → passed, 0 errors / 0 warnings.
- `npm run lint` → passed.
- `npm run test -- src/lib/inventory.test.ts src/lib/ComponentRow.test.ts src/lib/pages/SkillDetail.test.ts src/lib/pages/AgentDetail.test.ts src/lib/pages/McpDetail.test.ts` → cumulative affected slice passed: 5 files / 30 tests.
## Verification Failure Correction: App Legacy Formatter Fixture

- Updated `frontend/src/App.test.ts` so the legacy `Formatter` fixture with `claude-skills` plus `embedded-skills` no longer expects `Duplicate`/`Duplicado` under P16 semantics.
- No production code changed for this correction.

### App Fixture Correction TDD Evidence

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| Verification correction | `frontend/src/App.test.ts` | Component/application | N/A — known verification failure supplied by verify phase | ✅ `App.test.ts` failed first: stale `Duplicate` expectation for legacy Formatter fixture | ✅ `App.test.ts`: 1 file / 38 tests passed after expectation update | ✅ English visible text, Spanish visible text, and duplicate-title absence are covered | ✅ Test-only expectation update; no production code changed |

### Verification Correction Commands

- `npm run test -- src/App.test.ts` → RED reproduced: 1 failed / 38 tests.
- `npm run test -- src/App.test.ts` → GREEN passed: 1 file / 38 tests.
- `npm run test` → full frontend suite passed: 22 files / 175 tests.
- `npm run check` → passed, 0 errors / 0 warnings.
- `npm run lint` → passed.
