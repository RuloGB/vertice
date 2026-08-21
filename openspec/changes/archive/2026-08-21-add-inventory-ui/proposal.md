# Proposal: Add Inventory UI

## Intent

Deliver T11 (`internal-docs/plan-desarrollo-poc.md:250–266`): replace the smoke shell with the first inventory screen over existing `scan`/`rescan`. Closes **CA-1** (list on launch, zero configuration) and the **visual half of CA-3** (duplicate mark + all paths). Data half of CA-3 is already closed by T8.

## Scope

### In Scope
- Unified list of consolidated `ScanReport.components`: name, kind, description, origin path(s).
- Duplicate affordance when `locations.length > 1`; reveal **all** paths (tolerate `null`).
- View-only filter by kind + search by name over in-memory components.
- Startup `scan()`; Reload via `rescan()`; no watchers / auto-refresh.
- Minimal loading, hard-failure, and empty-list states (structural only).
- Pure helpers in `frontend/src/lib/` unit-tested with existing Node Vitest; manual `tauri dev` for visual CA-1/CA-3.

### Out of Scope
- DOM component harness (jsdom/happy-dom / Testing Library) — **explicitly out of T11**.
- i18n catalogs / language selector (T12).
- Rich empty/error/client-absent/unparseable/installations/roots UX (T13).
- Core, IPC commands, bindings, capabilities, CSP, watchers, writes, MCPs, project scope.

## Capabilities

### New Capabilities
- `inventory-ui`: Frontend inventory presentation — list, kind filter, name search, duplicate paths disclosure, load/reload lifecycle, minimal loading/error/empty chrome over `scan`/`rescan`.

### Modified Capabilities
None. Consumes `desktop-shell` and consolidated report shape as-is; no requirement changes to core specs.

## Approach

**Thin App + pure lib + presentational Svelte 5 (exploration Approach 1).**  
`App.svelte` owns `idle | loading | ready | failed`, current report, kind filter, and search query. Pure modules: `filterComponents`, `isDuplicate` (`locations.length > 1` only — never re-group by name). Small Svelte pieces for toolbar/list/row/path disclosure; Tailwind utilities. Reuse `frontend/src/lib/scan.ts` unchanged. Strict TDD on pure helpers with fixture `Component[]`; visual CA-3 via manual `tauri dev` checklist (known limitation — unit tests alone do not close visual CA-3).

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `frontend/src/App.svelte` | Modified | Inventory screen + lifecycle |
| `frontend/src/lib/*` | New | Filter/search/duplicate helpers + tests |
| `frontend/src/**/*.svelte` | New | List, row, paths, toolbar, shells |
| `openspec/specs/inventory-ui/` | New (post-archive) | Living UI capability |
| `crates/**`, `bindings/**`, `scan.ts`, capabilities | Untouched | By design |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Scope creep into T12/T13 | Med | Minimal English placeholders; no panels |
| Wrong duplicate rule | Low | Spec + tests: `locations.length > 1` only |
| CA-3 visual not in CI | Med | Manual `tauri dev` checklist on reference install |
| Frozen-looking UI | Low | Explicit loading state for full invoke |
| `ScanError` vs empty list | Low | Distinct `failed` vs empty `ready` |

## Rollback Plan

Revert frontend-only: restore smoke `App.svelte`, delete new lib modules/components/tests. No core, IPC, bindings, or capability changes; no persisted state or migration. Three-layer impact: core and app unchanged; frontend returns to T10 smoke shell.

## Dependencies

- T10 archived (`2026-08-20-add-tauri-scan-commands`): `scan`/`rescan` IPC + `frontend/src/lib/scan.ts`.
- T8 consolidated `Component.locations` for CA-3 mark semantics.

## Success Criteria

- [x] Launch invokes `scan` with no settings UI and renders the consolidated list (CA-1).
- [x] Multi-location rows show a clear duplicate mark and list every path (visual CA-3).
- [x] Kind filter and name search are view-only over `report.components`.
- [x] Reload uses `rescan`; startup uses `scan`; no watchers.
- [x] Loading, hard `ScanError`, and empty successful report are distinct non-crash states.
- [x] Pure helpers covered by Node Vitest; no DOM harness added; no core/IPC/bindings/capability edits.
