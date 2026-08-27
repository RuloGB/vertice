# Proposal: Add Prompts Page

## Intent

Give users a local prompt library so reusable AI prompts are not scattered across notes, chat history, or clients. Success means users can create, find, update, delete, and copy prompts.

## Scope

### In Scope
- Replace the Prompts placeholder with list, search, create, edit, delete, and copy actions.
- Persist prompts in versioned `prompts.json` in application data, with atomic writes behind a repository seam.
- Store `id`, `title`, `body`, `tags`, `bestForContext`, and `updatedAt`; `title` and `body` are required, while `tags` and `bestForContext` are optional.
- Search normalized `title`, `tags`, `body`, and `bestForContext` by substring.

### Out of Scope
- SQLite, full-text indexes, fuzzy/ranked search, history, import/export, sync, sharing, and client integrations.
- Automatic insertion into Claude, Codex, OpenCode, Copilot, or editors.

## Capabilities

### New Capabilities
- `prompt-library`: Local prompt CRUD, required title/body validation, normalized substring search, manual copy, prompt schema, app-data persistence, and page states.

### Modified Capabilities
- `desktop-shell`: Add typed prompt IPC while preserving async execution, minimal Tauri capabilities, app-data-only writes, and read-only audit proof.
- `frontend-i18n`: Add complete English and Spanish Prompts chrome.

## Approach

Add a `PromptRepository` seam in `vertice-app`, backed first by versioned JSON using the `settings/store.rs` temp-file-plus-rename durability pattern. Keep `vertice-core` model types I/O-free and generate TypeScript bindings if prompt types cross IPC. The Svelte page owns reactive filtering and uses the browser clipboard API.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/vertice-core/src/model/` | New | Shared prompt contract if needed. |
| `crates/vertice-app/src/commands.rs` | Modified | Prompt IPC. |
| `crates/vertice-app/src/prompts/` | New | Repository and JSON store. |
| `crates/vertice-app/tests/read_only_audit.rs` | Modified | App-data writer proof. |
| `frontend/src/lib/pages/` | New/Modified | Prompts page. |
| `frontend/src/lib/navigation.ts` | Modified | Content-bearing route. |
| `frontend/src/lib/i18n/catalogs.ts` | Modified | Localized chrome. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| JSON limits future scale | Medium | Keep repository seam; defer SQLite. |
| New writes weaken read-only posture | Medium | Confine writes to app data and audit exactly. |
| Clipboard behavior differs in tests/Tauri | Medium | Isolate copy handling and test success/failure UX. |

## Rollback Plan

Remove the page, IPC commands, repository module, generated bindings, catalog keys, and audit exception; return `prompts` to placeholder navigation. Existing `prompts.json` may remain inert.

## Dependencies

- Existing app-data resolution and settings JSON durability precedent.

## Success Criteria

- [ ] Users can create, edit, delete, list, search, and copy prompts manually, with saves blocked when title or body is empty.
- [ ] `prompts.json` is schema-versioned, atomically written, and app-data-only.
- [ ] Search covers normalized title, tags, body, and best-for context.
- [ ] Prompts UI is localized in English and Spanish.
- [ ] No filesystem capability grant or write outside app data is introduced.





