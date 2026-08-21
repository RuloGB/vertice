# Proposal: Frontend Internationalization

## Intent

Implement T12 frontend i18n so Vertice supports English and Spanish UI chrome. The selector must switch inventory UI text, initial locale follows supported system locale, core diagnostics stay untouched.

## Scope

### In Scope
- Frontend-only catalogs for `en` and `es`.
- Language selector with session override and system detection.
- Externalize UI chrome: labels, placeholders, states, title/aria text, and null-path copy.
- Update `inventory-ui` to include i18n.

### Out of Scope
- Heavy i18n framework/codegen unless needed.
- Core, IPC, bindings, Tauri capabilities, filesystem, MCP, or write changes.
- Localizing payload data, paths, names/descriptions, `ScanIssue.reason`, or `ScanError.detail.reason`.
- Rich T13 diagnostics; persistent locale preference.

## Capabilities

### New Capabilities
- `frontend-i18n`: locale resolution, catalogs, lookup/interpolation, and switching.

### Modified Capabilities
- `inventory-ui`: UI chrome MUST render through i18n messages; remove the old i18n exclusion.

## Approach

Use typed catalogs plus a small locale module/store. Resolve from `navigator.languages`/`navigator.language`: `es*` -> `es`, `en*` -> `en`, otherwise `en`. Manual changes update one reactive locale source. Keep node Vitest tests for resolution, fallback, parity, and interpolation.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `frontend/src/lib/locale*`, catalogs | New | Locale state and messages. |
| `frontend/src/App.svelte` | Modified | Locale owner, selector, lifecycle copy. |
| `frontend/src/lib/Inventory*.svelte`, `LocationList.svelte` | Modified | Replace visible literals. |
| `frontend/index.html` | Modified | Align document language/title. |
| `frontend/src/lib/*.test.ts` | Modified/New | Helper and catalog tests. |
| `openspec/specs/inventory-ui/spec.md` | Modified | Remove i18n exclusion. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| English fragments remain | Medium | String inventory plus parity tests/checklist. |
| Wrong-layer localization | Low | Spec chrome-vs-payload boundary. |
| Decorative selector | Medium | Single reactive locale source and smoke check. |
| T13 copy race | Medium | Establish message-key convention first. |

## Rollback Plan

Revert i18n modules, catalog wiring, selector UI, tests, and OpenSpec deltas. No core/app rollback expected because persistence, IPC, bindings, and capabilities stay unchanged.

## Dependencies

- T11 inventory UI is archived as `inventory-ui`.
- T12 acceptance: no UI literals remain; language changes update the whole interface.
- CA-1 through CA-17: no dedicated CA-N is closed.

## Success Criteria

- [ ] `en` and `es` cover every UI chrome message.
- [ ] Manual switching updates all inventory UI text.
- [ ] System locale selects `es`/`en` when supported and falls back safely.
- [ ] `ScanIssue.reason` and payload fields remain unlocalized passthrough data.


