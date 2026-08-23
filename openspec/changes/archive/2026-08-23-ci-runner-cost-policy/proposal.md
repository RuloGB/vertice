# Proposal: Cost-Aware CI Runner Policy

**Traces to**: T1 (Bootstrap del workspace y CI). **CA touched**: CA-17 (core tests on versioned fixtures across the three CI platforms, closed by T16). No other CA is affected.

## Intent

A documentation-only pull request bills ~82 runner minutes, ~150 per merge round-trip. Roughly 13 docs changes exhaust the 2000-minute monthly allowance on this private free-plan repo. Four independent causes, measured on run 32248760094:

| Cause | Cost | Note |
|---|---|---|
| `msrv` job has no `rust-cache` | ~37 min | Largest single item; rebuilds the `tauri` tree every run |
| No `paths-ignore` | full workflow | Docs commits run everything |
| `pull_request` + `push: main` both fire | ~2x | `concurrency` cannot dedupe: different refs |
| 3-OS matrix on every PR | ~23 min billed | Windows 2x, macOS 10x, despite being the fastest jobs |

## Scope

### In Scope

- `Swatinem/rust-cache@v2` on `msrv` with key `msrv-${{ env.MSRV }}`.
- `paths-ignore` on both triggers: explicit list `internal-docs/**`, `openspec/**`, `CLAUDE.md`.
- Conditional `rust` matrix: Linux-only on `pull_request`; full three-OS on `push: main`.
- `workflow_dispatch` as a manual pre-merge escape hatch for platform-sensitive work.
- `branches: [main]` filter on the `pull_request` trigger, so only pull requests targeting `main` are validated (see "Chained Pull Request Policy").
- `cancel-in-progress: false` for `push: main` runs only; pull request runs keep cancellation.
- Delta spec amending `ci-quality-gates`.
- `CLAUDE.md` CI description updated to match.

### Out of Scope

- Branch protection / required checks — unavailable on this plan (403 from the API).
- Removing the duplicated `push: main` run; it is now the load-bearing CA-17 gate.
- Self-hosted runners, job splitting, or `cargo-nextest`.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `ci-quality-gates`: **Cross-Platform CI Matrix** — three-OS matrix moves from every PR to push-to-`main` plus `workflow_dispatch`. **Scenario "Matrix triggers on pull request"** — becomes Linux-only. **Scenario "One platform failure blocks merge"** — already aspirational (no branch protection); reword to assert workflow conclusion, not merge blocking. **Every CI run MUST…** requirements (fmt, clippy, test, frontend lint, build) — scope to runs that occur; a docs-only PR now produces no run.

## Where CA-17 Now Lives

Push to `main`, plus `workflow_dispatch` on demand. Guarantee preserved; **detection latency changes from pre-merge to one merge later**.

This is safe because of two confirmed project facts, not assumptions:

1. **No release is built from `main` directly.** Every release goes through T15 packaging, which revalidates all three platforms before anything is published. A regression reaching `main` therefore cannot reach a user without passing a later three-platform gate.
2. **`main` receives one merge per real feature.** Chained pull requests stack onto a tracker branch and reach `main` as a single pull request (`feature-branch-chain`), so the full matrix runs once per feature rather than once per review slice.

## Chained Pull Request Policy

Review slices stack onto a tracker branch and reach `main` as one pull request (`feature-branch-chain`). Because `on: pull_request` without a `branches` filter fires for **any** base branch, every child slice would otherwise trigger its own full Linux validation — a four-slice feature would bill roughly 179 minutes, defeating the goal of validating once per real feature.

Decision: **child pull requests targeting a tracker branch run no CI at all.** Validation happens once, on the tracker → `main` pull request, and again as the full matrix on the resulting push to `main`.

Accepted consequence, recorded deliberately: slices inside a chain may contain code that does not compile or does not pass tests, and the whole validation debt lands at once when the chain closes. Bisecting which slice broke the build is correspondingly harder. This is in tension with `strict_tdd: true`, which is enforced by the developer running tests locally, not by CI, for intermediate slices. `workflow_dispatch` remains available to validate any individual slice on demand.

## Concurrency

`cancel-in-progress` stays `true` for pull requests, where cancelling a superseded run saves money and forfeits no guarantee. It becomes `false` for `push: main`, because `main` is now the sole carrier of the CA-17 three-platform guarantee and a cancelled run would silently leave a merge commit unvalidated on Windows and macOS. Under the one-merge-per-feature policy there is rarely a run to cancel, so the safety costs almost nothing.

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Platform regression lands on `main` | Med | Full matrix on push; `workflow_dispatch` before risky merges |
| Chain slices accumulate unvalidated code; failure at chain close is hard to bisect | Med | Accepted by decision; local `strict_tdd` runs plus `workflow_dispatch` per slice |
| Duplicated `paths-ignore` lists drift (no YAML anchors in Actions) | Med | Explanatory comment in `ci.yml`; verify in `sdd-verify` |
| A future `**/*.md` glob skips CI when `crates/vertice-core/tests/fixtures/` Markdown changes | Low | Spec forbids extension globs; explicit directory list only |
| If branch protection is later enabled, skipped runs never report and block merges | Low | Documented; revisit with `paths-filter` + skip-job pattern |

## Rollback

Single file: `git checkout .github/workflows/ci.yml` restores the unconditional matrix. **Zero impact on the three-layer architecture** — no Rust, Tauri, frontend, or binding change; core purity and read-only (CA-16) untouched. Spec rollback = do not merge the delta.

## Success Criteria

- [ ] Docs-only PR triggers no workflow run.
- [ ] A pull request whose base is not `main` triggers no workflow run.
- [ ] Two consecutive pushes to `main` both complete the full matrix; neither is cancelled.
- [ ] Code PR bills under ~25 minutes.
- [ ] `msrv` completes in single-digit minutes on a warm cache.
- [ ] Push to `main` runs all three OSes.
- [ ] `ci-quality-gates` spec contains no requirement contradicted by `ci.yml`.
