# Delta for Workspace Architecture

The "one module owns the parser" seam inventory grows from two seams
(`yaml.rs`, `jsonc.rs`) to three, with a new `toml.rs` seam added for Codex
agent parsing. This is the change's only new supply-chain cost: one new Rust
dependency, a TOML parser, contained by the same discipline the existing two
seams already enforce.

## ADDED Requirements

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
