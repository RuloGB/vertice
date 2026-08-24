# Domain Model Specification

## Purpose

Defines the core domain types shared by all adapters and the frontend, the deterministic identity rule that makes aggregation possible (T8), the `ScanIssue`/`ScanReport` error and result contract, and the Rust-to-TypeScript type generation surface. Traces to T2 of the completed PoC roadmap and enables CA-2, CA-3, CA-4, CA-5, CA-13. This spec performs zero disk I/O; scenarios are fixture-free. `add-client-version-freshness` (2026-08-24) grew the type enumeration by six: `Freshness`, `FreshnessSubject`, `FreshnessCheck`, `FreshnessReport`, `ClientInstallSlot`, and `FreshnessSettings`, plus the modified `ClientPresence` (gains `slot: ClientInstallSlot`).

## Requirements

### Requirement: Component Identity Is Deterministic, Not Content-Based

`Component.id` MUST be derived deterministically from `(kind, normalized name)` alone. The identity function MUST NOT incorporate `Location` data or file content in any form (including hashing). Normalization MUST apply Unicode NFC normalization to `name`, followed by Unicode default case folding (not ASCII-only lowercasing); this exact order is normative pending confirmation in `design.md`.

#### Scenario: Case variants collapse to one identity

- GIVEN two names `"Issue-Creation"` and `"issue-creation"` with the same `ComponentKind`
- WHEN each is passed through the id derivation function
- THEN both produce the identical `Component.id`

#### Scenario: Same kind and name always yield equal ids

- GIVEN the same `(kind, name)` pair derived twice, in separate calls
- WHEN both derivations run
- THEN the resulting ids are equal

#### Scenario: Different kind, same name, different identity

- GIVEN name `"triage"` with `ComponentKind::Skill` and the same name with `ComponentKind::Agent`
- WHEN each is passed through the id derivation function
- THEN the resulting ids differ

### Requirement: Location Path Is Optional and Distinguishable

`Location.path` MUST be `Option<PathBuf>`. A `Component` whose `Location.path` is `None` MUST be representable and MUST remain distinguishable from one where `path` is `Some(..)`.

#### Scenario: Pathless location is representable

- GIVEN a `Location` constructed with `path: None`
- WHEN it is inspected
- THEN it is a valid `Location` value, not an error or a required-field violation

#### Scenario: Present and absent paths are distinguishable

- GIVEN one `Location` with `path: None` and another with `path: Some(p)`
- WHEN both are compared
- THEN they are unequal, and each preserves its own path state independently

### Requirement: SearchRoot Distinguishes Absent From Present

`SearchRoot` (or an adjacent type it references) MUST be able to represent that a root path was resolved and looked for on disk but does not exist, distinguishable from a root that exists and produced zero components, and from a root that exists and produced one or more components. This state MUST be reportable by path alone; `SearchRoot` MUST NOT carry a client display name or label — client identification for UI purposes is derived elsewhere from `SearchRootKind`.

#### Scenario: An absent root is representable without a display label

- GIVEN a root path that does not exist on disk
- WHEN a `SearchRoot` value is constructed for it
- THEN it is a valid value reporting that path as not found, with no client-name field required or present

#### Scenario: Absent and present-and-empty are distinguishable values

- GIVEN one `SearchRoot` for a path that does not exist and another for a path that exists with zero entries
- WHEN both are compared
- THEN they are unequal and each preserves its own found/not-found state

#### Scenario: Existing SearchRoot fields are unaffected

- GIVEN a `SearchRoot` for a root that exists and produced components
- WHEN its `id`, `path`, and `kind` fields are inspected
- THEN they hold the same values and types as before this change

### Requirement: Component Holds Multiple Locations As One Entity

`Component` MUST hold `locations: Vec<Location>` rather than a single path field, so a component discovered under N search roots is one entity carrying N `Location` entries under one shared `id`.

#### Scenario: One component, multiple locations

- GIVEN a `Component` built with two `Location` values sharing the same derived id
- WHEN `locations` is inspected
- THEN `locations.len()` is 2 and `Component.id` remains a single value

