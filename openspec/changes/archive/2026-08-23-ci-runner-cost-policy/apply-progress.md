# Apply Progress: Cost-Aware CI Runner Policy

Change: `ci-runner-cost-policy`
Mode: **No TDD applies (design §10 / tasks.md preamble)** — CI configuration and documentation only, no code under test, no test harness for a GitHub Actions workflow file. Each task is verified by its stated method (static read of the YAML/Markdown, `git diff`), not by a RED/GREEN test cycle.
Delivery: single PR (per tasks.md's Review Workload Forecast — Low risk, no chaining needed).

## Status

**13/18 tasks complete.** 5 tasks left unchecked by design — they are explicitly DEFERRED to post-merge observation (Phase 3, tasks 3.5-3.9). `tasks.md` updated in place with `[x]` marks. No previous apply-progress existed for this change (first apply session).

## Files changed

| File | Action | What was done |
|---|---|---|
| `.github/workflows/ci.yml` | Modified | Added `branches: [main]` under `pull_request` (task 1.1); replaced `cancel-in-progress: true` with the per-event expression `${{ github.event_name == 'pull_request' }}` plus explanatory comment (task 1.2); added a comment next to the `rust` job's `os:` matrix expression documenting the `fromJSON` non-empty-array constraint (task 1.3). |
| `CLAUDE.md` | Modified | Replaced the unconditional "Linux + Windows + macOS" `rust` job description with the conditional wording (Linux-only on PRs, full matrix on push-to-`main`/`workflow_dispatch`, CA-17 enforcement point) and added the `paths-ignore` / non-`main`-PR consequence sentence (tasks 2.1, 2.2). Read the rest of "Versions and CI" for contradictions (task 2.3) — none found; no further edit made. |
| `openspec/changes/ci-runner-cost-policy/tasks.md` | Modified | Marked completed tasks `[x]`. |

Unchanged, confirmed by `git diff -- crates/ frontend/ Cargo.toml Cargo.lock deny.toml rust-toolchain.toml` (empty, exit 0): all Rust source, all frontend source, all dependency/toolchain files.

Already present in the tree before this apply session, uncommitted, untouched by this session (design §2 items 1-4 — verified byte-identical via the checkpoint diff read): `paths-ignore` on both triggers with its explanatory comment, the conditional `rust` matrix `fromJSON` ternary, `workflow_dispatch`, `Swatinem/rust-cache@v2` on `msrv` with key `msrv-${{ env.MSRV }}`.

## Task-by-task verification

| Task | Verification performed | Result |
|---|---|---|
| 1.1 | Static read of `ci.yml` | Both `pull_request` and `push` declare `branches: [main]` |
| 1.2 | Static read of `ci.yml` | Expression and two-line comment present; `group: ci-${{ github.ref }}` untouched |
| 1.3 | Static read of `ci.yml` | Comment present immediately above the `os:` line |
| 1.4 (checkpoint) | `git diff -- .github/workflows/ci.yml` (see exact diff in the return contract) | Design §2 items 1-4 present and unmodified; the only new hunks are 1.1/1.2/1.3 |
| 2.1 | Static read of `CLAUDE.md` | `rust` clause no longer asserts an unconditional three-OS matrix |
| 2.2 | Static read of `CLAUDE.md` | Both `paths-ignore` and non-`main`-PR statements present in the same bullet |
| 2.3 | Manual read of the full "Versions and CI" section post-edit | No remaining sentence contradicts the new trigger/matrix/concurrency behavior; no further edit needed |
| 3.1 | `python -c "import yaml,io; yaml.safe_load(...); print('ok')"` | Printed `ok` — the workflow parses as valid YAML |
| 3.2 | Manual read | The two `paths-ignore` lists (`internal-docs/**`, `openspec/**`, `CLAUDE.md`) are byte-identical on both triggers |
| 3.3 | Manual read | Both `fromJSON` branches (`["ubuntu-24.04"]` and `["ubuntu-24.04", "windows-2022", "macos-14"]`) are non-empty |
| 3.4 | Side-by-side read of `ci.yml` against `specs/ci-quality-gates/spec.md` | No contradiction found: matrix conditionality, `paths-ignore` scope, `branches: [main]` on `pull_request`, and the `cancel-in-progress` expression all match the delta spec's requirements and scenarios |
| 4.1 | `git diff --stat` | Only `.github/workflows/ci.yml` and `CLAUDE.md` changed (plus this apply session's own `tasks.md`/`apply-progress.md` artifacts) |
| 4.2 | `git diff -- crates/ frontend/ Cargo.toml Cargo.lock deny.toml rust-toolchain.toml` | Empty, exit 0 |

## Tasks left unchecked (deliberate, not oversight)

- **3.5** A docs-only pull request triggers no run — requires a real PR against the merged workflow. Not observable from this session.
- **3.6** A pull request whose base is not `main` triggers no run — requires a real chained-PR child slice after merge. Not observable from this session.
- **3.7** Two consecutive pushes to `main` both complete, neither cancelled — the direct test of the `cancel-in-progress` expression (D1 in design §0). Requires two real pushes after merge. Highest-risk deferred item; if it fails, the design §5 fallback (`cancel-in-progress: false` unconditionally) applies post-merge, not now.
- **3.8** `msrv` completes in single-digit minutes on a warm cache — requires a second real run after merge (the first is a cold-cache baseline).
- **3.9** A Markdown fixture change under `crates/vertice-core/tests/fixtures/` still triggers CI — requires a real PR touching that path after merge.

No workflow run was executed in this session; these five are explicitly out of scope for `sdd-apply` per the tasks.md preamble and design §10's verification table (Behavioral rows), and belong to `sdd-verify`/post-merge observation.

## Deviations from design

None. No task required touching `crates/`, `frontend/`, `Cargo.*`, `deny.toml`, or `rust-toolchain.toml`; none was touched. The `cancel-in-progress` fallback (design §5) was implemented as the expression form only, per explicit instruction — the unconditional-`false` fallback was not applied since no real workflow run has occurred yet to trigger it.

## Issues found

None.

## Remaining tasks

5 deferred behavioral tasks (3.5-3.9) — see above. All other tasks (13/18) complete.
