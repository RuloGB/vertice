# Workspace Architecture Specification

## Purpose

Defines the Cargo workspace layout and the structural invariants that keep `vertice-core` reusable as a headless library (enabling a future CLI binary without a rewrite), plus the MSRV floor, the YAML serialization crate decision, and the broader "one module owns the parser" seam convention that all later adapters depend on. Traces to T1 of `internal-docs/plan-desarrollo-poc.md` and to stack decision #5 (core stays Tauri-agnostic); the seam inventory was extended to a third seam, `toml.rs`, by T7's `add-codex-client-support` (2026-08-23).

## Requirements

### Requirement: Two-Crate Workspace Layout

The repository MUST be a Cargo workspace containing exactly two member crates: `crates/vertice-core` (library) and `crates/vertice-app` (Tauri 2 binary). `vertice-core` MUST contain no UI or IPC code; `vertice-app` MUST depend on `vertice-core` as a path dependency and MUST own the Tauri runtime, commands, and the bundled Svelte 5 + Vite + Tailwind frontend.

#### Scenario: Workspace resolves with two members

- GIVEN the repository root `Cargo.toml`
- WHEN `cargo metadata` is run
- THEN exactly two workspace members are reported: `vertice-core` and `vertice-app`

#### Scenario: Shared package metadata

- GIVEN the workspace `Cargo.toml` defines `[workspace.package]`
- WHEN either crate's manifest is inspected
- THEN it inherits edition, version, and license from `[workspace.package]` rather than duplicating them

### Requirement: Core Purity Invariant

`vertice-core` MUST NOT depend, directly or transitively, on the `tauri` crate or any `tauri-*` crate. This MUST be verified mechanically in CI, not by code review alone.

#### Scenario: Dependency graph contains no Tauri crates

- GIVEN the built dependency graph of `vertice-core`
- WHEN `cargo tree -p vertice-core` runs
- THEN no line matches `tauri` or `tauri-*`
- AND a CI job fails the build if any such dependency is introduced

#### Scenario: Accidental Tauri import is caught before merge

- GIVEN a pull request adds a `tauri` dependency (direct or transitive) to `vertice-core`
- WHEN the CI purity-check job runs
- THEN the job fails and the pull request cannot merge

### Requirement: MSRV Pinned and Enforced

The workspace MUST declare a Minimum Supported Rust Version (MSRV) in the root `Cargo.toml` (`rust-version` field) and a `rust-toolchain.toml` pinning the same version for local builds. CI MUST run a dedicated job that builds and tests against the declared MSRV, independent of the job(s) using the latest stable toolchain.

#### Scenario: MSRV declared and consistent

- GIVEN the root `Cargo.toml` (`rust-version`, the MSRV floor) and `rust-toolchain.toml` (`channel`, the pinned dev/CI toolchain)
- WHEN both files are inspected
- THEN `rust-toolchain.toml`'s `channel` is a version equal to or newer than `Cargo.toml`'s `rust-version` (a floor relationship, not an exact match — the toolchain pin may be newer than the MSRV floor it enforces)
- AND a CI consistency check fails the build if the toolchain channel is older than the declared MSRV

#### Scenario: MSRV violation fails CI

- GIVEN a change introduces a language feature or dependency requiring a Rust version newer than the declared MSRV
- WHEN the MSRV CI job runs
- THEN the build fails and the pull request cannot merge

### Requirement: YAML Serialization Crate Decision Recorded

A YAML serialization crate MUST be selected for `vertice-core` at T1 (not deferred to a later phase) to replace the archived `serde_yaml`. The decision MUST be documented with: the crates evaluated (`serde_norway`, `serde_yaml_ng`, `serde_yml`, `yaml-rust2`), current maintenance activity for each, whether each supports YAML block scalars (`description: >`), and whether each integrates with `serde` derive macros. The chosen crate MUST be added as a `vertice-core` dependency; rejected alternatives and the reason for rejection MUST be recorded.

#### Scenario: Decision documented with justification

- GIVEN the T1 design/decision artifact
- WHEN it is reviewed
- THEN it lists all four evaluated crates with maintenance status, block-scalar support, and serde integration for each
- AND it states which crate was selected and why the other three were rejected

#### Scenario: Block-scalar parsing verified before selection

- GIVEN a YAML fixture using a block scalar (`description: >`)
- WHEN the selected crate deserializes the fixture via `serde`
- THEN the multi-line value is parsed correctly with no data loss

### Requirement: A Third Parser Seam, toml.rs, Is Contained And MSRV-Compatible

`vertice-core` MUST add exactly one new parser-owning seam module, `toml.rs`,
mirroring the existing `yaml.rs`/`jsonc.rs` convention: it MUST be the
crate's sole importer of the TOML parsing crate, and every other module MUST
reach TOML parsing exclusively through that seam's exported entry point
(`toml::from_str`), never by importing the TOML crate directly. A dedicated
test, `tests/toml_seam_invariant.rs`, MUST pin this containment textually,
mirroring `tests/yaml_seam_invariant.rs`'s existing containment check for
`serde_norway`.

The selected TOML crate's declared Minimum Supported Rust Version MUST NOT
exceed the workspace's declared MSRV floor (`Cargo.toml`'s `rust-version`).
If a candidate crate's MSRV exceeds the floor, it MUST NOT be pinned; a
different TOML crate MUST be selected instead. The workspace MSRV floor is
not renegotiable to accommodate a dependency choice, and `Cargo.toml`
`rust-version`, the CI `MSRV` environment variable, and `rust-toolchain.toml`
`channel` MUST continue to agree after this dependency is added.

The selected TOML crate MUST already fall within `deny.toml`'s existing
license allow-list (MIT or Apache-2.0, dual-licensed or either); `deny.toml`
itself MUST remain unchanged by this addition, and `cargo deny check bans
licenses` MUST continue to pass.

#### Scenario: toml.rs is the sole importer of the TOML crate

- GIVEN the source of every module under `crates/vertice-core/src/` other than `toml.rs`
- WHEN it is inspected for imports of the TOML parsing crate
- THEN no module other than `toml.rs` imports it directly

#### Scenario: The seam invariant test pins containment textually

- GIVEN `tests/toml_seam_invariant.rs`
- WHEN it runs
- THEN it fails if any module other than `toml.rs` is found to import the TOML crate

#### Scenario: An MSRV-incompatible TOML crate is rejected before it is pinned

- GIVEN a candidate TOML crate whose declared MSRV exceeds the workspace's declared floor
- WHEN it is evaluated for selection
- THEN it is rejected, and a different candidate meeting the floor is selected instead, before any code depends on it

#### Scenario: The new dependency requires no deny.toml edit

- GIVEN the selected TOML crate's license
- WHEN `cargo deny check bans licenses` runs after the dependency is added
- THEN it passes with `deny.toml` byte-identical to its pre-change state
