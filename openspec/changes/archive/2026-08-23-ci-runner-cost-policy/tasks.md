# Tasks: Cost-Aware CI Runner Policy

> Trace: **T1** (`internal-docs/plan-desarrollo-poc.md`, workspace + CI bootstrap) / touches **CA-17** (core tests on versioned fixtures across the three CI platforms). No other CA is affected.
> Proposal: `openspec/changes/ci-runner-cost-policy/proposal.md`. Delta spec: `openspec/changes/ci-runner-cost-policy/specs/ci-quality-gates/spec.md`. Design: `openspec/changes/ci-runner-cost-policy/design.md`.
> CI-configuration and documentation only — no `crates/`, `frontend/`, `Cargo.*`, `deny.toml`, `rust-toolchain.toml`, or `frontend/src/bindings/**` change (design §1, §9). If any task below appears to require editing one of those, STOP and flag it.
> **Read-only invariant (CA-16) is unaffected**: no build artifact changes, so no `File::create`/`OpenOptions::write` surface is touched by this change.
> **Already in the tree, uncommitted — do NOT re-touch (design §2, items 1–4)**: `paths-ignore` on both triggers, the conditional `rust` matrix `fromJSON` ternary, `workflow_dispatch`, and `Swatinem/rust-cache@v2` on `msrv` with key `msrv-${{ env.MSRV }}`.
> `strict_tdd: true` does not apply in the RED-GREEN sense here — there is no code under test and no harness for a GitHub Actions workflow file (design §10). Each task below states its verification method instead of a test file.
> Environment note: no workflow run is executed during `sdd-apply`; static tasks are verified by reading the YAML, behavioral tasks are explicitly deferred to post-merge observation (see Phase 3).

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | well under 400 (two files: ~6-line YAML diff, one `CLAUDE.md` bullet) |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending — not needed at Low risk |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | `.github/workflows/ci.yml`: `branches: [main]` on `pull_request`, per-event `cancel-in-progress` expression, `fromJSON` non-empty comment | PR 1 (single PR) | Base `main`. Self-contained; the file already carries items 1-4 from design §2, uncommitted |
| 2 | `CLAUDE.md` CI description update, same PR | PR 1 (single PR) | Depends on Unit 1 landing first so the doc matches the actual trigger surface |

Both units are small enough, and dependent enough (docs describe the workflow), to ship as one PR rather than a chain.

## Phase 1: Implement — `.github/workflows/ci.yml`

- [x] 1.1 Add `branches: [main]` under the `pull_request` trigger, symmetrical with the existing `push` trigger's `branches: [main]` (currently asymmetric — `push` has it, `pull_request` does not). Do not touch the `paths-ignore` list beneath it. — *spec: "Chained Pull Request Policy", scenario "A child slice targeting a tracker branch triggers no run"; design §3*
  - Verification: static read of `ci.yml` — confirm both `pull_request` and `push` now declare `branches: [main]`.
- [x] 1.2 On the existing workflow-level `concurrency` block, replace `cancel-in-progress: true` with `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`. Leave `group: ci-${{ github.ref }}` unchanged. Add the two-line explanatory comment from design §5 (pull requests cancel superseded runs; pushes to `main` and dispatches never cancel because `main` is the sole carrier of the CA-17 three-platform guarantee). — *spec: "Concurrency Policy", both scenarios; design §5*
  - Verification: static read of `ci.yml` — confirm the expression and comment are present, `group` is untouched.
  - **Fallback, recorded per design §5**: if this expression fails to parse in the first real workflow run (e.g. `Invalid workflow file`), or cancellation is observed to behave as literal-truthy despite the conditional, replace it with the unconditional `cancel-in-progress: false` and record the loss of PR-side cancellation savings in `verify-report.md`. Do not attempt a `group`-based workaround (design §5 explicitly rejects this). This fallback can only be exercised after a real run is observed — it is a post-merge contingency, not a task to execute now.
- [x] 1.3 Add a comment near the `rust` job's `os:` matrix expression noting that the `fromJSON` idiom (`&&`/`||` returning the operand array) only holds while both arrays are non-empty, and that a future "run nothing" variant MUST use a job-level `if:`, never `fromJSON('[]')`. — *design §4*
  - Verification: static read of `ci.yml` — confirm the comment is present adjacent to the `os:` line.
