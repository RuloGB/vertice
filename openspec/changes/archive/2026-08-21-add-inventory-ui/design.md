# Design: Add Inventory UI

## Technical Approach

Implement T11 as a thin Svelte 5 composition over the existing typed `scan`/`rescan` IPC wrappers. `App.svelte` owns lifecycle state, the current `ScanReport`, and view controls; presentational components receive data and callbacks through props. Pure filtering and duplicate rules live in `frontend/src/lib/` and are tested with Node Vitest. Existing Tailwind utilities provide all styling; no DOM harness or new runtime dependency is introduced.

## Architecture Decisions

| Decision | Choice | Alternatives considered | Rationale |
|---|---|---|---|
| State ownership | Local `$state`/derived state in `App.svelte` | External store; state in rows | One screen has one consumer. Local state keeps IPC lifecycle explicit and avoids premature abstraction. |
| Component boundary | Small presentational toolbar, list, row, and locations components | Monolithic `App.svelte`; container store | Keeps markup reviewable and gives T12 a mechanical string-extraction seam without adding business logic to children. |
| Duplicate semantics | `component.locations.length > 1` via `isDuplicate` | Re-group by name/id; compare paths/content | T8 already consolidates identity; the generated model is authoritative and preserves CA-3 semantics. |
| Failure handling | `failed` state for rejected commands, distinct from successful empty report | Treat failure as empty; rich diagnostics | Prevents a blank/misleading UI while deferring T13 diagnostics and T12 copy. |

## Data Flow

```text
onMount ──scan()──────┐
reload ──rescan()─────┼─→ App state (report/status/error)
                      └─→ presentational controls/list
report.components + {kind, query} ─→ filterComponents() ─→ rows
row.locations ─→ isDuplicate() + all nullable paths
```

Startup calls `scan()` once. Reload sets `loading`, calls `rescan()`, and replaces the report only on success; settlement exits loading. Filtering never invokes IPC. A successful empty `components` array renders an empty region; a rejection renders a minimal failure region. `ScanReport.issues`, installations, roots, and duration remain available in the report but are not promoted to T11 UI.

## File Changes

| File | Action | Description |
|---|---|---|
| `frontend/src/App.svelte` | Modify | Own `idle/loading/ready/failed`, report, kind filter, search, startup scan, and reload; compose the screen. |
| `frontend/src/lib/filterComponents.ts` | Create | Pure kind/name filtering over `Component[]`; case-insensitive name matching and an explicit “all” kind. |
| `frontend/src/lib/inventory.ts` | Create | Pure `isDuplicate(component)` helper using only location count. |
| `frontend/src/lib/filterComponents.test.ts` | Create | Node Vitest cases for mixed kinds, query matching, empty input, and non-mutating results. |
| `frontend/src/lib/inventory.test.ts` | Create | Vitest cases for zero/one/multiple locations, including nullable paths. |
| `frontend/src/lib/InventoryToolbar.svelte` | Create | Search, kind filter, and reload controls; emits intent only. |
| `frontend/src/lib/InventoryList.svelte` | Create | Empty/list shell and row composition. |
| `frontend/src/lib/InventoryRow.svelte` | Create | Name, kind, optional description, duplicate affordance, and location disclosure. |
| `frontend/src/lib/LocationList.svelte` | Create | Renders every location safely, displaying a neutral placeholder for `path: null`; no filesystem action. |
| `frontend/src/app.css` | No change | Continue using the existing Tailwind import. |
| `frontend/src/lib/scan.ts`, `frontend/src/bindings/**`, `frontend/vitest.config.ts`, `frontend/package.json` | No change | Reuse typed IPC, generated contracts, Node environment, and current dependencies. |

## Interfaces / Contracts

Use generated `Component`, `ScanReport`, and `ScanError` types; never edit bindings. Presentational props should be data/callback-only, for example `components: Component[]`, `onReload: () => void`, and `onFilterChange` callbacks. The helper contract is:

```ts
filterComponents(components, { kind: "all" | ComponentKind, query: string }): Component[]
isDuplicate(component: Component): boolean // locations.length > 1
```

Helpers must not mutate the report or access Tauri, filesystem APIs, timers, or watchers.

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Unit | Filter/search, duplicate rule, null-path safety inputs | Strict RED/GREEN Node Vitest with typed fixtures; assert original arrays remain unchanged. |
| Integration | Existing `scan.ts` wrapper contract | Keep current mocked invoke tests unchanged; App wiring is checked by type-check/build. |
| Manual UI | CA-1 startup list and CA-3 duplicate/all-path disclosure | `tauri dev` on the reference installation; verify reload uses `rescan`, loading, hard failure, and empty chrome. No DOM harness. |

## Migration / Rollout

No migration, persisted state, feature flag, IPC, core, binding, capability, or permission change is required. Rollback is frontend-only: restore the smoke `App.svelte` and remove the new components/helpers/tests; core and Tauri remain unchanged.

## Open Questions

- [x] Confirm the cosmetic kind-control shape (select versus segmented control) during implementation.
- [x] Confirm minimal English placeholder wording until T12 introduces i18n.
