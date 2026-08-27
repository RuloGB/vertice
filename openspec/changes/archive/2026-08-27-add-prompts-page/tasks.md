# Tasks: Add Prompts Page

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | 650-900 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR1 core+repository -> PR2 IPC+audit -> PR3 frontend+i18n |
| Delivery strategy | single-pr |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|---|---|---|---|
| 1 | Core DTOs + JSON repository + store tests | PR 1 | Foundation slice; read-only app-data only |
| 2 | Tauri commands + state wiring + audit coverage | PR 2 | Depends on PR 1 |
| 3 | Prompts page + search/copy UX + i18n tests | PR 3 | Depends on PR 2; delivery remains one PR by approved exception |

## Phase 1: Core Contract
- [x] 1.1 RED: add `vertice-core/src/model/prompt.rs` tests for DTO serde/TS shape and export via `src/model/mod.rs` [CRUD]
- [x] 1.2 GREEN: add `Prompt`, `PromptDraft`, `PromptUpdate`, `PromptError`; regenerate `frontend/src/bindings/*` without hand edits [CRUD]

## Phase 2: Repository / Persistence
- [x] 2.1 RED: add `vertice-app/src/prompts/store.rs` tests for empty title/body rejection, byte preservation on failed mutation, missing-file empty load, and normalized saved fields [CRUD][Persistence]
- [x] 2.2 RED: add concurrency/schema failure tests for unique temp names, atomic rename, unsupported schema, corrupt JSON, and no writes outside app data [Persistence]
- [x] 2.3 GREEN: create `vertice-app/src/prompts/{mod.rs,store.rs}` with repository trait, mutex transaction, unique local IDs, RFC3339 `updatedAt`, trimmed tags, and app-data-only writes [CRUD][Persistence]

## Phase 3: IPC / Audit
- [x] 3.1 RED: add command tests around `vertice-app/src/commands.rs` / `src/lib.rs` for typed list/create/update/delete responses and error mapping [CRUD]
- [x] 3.2 GREEN: register repository state plus `list_prompts`, `create_prompt`, `update_prompt`, `delete_prompt` as thin async commands [CRUD]
- [x] 3.3 Update `vertice-app/tests/read_only_audit.rs` to whitelist only `src/prompts/store.rs` atomic app-data writes [Persistence]

## Phase 4: Frontend
- [x] 4.1 RED: add Vitest coverage for `frontend/src/lib/promptSearch.ts` matching title/tags/body/context and proving no fuzzy ranking [Search/Copy]
- [x] 4.2 RED: add page tests for blocked empty title/body saves, loading/empty/failure states, and copy success/failure UX in `PromptsPage.svelte` [CRUD][Search/Copy][Persistence]
- [x] 4.3 GREEN: create `frontend/src/lib/{prompts.ts,promptSearch.ts}` and `pages/PromptsPage.svelte` with derived filtering, CRUD form, delete confirm, and clipboard copy [CRUD][Search/Copy][Persistence]
- [x] 4.4 Wire `frontend/src/App.svelte`, `lib/navigation.ts`, and `lib/i18n/catalogs.ts` to replace the placeholder with EN/ES prompts chrome [CRUD][Search/Copy]

## Phase 5: Verify / Refactor
- [x] 5.1 REFACTOR: keep search client-only, commands pass-through, and store concerns isolated by layer; remove duplication found during GREEN [All]
- [x] 5.2 Run `cargo test -p vertice-core --locked`, `cargo test --workspace --locked`, and `npm run test` from `frontend/` after each slice [All]

## Post-Apply Review Remediation
- [x] R1 Ensure rejected create/update/delete Prompts IPC calls settle into deterministic retryable UI failure states without unhandled promise rejections.
- [x] R2 Ensure prompt store/schema/write failures emit diagnostics at the prompt command boundary.
- [x] R3 Remove the new direct `uuid` dependency and confirm remaining `uuid` usage is only transitive through Tauri.

## Approved UI Polish Follow-up
- [x] F1 RED: extend `frontend/src/lib/pages/PromptsPage.test.ts` with semantic-button assertions for Copy/Edit/Delete plus keyboard-focus coverage proving names stay stable and Delete keeps danger treatment [Search/Copy]
- [x] F2 RED: add pagination tests in `frontend/src/lib/pages/PromptsPage.test.ts` for default size 5, size choices 10/15, query resets to page 1, and page-size/filter shrink clamping only when the current page becomes invalid [Pagination]
- [x] F3 GREEN: update `frontend/src/lib/pages/PromptsPage.svelte` to add 5/10/15 pagination, first/prev/next/last navigation, query-only reset logic, out-of-range clamping, and stronger hover/focus-visible action states without renaming controls [Pagination][Search/Copy]
- [x] F4 GREEN: adjust `frontend/src/lib/i18n/catalogs.ts` only if prompt pagination labels are missing or wording differs from the approved Skills/Agents navigation copy [Pagination]
- [x] F5 REFACTOR: keep pagination/filter derivations local to `PromptsPage.svelte`, preserve existing prompt CRUD/copy handlers, and rerun `npm run test` from `frontend/` after the follow-up slice [Pagination][All]

## Verify Remediation (parallel after implementation)
- [x] V1 Run `rustfmt` on `crates/vertice-app/src/commands.rs`, then prove `cargo fmt --all --check` passes to clear the archive-blocking formatter gate. [Desktop Shell][Quality]
- [x] V2 Update `openspec/changes/add-prompts-page/apply-progress.md` to include completed R1-R3 and F1-F5 work, preserve cumulative strict-TDD evidence, and correct final prompt ID wording to the current `prompt-{nanos}-{sequence}` behavior. [Artifacts][Prompt Library]
