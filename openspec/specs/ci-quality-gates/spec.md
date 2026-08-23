# CI Quality Gates Specification

## Purpose

Defines the cross-platform continuous integration gates that must pass before any change merges. Traces to T1 of `internal-docs/plan-desarrollo-poc.md` and to CA-17 (core tests pass on versioned fixtures across the three CI platforms).

## Requirements

### Requirement: Cross-Platform CI Matrix

CI MUST run the full matrix of macOS, Windows, and Linux runners on every push to `main` and on manual `workflow_dispatch`. Pull requests targeting `main` MUST validate on Linux only. Each matrix leg MUST run independently; a failure on one platform MUST NOT be masked by success on another.

CA-17 (core tests pass on versioned fixtures across the three CI platforms) is now enforced exclusively by the push-to-`main` matrix run, with `workflow_dispatch` available as an on-demand pre-merge check for platform-sensitive changes. Detection latency for a platform-specific regression therefore moves from pre-merge (every pull request) to post-merge (the push to `main` that follows the merge, or an earlier manual `workflow_dispatch` run). This is accepted because no release is built directly from `main` — T15 packaging revalidates all three platforms before publishing — and because `main` receives one merge per real feature under the Chained Pull Request Policy, so the full matrix still runs once per feature rather than once per review slice.

#### Scenario: Matrix triggers on pull request

- GIVEN a pull request is opened or updated with base branch `main`
- WHEN the CI workflow evaluates its triggers
- THEN it runs on Linux only; it does not run on macOS or Windows

#### Scenario: Matrix triggers on push to main

- GIVEN a commit is pushed directly to `main`
- WHEN the CI workflow evaluates its triggers
- THEN it runs the full matrix on macOS, Windows, and Linux
- AND the run MUST NOT be cancelled by the workflow's concurrency policy, even if a later push to `main` starts before it completes

#### Scenario: Matrix triggers on manual workflow_dispatch

- GIVEN a maintainer manually triggers the workflow on any branch
- WHEN the CI workflow runs
- THEN it runs the full matrix on macOS, Windows, and Linux, providing pre-merge validation for platform-sensitive changes without waiting for a push to `main`

#### Scenario: One platform failure marks the workflow run failed

- GIVEN the Linux leg fails while macOS and Windows pass on a push-to-`main` or `workflow_dispatch` run
- WHEN the workflow run's overall conclusion is evaluated
- THEN the run's conclusion is `failure`, and the failure is visible on the associated commit and, when applicable, on the pull request that introduced it
- AND this requirement asserts the workflow's conclusion only, not merge blocking: branch protection / required checks are unavailable on this repository's plan (verified: the branch-protection API returns HTTP 403), so no GitHub check has ever mechanically prevented a merge

### Requirement: Formatting Gate

Every CI run that occurs (see Requirement: Documentation Path Filtering and Requirement: Chained Pull Request Policy for the conditions under which no run occurs) MUST execute `cargo fmt --check` and fail if any Rust source file is not formatted per the project's `rustfmt` configuration.

#### Scenario: Unformatted code fails CI

- GIVEN a pull request contains a Rust file not matching `cargo fmt` output
- WHEN the formatting job runs
- THEN it fails, and the run's conclusion is `failure`
- AND the failure is visible on the pull request; it does not mechanically prevent a merge, because branch protection is unavailable on this repository's plan (see Requirement: Cross-Platform CI Matrix)

### Requirement: Lint Gate (Clippy, Warnings as Errors)

Every CI run that occurs (see Requirement: Documentation Path Filtering and Requirement: Chained Pull Request Policy for the conditions under which no run occurs) MUST execute `cargo clippy -D warnings` across all workspace crates and fail on any warning.

#### Scenario: Clippy warning fails CI

- GIVEN a pull request introduces code that triggers a Clippy lint
- WHEN the clippy job runs with `-D warnings`
- THEN it fails

### Requirement: Test Gate

Every CI run that occurs (see Requirement: Documentation Path Filtering and Requirement: Chained Pull Request Policy for the conditions under which no run occurs) MUST execute `cargo test` for the full workspace on each platform present in that run's matrix.

#### Scenario: Core tests pass on all three platforms (CA-17)

- GIVEN `vertice-core` has at least one test using a versioned fixture
- WHEN `cargo test` runs as part of a push-to-`main` or `workflow_dispatch` matrix run, on macOS, Windows, and Linux
- THEN the test passes on all three platforms with no platform-specific skip
- AND this is the sole point in the pipeline where the three-platform guarantee is exercised; pull requests exercise the Linux leg only

#### Scenario: Failing test fails the run

- GIVEN a pull request breaks an existing `cargo test` case on any matrix platform present in that pull request's run
- WHEN the test job runs
- THEN it fails

### Requirement: Frontend Lint Gate

Every CI run that occurs (see Requirement: Documentation Path Filtering and Requirement: Chained Pull Request Policy for the conditions under which no run occurs) MUST execute the frontend linter (e.g. `npm run lint`) against the Svelte/TypeScript sources in `vertice-app`'s frontend and fail on any lint error.

