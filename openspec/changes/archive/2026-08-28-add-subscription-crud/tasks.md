# Tasks: Subscription CRUD

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | 620-780 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 core/store -> PR 2 IPC/audit -> PR 3 frontend |
| Delivery strategy | single-pr (`size:exception` accepted by maintainer) |
| Chain strategy | size:exception |

Decision needed before apply: No — `size:exception` accepted by maintainer
Chained PRs recommended: Yes
Chain strategy: size:exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|---|---|---|---|
| 1 | Durable core/store CRUD | PR 1 | Tests and generated bindings; independently verifiable. |
| 2 | Typed shell boundary and audit | PR 2 | Depends on PR 1; preserves app-data-only writes. |
| 3 | CRUD page and localized UI | PR 3 | Depends on PR 2; includes mocked-IPC tests. |

## Phase 1: Core and Repository - PR 1

- [x] 1.1 **RED** - Add DTO JSON/typed-error tests in `crates/vertice-core/src/model/subscription.rs` for camelCase contracts. *(Subscription CRUD, Validation)*
- [x] 1.2 **GREEN** - Create `subscription.rs`; re-export it from `model/mod.rs`, then regenerate `frontend/src/bindings/*.ts` via core tests. *(Subscription CRUD, Validation)*
- [x] 1.3 **RED** - Add temp-dir tests in `crates/vertice-app/src/subscriptions/store.rs` for empty/malformed stores, invalid inputs, atomic restart, concurrency, not-found, and immediate monotonic timestamps. *(Durable Local Persistence, CRUD, Validation)*
- [x] 1.4 **GREEN** - Create `subscriptions/{mod.rs,store.rs}` with semantic read validation, schema v1, atomic sibling rename, ID sequence, and app-data-only writes. *(Durable Local Persistence, Validation)*
- [x] 1.5 **REFACTOR** - Consolidate store validation/helpers without weakening byte-preservation or no-temp-file assertions. *(Durable Local Persistence)*

## Phase 2: IPC and Read-only Audit - PR 2 (after 1)

- [x] 2.1 **RED** - Extend `commands.rs` helper tests for typed CRUD results, validation/not-found, and store warning logging. *(Minimal Scan Command Surface)*
- [x] 2.2 **GREEN** - Register subscription state in `lib.rs`; add four `spawn_blocking` pass-through commands in `commands.rs`. *(Minimal Scan Command Surface)*
- [x] 2.3 **RED->GREEN** - Update `tests/read_only_audit.rs` to require 14 commands, five writers, and the subscription module's app-data-only allow-list; preserve `core:default`. *(Minimal Capability Grant, Fifth Write Exception)*
- [x] 2.4 **REFACTOR** - Align command/store naming with the prompt-library pattern; run targeted Rust tests. *(Desktop Shell delta)*

## Phase 3: Frontend CRUD - PR 3 (after 2)

- [x] 3.1 **RED** - Create `SubscriptionsPage.test.ts` with mocked IPC for failed-load retry, empty list, invalid form, save/delete retry, and confirm/cancel in EN/ES. *(Subscription CRUD, Validation)*
- [x] 3.2 **GREEN** - Replace samples with generated bindings and IPC wrappers in `subscriptions.ts`; adapt `SubscriptionCard.svelte` and `App.svelte`. *(Subscription CRUD)*
- [x] 3.3 **GREEN** - Implement accessible loading, form, confirmation, retry, empty, and error states in `SubscriptionsPage.svelte`; add both locale catalogs. *(Subscription CRUD)*
- [x] 3.4 **REFACTOR** - Remove obsolete sample-only wiring; run frontend lint/check/test/build and full prescribed gates. *(Subscription CRUD)*

## Remediation: Verification Findings

- [x] 4.1 Add manual, localized recovery guidance for corrupt or unsupported subscription storage without a misleading Retry action. *(Durable Local Persistence, Subscription CRUD)*
- [x] 4.2 Sync staged subscription data before rename and the parent directory after rename where supported; preserve typed errors and temp cleanup. *(Durable Local Persistence)*
- [x] 4.3 Extract the page test's scoped semantic role/label queries into a shared DOM-accurate helper. *(Subscription CRUD, Accessibility)*
- [x] 4.4 Share frontend renewal constants and run the full Rust/frontend verification matrix. *(Validation, Subscription CRUD)*
- [x] 4.5 Refine typed store errors: corrupt bytes vs temporary failures vs unreconciled committed durability warnings; add direct command-boundary tests. *(Durable Local Persistence, Desktop Shell)*
- [x] 4.6 Close final review findings: assert typed store diagnostics and exact error propagation through `spawn_blocking`, and model recovery as one discriminated page state. *(Desktop Shell, Subscription CRUD)*
- [x] 4.7 Prevent durability-warning reconciliation from replaying committed form or delete mutations; reset mutation context before reload. *(Subscription CRUD, Reliability)*

