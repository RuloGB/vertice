# Exploration: P6 — CRUD suscripciones IA

## Current State

The Subscriptions page already exists as a **read-only, mock-data view**:

- **`frontend/src/lib/subscriptions.ts`** defines the `Subscription` interface (id, provider, plan, amount, currency, cycle, renewalDay, renewalMonth?), `BillingCycle`, `Currency` types, and a `SAMPLE_SUBSCRIPTIONS` constant with 6 illustrative entries. Pure functions handle renewal math (`nextRenewal`, `daysUntil`, `sortByRenewal`, `monthlyEquivalent`, `monthlyTotalsByCurrency`, `formatAmount`, `formatRenewalDate`). No I/O, no clock reads — mirrors the core convention.
- **`frontend/src/lib/SubscriptionCard.svelte`** renders one subscription card with provider, plan, amount, cycle badge, renewal date, and countdown.
- **`frontend/src/lib/pages/SubscriptionsPage.svelte`** renders the page header (with a "Sample data" badge), summary cards (active count + monthly spend), and a grid of `SubscriptionCard`s.
- **`frontend/src/App.svelte`** wires it: `<SubscriptionsPage subscriptions={SAMPLE_SUBSCRIPTIONS} {today} />` — hardcoded mock data, no IPC call.
- **No Rust-side subscription code exists.** Zero matches for "subscription" or "suscripcion" in any `.rs` file. No model types, no store, no IPC commands.

The page is a display-only prototype. The "Sample data" badge in the header explicitly signals this.

## Affected Areas

### New files (likely)
- `crates/vertice-core/src/model/subscription.rs` — Subscription DTOs (Subscription, SubscriptionDraft, SubscriptionUpdate, SubscriptionError)
- `crates/vertice-app/src/subscriptions/mod.rs` + `store.rs` — JsonSubscriptionRepository (mirrors prompts/store.rs)
- `frontend/src/lib/subscriptions.api.ts` — IPC wrappers (fetchSubscriptions, createSubscription, etc.)
- `openspec/specs/subscription-library/spec.md` — new living spec

### Modified files
- `crates/vertice-core/src/model/mod.rs` — add `mod subscription;` + re-exports
- `crates/vertice-app/src/lib.rs` — manage SubscriptionRepositoryState, register commands
- `crates/vertice-app/src/commands.rs` — list/create/update/delete_subscription commands
- `crates/vertice-app/tests/read_only_audit.rs` — add new commands to the audit allowlist
- `frontend/src/lib/subscriptions.ts` — remove SAMPLE_SUBSCRIPTIONS (or keep as seed), keep pure functions
- `frontend/src/lib/pages/SubscriptionsPage.svelte` — replace props-driven mock with IPC-backed CRUD (form, edit, delete confirmation)
- `frontend/src/App.svelte` — remove SAMPLE_SUBSCRIPTIONS import, change SubscriptionsPage wiring
- `frontend/src/lib/i18n/` — new i18n keys for CRUD actions, form labels, validation messages
- `frontend/src/bindings/` — regenerated from Rust types (Subscription.ts, SubscriptionDraft.ts, etc.)

## Approaches

### Approach 1: Mirror the Prompt Library pattern (recommended)

Replicate the exact vertical slice that prompts established:

1. **Core model** (`vertice-core/src/model/subscription.rs`): Plain DTOs with `serde` + `ts_rs`. `Subscription` (id, provider, plan, amount, currency, cycle, renewalDay, renewalMonth?, updatedAt). `SubscriptionDraft` (provider, plan, amount, currency, cycle, renewalDay, renewalMonth?). `SubscriptionUpdate` (id + all draft fields). `SubscriptionError` (InvalidInput { field }, NotFound { id }, StoreUnavailable { reason }).

2. **App store** (`vertice-app/src/subscriptions/store.rs`): `JsonSubscriptionRepository` with `subscriptions.json` in app_data_dir. Schema-versioned document. Atomic writes via temp-file-plus-rename (same as prompts). `SubscriptionRepository` trait with list/create/update/delete.

3. **Commands** (`vertice-app/src/commands.rs`): `list_subscriptions`, `create_subscription`, `update_subscription`, `delete_subscription`. Same `Arc<Mutex<...>>` state pattern. Same `spawn_blocking` + `prompt_join_error`-equivalent pattern.

4. **Frontend**: Replace `SubscriptionsPage` props-driven mock with an IPC-backed page mirroring `PromptsPage.svelte` — inline form for create/edit, ConfirmDialog for delete, toast feedback, loading/empty/failure states. Remove the "Sample data" badge.

5. **Read-only audit**: Add the 4 new commands to `read_only_audit.rs`'s allowlist.

