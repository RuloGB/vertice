# Tasks: Frontend Internationalization

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | 380-520 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 i18n core + tests -> PR 2 UI wiring + smoke checks |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|---|---|---|---|
| 1 | Typed catalogs, locale model, failing/passing Vitest coverage | PR 1 | Independent RED->GREEN->REFACTOR base |
| 2 | App and inventory components consume i18n context | PR 2 | Depends on PR 1; keep manual smoke notes with code |

## Phase 3: Surface - RED

- [x] 3.1 T12 / Spec `frontend-i18n: Supported Locale Resolution` / CA:T12 — Add failing Vitest cases in `frontend/src/lib/i18n/locale.test.ts` for `es*`, `en*`, unsupported fallback, interpolation, and catalog parity.
- [x] 3.2 T12 / Spec `frontend-i18n: Catalog Completeness and Boundary` / CA:T12 — Inventory all chrome literals in `frontend/src/App.svelte`, `frontend/src/lib/InventoryToolbar.svelte`, `InventoryList.svelte`, `InventoryRow.svelte`, and `LocationList.svelte`; lock untranslated payload/diagnostic fields in assertions/checklist.

## Phase 3: Surface - GREEN

- [x] 3.3 T12 / Spec `frontend-i18n: Supported Locale Resolution` / CA:T12 — Create `frontend/src/lib/i18n/catalogs.ts` with typed `en` and `es` catalogs covering labels, placeholders, loading, empty, failure, title, aria, duplicate, kind, and null-path copy.
- [x] 3.4 T12 / Spec `frontend-i18n: Reactive UI Locale Switching` / CA:T12 — Create `frontend/src/lib/i18n/locale.svelte.ts` with `SupportedLocale`, `resolveLocale()`, interpolation, context helpers, and one reactive `setLocale()` source.
- [x] 3.5 T12 / Spec `frontend-i18n: Reactive UI Locale Switching` / CA:T12 — Update `frontend/src/App.svelte` to initialize locale from browser detection, render the selector, translate lifecycle/failure chrome, and sync `document.documentElement.lang` plus `document.title`.
- [x] 3.6 T12 / Spec `inventory-ui: Localized Inventory Chrome` / CA:T12 — Replace literals in `frontend/src/lib/InventoryToolbar.svelte`, `InventoryList.svelte`, `InventoryRow.svelte`, and `LocationList.svelte` with catalog reads while keeping names, paths, and `error.detail.reason` verbatim.
- [x] 3.7 T12 / Design metadata fallback / CA:T12 — Keep `frontend/index.html` bootstrap language/title aligned with the English default only.

## Phase 3: Surface - REFACTOR / VERIFY

- [x] 3.8 T12 / Spec `frontend-i18n` + `inventory-ui` / CA:T12 — Refactor repeated message access into small helpers only if tests stay green and components remain presentational.
- [x] 3.9 T12 / Scenario `Manual language change` and `Chrome follows locale changes` / CA:T12 — Run `npm run test`; then smoke-check startup locale, selector switching, failure state, duplicate badge/title, aria labels, and null-path copy without rescan.

## Fresh Review Remediation

- [x] R1 / Reliability blocker — Add jsdom-backed Svelte component coverage proving language selector updates visible inventory chrome, `document.documentElement.lang`, and `document.title` without calling `scan()` again or `rescan()`.
- [x] R2 / Readability warning — Reintegrate `frontend/src/lib/appTitle.ts` into productive catalog title construction so the helper is no longer test-only code.
- [x] R3 / Reliability blocker - Configure Vitest to reject focused tests via `allowOnly: false`, with executable guard evidence proving `it.only` fails while the normal frontend suite remains green.
