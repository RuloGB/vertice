# Proposal: Add Tauri Scan Commands

## Intent

Deliver T10 (`internal-docs/plan-desarrollo-poc.md:231`): expose the T9 core scan to the frontend through the minimal IPC surface with minimal permissions. No CA-n row closes at T10 — the plan-level criteria are generated IPC types and a reviewable minimal capabilities declaration, which T14 will audit against CA-16.

## Scope

### In Scope
- Two async commands `scan`/`rescan` (`crates/vertice-app/src/commands.rs`, new; registered in `lib.rs`): thin `spawn_blocking(vertice_core::scan::scan)` wrappers returning `Result<ScanReport, ScanError>`; `JoinError` maps to the existing `ScanError::Internal` variant.
- Capabilities stay `core:default` only, rationale documented in the file.
- CSP hardening in `tauri.conf.json`: `object-src 'none'; base-uri 'none'`; no remote content; no production `unsafe-inline`.
- `frontend/src/lib/scan.ts` + Vitest tests (mocked `@tauri-apps/api/core`), generated bindings only; `App.svelte` gets at most a smoke invocation.

### Out of Scope
- Changes to `crates/vertice-core`; new bindings; new dependencies.
- `tauri-plugin-fs` or any fs/dialog/shell capability.
- Inventory UI (T11), i18n (T12), error states (T13), watchers.
- PoC exclusions verified: no MCPs, no project/local scope, no writes, no persistence.

## Capabilities

### New Capabilities
- `desktop-shell`: Tauri IPC surface exposing core scan — command contract, ACL posture, CSP policy.

### Modified Capabilities
None.

## Approach

**Capabilities interpretation (explicit):** the plan's "disk access restricted by scope" is satisfied by construction — all disk access lives in vertice-core against compile-time-fixed roots, and with no fs plugin the webview holds zero filesystem capability. A literal reading ("scoped fs access + app data dir") would grant the webview fs access the frontend never uses; reviewers MUST NOT add `tauri-plugin-fs`.

**Async required:** sync commands run on the main thread — up to the CA-15 2s budget of frozen UI.

**Direct transport:** `ScanError` is already `Serialize + TS`; the IPC payload matches the generated binding exactly — "generated types, not hand-written" by construction.

**Open confirmation — scan vs rescan:** the core has no cache/state, so both commands are semantically identical in the PoC (difference is frontend intent: startup auto-scan vs T11 reload). Recommendation: implement both per the plan's explicit scope ("Comandos: `scan()` y `rescan()`. Nada más."); the spec will state no caching exists and `rescan` keeps the IPC contract stable for future cache semantics. **Confirm before sdd-spec.**

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/vertice-app/src/commands.rs` | New | Async scan/rescan handlers |
| `crates/vertice-app/src/lib.rs` | Modified | Register invoke handler |
| `crates/vertice-app/capabilities/default.json` | Modified | Stays `core:default` |
| `crates/vertice-app/tauri.conf.json` | Modified | CSP directives |
| `frontend/src/lib/scan.ts` | New | Typed invoke wrapper + tests |
| `frontend/src/App.svelte` | Modified | Smoke invocation at most |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Plan misread → fs plugin added | Medium | Explicit interpretation above; spec pins it |
| Sync command freezes UI | Low | Async + `spawn_blocking` mandated |
| scan/rescan reads as dead API | Low | Spec justifies duplication (plan-mandated) |

## Rollback Plan

Delete the command module/registration, CSP directives, and frontend wrapper. vertice-core and bindings untouched by construction; no persisted state, no migration. Three-layer impact: core unaffected, app returns to bare skeleton, frontend loses one module.

## Dependencies

- T9 scan orchestrator — merged (`2026-08-20-add-scan-orchestrator`).

## Success Criteria

- [ ] `scan`/`rescan` return T2-generated `ScanReport`/`ScanError` over IPC.
- [ ] Capabilities declaration reviewable, granting no more than `core:default`.
- [ ] CSP includes `object-src 'none'; base-uri 'none'`; no remote content.
- [ ] No fs plugin, no new dependency, no core change, no writes outside the app data directory.