**Pros**:
- Follows an established, proven pattern — minimal design risk
- Same persistence semantics (atomic writes, schema versioning, fail-closed on corrupt data)
- Same IPC shape (typed errors, camelCase JSON, ts-rs bindings)
- Same test patterns (temp_app_data_dir, byte preservation, concurrent mutations)
- Read-only audit already understands this shape

**Cons**:
- Subscriptions are conceptually different from prompts (user-managed billing data vs. prompt templates) — but the CRUD mechanics are identical
- Currency handling: the current frontend model restricts to EUR/USD; the Rust model should decide whether to keep this restriction or open it

**Effort**: Medium — well-trodden path, mostly mechanical replication with subscription-specific validation (amount > 0, renewalDay 1-28, renewalMonth 1-12 for yearly).

### Approach 2: Shared generic CRUD repository

Extract a generic `JsonCrudRepository<T>` that both prompts and subscriptions share, parameterized by the document type.

**Pros**:
- Reduces code duplication if more CRUD entities are added later
- Single place for atomic-write, schema-versioning, and error-handling logic

**Cons**:
- Over-engineering for exactly 2 entities — YAGNI
- Rust's type system makes generic JSON repositories awkward (each entity has different validation rules, different field normalization, different error variants)
- Prompts are already implemented concretely — refactoring to generic would be a separate change that touches working code
- The prompt store's `normalize_draft` logic is entity-specific; a generic layer would need trait-based hooks that add complexity without clear benefit at this scale

**Effort**: High — requires refactoring working prompt code, designing the right abstraction, and migrating both stores.

## Recommendation

**Approach 1: Mirror the Prompt Library pattern.** It's the proven vertical slice. The project already has a clear precedent for "user-managed data persisted in app_data_dir as JSON" — the prompt library. Subscriptions are a second entity of the same kind. Follow the same shape, same layering, same testing patterns. A generic abstraction can be extracted later if a third entity appears (rule of three).

### Key design decisions to resolve in the proposal

1. **Currency model**: The frontend currently restricts to `"EUR" | "USD"`. Should the Rust model use an enum (closed set) or a string (open set)? Recommendation: start with an enum matching the frontend (`Eur`, `Usd`), expand later if needed. This keeps the type contract tight.

2. **Amount validation**: Must be > 0. Should it be stored as cents (integer) to avoid floating-point issues? The frontend uses `number` (float). Recommendation: store as `f64` in Rust (matching the frontend's `number`), validate > 0 at the boundary. Floating-point precision is acceptable for display-only amounts formatted via `Intl.NumberFormat`.

3. **renewalDay cap**: The frontend comment says "Capped at 28 so every month has it." The Rust model should enforce 1-28 at validation time.

4. **renewalMonth**: Required for yearly, ignored for monthly. The Rust model should validate: if cycle == "yearly", renewalMonth must be Some(1-12).

5. **ID generation**: Follow the prompt pattern (`sub-{unique_suffix}`) or use UUID? Recommendation: follow the prompt pattern for consistency.

6. **Sample data migration**: Should the 6 sample subscriptions be offered as a "seed" on first run? Or start empty? Recommendation: start empty. The sample data was illustrative; real users should enter their own. The empty state already exists in the UI.

7. **Tauri capabilities**: No change needed. The current `default.json` capability grants `core:default` only — no fs/shell/dialog permissions. All persistence goes through Rust-side `std::fs` in `vertice-app`, which has full filesystem access by construction (it's the native side). The webview remains sandboxed.

## Risks

1. **Read-only audit update**: The `read_only_audit.rs` test hardcodes the list of known commands. Adding subscription commands without updating the audit will fail CI. Must add the 4 new commands to the allowlist.

2. **Type binding regeneration**: New Rust types with `#[ts(export)]` will regenerate `frontend/src/bindings/`. CI checks for binding drift. Must run `cargo test -p vertice-core` and commit the regenerated bindings.

3. **i18n coverage**: New CRUD UI needs English + Spanish translations for form labels, validation errors, action buttons, toast messages, confirmation dialogs. The i18n spec requires both languages from the first commit.

4. **Concurrent access**: The prompt store uses `Arc<Mutex<JsonPromptRepository>>` for serialized writes. The subscription store needs the same pattern. The `concurrent_whole_mutations_serialize_without_lost_updates_or_temp_collisions` test pattern should be replicated.

5. **Schema evolution**: If the subscription model changes later (e.g., adding a "notes" field), the schema_version in the JSON document allows migration. Start at version 1.

## Ready for Proposal

**Yes.** The exploration has enough information to write a formal proposal:

- The pattern to follow is clear and proven (prompt library)
- The affected files are identified
- The key design decisions are enumerated with recommendations
- The risks are known and manageable
- No architectural unknowns remain — this is a mechanical replication of an existing vertical slice into a new domain

The orchestrator should proceed to the **propose** phase to create `proposal.md` with the formal change description, capabilities touched, and scope boundaries.
