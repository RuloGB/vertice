# Proposal: Subscription CRUD

## Intent

The Subscriptions page is a read-only mock rendering hardcoded sample data; users cannot track real AI subscription costs. This change delivers full CRUD persisted locally so users manage their own billing data across restarts — the same user-managed-data class as the Prompt Library, which is the proven reference pattern.

## Scope

### In Scope
- Core DTOs: `Subscription`, `SubscriptionDraft`, `SubscriptionUpdate`, `SubscriptionError` (serde + ts-rs)
- `JsonSubscriptionRepository`: schema-versioned `subscriptions.json` in app_data_dir, atomic temp-file-plus-rename writes, `Arc<Mutex>` state
- 4 typed commands: `list_subscriptions`, `create_subscription`, `update_subscription`, `delete_subscription`
- Validation: amount > 0 (f64); `renewalDay` 1–28; `renewalMonth` 1–12 required for yearly; currency closed enum EUR/USD; IDs `sub-{suffix}`; store starts empty (no seed)
- IPC-backed page: inline create/edit form, delete confirmation, loading/empty/success/failure states, EN+ES i18n keys from first commit

### Out of Scope
- Generic shared CRUD repository (rule of three; only two entities exist)
- Currencies beyond EUR/USD, integer-cents money model, seed/sample-data migration
- Renewal reminders, notifications, or external billing integrations

## Capabilities

### New Capabilities
- `subscription-library`: user-managed subscription CRUD, validation, renewal-date rules, and durable JSON persistence confined to app_data_dir

### Modified Capabilities
- `desktop-shell`: command surface grows by four subscription commands; the read-only audit gains a fifth sanctioned write exception (subscription persistence module), changing its entry count from exactly four to exactly five

## Approach

Mirror the Prompt Library vertical slice: core DTOs → JSON store in app → thin async commands → Svelte CRUD page. Commands stay thin pass-throughs (no business logic in the app layer). Structural invariants hold: core stays Tauri-free and model stays I/O-free; all writes live in the app-layer store; no new Tauri capability grants (`core:default` only).

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/vertice-core/src/model/subscription.rs` | New | DTOs + validation |
| `crates/vertice-app/src/subscriptions/store.rs` | New | JsonSubscriptionRepository |
| `crates/vertice-app/src/commands.rs`, `lib.rs` | Modified | 4 commands, state registration |
| `crates/vertice-app/tests/read_only_audit.rs` | Modified | Command allowlist + 5th write exception |
| `frontend/src/lib/subscriptions.ts`, `pages/SubscriptionsPage.svelte`, `App.svelte` | Modified | Remove mock wiring; IPC-backed CRUD |
| `frontend/src/bindings/` | Regenerated | From Rust types (committed) |
| `openspec/specs/subscription-library/` | New | Living spec |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `read_only_audit.rs` fails CI (allowlist + exception count) | High | Update allowlist and exception in same commit |
| Binding drift fails CI | High | Run `cargo test -p vertice-core`, commit regenerated bindings |
| Missing EN/ES keys | Medium | i18n keys added with both locales from first commit |
| Lost updates under concurrency | Low | `Arc<Mutex>` + replicate prompts' concurrent-mutation test |

## Rollback Plan

Revert the merge commit. All three layers revert cleanly: core adds only new modules, app registers independent state, frontend changes are page-local. No schema coupling with existing stores. A user-created `subscriptions.json` is orphaned but harmless; delete app_data_dir to remove it.

## Dependencies

- None. Reuses the prompt-library pattern and existing CI gates.

## Success Criteria

- [ ] Create/list/update/delete work end-to-end and survive app restart
- [ ] Validation rejects amount ≤ 0, `renewalDay` outside 1–28, yearly without `renewalMonth`
- [ ] CI green: read-only audit, bindings-in-sync, fmt/clippy/tests, frontend lint/check/test
- [ ] EN + ES coverage for all new UI strings
- [ ] No writes outside app_data_dir (audit proves it)
