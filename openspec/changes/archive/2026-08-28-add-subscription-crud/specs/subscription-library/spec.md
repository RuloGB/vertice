# Subscription Library Specification

## Purpose

Provide a local library for tracking user-managed AI subscriptions and their recurring costs.

## Requirements

### Requirement: Subscription CRUD

The system MUST let users list, create, edit, and delete subscriptions. Each subscription MUST contain `id`, `provider`, `plan`, `amount`, `currency`, `cycle`, `renewalDay`, and optional `renewalMonth` fields, plus its update timestamp. New libraries MUST start empty and MUST NOT seed sample data.

#### Scenario: Create and list a subscription
- GIVEN the subscription library is empty
- WHEN the user saves valid provider, plan, amount, currency, cycle, and renewal-date data
- THEN the subscription appears in the list with an ID matching `sub-{suffix}`

#### Scenario: Edit preserves identity
- GIVEN an existing subscription
- WHEN the user saves changes to its editable fields
- THEN the same ID is retained and the update timestamp changes

#### Scenario: Delete removes a subscription
- GIVEN an existing subscription
- WHEN the user confirms deletion
- THEN the subscription is absent from subsequent list results

### Requirement: Subscription Validation

The system MUST accept only `EUR` or `USD` currency values. Amount MUST be greater than zero, `renewalDay` MUST be 1 through 28, and yearly subscriptions MUST provide `renewalMonth` from 1 through 12. Monthly subscriptions MAY omit `renewalMonth`, and invalid input MUST NOT be persisted.

#### Scenario: Invalid billing data is rejected
- GIVEN a create or update request with a non-positive amount or renewal day outside 1–28
- WHEN the request is submitted
- THEN it fails with a typed validation error and the stored subscription remains unchanged

#### Scenario: Yearly renewal month is required
- GIVEN a yearly create or update request without a valid renewal month
- WHEN the request is submitted
- THEN it fails with a typed validation error and no invalid record is stored

### Requirement: Durable Local Persistence

The system MUST persist subscriptions in a schema-versioned `subscriptions.json` under the application data directory. Writes MUST be atomic, and a missing file MUST load as an empty library. Store, not-found, and malformed-data failures MUST surface as typed errors rather than being reported as an empty successful library.

#### Scenario: Subscriptions survive restart
- GIVEN one or more subscriptions were saved successfully
- WHEN the application restarts
- THEN the same subscriptions are loaded from the application data directory

#### Scenario: Missing, corrupt, and temporarily unavailable storage are distinguished
- GIVEN the storage file is missing or contains malformed data
- WHEN the library loads
- THEN a missing file yields an empty library, malformed or unsupported persisted data yields `StoreCorrupt`, and I/O, permission, or lock-contention failures yield retryable `StoreUnavailable`

#### Scenario: Post-rename durability ambiguity is reconciled
- GIVEN a subscription write has renamed the staged document but syncing the parent directory fails
- WHEN the repository can read back the expected document
- THEN the mutation succeeds without a retryable failure
- AND WHEN it cannot read back the expected document
- THEN it yields `CommittedWithDurabilityWarning` so the UI reloads instead of repeating the mutation
