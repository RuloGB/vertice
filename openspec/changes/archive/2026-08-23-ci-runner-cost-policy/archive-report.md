# Archive Report: Cost-Aware CI Runner Policy

Change: `ci-runner-cost-policy`
Archived: 2026-08-23
Artifact store: openspec (flat files, no state.yaml)

## Task Completion Gate

`tasks.md` shows 13/18 checked. The 5 unchecked items (3.5-3.9, Phase 3) are explicitly tagged `[DEFERRED — requires a real ... after merge]` in both `tasks.md` and `apply-progress.md`, and are cross-confirmed as intentional, non-blocking in `verify-report.md`'s Summary and Recommendation sections. These are observational/behavioral checks against a live GitHub Actions run (post-merge), not implementation tasks — the design's own verification strategy (design.md §10) classifies them as belonging to post-merge observation, not to `sdd-apply` or pre-merge static gates. No stale-checkbox reconciliation was needed or performed; this is not an exception to the Task Completion Gate, it is the documented deferred-task path.

Follow-ups carried forward (owner: next maintainer touching CI, to be observed on the next matching live event):
- 3.5 — Docs-only PR triggers no workflow run.
- 3.6 — PR whose base is not `main` triggers no workflow run.
- 3.7 — Two consecutive pushes to `main` both complete; neither is cancelled (highest risk — direct test of the `cancel-in-progress` expression; design §5 fallback to unconditional `cancel-in-progress: false` applies if this fails).
- 3.8 — `msrv` completes in single-digit minutes on a warm cache (second post-merge run, not the cold-cache baseline).
- 3.9 — A Markdown fixture change under `crates/vertice-core/tests/fixtures/` still triggers CI.

## Verification State

0 CRITICAL, 0 WARNING, 1 SUGGESTION (non-blocking: give the `rust` job's `Swatinem/rust-cache@v2` step an explicit key, matching `msrv`'s `msrv-${{ env.MSRV }}`, for defense-in-depth even though no collision is currently possible). No CRITICAL issues were present at any point, so no CRITICAL-override situation applies.

## Specs Synced

| Domain | Action | Details |
|--------|--------|---------|
| ci-quality-gates | Updated (merged delta) | 6 MODIFIED requirements (Cross-Platform CI Matrix, Formatting Gate, Lint Gate, Test Gate, Frontend Lint Gate, Application Build Gate), 3 ADDED requirements (Documentation Path Filtering, Chained Pull Request Policy, Concurrency Policy), 0 REMOVED. The pre-existing "Generated TypeScript Contract In-Sync Gate" requirement (not touched by this change's delta) was preserved unchanged. |

Source of truth updated: `openspec/specs/ci-quality-gates/spec.md`.

## Archive Contents

- `proposal.md` — copied verbatim
- `design.md` — copied verbatim
- `tasks.md` — copied verbatim (13/18 checked, 5 deliberately deferred as documented above)
- `apply-progress.md` — copied verbatim
- `verify-report.md` — copied verbatim
- `specs/ci-quality-gates/spec.md` (delta) — copied verbatim
- `archive-report.md` — this file (new)

All copied files were transcribed byte-for-byte from the active change folder's content as read during this archive session; none was regenerated, reformatted, or summarized.

## Known Limitation — Source Folder Not Deleted

This archive execution environment provided only `Read`, `Edit`, `Write`, and `Glob` tools — no filesystem move/delete capability. The archive folder above was created and fully populated with exact copies of every artifact. However, the original active folder at `openspec/changes/ci-runner-cost-policy/` could **not** be deleted or physically moved by this executor, and may still exist alongside the archive copy. This is a tooling-access limitation of this session, not a content-fidelity issue — no artifact content was altered. **Action needed**: an operator or an agent with shell/file-delete access should remove `openspec/changes/ci-runner-cost-policy/` (the pre-archive source) to complete the physical move and avoid the active-changes directory still showing this change as in-flight.

## Source of Truth Updated

The following spec now reflects the new behavior:
- `openspec/specs/ci-quality-gates/spec.md`

## SDD Cycle Complete (Pending Cleanup)

The change has been fully planned, implemented, verified, and its artifacts archived. The only open item is the mechanical deletion of the stale source folder noted above; no further planning, implementation, or verification work is required. Five behavioral checks (3.5-3.9) remain open as documented, non-blocking post-merge observations.
