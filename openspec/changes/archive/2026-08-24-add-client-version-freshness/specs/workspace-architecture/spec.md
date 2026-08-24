# Delta for Workspace Architecture

This is the first change whose single-owner seam is rooted in `vertice-app` rather than `vertice-core` — the reference-version fetcher joins `yaml.rs`, `jsonc.rs`, and `toml.rs` in spirit (one module owns the outside world) but lives on the other side of the crate boundary, because `vertice-core` acquires no HTTP dependency at all. This delta also restates core's containment invariant with that new dependency explicitly in view.

## ADDED Requirements

### Requirement: vertice-core Stays HTTP-Free

`vertice-core` MUST NOT depend, directly or transitively, on any HTTP client crate. `vertice-core` MUST obtain a reference version only through the trait abstraction it defines; it MUST NOT import, link, or transitively pull in an HTTP stack under any circumstance, including through a future convenience refactor. This containment MUST be structurally possible to verify, whether by an automated dependency-graph check or by an equivalent mechanical gate decided in design.

#### Scenario: Core's dependency graph contains no HTTP client crate

- GIVEN the built dependency graph of `vertice-core` after this change
- WHEN it is inspected
- THEN no direct or transitive dependency is an HTTP client crate

#### Scenario: An accidental HTTP dependency in core is structurally detectable

- GIVEN a hypothetical change adds an HTTP client dependency to `vertice-core`, directly or transitively
- WHEN the workspace's dependency containment check runs
- THEN the violation is detectable without relying on manual code review alone

### Requirement: The Reference-Version Seam Is Owned By vertice-app

The concrete reference-version fetcher — network transport, per-subject upstream resolution, response parsing, and the response cache — MUST be owned by exactly one module in `vertice-app`. `vertice-core` MUST depend only on the trait; it MUST NOT be able to construct or reference a concrete fetcher implementation. This is the first single-owner seam in the workspace whose owner lives outside `vertice-core`, and that placement MUST be documented as deliberate rather than left implicit.

#### Scenario: vertice-core has no path to a concrete fetcher

- GIVEN the source of `vertice-core`
- WHEN it is inspected for any concrete reference-source implementation
- THEN none exists; only the trait definition and its test stub are present in core

#### Scenario: vertice-app owns the sole concrete implementation

- GIVEN the source of `vertice-app`
- WHEN it is inspected for reference-version fetching code
- THEN exactly one module implements the trait, and no other module in the workspace does