#### Scenario: Frontend lint error fails CI

- GIVEN a pull request introduces a TypeScript or Svelte lint violation
- WHEN the frontend lint job runs
- THEN it fails

### Requirement: Application Build Gate

Every CI run that occurs (see Requirement: Documentation Path Filtering and Requirement: Chained Pull Request Policy for the conditions under which no run occurs) MUST build the full Tauri application (frontend bundled via Vite, packaged by Tauri's bundler) on each platform present in that run's matrix and fail if the build does not complete.

#### Scenario: App builds on all platforms in the triggering run's matrix

- GIVEN a pull request with a valid workspace state
- WHEN the build job runs on the platform(s) present in that pull request's matrix (Linux only for a pull request, or macOS/Windows/Linux for a push to `main` or `workflow_dispatch`)
- THEN `vertice-app` builds successfully on each of those platforms

#### Scenario: Build failure fails the run

- GIVEN a change breaks the Tauri build on any one platform present in the triggering run's matrix
- WHEN the build job runs
- THEN it fails on that platform

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

### Requirement: Documentation Path Filtering

A documentation-only change MUST NOT trigger a CI run. The ignore list used to detect a documentation-only change MUST be an explicit list of documentation directories and files (`internal-docs/**`, `openspec/**`, `CLAUDE.md`). It MUST NOT be implemented as an extension-based glob such as `**/*.md`, because `crates/vertice-core/tests/fixtures/` contains Markdown fixture files that CA-17-pinned tests read, and an extension glob would silently skip CI exactly when those fixtures — and therefore the scanner behavior they pin — change.

This filter MUST apply identically to both the `pull_request` and `push` triggers.

#### Scenario: Docs-only pull request triggers no run

- GIVEN a pull request changes only files under `internal-docs/**`, `openspec/**`, or `CLAUDE.md`
- WHEN GitHub Actions evaluates the workflow's `paths-ignore` filter
- THEN no CI run is triggered for that pull request

#### Scenario: Docs-only push to main triggers no run

- GIVEN a push to `main` changes only files under `internal-docs/**`, `openspec/**`, or `CLAUDE.md`
- WHEN GitHub Actions evaluates the workflow's `paths-ignore` filter
- THEN no CI run is triggered for that push

#### Scenario: A Markdown fixture change still triggers CI

- GIVEN a change modifies a Markdown file under `crates/vertice-core/tests/fixtures/`
- WHEN GitHub Actions evaluates the workflow's `paths-ignore` filter
- THEN the change is NOT excluded and a CI run is triggered, because the ignore list is an explicit directory/file list, not an extension glob that would have matched this file too

### Requirement: Chained Pull Request Policy

A pull request whose base branch is not `main` MUST NOT trigger a CI run. Validation of a feature developed as a stacked chain of review slices (a Feature Branch Chain) MUST occur exactly once, on the pull request whose base branch is `main` (the tracker → `main` pull request), and again as the full matrix on the resulting push to `main`.

Accepted consequence: individual slices inside a chain may contain code that does not compile or does not pass tests, and the full validation debt lands at once when the chain closes against `main`. Bisecting which slice introduced a failure is correspondingly harder under this policy. This is in tension with the project's `strict_tdd: true` convention, which for intermediate chain slices is enforced by the developer running tests locally, not by CI. `workflow_dispatch` remains available to validate any individual slice on demand before it is merged into its tracker branch.

#### Scenario: A child slice targeting a tracker branch triggers no run

- GIVEN a pull request's base branch is a feature tracker branch, not `main`
- WHEN the CI workflow evaluates its triggers
- THEN no CI run is triggered for that pull request

#### Scenario: The tracker-to-main pull request validates the whole chain once

- GIVEN a tracker branch accumulates several merged child slices and a pull request is opened from the tracker branch with base `main`
- WHEN the CI workflow evaluates its triggers
- THEN it runs the Linux-only matrix for that pull request, covering the combined content of every slice in the chain
- AND this is the first point at which any slice in the chain is validated by CI

### Requirement: Concurrency Policy

A pull request's CI run MAY be cancelled when a newer run starts for the same pull request, since cancelling a superseded pull-request run saves runner minutes and forfeits no guarantee that a subsequent run does not also provide. A push-to-`main` CI run MUST NOT be cancelled by the workflow's concurrency policy, because `main` is the sole location where the CA-17 three-platform guarantee is exercised (per Requirement: Cross-Platform CI Matrix), and a cancelled push run would silently leave a merge commit unvalidated on Windows and macOS.

#### Scenario: A superseded pull request run may be cancelled

- GIVEN a pull request receives a new commit while its previous CI run is still in progress
- WHEN the workflow's concurrency policy evaluates the two runs
- THEN the earlier, now-superseded run for that pull request MAY be cancelled

#### Scenario: A push-to-main run is never cancelled by a later push

- GIVEN a push to `main` starts a CI run, and a second push to `main` starts before the first run completes
- WHEN the workflow's concurrency policy evaluates the two runs
- THEN the first push-to-`main` run continues to completion and is NOT cancelled
