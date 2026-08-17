# Delta for CI Quality Gates

## ADDED Requirements

### Requirement: Generated TypeScript Contract In-Sync Gate

The ubuntu-only `quality` CI job MUST verify that the checked-in generated TypeScript bindings in `frontend/src/bindings/` are in sync with the Rust domain model source in `crates/vertice-core/src/model/`. Verification MUST regenerate the bindings and fail the job on any difference from what is committed.

The check MUST detect a **newly generated, never-committed** binding file, not only a modification to an existing one. A bare `git diff --exit-code` is insufficient: it ignores untracked files, so a new domain type whose binding was never committed would pass the gate silently — the exact case the gate exists to catch. The implementation MUST register regenerated files with the index first (`git add --intent-to-add`) or use an equivalent check that accounts for untracked files.

This check MUST run in the same job as `cargo deny check bans`, not as a new matrix leg, because binding generation performs zero disk I/O and is not OS-path-sensitive.

#### Scenario: In-sync bindings pass CI

- GIVEN the checked-in `frontend/src/bindings/` matches what the current Rust domain types would generate
- WHEN the `quality` job's in-sync step runs
- THEN it passes with no diff

#### Scenario: Stale bindings fail CI (negative path)

- GIVEN a Rust domain type in `crates/vertice-core/src/model/` changes without regenerating `frontend/src/bindings/`
- WHEN the `quality` job's in-sync step runs
- THEN it fails on the resulting diff, and the pull request cannot merge

#### Scenario: Check stays ubuntu-only, matrix unaffected

- GIVEN the `quality` job runs only on the ubuntu runner
- WHEN the in-sync step is added to it
- THEN the macOS, Windows, and Linux legs of the `rust` matrix job are unaffected and unchanged
