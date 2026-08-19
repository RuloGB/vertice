# Design: Cost-Aware CI Runner Policy

> Trace: **T1** (`internal-docs/plan-desarrollo-poc.md`, workspace + CI bootstrap) / touches **CA-17** (core tests on versioned fixtures across the three CI platforms). No other CA is affected.
> Proposal: `openspec/changes/ci-runner-cost-policy/proposal.md` (approved). Delta spec: `openspec/changes/ci-runner-cost-policy/specs/ci-quality-gates/`.
> `rules.design` coverage: this change touches **no** core data model type, **no** IPC surface, **no** platform-specific path resolution and **no** `ScanIssue` path — §1 states why that is structurally guaranteed, not merely observed. The design-time equivalents (CI trigger surface, matrix contract, failure paths of the workflow itself) are covered in §3–§7.
> **Environment note.** No workflow run was executed in this phase. §0 separates what was read from the repository or from documented GitHub Actions behavior from what is asserted by reasoning.

## 0. Verified vs. assumed

| # | Statement | Basis | Confidence |
|---|---|---|---|
| V1 | `paths-ignore` already lists exactly `internal-docs/**`, `openspec/**`, `CLAUDE.md` on both triggers, with the anti-drift and anti-`**/*.md` rationale in a comment | `.github/workflows/ci.yml:4-20` (uncommitted) | Read |
| V2 | `push` already has `branches: [main]`; `pull_request` has **no** `branches` filter | `ci.yml:10-16` | Read |
| V3 | Concurrency is a single workflow-level block, `group: ci-${{ github.ref }}`, `cancel-in-progress: true` | `ci.yml:25-27` | Read |
| V4 | `rust` and `msrv` both declare `needs: frontend` and both `download-artifact` `frontend-dist`; `frontend` uploads it once from ubuntu | `ci.yml:128-133,137,178-182,195,222-226` | Read |
| V5 | `msrv` now has `Swatinem/rust-cache@v2` with `key: msrv-${{ env.MSRV }}`; `rust` uses the action with no `key` | `ci.yml:176,218-220` | Read |
| V6 | `CLAUDE.md:83` describes `rust` as "clippy/test/release build on Linux + Windows + macOS" unconditionally | `CLAUDE.md:83` | Read |
| V7 | The main spec's *Cross-Platform CI Matrix* requirement says the full matrix runs "on every pull request", which `ci.yml` now contradicts | `openspec/specs/ci-quality-gates/spec.md:9-29` | Read |
| D1 | `concurrency.cancel-in-progress` accepts a `${{ }}` expression; GitHub documents the pattern `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}` | GitHub Actions documentation, "Using concurrency" | Documented, **not executed here** |
| D2 | `paths`/`paths-ignore` filters are evaluated at the **workflow** level: a filtered-out event produces **no workflow run at all**, not a run with skipped jobs | GitHub Actions documentation, "Events that trigger workflows" | Documented |
| D3 | `Swatinem/rust-cache@v2` folds the rustc version/host and the job id into the cache key by default | action README | **Assumed** — not re-read in this phase; §7 does not depend on it |
| A1 | GitHub renders a matrixed job's check name as the explicit `name:` followed by the matrix values in parentheses | Prior observation of this repo's runs | **Assumed** — consequence recorded in §4, no behavior depends on it |

## 1. Blast radius: CI configuration and documentation only

```
  .github/workflows/ci.yml   ← trigger surface, concurrency, matrix   (MODIFIED)
  CLAUDE.md:83               ← job description                        (MODIFIED)
  openspec/.../specs/**      ← delta spec (sibling sdd-spec phase)    (MODIFIED)
  ─────────────────────────────────────────────────────────────────────
  crates/vertice-core/**  crates/vertice-app/**  frontend/**
  Cargo.toml  Cargo.lock  deny.toml  rust-toolchain.toml
  frontend/src/bindings/**                                            (UNTOUCHED)
```

No Rust source, no `model/` type, no `ts-rs` regeneration, no Tauri capability, no dependency. Core purity (`cargo deny check bans`) and the read-only invariant (CA-16) are untouched **because no compiled artifact changes** — the same commit builds byte-for-byte the same binaries before and after. The MSRV three-place agreement (`Cargo.toml`, `MSRV` env, `rust-toolchain.toml`) is unchanged; `msrv-${{ env.MSRV }}` *reads* the env, it does not redefine it.