### Requirement: Scope Is Always Populated

`Component.scope` MUST be a non-optional field of type `Scope`, set explicitly on every constructed `Component`. `Scope` MUST be a closed enum with exactly the variants `User`, `Project`, `Local`, and no `#[non_exhaustive]` attribute. The PoC MUST only construct `Scope::User`.

#### Scenario: Scope is set on construction

- GIVEN a `Component` constructed with `scope: Scope::User`
- WHEN the field is inspected
- THEN it holds `Scope::User`, never an absent or default-omitted value

#### Scenario: Scope enum is exhaustively matchable

- GIVEN a `match` over a `Scope` value covering `User`, `Project`, and `Local`
- WHEN the code is compiled with no wildcard arm
- THEN compilation succeeds, proving the enum is closed

### Requirement: ComponentKind Is a Closed Enumeration

`ComponentKind` MUST be a closed enum (no `#[non_exhaustive]`) admitting exactly the PoC-defined variants (`Skill`, `Agent`).

#### Scenario: ComponentKind is exhaustively matchable

- GIVEN a `match` over every `ComponentKind` variant with no wildcard arm
- WHEN the code is compiled
- THEN compilation succeeds

### Requirement: ScanIssue Severity Has Two Non-Aborting Levels

`ScanIssue.severity` MUST be an enum with exactly two variants: `Warning` and `Error`. Neither severity level MUST abort the scan; every `ScanIssue`, of either severity, is accumulated into `ScanReport.issues`.

#### Scenario: Warning issue is accumulated

- GIVEN a `ScanIssue` constructed with `severity: Warning`
- WHEN it is appended to a `ScanReport.issues` collection
- THEN it is present in `issues` and the report remains a valid, complete value

#### Scenario: Error issue does not abort the report

- GIVEN a `ScanIssue` constructed with `severity: Error`
- WHEN it is appended to `ScanReport.issues`
- THEN the `ScanReport` remains constructible and complete; no panic or early return is triggered by the presence of an `Error`-severity issue

### Requirement: Empty Scan Result Is Not an Error

`ScanReport` MUST accept empty `components`, `installations`, and `issues` collections as a legitimate, non-error value. `Err` is reserved exclusively for orchestration-level failure (the scan could not run at all), never for "the scan ran and found nothing."

#### Scenario: Empty ScanReport is a valid value

- GIVEN a `ScanReport` constructed with empty `components`, `installations`, and `issues` vectors and `roots_scanned: 0`
- WHEN it is serialized and deserialized
- THEN it round-trips without error and represents a complete, non-error result

### Requirement: provenance_hint Is Opaque, Not a Discriminator

`Component.provenance_hint` MUST be typed as `Option<String>`. Consumers MUST NOT branch on its value to drive behavior; any machine-readable classification of a location's origin MUST live on `Location.origin` instead.

The absence of a provenance hint MUST be represented as `None`, never as an empty `String`. An empty string is a sentinel value, and the model already rejects sentinels elsewhere (`Location.path` uses `Option` for the same reason).

#### Scenario: provenance_hint is an optional plain string, not an enum

- GIVEN the `Component.provenance_hint` field definition
- WHEN its type is inspected
- THEN it is `Option<String>`, not an enum or tagged union, signaling it is display-only

#### Scenario: a component with no provenance hint is representable without a sentinel

- GIVEN a `Component` whose adapter reported no provenance information
- WHEN it is constructed with `provenance_hint: None`
- THEN it is distinguishable from a component carrying `Some("")` and from one carrying `Some("claude-code")`

### Requirement: ClientKind Is A Closed Enumeration Admitting Three Named Clients

`ClientKind` MUST be a closed enum (no `#[non_exhaustive]`) admitting exactly
the variants `ClaudeCode`, `OpenCode`, and `Codex`. Adding `Codex` MUST NOT
require or introduce a `#[non_exhaustive]` attribute, and every existing
exhaustive `match` over `ClientKind` in core MUST be updated to cover the new
variant rather than gaining a wildcard arm.

