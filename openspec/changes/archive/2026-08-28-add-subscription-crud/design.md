# Design: Subscription CRUD

## Technical Approach

Mirror the Prompt Library: core serde/ts-rs DTOs → app JSON repository behind `Arc<Mutex>` → four thin commands → an IPC-backed Svelte page. It implements both specs without dependencies or Tauri capabilities.

## Architecture Decisions

| Decision | Choice | Rejected | Rationale |
|---|---|---|---|
| Validation boundary | Store-owned `normalize_draft` plus `validate_record` on reads | Core validation; decode-only reads | `model/` must remain plain data; the store is the persistence boundary. Read validation reuses draft rules and prevents valid JSON from becoming trusted invalid state. |
| Update shape | Full replacement (`id` + all draft fields) | Partial patch | The inline form submits a complete record; patches need merge semantics when changing cycle. |
| Money/currency | finite `f64` > 0; closed `EUR`/`USD` enums | integer cents; arbitrary strings | Matches the ratified spec and existing formatter while rejecting invalid values. |
| Timestamp guarantee | RFC3339 UTC nanoseconds plus a per-entity monotonic bump | seconds-only clock | An immediate update MUST change `updatedAt`, even where clock sampling repeats. On update, use `max(Utc::now(), parsed previous + 1ns)`. |
| Persistence failure | `StoreCorrupt` for invalid persisted bytes; retryable `StoreUnavailable` for I/O, permissions, and lock contention | Treat every store failure identically | Only corrupted data needs preserve/rename recovery; temporary failures must never tell users to delete valid data. |
| Corrupt-store recovery | Manual recovery guidance; no retry action | Treat `StoreUnavailable` as a transient request failure | Retrying cannot repair corrupt or unsupported bytes, and `core:default` deliberately gives the UI no filesystem or dialog capability to alter them. |
| Atomic-write durability | Sync staged file before rename; sync parent directory after rename on supported platforms | Rename-only persistence | A successful rename is not enough to establish a durable update after a crash; Windows keeps the platform-safe no-op directory-sync path. |

## Data Flow

    Form → typed IPC wrapper → command (spawn_blocking) → Mutex repository
      ↑                         Result<Subscription, SubscriptionError> │
      └── role/labelled UI state ←───────────────────────────────────────┘
                                        validate → read/validate document
                                                     │
                 app_data_dir/subscriptions.json ← atomic temp + rename

Tauri resolves the app-data directory from `com.vertice.app`: `%APPDATA%\\com.vertice.app` (Windows), `~/Library/Application Support/com.vertice.app` (macOS), and `~/.config/com.vertice.app` (Linux).

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/model/subscription.rs` | Create | DTOs, enums and typed errors; camelCase ts-rs exports. |
| `crates/vertice-core/src/model/mod.rs` | Modify | Re-export subscription DTOs. |
| `crates/vertice-app/src/subscriptions/{mod.rs,store.rs}` | Create | Repository, semantic document validation, atomic persistence and tests. |
| `crates/vertice-app/src/{commands.rs,lib.rs}` | Modify | State registration and four command handlers. |
| `crates/vertice-app/tests/read_only_audit.rs` | Modify | Four commands and fifth sanctioned app-data writer. |
| `frontend/src/bindings/*.ts` | Regenerate | Generated from Rust; never hand-edited. |
| `frontend/src/lib/{subscriptions.ts,SubscriptionCard.svelte}` | Modify | Generated types, IPC wrappers and card typing. |
| `frontend/src/lib/pages/SubscriptionsPage.svelte` | Modify | Loading, empty, form, delete-confirmation and retry states. |
| `frontend/src/{App.svelte,lib/i18n/catalogs.ts}` | Modify | Remove sample wiring and add EN/ES subscription strings. |
| `frontend/src/lib/pages/SubscriptionsPage.test.ts` | Create | Semantic page-state tests with mocked IPC. |

## Interfaces / Contracts

`Subscription` has `id`, `provider`, `plan`, `amount`, `currency`, `cycle`, `renewalDay`, optional `renewalMonth`, and `updatedAt`; `SubscriptionDraft` omits identity/timestamp; `SubscriptionUpdate` is `id` plus all draft fields. `SubscriptionError` is `InvalidInput { field } | NotFound { id } | StoreCorrupt { reason } | StoreUnavailable { reason } | CommittedWithDurabilityWarning { reason }`.

The document is `{ "schemaVersion": 1, "subscriptions": [...] }`. Missing storage returns an empty list. After structural JSON decode and supported-schema check, every record is semantically validated **before it is returned or mutated**: its ID format, parseable RFC3339 `updatedAt`, and its draft through `normalize_draft` (trimmed non-empty provider/plan; finite positive amount; day 1–28; EUR/USD; yearly requires month 1–12; monthly month is optional). A semantic failure maps to `StoreCorrupt`, does not call save, and leaves file bytes untouched. I/O, permissions, and lock contention map to retryable `StoreUnavailable`. Mutation input failures remain `InvalidInput` and are never persisted. A failed parent-directory sync reconciles by rereading the expected document; only an unreconciled post-rename state maps to `CommittedWithDurabilityWarning`, which the UI resolves by reloading rather than retrying the mutation.

The repository writes through a sibling temporary file, syncs its contents, then renames it. On Unix it also syncs the parent directory after a successful rename; platforms that cannot open a directory for sync retain a safe no-op. IDs are `sub-{nanos}-{seq}` from a module-local `AtomicU64`. Creation stamps RFC3339 UTC with nanoseconds. Updating retains the ID and computes `updatedAt` as the later of current UTC and one nanosecond after the prior parsed timestamp, guaranteeing strict per-entity increase.

Commands are `list_subscriptions`, `create_subscription(draft)`, `update_subscription(update)`, and `delete_subscription(id)`, with thin `spawn_blocking` pass-throughs and warning logs for store failures. The audit adds them before `log_file_path` and explicitly allows only the repository's app-data write primitives.

## Testing Strategy

| Layer | What | Approach |
|---|---|---|
| Core | JSON/binding shape and typed error union | `subscription.rs` unit tests. |
| Store | Missing file, atomic restart persistence, staged-write readability, invalid mutations, not-found, concurrency and no temp files | Temp-dir unit tests. Include immediate create→update under a repeated/fixed clock: parse both RFC3339 values and assert same ID plus `updated_at > previous_updated_at`. For stored JSON, assert day `0`/`29`, invalid amount, and yearly without `renewalMonth` each return `StoreCorrupt` and preserve original bytes. |
| Audit | 14-command list and 5 sanctioned writers | `read_only_audit.rs`. |
| Frontend | Pure renewal functions and page semantics | Vitest with `vi.mock` of IPC wrappers; query visible roles/labels, not component internals. Cover failed transient `StoreUnavailable` load then retry; `StoreCorrupt` manual recovery guidance without a retry action; `CommittedWithDurabilityWarning` reload-only reconciliation; empty response with no seed; backend `InvalidInput` shown in the form; save failure then retry; delete failure then retry; and both confirm and cancel deletion paths. Test EN and ES visible messages. |

## Migration / Rollout

No migration. Remove `SAMPLE_SUBSCRIPTIONS`; a missing file starts empty. Existing malformed/future documents are not rewritten. They surface manual recovery guidance: close the app, back up `subscriptions.json`, then use the system file manager to rename or remove that file before reopening. The app does not attempt repair or expose a retry action for this case.

## Open Questions

None.