## 2. What is already in the tree, and what is still missing

| # | Item | State |
|---|---|---|
| 1 | `paths-ignore` on both triggers, explicit directory list | **Present** (V1) |
| 2 | Conditional `rust` matrix via `fromJSON` ternary | **Present** |
| 3 | `workflow_dispatch` | **Present** |
| 4 | `rust-cache` on `msrv`, key `msrv-${{ env.MSRV }}` | **Present** (V5) |
| 5 | `branches: [main]` on `pull_request` | **Missing** — §3 |
| 6 | Per-event `cancel-in-progress` | **Missing** — §5 |
| 7 | `CLAUDE.md:83` job description | **Missing** — §8 |

`sdd-apply` implements 5, 6 and 7 and touches nothing else in `ci.yml`.

## 3. Decision: `branches: [main]` on the pull-request trigger

> **Choice: add `branches: [main]` under `pull_request`, symmetrical with the existing `push` filter (V2).**

```yaml
  pull_request:
    branches: [main]
    paths-ignore:
      - "internal-docs/**"
      - "openspec/**"
      - "CLAUDE.md"
```

`branches` on `pull_request` filters by the **base** branch. A child slice whose base is a tracker branch therefore creates no run at all (D2), which is the outcome the proposal decided: validate once per real feature, on the tracker → `main` pull request.

| Alternative | Consequence | Verdict |
|---|---|---|
| **`branches: [main]`** | Chain slices cost zero minutes; validation happens on the tracker PR and again as the full matrix on the push to `main` | **Chosen** |
| `if: github.base_ref == 'main'` on each job | The run is still created and each job still occupies a queue slot; it also produces four "skipped" checks per slice, which is visual noise and would be *un*satisfiable if branch protection ever arrives | Rejected |
| No filter (status quo) | ~45 min per slice; a four-slice feature bills ~179 min, defeating the change | Rejected |

**Accepted cost, already recorded in the proposal:** intermediate slices may contain non-compiling code, and the whole validation debt lands when the chain closes. `workflow_dispatch` on the slice branch is the per-slice escape hatch.

## 4. Decision: conditional matrix — validity and the `needs` interaction

> **Choice: keep the `fromJSON` ternary exactly as written. No change.**

```yaml
os: ${{ github.event_name == 'pull_request' && fromJSON('["ubuntu-24.04"]') || fromJSON('["ubuntu-24.04", "windows-2022", "macos-14"]') }}
```

Both operands are non-empty JSON arrays, and GitHub's `&&`/`||` return the *operand value* rather than a boolean, so the expression yields a real array on either branch. **This idiom only holds because both arrays are non-empty**: an empty array is not reliably truthy, so a future "run nothing" variant MUST use a job-level `if:`, never `fromJSON('[]')`. That constraint belongs in the workflow comment.

**`needs` is unaffected (V4).** `needs` is declared at job level and evaluated once for the whole job; `strategy.matrix` fans the job out *after* its dependency is satisfied. Every leg — one or three — waits for `frontend` and downloads the same `frontend-dist` artifact, which is built once on ubuntu and is platform-independent (it is Vite output). A single-leg matrix satisfies `needs: frontend` identically to a three-leg one.

**`paths-ignore` cannot strand a `needs` edge either (D2).** The filter suppresses the entire workflow run; there is no state in which `frontend` is skipped while `rust` waits on it. This is why the docs-skip mechanism is a trigger filter and not a per-job `if:` — a per-job `if:` on `frontend` *would* strand `rust` and `msrv`.

**Check-name consequence (A1).** The check names of the `rust` job are matrix-derived, so a pull-request run publishes one `rust` check and a push run publishes three. If branch protection is ever enabled (out of scope; 403 on this plan), a required check named after the `windows-2022` leg could never be satisfied by a pull request. Recorded here so a future enablement does not discover it live.

## 5. Decision: per-event `cancel-in-progress`

> **Choice: one expression on the existing workflow-level `concurrency` block. Keep `group` unchanged.**