#### Scenario: ClientKind is exhaustively matchable

- GIVEN a `match` over every `ClientKind` variant with no wildcard arm
- WHEN the code is compiled
- THEN compilation succeeds, proving the enum remains closed

#### Scenario: ClientInstallation and ClientPresence can carry Codex

- GIVEN a `ClientInstallation` or `ClientPresence` value constructed with `client: ClientKind::Codex`
- WHEN the value is inspected
- THEN it is a valid, representable value, structurally identical in shape to one carrying `ClaudeCode` or `OpenCode`

### Requirement: Client Presence Is A Typed Per-Slot Status Record

`ClientPresence` MUST be a plain-data struct in `model/` deriving `Serialize`, `Deserialize`, and `TS`, respecting the module's import allow-list (no I/O, no clock). It MUST carry exactly: `label: String` (the untranslated slot name, e.g. `"OpenCode (npm)"`, unique within a report), `probed_paths: Vec<PathBuf>` (every path probed for this slot, in deterministic order, non-empty by construction; carried but not displayed), `status: ClientPresenceStatus`, and `installations: Vec<ClientInstallation>`. `installations` MUST be a `Vec`, never an `Option` or a single value, so a slot resolving to more than one installation (CA-7: coexisting bundled versions) stays fully representable without merging or reduction. `ClientPresenceStatus` MUST be a closed enum with exactly two variants, `Detected` and `NotDetected`, no `#[non_exhaustive]`.

`ClientPresenceStatus::Detected` MUST mean "a candidate root for this slot exists on disk", NOT "a usable version was extracted". A record with `status: Detected` and an empty `installations` (a present-but-broken slot, e.g. an unreadable `package.json`) MUST be representable and MUST NOT be conflated with `NotDetected`.

`ScanReport` MUST gain a field `client_presence: Option<Vec<ClientPresence>>`. `None` MUST mean the current platform has no probe table at all (client detection was not attempted); `Some(vec)` MUST mean the platform was probed, with one `ClientPresence` per defined slot. `None` and `Some(vec![])` are distinct and MUST NOT be used interchangeably: an empty `Vec` under `Some` is not a meaning this field is expected to produce today, since every defined platform's probe table is non-empty by construction, but the type MUST NOT forbid it.

This field MUST NOT replace or narrow `ScanReport.installations`, which is unchanged and remains the flat list of every detected `ClientInstallation` across all slots.

#### Scenario: A presence record with multiple installations stays a collection

- GIVEN a `ClientPresence` value constructed with two `ClientInstallation` entries in `installations`
- WHEN the value is inspected
- THEN `installations.len()` is 2 and neither entry is dropped or merged

#### Scenario: Detected does not require a resolved installation

- GIVEN a `ClientPresence` value with `status: ClientPresenceStatus::Detected` and an empty `installations`
- WHEN the value is inspected
- THEN it is a valid, representable value distinct from `status: NotDetected`

#### Scenario: A not-detected slot has an empty installations collection

- GIVEN a `ClientPresence` value with `status: ClientPresenceStatus::NotDetected`
- WHEN `installations` is inspected
- THEN it is an empty `Vec`, never a placeholder entry

#### Scenario: No probe table is represented as None, not an empty Vec

- GIVEN a `ScanReport` built for a platform with no probe table
- WHEN `client_presence` is inspected
- THEN it is `None`, distinguishable from `Some(vec![])`

#### Scenario: ClientPresenceStatus is exhaustively matchable

- GIVEN a `match` over every `ClientPresenceStatus` variant with no wildcard arm
- WHEN the code is compiled
- THEN compilation succeeds, proving the enum is closed

### Requirement: Rust Types Generate a Matching TypeScript Contract

