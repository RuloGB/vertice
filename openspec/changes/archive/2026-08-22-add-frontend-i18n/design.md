# Design: Frontend Internationalization

## Technical Approach

Add a zero-dependency, frontend-only i18n layer for `en` and `es`. `App.svelte` initializes the active locale from browser language, owns the session override, provides typed locale context to inventory components, and keeps document metadata aligned. Components remain presentational: they render translated chrome from context while payload data from `ScanReport` stays verbatim.

## Architecture Decisions

| Decision | Choice | Alternatives considered | Rationale |
|---|---|---|---|
| I18n engine | Typed local catalogs and helpers under `frontend/src/lib/i18n/` | Paraglide/typesafe-i18n/heavy framework | Current UI has ~20 strings, Vitest is node-only, and the PoC benefits from no codegen or Vite plugin surface. |
| Locale ownership | Svelte 5 locale model created in `App.svelte` and exposed through typed context | Global shared store; prop drilling through every component | Context matches Svelte 5 guidance, avoids module-level state leaks, and keeps components decoupled from `App` wiring. |
| Runtime switching | One reactive locale source with `setLocale(locale)` and `t(key, params?)` | Per-component local state; static browser-only locale | Prevents SkillDock-style decorative selector failure; every chrome read goes through the same source. |
| Boundary | Localize UI chrome only; pass payload/diagnostics through | Translate component names, paths, `ScanIssue.reason`, `ScanError.detail.reason` | Proposal/spec require chrome localization but preserve core diagnostics and scanned data as source-of-truth. |
| Document metadata | Update `document.documentElement.lang` and `document.title` from the active locale | Leave `index.html` permanently English | Static HTML stays a bootstrap fallback; runtime metadata must reflect the selected locale. |

## Data Flow

```text
navigator.languages ──→ resolveLocale() ──→ createI18n(initial)
                                      │
LanguageSelector ── setLocale() ──────┤
                                      ▼
              App context ──→ InventoryToolbar/List/Row/LocationList
                                      │
                         t(key, params) renders chrome
                                      │
                      ScanReport payload remains verbatim
```

## File Changes

| File | Action | Description |
|---|---|---|
| `frontend/src/lib/i18n/catalogs.ts` | Create | `en` and `es` catalogs; base catalog defines key shape. |
| `frontend/src/lib/i18n/locale.svelte.ts` | Create | Supported locale types, browser resolution, interpolation, context helpers, reactive model. |
| `frontend/src/lib/i18n/locale.test.ts` | Create | Locale resolution, fallback, interpolation, catalog parity. |
| `frontend/src/App.svelte` | Modify | Initialize i18n, provide context, render selector, translate lifecycle/failure chrome, update document lang/title. |
| `frontend/src/lib/InventoryToolbar.svelte` | Modify | Replace placeholders, aria labels, options, and reload text with catalog messages. |
| `frontend/src/lib/InventoryList.svelte` | Modify | Translate empty state. |
| `frontend/src/lib/InventoryRow.svelte` | Modify | Translate duplicate badge/title and kind display label; leave name/description raw. |
| `frontend/src/lib/LocationList.svelte` | Modify | Translate null-path placeholder; leave paths raw. |
| `frontend/index.html` | Modify | Keep bootstrap `lang="en"`/title as fallback; runtime overrides in `App.svelte`. |

## Interfaces / Contracts

- `SupportedLocale = "en" | "es"`.
- `resolveLocale(languages?: readonly string[] | string | null): SupportedLocale` maps `es* → es`, `en* → en`, else `en`.
- `t(key, params?)` accepts only keys present in the base catalog; missing keys are compile-time/test failures, not runtime blanks.
- Interpolation is simple named replacement for small messages such as duplicate location counts and scan failure wrappers.
- Error copy uses localized chrome plus raw `error.detail.reason` passthrough.

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Unit | Locale resolution, unsupported fallback, session override helper, interpolation | `frontend/src/lib/i18n/locale.test.ts` under existing node Vitest. |
| Unit | `en`/`es` key parity and no blank messages | Compare recursive catalog key sets. |
| Integration/manual | Selector updates toolbar, list, row, lifecycle, aria/title/null-path chrome without rescan | Tauri/Vite smoke checklist; no DOM harness added in T12. |
| Regression | Payload fields and diagnostics remain verbatim | Unit fixture for scan error message composition plus manual loaded-row check. |

## Migration / Rollout

No data migration, IPC change, binding regeneration, Tauri capability, or filesystem permission change required. Roll out in one frontend PR after failing locale/catalog tests are added first under strict TDD.

## Open Questions

- [ ] None blocking. Locale persistence remains session-only for T12 unless product explicitly requests sticky `localStorage` later.