# Tasks: T13 Error, Empty, and Non-Actionable States

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | 350–480 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | Single PR — approved size exception |
| Delivery strategy | exception-ok |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|---|---|---|---|
| 1 | T13 frontend diagnostics, badge, i18n, and tests | Single PR | User-approved `size:exception`; no branch, push, or PR creation. |

## Phase 0: Test Characterization (T13; CA-11, CA-12)

- [x] 0.1 **RED** — Create `frontend/src/lib/scanDiagnostics.test.ts`: exact three-reason missing-client predicate; reject suffix, error severity, and null-path collisions. (Sequential: first.)
- [x] 0.2 **RED** — Add mixed-partition tests: two `notFound` roots, paired root warnings, client warning, ordinary issue; prove root de-duplication and preservation. (Depends: 0.1; can run with 0.1 after fixtures are shared.)

## Phase 1: Diagnostic Domain Helper (T13; CA-11, CA-12)

- [x] 1.1 **GREEN** — Create `frontend/src/lib/scanDiagnostics.ts` with documented closed reason allow-list and `partitionDiagnostics(roots, issues)`; derive root warnings only by exact core grammar. (Depends: 0.1–0.2.)
- [x] 1.2 **REFACTOR** — Simplify helper names/types without broadening string matching; rerun `scanDiagnostics.test.ts`. (Depends: 1.1.)

## Phase 2: UI Red Tests (T13; CA-11, CA-12, CA-13)

- [x] 2.1 **RED** — Extend `frontend/src/App.test.ts` for mixed and clean ready reports: inventory remains, concise diagnostics render once, raw reasons/paths remain verbatim. (Depends: 1.2.)
- [x] 2.2 **RED** — Add embedded-path and null-path non-embedded cases plus English→Spanish chrome assertions. (Depends: 2.1; same test file, sequential.)

## Phase 3: Frontend Green Implementation (T13; CA-11, CA-12, CA-13)

- [x] 3.1 **GREEN** — Create `frontend/src/lib/ScanDiagnostics.svelte`; render nothing for empty groups and render non-alert unavailable-root, discreet client, and generic-issue sections. (Depends: 2.1.)
- [x] 3.2 **GREEN** — Modify `frontend/src/lib/i18n/catalogs.ts` with complete paired diagnostic and embedded/non-actionable chrome; never translate payload values. (Can run in parallel with 3.1; required before UI passes.)
- [x] 3.3 **GREEN** — Modify `frontend/src/App.svelte` to compose diagnostics only in ready state before `InventoryList`, retaining filtering, loading, and failure behavior. (Depends: 3.1–3.2.)
- [x] 3.4 **GREEN** — Modify `frontend/src/lib/InventoryRow.svelte` to derive and display embedded/non-actionable status from any `location.origin === "embedded"`, with no action control. (Depends: 2.2–3.2; parallel with 3.3.)

## Phase 4: Refactor and Verification (T13; CA-11, CA-12, CA-13)

- [x] 4.1 **REFACTOR** — Remove duplication and tighten component markup/types only after all new frontend tests are green. (Depends: 3.3–3.4.)
- [x] 4.2 Verify `npm run lint && npm run check && npm run test && npm run build`; inspect changed paths to confirm no Rust, IPC, bindings, filesystem writes, branches, commits, pushes, or PRs. (Depends: 4.1.)
