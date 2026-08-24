# Delta for Client Installation Detector

`add-client-version-freshness` requires a machine-readable slot identity outside `vertice-core`, which this capability's private `InstallSlot` could not provide. Detection *behaviour* — probes, paths, version sources, ordering, and issues — is byte-identical to before this change; only the published *shape* of `ClientPresence` changes.

## ADDED Requirements

### Requirement: Published Presence Records Carry A Stable, Non-Display Slot Discriminator

`ClientPresence` MUST publish a machine-readable slot discriminator, distinct from and independent of `label`. This discriminator MUST be a closed enum type in `model/`, exhaustively matchable, mirroring the existing `ClientPresenceStatus`/`Scope` pattern. Every requirement or consumer that must identify which slot a `ClientPresence` record describes MUST key on this discriminator, and MUST NOT key on `ClientPresence.label`, `ClientInstallation.client`, or any other string-matching or prose-parsing technique. `label` remains display-only copy — it has already been reworded once by a prior change — and MUST NOT be treated as a stable identity.

This requirement changes only the published shape of `ClientPresence`. Every probe path, version-extraction source, ordering guarantee, and issue-emission rule already specified in this capability's other requirements is unchanged by it — no probe, path, version source, or issue behavior is added, removed, or altered.

#### Scenario: The slot discriminator is exhaustively matchable

- GIVEN a `match` over every variant of the slot discriminator type with no wildcard arm
- WHEN the code is compiled
- THEN compilation succeeds, proving the enum is closed

#### Scenario: Two slots with identical labels remain distinguishable by discriminator

- GIVEN a hypothetical future revision to `label` wording that could produce ambiguity if strings were compared
- WHEN two `ClientPresence` records are compared by their slot discriminator instead of their label
- THEN the comparison is unambiguous and independent of the current wording of either record's `label`

#### Scenario: Detection behavior is unchanged by the new field

- GIVEN the existing fixture suite for probes, version extraction, and issue emission
- WHEN the scanner runs after this change
- THEN every `ClientInstallation`, `ScanIssue`, and ordering outcome is byte-identical to the pre-change result, with only the added slot discriminator present on each `ClientPresence` record

#### Scenario: A consumer resolving upstream identity dispatches on the discriminator, not on label text

- GIVEN a consumer that must determine which upstream registry or repository a `ClientPresence` record corresponds to
- WHEN that resolution is implemented
- THEN it dispatches on the slot discriminator value
- AND it does not parse, match, or compare against the record's `label` string to make that determination
