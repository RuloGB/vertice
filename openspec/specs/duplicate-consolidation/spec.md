# Duplicate Consolidation Specification

## Purpose

Defines the contract for `consolidate(Vec<Component>) -> Vec<Component>`, a pure post-scan transform that groups scanned components by derived identity, merges their locations without loss, resolves divergent display fields by first-non-empty precedence, and returns a deterministically ordered result. Traces to **T8** of the completed PoC roadmap; closes **CA-2** (25 not 69), **CA-3** (22 duplicated with three paths), **CA-4** (3 without a duplicate mark), **CA-8** (`_shared` consolidated with no name filtering). Bound by **CA-16** (read-only) and **CA-17** (versioned fixtures only). Core (Rust) only — no IPC or frontend surface.

## Requirements

### Requirement: Consolidation Is a Pure Function With No I/O

`consolidate` MUST perform no filesystem read or write, no environment read, and no clock read. It MUST live in `crates/vertice-core/src/consolidate.rs` and MUST NOT import `std::fs`, `std::io`, `std::env`, `SystemTime`, or `Instant`.

#### Scenario: Module contains no I/O primitives

- GIVEN the source of `consolidate.rs`
- WHEN it is inspected
- THEN it contains no `std::fs`, `std::io`, `std::env`, or clock use

### Requirement: Grouping Key Is the Existing Derived Identity

Components MUST be grouped by `Component.id` exactly as produced by `ComponentId::derive(kind, name)`. No second normalization rule, hashing, or content comparison MUST be introduced.

#### Scenario: Reference fixture collapses 69 inputs into 25 components

- GIVEN the flattened skill scan of `crates/vertice-core/tests/fixtures/roots/reference/` (69 entries)
- WHEN `consolidate` runs over them
- THEN the result contains exactly 25 components

#### Scenario: Case and NFC/NFD name variants collapse to one component

- GIVEN two components whose names differ only by case or by NFC/NFD normalization form
- WHEN `consolidate` runs
- THEN both collapse into a single component, via `ComponentId::derive` alone

#### Scenario: A skill and an agent sharing a name are not merged

- GIVEN a skill component and an agent component with the same display name
- WHEN `consolidate` runs
- THEN they remain two separate components, since `kind` is part of the identity

### Requirement: No Name-Convention Filtering

`consolidate` MUST NOT apply any name-prefix or name-convention rule to include, exclude, or treat any component differently.

#### Scenario: `_shared` consolidates like any other name

- GIVEN three `_shared` components, one per root
- WHEN `consolidate` runs
- THEN they merge into one component with three locations, exactly as any other duplicated name

### Requirement: Every Input Location Is Preserved

For a group of N components sharing an identity, the merged component's `locations` MUST contain exactly the union of all N inputs' locations, in canonical root order. No location MUST be dropped, deduplicated, or elected as a winner.

#### Scenario: Exact location-count distribution over the reference fixture

- GIVEN the reference fixture's 69 flattened entries
- WHEN `consolidate` runs
- THEN exactly 22 components have `locations.len() == 3`, exactly 3 have `locations.len() == 1`, and none has `locations.len() == 2`

#### Scenario: Total location count is conserved

- GIVEN any input `Vec<Component>`
- WHEN `consolidate` runs
- THEN the sum of `locations.len()` across the output equals the input length

#### Scenario: Locations within a component follow canonical root order

- GIVEN a component with copies in more than one root
- WHEN its `locations` are inspected
- THEN they appear in the same order as `roots::skill_roots()` / `roots::agent_roots()`, independent of scan or input order

### Requirement: First-Non-Empty Field Precedence

For `name`, `description`, `provenance_hint`, and `scope`, the merged component MUST take the first present and non-empty value found while walking roots in canonical `roots.rs` order — never simply the first root's value regardless of content, and never dependent on input arrival order.

"Non-empty" MUST mean `trim().is_empty() == false`, not `!= ""`. The frontmatter seam performs no trimming, so an empty folded block scalar (`description: >`) arrives as `Some("\n")` — present, not equal to the empty string, and still blank to a reader. A narrower emptiness test would let that value win precedence over a real description in a later root.

#### Scenario: A later root's non-empty description wins over an earlier root's empty one

- GIVEN a duplicated component whose first-root copy has an absent or empty `description` and whose later-root copy has a non-empty one
- WHEN `consolidate` runs
- THEN the merged component's `description` is the later root's value

#### Scenario: A whitespace-only description does not win precedence

- GIVEN a duplicated component whose first-root copy has `description` `Some("\n")` from an empty folded block scalar and whose later-root copy has a real description
- WHEN `consolidate` runs
- THEN the merged component's `description` is the later root's value, not `Some("\n")`

#### Scenario: Precedence is independent of input arrival order

- GIVEN the same set of duplicate copies fed in two different (shuffled) input orders
- WHEN `consolidate` runs on each
- THEN both runs produce identical merged field values

### Requirement: Duplication Is Derived, Not Stored

The consolidated model MUST introduce no new field. Whether a component is duplicated MUST be derivable as `locations.len() > 1`. `crates/vertice-core/src/model/` and `frontend/src/bindings/` MUST remain byte-identical to their pre-change state.

#### Scenario: A single-location component is not marked as duplicated

- GIVEN a component with exactly one location after consolidation
- WHEN its duplication status is derived
- THEN `locations.len() > 1` evaluates to `false`, using no additional field

### Requirement: Deterministic Output Ordering

The returned component list MUST be sorted by display `name`, using `ComponentId` as the tiebreak for equal names. Ordering MUST be stable and identical across Linux, Windows, and macOS.

#### Scenario: Two components sharing a display name are ordered by identity

- GIVEN a skill and an agent that share the same display name
- WHEN `consolidate` returns its result
- THEN the two are ordered by their `ComponentId` value, not left in arrival order

#### Scenario: Output order is stable across shuffled inputs

- GIVEN the same input components fed in two different orders
- WHEN `consolidate` runs on each
- THEN both outputs are identical, including component order

### Requirement: Edge Cases Are Explicit

`consolidate` MUST handle empty and single-element inputs without special-casing that violates the rules above.

#### Scenario: Empty input yields empty output

- GIVEN an empty `Vec<Component>`
- WHEN `consolidate` runs
- THEN it returns an empty `Vec<Component>`

#### Scenario: Single-component input is passed through with one location

- GIVEN a single input component
- WHEN `consolidate` runs
- THEN the output contains that one component with `locations.len() == 1`

### Requirement: Content Comparison Is Out of Scope

`consolidate` MUST NOT compare file contents, hash bytes, or distinguish identical from divergent duplicates. `locations.len() > 1` is the sole duplicate signal.

#### Scenario: Divergent duplicate copies still merge without a content diff

- GIVEN two copies of the same identity with different `description` values
- WHEN `consolidate` runs
- THEN they merge into one component via field precedence alone, with no comparison of the copies' content bytes
