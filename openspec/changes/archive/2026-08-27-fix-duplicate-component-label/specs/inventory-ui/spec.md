# Delta for inventory-ui

## MODIFIED Requirements

### Requirement: Duplicate Rows and Complete Paths

The UI MUST mark a component as duplicated only when the same AI client can
consume both a shared-root copy and that client’s specific-root copy. Copies
that exist only across distinct client-specific roots MUST NOT be marked as
duplicated. The UI MUST disclose every location path, including nullable paths,
and MUST NOT regroup components by name or compare file contents.
(Previously: any component with `locations.length > 1` was marked duplicated.)

#### Scenario: Shared plus consuming client-specific copy is duplicated

- GIVEN a component has at least one shared location and at least one location
  for the same client
- WHEN its row is rendered
- THEN the row shows a duplicate mark
- AND all location entries remain visible

#### Scenario: Distinct client-specific copies are not duplicates

- GIVEN a component has only client-specific locations for different clients
- WHEN its row is rendered
- THEN the row does not show a duplicate mark
- AND all location entries remain visible

#### Scenario: Nullable location path remains renderable

- GIVEN a component contains a location whose path is null
- WHEN its locations are rendered
- THEN the row remains renderable
- AND the null path is represented safely without inventing a filesystem action

