# Delta for Domain Model

## ADDED Requirements

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