- [x] 1.4 **Checkpoint.** Re-read the full `ci.yml` trigger, concurrency, and matrix blocks. Confirm design §2 items 1-4 (already present) are byte-identical to before this phase — no accidental rewrite — and that 1.1-1.3 are the only diff in this file.
  - Verification: manual diff review (`git diff -- .github/workflows/ci.yml`).

## Phase 2: Implement — `CLAUDE.md`

- [x] 2.1 Replace the `rust` job clause in the "Versions and CI" section (currently "`rust` (clippy/test/release build on Linux + Windows + macOS)") with wording equivalent to design §8: Linux only on pull requests; Linux + Windows + macOS on push to `main` and on `workflow_dispatch`, which is where CA-17 is enforced. — *design §8*
  - Verification: static read of `CLAUDE.md` — confirm the `rust` job clause no longer asserts an unconditional three-OS matrix.
- [x] 2.2 In the same bullet or the immediately adjacent sentence, add that `paths-ignore` (`internal-docs/**`, `openspec/**`, `CLAUDE.md`) means a documentation-only change produces no run at all, and that pull requests not targeting `main` are not validated. — *design §8*
  - Verification: static read of `CLAUDE.md` — confirm both statements are present.
- [x] 2.3 Read the rest of the "Versions and CI" section for any other sentence this change invalidates (e.g. references to "every push/PR runs the full matrix"); fix only what is actually contradicted — do not rewrite the whole section. — *design §1, §8*
  - Verification: manual read of the full section after edit; confirm no remaining sentence contradicts the new trigger/matrix/concurrency behavior.

## Phase 3: Verification (Static Now, Behavioral Deferred)

These map directly to the proposal's Success Criteria and the design §10 verification table. Each is tagged with when it can actually be checked.

- [x] 3.1 [STATIC — checkable now, by reading the YAML] Confirm the workflow still parses as valid YAML (no obvious syntax break introduced by 1.1-1.3).
- [x] 3.2 [STATIC — checkable now] Confirm the two `paths-ignore` lists (`pull_request` and `push`) remain byte-identical (design §6 — comment-only guard, no mechanical check).
- [x] 3.3 [STATIC — checkable now] Confirm both `fromJSON` branches in the `rust` matrix expression are non-empty arrays.
- [x] 3.4 [STATIC — checkable now] Confirm no requirement in `openspec/changes/ci-runner-cost-policy/specs/ci-quality-gates/spec.md` is contradicted by the resulting `ci.yml` (read both side by side; this is the delta that will merge into the main spec at archive time).
- [ ] 3.5 [DEFERRED — requires a real pull request after merge] A docs-only pull request (touching only `internal-docs/**`, `openspec/**`, or `CLAUDE.md`) triggers no workflow run. Cannot be observed from a branch that has not been pushed against the merged workflow; observe via `gh run list` or the Actions tab on the next docs-only PR.
- [ ] 3.6 [DEFERRED — requires a real pull request after merge] A pull request whose base is not `main` (a chained-PR child slice) triggers no workflow run. Observe on the next chained feature.
- [ ] 3.7 [DEFERRED — requires two real pushes to `main` after merge] Two consecutive pushes to `main` both complete the full matrix; neither is cancelled. This is the highest-risk item — the direct test of design item D1 (whether the `cancel-in-progress` expression parses and behaves as intended). Observe via the Actions tab: both runs must reach a terminal `completed` conclusion, neither `cancelled`.
- [ ] 3.8 [DEFERRED — requires a real `msrv` run on a warm cache, i.e. the second run after merge] The `msrv` job completes in single-digit minutes on a warm cache. The first run after merge is a cold-cache baseline and will NOT satisfy this; only a subsequent run reuses the cache.
- [ ] 3.9 [DEFERRED — requires a real pull request touching that path after merge] A change to a Markdown fixture under `crates/vertice-core/tests/fixtures/` DOES trigger a run (i.e. is NOT excluded by `paths-ignore`, since the ignore list is directory-based, not an extension glob). Observe on the next such PR.

## Phase 4: Local Gates

- [x] 4.1 No Rust/frontend/dependency gate applies to this change (design §1: no `crates/`, `frontend/`, or `Cargo.*` diff). Confirm via `git diff --stat` that only `.github/workflows/ci.yml` and `CLAUDE.md` changed.
- [x] 4.2 `git diff -- crates/ frontend/ Cargo.toml Cargo.lock deny.toml rust-toolchain.toml` must be empty — confirms the blast-radius boundary from design §1 held.
