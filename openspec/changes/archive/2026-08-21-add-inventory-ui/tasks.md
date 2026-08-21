# Tasks: Add Inventory UI

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 430-560 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1: pure helpers and tests -> PR 2: Svelte components and App wiring -> PR 3: verification/polish |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Implement tested pure inventory rules | PR 1 | Frontend-only; RED/GREEN/REFACTOR for filtering and duplicate semantics |
| 2 | Deliver the inventory screen and lifecycle wiring | PR 2 | Depends on PR 1; includes presentational Svelte components and `App.svelte` |
| 3 | Verify acceptance behavior and polish | PR 3 | Depends on PR 2; manual `tauri dev`, no DOM harness |

## Phase 1: T11 Foundation / Pure Rules (T11, CA-3)

- [x] 1.1 RED: add typed fixture cases in `frontend/src/lib/filterComponents.test.ts` for all-kind, `skill`/`agent`, case-insensitive name queries, empty input, and unchanged source arrays.
- [x] 1.2 RED: add typed fixture cases in `frontend/src/lib/inventory.test.ts` proving `isDuplicate` is false for zero/one location and true only for multiple locations, including `path: null`.

## Phase 2: T11 Pure Implementation (T11, CA-3)

- [x] 2.1 GREEN: create `frontend/src/lib/filterComponents.ts` with non-mutating kind/name filtering and the explicit `"all"` kind contract.
- [x] 2.2 GREEN: create `frontend/src/lib/inventory.ts` with `isDuplicate(component)` based only on `locations.length > 1`.
- [x] 2.3 REFACTOR: keep helper APIs typed with generated bindings and confirm `npm run test` passes without Tauri, filesystem, timers, or watcher access.

## Phase 3: T11 Svelte UI and App Wiring (T11, CA-1, CA-3)

- [x] 3.1 Create `frontend/src/lib/InventoryToolbar.svelte` with name search, `skill`/`agent`/all control, and reload intent callbacks only.
- [x] 3.2 Create `frontend/src/lib/LocationList.svelte` to render every location and a neutral safe placeholder for nullable paths without filesystem actions.
- [x] 3.3 Create `frontend/src/lib/InventoryRow.svelte` and `InventoryList.svelte` for names, kinds, optional descriptions, duplicate marking, all paths, and distinct empty/list regions.
- [x] 3.4 Wire `frontend/src/App.svelte` with local lifecycle state, startup `scan()`, reload `rescan()`, loading/failed/empty states, and derived in-memory filtering; preserve `frontend/src/lib/scan.ts` and generated bindings.

## Phase 4: T11 Verification (T11, CA-1, CA-3)

- [x] 4.1 Run `npm run lint`, `npm run check`, `npm run test`, and `npm run build`; verify existing scan-wrapper tests remain unchanged and no DOM harness, T12, T13, core, or IPC files were added.
- [x] 4.2 Manually run `npx --prefix frontend tauri dev` against the reference installation: verify startup list, duplicate/all-path disclosure, nullable path safety, filters without rescans, reload via `rescan`, loading, hard failure, and empty success.
