# Design: Add Prompts Page

## Technical Approach

Preserve the existing prompt-library vertical slice: pure prompt DTOs in `vertice-core/src/model`, app-data persistence in `vertice-app/src/prompts`, typed Tauri commands, and `PromptsPage.svelte` owning client-side filtering, pagination, CRUD form state, action feedback, and clipboard copy. The new work is UI-level only: reuse the Skills/Agents pagination behavior from `ComponentKindPage.svelte` and harden prompt action hover/focus states without changing command or storage boundaries.

## Architecture Decisions

| Decision | Choice | Rejected alternatives | Rationale |
|---|---|---|---|
| Storage boundary | Keep versioned `app_data_dir/prompts.json` behind `PromptRepository` and atomic rename | SQLite; localStorage; moving persistence into core | Existing design already satisfies app-data-only writes and keeps `vertice-core` I/O-free. |
| Pagination ownership | Add pagination state/helpers inside `frontend/src/lib/pages/PromptsPage.svelte`, after `visiblePrompts` | Backend pagination; shared component refactor now | Prompt results are local and already filtered in the page; matching `ComponentKindPage.svelte` minimizes scope and risk. |
| Page bounds | Use `PAGE_SIZES = [5, 10, 15]`, `page`, `pageSize`, `previousQuery`, derived `pageCount/pageStart/pagePrompts/rangeStart/rangeEnd`, one `$effect` that resets only when `query` changes, and one `$effect` that clamps `page` to `pageCount` after filtering, deletion, creation/update ordering, or page-size changes | Resetting on page-size change; clamping only in event handlers | Matches the spec exactly: query changes restart discovery at page 1, while page-size changes preserve position unless the current page is no longer valid. |
| Actions | Style Copy/Edit/Delete as explicit buttons with stable accessible text, hover background/border feedback, and `focus-visible` ring; Delete keeps red/danger colors | Icon-only controls; hover-only disclosure; changing labels on state | Meets accessibility requirements: keyboard users see state, names remain stable, destructive semantics stay visible. |

## Data Flow

```text
prompts[] + query -> filterPrompts -> visiblePrompts
visiblePrompts + pageSize + page -> pagePrompts -> rendered cards
query change -> page = 1
visible/pageSize shrink -> page = min(page, pageCount)
Copy/Edit/Delete button -> existing handlers; no backend contract change
```

## File Changes

| File | Action | Description |
|---|---|---|
| `frontend/src/lib/pages/PromptsPage.svelte` | Modify | Add pagination state, derived slice/range helpers, pagination nav markup matching Skills/Agents, and stronger action hover/focus classes. |
| `frontend/src/lib/pages/PromptsPage.test.ts` | Modify | Add tests for action hover/focus class contracts and pagination behavior. |
| `frontend/src/lib/i18n/catalogs.ts` | Modify | Add prompt pagination labels or reuse existing component pagination copy only if wording remains correct in EN/ES. |
| Existing Rust/core/app files | No change | Storage, IPC, DTOs, and read-only audit remain as designed. |

## Interfaces / Contracts

No IPC or storage contract changes. `PromptsPage.svelte` should introduce:

```ts
const PAGE_SIZES = [5, 10, 15] as const;
let page = $state(1);
let pageSize = $state<(typeof PAGE_SIZES)[number]>(PAGE_SIZES[0]);
let previousQuery = $state("");
const pageCount = $derived(Math.max(1, Math.ceil(visiblePrompts.length / pageSize)));
const pageStart = $derived((page - 1) * pageSize);
const pagePrompts = $derived(visiblePrompts.slice(pageStart, pageStart + pageSize));
```

Also derive `rangeStart`/`rangeEnd`, render `{#each pagePrompts as prompt (prompt.id)}`, and use first/previous/next/last buttons disabled at bounds. Page-size changes keep the current page unless the new page count makes it invalid; the clamp effect then moves it to the final available page. Only query changes reset to page 1.

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Frontend pagination | Initial page size 5; page-size choices 5/10/15; next/previous/first/last bounds; query change resets to page 1; page-size changes do not reset when the current page remains valid; page-size/result shrink clamps to the last available page when out of range. | Vitest/Svelte Testing Library with mocked `fetchPrompts()` returning 6+ prompts; interact via labels/buttons. |
| Frontend actions | Copy/Edit/Delete are discoverable as semantic buttons by accessible name; keyboard tab/focus produces visible focus feedback; accessible names stay Copy/Edit/Delete; Delete retains danger semantics and visual danger treatment. | Use Testing Library role/name queries (`getByRole("button", { name: ... })`) and keyboard focus assertions as the primary proof. Preserve `data-testid` and Tailwind/class-token checks only as supplemental regression checks for the exact styling contract. |
| Regression | Existing create/edit/delete/copy/search tests keep passing. | `npm run test` from `frontend/`. |

## Migration / Rollout

No migration required. This is a presentational/reactive page update over existing local prompt data.

## Open Questions

None.