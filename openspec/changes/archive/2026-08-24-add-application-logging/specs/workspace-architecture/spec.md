# Delta for Workspace Architecture

## ADDED Requirements

### Requirement: The Logging Sink Is A Single-Owner Seam Owned By vertice-app

`vertice-core` MUST NOT acquire any logging dependency, logging module, or emission port as a
result of this change; it continues to return fully-formed typed values (`ScanReport`,
`FreshnessReport`) with no I/O side channel. The concrete logging sink — file ownership, format,
size check, and rotation — MUST be owned by exactly one module in `vertice-app`, mirroring the
existing single-owner-seam convention (`yaml.rs` owns `serde_norway`, `toml.rs` owns the TOML crate,
`freshness/cache.rs` owns the settings/cache write). No other module in the workspace MAY open,
write, or rotate the log file. Promoting the `log` facade crate — already resolved in `Cargo.lock`
transitively before this change — to a direct dependency of `vertice-app` MUST NOT introduce a
logging dependency into `vertice-core`, and `cargo deny check bans licenses` MUST continue to pass
after the promotion.

#### Scenario: vertice-core's dependency graph gains no logging crate

- GIVEN the built dependency graph of `vertice-core` after this change
- WHEN it is inspected
- THEN no logging facade or backend crate is a direct or transitive dependency introduced by this
  change

#### Scenario: Exactly one module owns the log file

- GIVEN the source of `vertice-app` after this change
- WHEN it is inspected for code that opens, writes, or rotates the log file
- THEN exactly one module does so, and no other module in the workspace performs any of those
  operations

#### Scenario: The dependency gate passes with the promoted direct dependency

- GIVEN `log` is promoted from a transitive to a direct dependency of `vertice-app`
- WHEN `cargo deny check bans licenses` runs
- THEN it passes, with `deny.toml`'s ban list unchanged
