## Exploration: Prompts page (add-prompts-page)

### Current State
Vertice already reserves a `prompts` sidebar route, but it currently renders `PlaceholderPage` because `navigation.ts` excludes `prompts` from `ROUTES_WITH_CONTENT` and `App.test.ts` asserts that Prompts shows the explicit empty placeholder. Durable local persistence already exists in `vertice-app` app-data storage through JSON files: `settings/store.rs` owns `settings.json` with atomic temp-file-plus-rename writes, while `freshness/cache.rs` owns a simpler disposable JSON cache. Frontend navigation, i18n, and page composition are centralized in `App.svelte`, `Sidebar.svelte`, `navigation.ts`, and `i18n/catalogs.ts`. Existing list-page search is entirely client-side over in-memory data (`ComponentToolbar` + `filterComponents`), with no routing library and no backend-owned page state.

### Affected Areas
- `frontend/src/lib/navigation.ts` — `prompts` already exists as a route, but must move from placeholder-only to content-bearing.
- `frontend/src/App.svelte` — routes are shell-owned; prompts page state, load/save lifecycle, and copy-to-clipboard wiring would be threaded here.
- `frontend/src/lib/pages/PlaceholderPage.svelte` — current Prompts behavior is an explicit placeholder that the feature would replace.
- `frontend/src/lib/i18n/catalogs.ts` — new prompts-page chrome, empty/loading/error/search/copy strings, and Spanish parity.
- `frontend/src/App.test.ts` — currently pins Prompts as a placeholder; must be replaced with real-page behavior tests.
- `frontend/src/lib/navigation.test.ts` — `hasContent("prompts")` currently expects `false`; the contract must flip.
- `frontend/src/lib/ComponentToolbar.svelte` or a sibling prompts toolbar — useful pattern for search/reload-style chrome and accessible search input.
- `frontend/src/lib/settings.ts` + `crates/vertice-app/src/commands.rs` — existing IPC pattern to mirror for a prompts read/write command pair.
- `crates/vertice-app/src/settings/store.rs` — strongest local precedent for durable JSON in app data with atomic rename.
- `crates/vertice-core/src/model/` + generated `frontend/src/bindings/` — if prompts become shared typed data across Rust and Svelte, the model/binding seam is the existing contract.
- `crates/vertice-app/tests/read_only_audit.rs` — any new prompt persistence module must remain a sanctioned app-data-only write path.

### Approaches
1. **Versioned JSON document in app data** — store all prompts in one `prompts.json` document, loaded through Tauri IPC and searched in memory on the page.
   - Pros: aligns with the existing `settings.json` precedent; lowest implementation cost; no new database dependency; atomic rename pattern can be reused; easy export/debug story; fast enough for tens to low-hundreds of prompts.
   - Cons: whole-document rewrites on every mutation; no indexed search beyond in-memory substring/tag matching; schema evolution and concurrent-write protection are manual; will get clumsy if prompts grow into the high hundreds/thousands or need sorting/filter analytics.
   - Effort: Low/Medium.

2. **SQLite-backed prompts store** — persist each prompt as rows with indexed title/tag/content fields and query through IPC.
   - Pros: best long-term fit for larger prompt libraries, indexed search, future favorites/history/sorting, partial updates, and migration discipline; consistent with `openspec/config.yaml`'s broader `Persistence: SQLite (v1+)` direction.
   - Cons: higher first-slice cost; new schema/migration/test surface; heavier than the current app-data patterns; overkill unless prompt count and query complexity are expected to grow quickly.
   - Effort: Medium/High.

3. **Hybrid first slice: versioned JSON now, storage seam designed for later SQLite migration** — define a `PromptRepository`-style boundary in `vertice-app`, start with JSON implementation, keep frontend and IPC agnostic.
   - Pros: keeps first release cheap while avoiding a painted-into-a-corner architecture; lets tests pin behavior instead of storage; smooth migration path if search or scale outgrows JSON.
   - Cons: slightly more upfront design work than direct JSON; still inherits JSON limits until migration actually happens.
   - Effort: Medium.

### Recommendation
Recommend **Approach 3**: ship the first slice on a **versioned JSON document in the app data directory**, but hide it behind a repository seam and **reuse the `settings/store.rs` durability pattern** (same-directory temp file + atomic rename, schema version, app-data-only path derivation, read-only-audit proof). WHY: the current product is a single-user desktop app, the search requirement is “find in seconds” rather than full-text ranking, and the codebase already has proven JSON persistence patterns plus a placeholder `prompts` route ready to become real UI. For a first slice, load the whole document once, search client-side by normalized title, tags, and body/context text, and provide a copy button per prompt using the browser clipboard API in the Svelte page. Design the IPC around behavior (`list_prompts`, `save_prompt`, `delete_prompt`, possibly `copy` stays frontend-only) rather than around JSON so a later SQLite swap does not leak into the UI.

**Recommended product first slice**
- Prompt fields: `id`, `title`, `body`, `tags: string[]`, `bestForContext`, `updatedAt`.
- Discovery: one search box matching normalized title, tags, body, and context text; optional tag chips later, not required on day one.
- UI: replace the Prompts placeholder with a dedicated page showing search, empty state, list/card results, and copy action.
- Persistence: one app-data `prompts.json` file with `schemaVersion` and atomic writes.
- Search performance assumption: keep the active dataset in memory and filter reactively; for the expected first-slice scale, this comfortably meets a “seconds” requirement.

### Risks
- JSON is the WRONG tool if product intent is a large prompt vault with hundreds/thousands of entries, ranking, or compound filters; in that case SQLite should start on day one.
- Copy-to-clipboard must be tested in Tauri/jsdom carefully; there is no existing clipboard abstraction in the repo.
- New write paths must preserve CA-16/read-only discipline by writing only inside app data and being explicitly audited.
- If prompts are modeled in `vertice-core`, keep `model/` I/O-free; storage logic MUST stay in `vertice-app`.
- Search semantics need a product decision: plain substring matching is enough for first slice, but fuzzy ranking/stemming is a different scope.
- Tag normalization (case, trim, duplicates) needs one canonical rule early or search quality will drift.

### Ready for Proposal
Yes — the change is ready for proposal if the proposal explicitly states that first slice scope is a local prompt library with client-side search over a modest dataset, JSON persistence with atomic writes, and a storage seam that keeps a future SQLite migration open.
