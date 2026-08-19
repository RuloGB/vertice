# Verify Report: Cost-Aware CI Runner Policy

Change: ci-runner-cost-policy
Artifact store: openspec (flat files, no state.yaml)

## Summary

0 CRITICAL, 0 WARNING, 1 SUGGESTION. All checks that can be run without a live GitHub Actions run pass. The five behavioral tasks (3.5-3.9) remain correctly deferred to post-merge observation; they are reported here as not yet observable, neither pass nor fail.

## What was executed vs. reasoned about

| Check | Method | Result |
|---|---|---|
| Blast radius (git diff -- crates/ frontend/ Cargo.toml Cargo.lock deny.toml rust-toolchain.toml) | Executed | Empty output, confirmed. Only .github/workflows/ci.yml and CLAUDE.md changed. |
| ci.yml parses as valid YAML | Executed (python yaml.safe_load) | parsed: True |
| Full Rust/npm test suite (cargo test, npm run test, cargo test --release) | Not executed, deliberately | Out of scope: this change touches no Rust or frontend source; blast-radius diff above is the direct proof. Running it would validate nothing about this diff. |
| Spec-to-YAML reconciliation, requirement by requirement | Reasoned, from direct reads of the delta spec, the merged living spec, and ci.yml | See table below |
| Two paths-ignore lists byte-identical | Executed (regex extraction + string equality) | True |
| Both fromJSON matrix branches non-empty | Read | [ubuntu-24.04] and [ubuntu-24.04, windows-2022, macos-14], both non-empty |
| pull_request and push both declare branches: [main] | Read | Confirmed, lines 11 and 17 |
| concurrency.group unchanged, cancel-in-progress expression correct | Read, diffed against design section 5 | group: ci-${{ github.ref }} untouched; cancel-in-progress: ${{ github.event_name == 'pull_request' }} present with rationale comment |
| CLAUDE.md Versions and CI section, no remaining contradiction | Read | Confirmed, line 83 rewritten, no other line asserts an unconditional matrix |
| msrv cache key vs rust cache collision | Read | msrv uses explicit key msrv-${{ env.MSRV }} (literal prefix); rust's rust-cache step has no explicit key. Literal-prefix argument holds regardless; no collision possible. |
| needs: frontend resolves under single-leg matrix | Reasoned against GitHub Actions semantics | Holds for both rust (1 or 3 legs) and msrv (fixed 1 leg) |

## Requirement-by-requirement spec reconciliation

Delta at openspec/changes/ci-runner-cost-policy/specs/ci-quality-gates/spec.md checked against .github/workflows/ci.yml:

- Cross-Platform CI Matrix: delta requires Linux-only on pull_request, full matrix on push-to-main and workflow_dispatch. ci.yml's os expression satisfies this: workflow_dispatch falls into the OR branch, giving the full matrix. Satisfied.
- Formatting / Lint / Test / Frontend Lint / Application Build Gates: all delta wording is 'every CI run that occurs MUST...'; the corresponding ci.yml steps run unconditionally within their jobs, and the trigger filters determine whether a run occurs at all. Satisfied.
- Documentation Path Filtering (ADDED): explicit directory list (internal-docs/**, openspec/**, CLAUDE.md), not an extension glob, on both triggers. Confirmed byte-identical, confirmed no **/*.md pattern present. Satisfied.
- Chained Pull Request Policy (ADDED): requires branches: [main] on pull_request so a child slice (base not main) triggers no run. Confirmed present. Satisfied.
- Concurrency Policy (ADDED): PR runs may be cancelled, push-to-main runs must not be. cancel-in-progress expression matches exactly; workflow_dispatch also protected. Satisfied.

Cross-checked against the merged living spec at openspec/specs/ci-quality-gates/spec.md: it still contains pre-change wording (full matrix on every pull request, the pull request cannot merge) that the delta is designed to supersede. This is expected -- merge happens at archive time -- and every contradiction identified there is resolved by the corresponding MODIFIED requirement in the delta. No requirement in the delta is itself contradicted by ci.yml.

## Deferred behavioral tasks (3.5-3.9) -- status: not yet observable

None of these were run; none is fabricated. They require real workflow executions after this change reaches main:

- 3.5: Docs-only PR triggers no run -- unobservable pre-merge.
- 3.6: Non-main-base PR triggers no run -- unobservable pre-merge.
- 3.7: Two consecutive pushes to main both complete, neither cancelled -- highest-risk deferred item, the direct test of the cancel-in-progress expression (design D1). Pre-recorded fallback, restated here so it is not lost: if the expression fails to parse (Invalid workflow file) or cancellation behaves as literal-truthy despite the conditional, replace it with the unconditional cancel-in-progress: false and record the loss of PR-side cancellation savings in a follow-up. Do NOT attempt a group-based workaround (design section 5 explicitly rejects this).
- 3.8: msrv single-digit minutes on warm cache -- requires a second post-merge run; the first is a cold-cache baseline.
- 3.9: Markdown fixture change under crates/vertice-core/tests/fixtures/ still triggers CI -- requires a real PR touching that path.

## Success Criteria assessment (proposal.md)

| Criterion | Status |
|---|---|
| Docs-only PR triggers no workflow run | Deferred -- 3.5 |
| PR whose base is not main triggers no workflow run | Deferred -- 3.6 |
| Two consecutive pushes to main both complete; neither cancelled | Deferred -- 3.7 (highest risk, fallback recorded above) |
| Code PR bills under ~25 minutes | Deferred -- requires runner-minute billing observation |
| msrv completes in single-digit minutes on warm cache | Deferred -- 3.8 |
| Push to main runs all three OSes | Statically verified via the fromJSON else-branch; behaviorally deferred |
| ci-quality-gates spec contains no requirement contradicted by ci.yml | Met -- verified above, delta-to-workflow reconciliation complete |

All deferred items have an explicit, stated observation method (Actions tab / gh run list on the next matching event); none is silently unresolved.

## Read-only invariant (CA-16) and core-purity check

Confirmed no -- this change does not touch either invariant. No crates/, frontend/, Cargo.toml, Cargo.lock, deny.toml, or rust-toolchain.toml diff (blast-radius git diff above, empty). No compiled artifact changes, so cargo deny check bans and the read-only file-write surface are structurally unaffected -- confirmed rather than assumed.

## apply-progress.md vs. working tree

apply-progress.md claims: branches: [main] on pull_request (task 1.1), per-event cancel-in-progress expression + comment (1.2), fromJSON non-empty-array comment (1.3), and the CLAUDE.md rewrite (2.1/2.2/2.3). All four are present in the working tree exactly as described. No discrepancy found.

## Findings

CRITICAL: none.

WARNING: none.

SUGGESTION (1):
- The rust job's own Swatinem/rust-cache@v2 step has no explicit key, unlike msrv's explicit key: msrv-${{ env.MSRV }}. The design argues no collision is possible either way, and this verification concurs -- but an explicit key on rust too would make the non-collision guarantee independent of the action's default behavior for both jobs, not just one. Not a defect; purely a robustness suggestion, non-blocking.

## Recommendation

Static verification is clean. Nothing blocks archive. The five deferred behavioral tasks are expected to remain open until post-merge; they do not gate this SDD cycle per the design's own verification strategy, which classifies them as observational and assigns them to sdd-verify/post-merge, not to sdd-apply or pre-merge static gates.