```yaml
concurrency:
  group: ci-${{ github.ref }}
  # Pull requests cancel superseded runs: money saved, no guarantee lost.
  # Pushes to `main` and manual dispatches never cancel -- `main` is now the
  # sole carrier of the CA-17 three-platform guarantee, and a cancelled run
  # would silently leave a merge commit unvalidated on Windows and macOS.
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

This relies on D1 (documented, not executed here). The predicate is written positively — cancel *only* for pull requests — so `workflow_dispatch`, which is always a deliberate validation request, is also protected.

The existing `group` already separates events by ref: a pull request run groups under `refs/pull/N/merge` while a push groups under `refs/heads/main`, so PR and push runs never contended in the first place. The change is strictly about **push-vs-push on `main`**. A push and a dispatch on `main` do share a group; with cancellation off they queue rather than cancel, which loses nothing.

| Alternative | Consequence | Verdict |
|---|---|---|
| **Expression on `cancel-in-progress`** | One line, intent legible at the point of effect | **Chosen** |
| Event-derived unique `group` for pushes (e.g. `ci-${{ github.ref }}-${{ github.run_id }}`) | Works without D1, but disables *grouping* rather than *cancellation*, so the block stops meaning what it says and PR grouping must be reconstructed | Rejected |
| `cancel-in-progress: false` globally | Safe and D1-free, but pays for every superseded PR push — a direct cost regression | Rejected as the primary; **kept as the fallback** |

**Fallback if D1 turns out to be false** (the workflow fails to parse, or cancellation behaves as literal-truthy): set `cancel-in-progress: false` unconditionally and record the loss in `verify-report.md`. Do **not** improvise a `group` hack.

## 6. Decision: no mechanical guard for the duplicated `paths-ignore` lists

> **Choice: the explanatory comment already in the file (V1) is sufficient. Do not add a CI assertion.**

| Alternative | Consequence | Verdict |
|---|---|---|
| **Comment only** | Zero runner minutes, zero new failure mode | **Chosen** |
| A `quality` step parsing both lists and diffing them | Adds a step (and a YAML-parsing dependency such as `yq`) to *every* run of a change whose entire purpose is to reduce per-run cost. The list is three entries whose diff is visible in any review of the trigger block | Rejected |
| A dedicated guard job | Worse: a whole runner for a three-line comparison | Rejected |

The reasoning is proportionality of **consequence**, not just of size. Drift here cannot produce a wrong build or a false green on code: the two lists gate *different events*, and the worst outcome is that one event type is slightly more or less eager than intended — a cost or latency deviation, visible the first time someone looks at the Actions tab. That is not worth a permanent tax on every run. `sdd-verify` asserts the two lists are identical once, as a review check.

## 7. Cache keys: no collision, and the cold-start bill

`rust` (pinned toolchain from `rust-toolchain.toml`, currently 1.97.1) and `msrv` (1.88) now both use `Swatinem/rust-cache@v2`. The explicit `key: msrv-${{ env.MSRV }}` makes the separation **independent of D3**: even if the action's default keying did not already include the rustc version, the literal prefix differs, so the two caches cannot alias. It also self-invalidates on an MSRV bump, which is the behavior we want — a new floor must not restore artifacts built by the old one.

**Cold-cache cost, stated so it is not mistaken for a regression:** the first run after merge saves nothing and `msrv` will bill its full ~37 min once more. Caches are also per-runner-OS, so the `windows-2022` and `macos-14` legs only ever warm on push-to-`main` runs and on dispatches. **Under the one-merge-per-feature policy those runs are rare, and GitHub evicts caches untouched for 7 days (and LRU-evicts against the 10 GB per-repository ceiling), so a feature that takes longer than a week will find those two caches cold again.** That is acceptable — those legs are the cheap, fast ones in wall-clock terms; the expensive item was `msrv`, which runs on `ubuntu-24.04` on every code pull request and will therefore stay warm.

## 8. Where CA-17 lives after this change

| Carrier | Trigger | Guarantee |
|---|---|---|
| `rust` job, three-leg matrix | `push` to `main` | Primary. `cargo test --workspace --locked` on ubuntu + windows + macOS over the versioned fixtures |
| `rust` job, three-leg matrix | `workflow_dispatch` | On-demand, pre-merge, any branch |
| T15 packaging | release | Backstop: no release is built from `main` directly; every release revalidates all three platforms before publication |

CA-17 is **not** deleted, relocated to a manual-only path, or downgraded to a single platform. What changes is **detection latency**: from pre-merge to one merge later. The delta spec (sibling `sdd-spec` phase) must carry this, and `CLAUDE.md:83` must stop asserting the unconditional three-OS matrix. Required `CLAUDE.md` edit — replace the `rust` clause of line 83 with wording equivalent to:

> `rust` (clippy/test/release build — Linux only on pull requests; Linux + Windows + macOS on push to `main` and on `workflow_dispatch`, which is where CA-17 is enforced)

and add, in the same bullet, that `paths-ignore` (`internal-docs/**`, `openspec/**`, `CLAUDE.md`) means a documentation-only change produces no run at all, and that pull requests not targeting `main` are not validated.

## 9. File changes

| File | Action | Description |
|---|---|---|
| `.github/workflows/ci.yml` | Modify | §3 `branches: [main]`; §5 `cancel-in-progress` expression + comment; §4 comment noting both `fromJSON` arrays must stay non-empty. Items 1–4 of §2 already present — **do not rewrite them** |
| `CLAUDE.md` (line 83) | Modify | §8 wording |
| `openspec/changes/ci-runner-cost-policy/specs/ci-quality-gates/spec.md` | Modify | Sibling `sdd-spec` phase — **not this phase's file** |
| `crates/**`, `frontend/**`, `Cargo.*`, `deny.toml`, `rust-toolchain.toml`, `frontend/src/bindings/**` | **Unchanged** | §1 |

## 10. Verification strategy

CI configuration cannot be unit-tested; the gates are observational and belong to `sdd-verify`.

| Layer | What | How |
|---|---|---|
| Static | Workflow parses; no `Invalid workflow file` annotation | First push after merge |
| Static | The two `paths-ignore` lists are byte-identical (§6) | Manual read of the trigger block |
| Static | Both `fromJSON` branches are non-empty arrays (§4) | Manual read |
| Behavioral | A docs-only pull request produces **no** run | Observe the Actions tab / `gh run list` |
| Behavioral | A pull request whose base is not `main` produces **no** run | Observe on the next chained feature |
| Behavioral | A code pull request produces exactly one `rust` leg (`ubuntu-24.04`) | Observe |
| Behavioral | A push to `main` produces three `rust` legs | Observe |
| Behavioral | Two consecutive pushes to `main` both complete; neither is cancelled (§5) | Observe — the direct D1 test |
| Cost | Code pull request bills under ~25 min; `msrv` in single digits on a warm cache | Run timing |
| Regression | `cargo test --workspace --locked` still green on all three legs of the push-to-`main` run (CA-17, §8) | Existing suites, unchanged |

**Note on `strict_tdd: true`:** it does not apply here in the RED-GREEN sense — there is no code under test and no test harness for a workflow file. The behavioral rows above are the substitute, and they are observational by nature.

## 11. Rollback

`git checkout .github/workflows/ci.yml && git checkout CLAUDE.md` restores the previous policy exactly. No build artifact, lockfile, binding, or dependency is involved, so reverting cannot leave the workspace in a mixed state and requires no regeneration. Spec rollback = do not merge the delta.

## 12. Open questions

- [x] **Per-event cancellation** — expression on `cancel-in-progress`, `group` unchanged; unconditional `false` is the fallback if D1 is wrong. §5.
- [x] **`paths-ignore` drift guard** — comment only; an assertion is disproportionate to a three-entry list whose worst failure is a cost deviation. §6.
- [x] **`needs` under a reduced matrix** — unaffected; `needs` is job-level and `paths-ignore` suppresses whole runs, never individual jobs. §4.
- [x] **Matrix expression validity** — valid, and valid *only* while both arrays are non-empty. §4.
- [x] **Cache-key collision** — impossible by literal prefix, independent of the action's default keying. §7.
- [x] **Where CA-17 lives** — push to `main` + `workflow_dispatch`, backstopped by T15 packaging; latency, not coverage, is what changes. §8.
- [ ] **Branch protection and required checks** — blocked by plan (403). When it becomes available, revisit: matrix-derived check names and skipped runs both interact with it (§4, §3). Out of scope.
- [ ] **`paths-filter` + skip-job pattern** — the standard workaround for "required check never reports on a skipped run". Only relevant once branch protection exists. Out of scope.
