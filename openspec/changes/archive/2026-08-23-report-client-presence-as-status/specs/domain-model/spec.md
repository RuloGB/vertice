# Delta for Domain Model

This is the first `client-installation-detector` change to touch `domain-model`. `design.md` §2 of `client-installation-detection` closed absence-reporting on the `ScanIssue` carrier and explicitly named this retrofit as its own future condition once a consumer existed (T10/T11). T11 is now complete; this delta exercises that named retrofit, not a fresh decision. Field names and semantics below match `openspec/changes/report-client-presence-as-status/design.md` §2 and §4 exactly.

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Rust Types Generate a Matching TypeScript Contract

All ten core types (`Component`, `ComponentKind`, `Scope`, `Location`, `SearchRoot`, `ClientInstallation`, `ClientPresence`, `ClientPresenceStatus`, `ScanIssue`, `ScanReport`) and their nested enums MUST derive `Serialize`, `Deserialize`, and `TS`. Generated TypeScript bindings MUST be checked into `frontend/src/bindings/` and MUST structurally mirror the Rust definitions: field names, optionality, and closed union variants.
(Previously: enumerated exactly eight core types; `ClientPresence` and `ClientPresenceStatus` did not exist.)

#### Scenario: Struct exports a TypeScript binding

- GIVEN `Component` derives `TS` and its export test runs
- WHEN the generated `.ts` file is inspected
- THEN it declares a type with the same field names as the Rust struct

#### Scenario: Optional path crosses as a nullable string

- GIVEN `Location.path: Option<PathBuf>`
- WHEN its TypeScript binding is generated
- THEN the field's type is `string | null`

#### Scenario: The new presence types export their own bindings

- GIVEN `ClientPresence` and `ClientPresenceStatus` derive `TS` and their export tests run
- WHEN `frontend/src/bindings/ClientPresence.ts` and `frontend/src/bindings/ClientPresenceStatus.ts` are inspected
- THEN `ClientPresence.ts` declares `label: string`, `probedPaths: string[]`, `status: ClientPresenceStatus`, and `installations: ClientInstallation[]`
- AND `ClientPresenceStatus.ts` declares the union `"detected" | "notDetected"`

#### Scenario: ScanReport's new field is optional at the binding boundary

- GIVEN `ScanReport.client_presence: Option<Vec<ClientPresence>>`
- WHEN its TypeScript binding is generated
- THEN the field's type is `ClientPresence[] | null`

Note for archive: `client-installation-detector`'s Purpose line records "no `domain-model` requirement is added or modified by this capability" and "`domain-model` is not a Modified Capability". That sentence is superseded by this delta and MUST be corrected when merging, alongside the requirement bodies above. "ScanIssue Severity Has Two Non-Aborting Levels" is intentionally untouched by this change; no delta is written against it.
