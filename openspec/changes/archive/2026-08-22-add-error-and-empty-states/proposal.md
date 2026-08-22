# Proposal: T13 Error, Empty, and Non-Actionable States

## Intent

Complete T13 (CA-11--CA-13): expose successful-scan diagnostics and embedded-component status without hiding inventory. Missing clients appear as a discreet inventory notice; T11 failure and empty states remain distinct.

## Scope

### In Scope
- Render missing-client, unavailable-root, and recoverable-issue diagnostics after a successful scan.
- Mark components with an embedded location as embedded/non-actionable, based on `Location.origin === "embedded"`.
- Add bilingual chrome and strict-TDD jsdom coverage.
- Isolate the existing untyped missing-client reason classifier behind a documented presentation helper.

### Out of Scope
- Core, IPC, bindings, scan taxonomy, filesystem access, persistence, or writes.
- Update/uninstall controls, watchers, timers, retries, or new scan commands.
- Localizing diagnostic payloads (paths, reasons, versions, names) or replacing the inventory with diagnostics.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `inventory-ui`: successful reports render non-blocking diagnostics and embedded-component state while retaining existing lifecycle behavior.
- `frontend-i18n`: catalogs include the new diagnostic and embedded-state chrome while preserving raw payload passthrough.

## Approach

Use a bounded diagnostic panel fed solely by `ScanReport`. Prevent duplicate root warnings: roots come from `rootsScanned`, recoverable issues from `issues`, and missing clients through the isolated classifier. Inventory rows own the embedded badge; `path === null` is insufficient.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `frontend/src/App.svelte` | Modified | Compose non-blocking successful-report diagnostics. |
| `frontend/src/lib/InventoryRow.svelte` | Modified | Render embedded/non-actionable status. |
| `frontend/src/lib/i18n/catalogs.ts` | Modified | Add paired diagnostic chrome. |
| `frontend/src/App.test.ts` | Modified | Add RED->GREEN UI coverage for CA-11--CA-13. |
| `frontend/src/lib/` | New/Modified | Diagnostics and missing-client classifier. |
| `openspec/specs/inventory-ui/spec.md` | Modified | Define T13 UI behavior. |
| `openspec/specs/frontend-i18n/spec.md` | Modified | Define added catalog boundary. |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Untyped client-missing reason drifts | Medium | One tested classifier; do not spread string matching. |
| Duplicate root diagnostics | Medium | Use `rootsScanned` as the sole root-status source. |
| Diagnostic UI suppresses inventory | Low | Test mixed report with components and diagnostics. |

## Rollback Plan

Revert frontend diagnostic, badge, catalog, and test changes. Core, IPC, bindings, scan behavior, and lifecycle states remain untouched.

## Dependencies

- T11 inventory UI and T12 frontend i18n.
- Existing `ScanReport` diagnostic fields from T9.

## Success Criteria

- [ ] Mixed reports retain inventory and show discreet missing-client, unavailable-root, and recoverable-issue diagnostics (CA-11, CA-12).
- [ ] Embedded components are visibly marked without invented actions (CA-13).
- [ ] New chrome switches between English and Spanish; payload values remain verbatim.
- [ ] Strict-TDD jsdom coverage proves each T13 surface; no core, IPC, binding, or filesystem behavior changes.