All core types — `Component`, `ComponentKind`, `Scope`, `Location`, `SearchRoot`, `ClientInstallation`, `ClientPresence`, `ClientPresenceStatus`, `ScanIssue`, `ScanReport`, `ClientKind`, `Freshness`, `FreshnessSubject`, `FreshnessCheck`, `FreshnessReport`, `ClientInstallSlot`, and `FreshnessSettings` — and their nested enums MUST derive `Serialize`, `Deserialize`, and `TS`. Generated TypeScript bindings MUST be checked into `frontend/src/bindings/` and MUST structurally mirror the Rust definitions: field names, optionality, and closed union variants. Every new or modified `.ts` binding, including `ClientPresence.ts`'s new `slot` field, MUST land in the same commit as its Rust type change; CI's drift gate MUST fail if it is not regenerated.

#### Scenario: Struct exports a TypeScript binding

- GIVEN `Component` derives `TS` and its export test runs
- WHEN the generated `.ts` file is inspected
- THEN it declares a type with the same field names as the Rust struct

#### Scenario: Optional path crosses as a nullable string

- GIVEN `Location.path: Option<PathBuf>`
- WHEN its TypeScript binding is generated
- THEN the field's type is `string | null`

#### Scenario: ClientKind's binding reflects three variants

- GIVEN `ClientKind` gains the `Codex` variant and its export test runs
- WHEN `frontend/src/bindings/ClientKind.ts` is inspected
- THEN it declares the union `"claudeCode" | "openCode" | "codex"`, and the CI bindings-drift gate reports no diff between the committed file and a freshly regenerated one

#### Scenario: The new presence types export their own bindings

- GIVEN `ClientPresence` and `ClientPresenceStatus` derive `TS` and their export tests run
- WHEN `frontend/src/bindings/ClientPresence.ts` and `frontend/src/bindings/ClientPresenceStatus.ts` are inspected
- THEN `ClientPresence.ts` declares `label: string`, `probedPaths: string[]`, `status: ClientPresenceStatus`, `slot: ClientInstallSlot`, and `installations: ClientInstallation[]`
- AND `ClientPresenceStatus.ts` declares the union `"detected" | "notDetected"`

#### Scenario: ScanReport's new field is optional at the binding boundary

- GIVEN `ScanReport.client_presence: Option<Vec<ClientPresence>>`
- WHEN its TypeScript binding is generated
- THEN the field's type is `ClientPresence[] | null`

#### Scenario: The Freshness verdict exports a closed union binding

- GIVEN `Freshness` derives `TS` and its export test runs
- WHEN its generated `.ts` binding is inspected
- THEN it declares a closed union with exactly three variants, structurally mirroring `UpToDate`, `Outdated { latest }`, and `Unknown { reason }`

#### Scenario: FreshnessSubject, FreshnessCheck, and FreshnessReport export their own bindings

- GIVEN `FreshnessSubject`, `FreshnessCheck`, and `FreshnessReport` derive `TS` and their export tests run
- WHEN the corresponding generated `.ts` files are inspected
- THEN each field name and each closed enum variant in the Rust source has a structurally matching entry in the binding
- AND `FreshnessSubject`'s only populated variant identifies a client installation by slot and path; any other declared variant carries no producer yet
- AND `FreshnessReport.enabled` crosses as a non-optional boolean, distinguishable from a report whose `checks` are all `Unknown`

#### Scenario: ClientInstallSlot exports a closed union binding

- GIVEN `ClientInstallSlot` derives `TS` and its export test runs
- WHEN `frontend/src/bindings/ClientInstallSlot.ts` is inspected
- THEN it declares a closed union with one variant per probe slot this capability's detector defines

#### Scenario: ClientPresence's binding gains the slot field

- GIVEN `ClientPresence` gains `pub slot: ClientInstallSlot`
- WHEN `frontend/src/bindings/ClientPresence.ts` is regenerated
- THEN it declares a `slot: ClientInstallSlot` field alongside its existing `label`, `probedPaths`, `status`, and `installations` fields

#### Scenario: A forgotten binding fails the CI drift gate

- GIVEN a new or modified public model type from this change is committed without its regenerated `.ts` binding
- WHEN CI's bindings-drift check runs
- THEN it fails, using `git add --intent-to-add` so a brand-new file is also caught
