# Delta for duplicate-consolidation

## MODIFIED Requirements

### Requirement: Duplication Is Derived, Not Stored

The consolidated model MUST introduce no new field. Whether a component is
duplicated for technical consolidation purposes MUST be derivable as
`locations.len() > 1`. This requirement defines consolidation output only and
MUST NOT be read as the frontend duplicate-badge contract. `crates/vertice-core/src/model/`
and `frontend/src/bindings/` MUST remain byte-identical to their pre-change
state.
(Previously: duplication was implicitly treated as the UI duplicate signal too.)

#### Scenario: A single-location component is not marked as duplicated

- GIVEN a component with exactly one location after consolidation
- WHEN its duplication status is derived
- THEN `locations.len() > 1` evaluates to `false`
- AND no additional field is introduced

#### Scenario: Consolidation output still preserves technical aggregation

- GIVEN multiple copies of the same identity across roots
- WHEN `consolidate` runs
- THEN the merged component still carries all locations
- AND the output remains a pure technical aggregation result

