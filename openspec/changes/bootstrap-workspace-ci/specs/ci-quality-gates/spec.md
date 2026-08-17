# CI Quality Gates Specification

## Purpose

Defines the cross-platform continuous integration gates that must pass before any change merges. Traces to T1 of `internal-docs/plan-desarrollo-poc.md` and to CA-17 (core tests pass on versioned fixtures across the three CI platforms).

## Requirements

### Requirement: Cross-Platform CI Matrix

CI MUST run on a matrix of macOS, Windows, and Linux runners on every pull request and on every push to `main`. Each matrix leg MUST run independently; a failure on one platform MUST NOT be masked by success on another.

#### Scenario: Matrix triggers on pull request

- GIVEN a pull request is opened or updated against `main`
- WHEN the CI workflow evaluates its triggers
- THEN it runs the full matrix on macOS, Windows, and Linux

#### Scenario: Matrix triggers on push to main

- GIVEN a commit is pushed directly to `main`
- WHEN the CI workflow evaluates its triggers
- THEN it runs the full matrix on macOS, Windows, and Linux

#### Scenario: One platform failure blocks merge

- GIVEN the Linux leg fails while macOS and Windows pass
- WHEN the pull request's required checks are evaluated
- THEN the pull request cannot merge

### Requirement: Formatting Gate

Every CI run MUST execute `cargo fmt --check` and fail if any Rust source file is not formatted per the project's `rustfmt` configuration.

#### Scenario: Unformatted code fails CI

- GIVEN a pull request contains a Rust file not matching `cargo fmt` output
- WHEN the formatting job runs
- THEN it fails
- AND the pull request cannot merge

### Requirement: Lint Gate (Clippy, Warnings as Errors)

Every CI run MUST execute `cargo clippy -D warnings` across all workspace crates and fail on any warning.

#### Scenario: Clippy warning fails CI

- GIVEN a pull request introduces code that triggers a Clippy lint
- WHEN the clippy job runs with `-D warnings`
- THEN it fails

### Requirement: Test Gate

Every CI run MUST execute `cargo test` for the full workspace on each platform in the matrix.

#### Scenario: Core tests pass on all three platforms (CA-17)

- GIVEN `vertice-core` has at least one test using a versioned fixture
- WHEN `cargo test` runs on macOS, Windows, and Linux
- THEN the test passes on all three platforms with no platform-specific skip

#### Scenario: Failing test blocks merge

- GIVEN a pull request breaks an existing `cargo test` case on any matrix platform
- WHEN the test job runs
- THEN it fails

### Requirement: Frontend Lint Gate

Every CI run MUST execute the frontend linter (e.g. `npm run lint`) against the Svelte/TypeScript sources in `vertice-app`'s frontend and fail on any lint error.

#### Scenario: Frontend lint error fails CI

- GIVEN a pull request introduces a TypeScript or Svelte lint violation
- WHEN the frontend lint job runs
- THEN it fails

### Requirement: Application Build Gate

Every CI run MUST build the full Tauri application (frontend bundled via Vite, packaged by Tauri's bundler) on each of the three platforms and fail if the build does not complete.

#### Scenario: App builds on all three platforms

- GIVEN a pull request with a valid workspace state
- WHEN the build job runs on macOS, Windows, and Linux
- THEN `vertice-app` builds successfully on each platform

#### Scenario: Build failure blocks merge

- GIVEN a change breaks the Tauri build on any one platform
- WHEN the build job runs
- THEN it fails on that platform
