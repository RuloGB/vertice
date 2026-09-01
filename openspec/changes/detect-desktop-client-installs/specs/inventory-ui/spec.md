# Delta for Inventory UI

## ADDED Requirements

### Requirement: ClientsPage Selects The First Detected Slot Among A Product's Group

For a product group of N `ClientPresence` slots, `ClientsPage` MUST select
the first record with `status: Detected`, in the group's defined slot
order, to drive that card's detected badge, version string, and freshness
badge, never `Array.find`'s first-match-by-slot-order-alone semantics. When
no slot in the group is `Detected`, `ClientsPage` MUST select the group's
first record and render the card as not detected. The detected badge, the
version string, and the freshness badge MUST all be read from the same
selected record, so they can never disagree.

#### Scenario: A later slot detected while an earlier slot is not renders as detected

- GIVEN a product group whose first slot is `NotDetected` and whose second slot is `Detected` with version `"1.5.0"`
- WHEN the card is rendered
- THEN the card shows the detected badge, version `"1.5.0"`, and a freshness badge evaluated against the detected slot's record
- AND none of the group's other, non-selected records influence the render

#### Scenario: No slot detected renders as not detected

- GIVEN a product group where every slot's `ClientPresence` record has `status: NotDetected`
- WHEN the card is rendered
- THEN the card shows the not-detected state, driven by the group's first record

#### Scenario: The rule holds for a group of three slots, not just two

- GIVEN a product group of three slots where the first two are `NotDetected` and the third is `Detected`
- WHEN the card is rendered
- THEN the card shows the third slot's detected status, version, and freshness badge, proving the selection rule generalizes to any N, not only two

#### Scenario: OpenCode's card is driven by the same rule across its two slots

- GIVEN the OpenCode group with `openCodeNpm: NotDetected` and `openCodeDesktop: Detected`
- WHEN the OpenCode card is rendered
- THEN it shows the detected badge and the desktop slot's version, exactly as the Claude Code group does for its own two slots
