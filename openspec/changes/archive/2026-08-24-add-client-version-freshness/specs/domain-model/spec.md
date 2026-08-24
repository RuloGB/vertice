# Delta for Domain Model

`add-client-version-freshness` introduces six new plain-data types in `model/` — `Freshness` (the verdict), `FreshnessSubject` (a closed, data-carrying discriminator identifying what a verdict is about), `FreshnessCheck` (one subject's installed version paired with its verdict), `FreshnessReport` (the collection returned to the frontend, plus an `enabled` flag), `ClientInstallSlot` (the slot discriminator promoted from `client-installation-detector`'s previously-private type), and `FreshnessSettings` (the persisted opt-out and disclosure-seen state, required by the confirmed default posture) — and modifies one existing type, `ClientPresence`, which gains a `slot: ClientInstallSlot` field. `FreshnessSubject`'s sole populated variant for this change identifies a client installation by `(slot: ClientInstallSlot, path)`. All seven follow the exact pattern `ClientPresence`/`ClientPresenceStatus` already established: plain data, `Serialize`/`Deserialize`/`TS`-derived, within `model/`'s import allow-list (no I/O, no clock). The behavioral contract for these types (comparison totality, degradation, upstream mapping) lives in the new `component-freshness` capability, not here; the slot discriminator's behavioral contract (byte-identical detection, no label-keying) lives in `client-installation-detector`'s delta.

## MODIFIED Requirements

### Requirement: Rust Types Generate a Matching TypeScript Contract

All core types — `Component`, `ComponentKind`, `Scope`, `Location`, `SearchRoot`, `ClientInstallation`, `ClientPresence`, `ClientPresenceStatus`, `ScanIssue`, `ScanReport`, `ClientKind`, `Freshness`, `FreshnessSubject`, `FreshnessCheck`, `FreshnessReport`, `ClientInstallSlot`, and `FreshnessSettings` — and their nested enums MUST derive `Serialize`, `Deserialize`, and `TS`. Generated TypeScript bindings MUST be checked into `frontend/src/bindings/` and MUST structurally mirror the Rust definitions: field names, optionality, and closed union variants. Every new or modified `.ts` binding, including `ClientPresence.ts`'s new `slot` field, MUST land in the same commit as its Rust type change; CI's drift gate MUST fail if it is not regenerated.
(Previously: enumerated ten core types plus `ClientKind`; the freshness types and the `ClientInstallSlot`/`ClientPresence.slot` change did not exist.)

#### Scenario: Struct exports a TypeScript binding

- GIVEN `Component` derives `TS` and its export test runs
- WHEN the generated `.ts` file is inspected
- THEN it declares a type with the same field names as the Rust struct

#### Scenario: Optional path crosses as a nullable string

- GIVEN `Location.path: Option<PathBuf>`
- WHEN its TypeScript binding is generated
- THEN the field's type is `string | null`

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
