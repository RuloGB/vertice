# Exploration: T13 — Error, Empty, and Non-Actionable States

## Current State

T11 and T12 already provide a useful foundation: `App.svelte` distinguishes loading, rejected-command failure, and a successful empty component list; frontend copy is localized through typed English/Spanish catalogs; and `LocationList.svelte` safely represents `path: null`. The remaining T13 work is the report-level diagnostic surface required by CA-11 and CA-12, plus an explicit embedded-component marker required by CA-13.

A successful `ScanReport` already carries all required data without a core, IPC, binding, or capability change:
- `installations` contains detected installations.
- `issues` contains recoverable diagnostics with `severity`, optional `path`, and raw `reason`.
- `rootsScanned` reports each root's `found` / `notFound` state.
- `locations[].origin` distinguishes `embedded` from `file`.

The current UI only renders filtered components. It does not render installations, `issues`, or missing roots. A `null` location displays localized “no path on disk” copy, but its owning row is not explicitly labelled as embedded. Existing jsdom `App.test.ts` covers rejected command failure and a successful empty report; it does not cover report diagnostics, missing-client visibility, absent roots, or embedded labelling.

## Affected Areas

- `frontend/src/App.svelte` — retains report ownership and should compose report-level diagnostic sections only after a successful scan.
- `frontend/src/lib/InventoryRow.svelte` — should derive an embedded/non-actionable label from `locations[].origin` (not from a name or missing path alone).
- `frontend/src/lib/LocationList.svelte` — continues rendering all locations verbatim; may expose embedded origin context if the row does not own it.
- `frontend/src/lib/i18n/catalogs.ts` — requires paired `en`/`es` chrome keys for client absence, root absence, issue labels, severity, and embedded state; raw paths and reasons remain passthrough data.
- `frontend/src/App.test.ts` — primary strict-TDD jsdom coverage for CA-11/12/13 at the user-visible boundary.
- `openspec/specs/inventory-ui/spec.md` — must be extended because it currently only specifies minimal lifecycle failure/empty states, not T13 report diagnostics.
- `frontend/src/bindings/ScanReport.ts`, `ScanIssue.ts`, `Location.ts`, `LocationOrigin.ts` — read-only generated contract inputs; do not edit them.

## Constraints and Coupling

- T13 depends on T11 and uses the T12 i18n seam; it must not add frontend filesystem access, timers/watchers, write actions, or additional IPC commands.
- `ScanIssue.reason`, paths, component names, and installation versions are diagnostic payloads and MUST remain unlocalized verbatim data.
- Missing client status cannot be inferred from an empty `installations` array: the core emits explicit `issues` whose reason ends with `not detected`. The UI should use a narrow, documented presentation classifier or an explicit core contract only if existing data proves insufficient.
- The same successful report can include components, missing clients, unreadable files, and absent roots. Diagnostic rendering must not replace or suppress the inventory list.
- CA-13 only requires marking embedded components and avoiding impossible actions. The current UI has no update/uninstall actions, so T13 must not invent action controls merely to disable them.
- Strict TDD applies: write failing jsdom UI tests for each CA surface before markup/catalog implementation. Existing core fixture tests remain the authority for diagnostic production.

## Approaches

| Approach | Pros | Cons | Effort |
|---|---|---|---|
| A. Thin presentational diagnostic components over the existing report | Keeps `App.svelte` focused; makes CA surfaces independently testable; uses generated types and T12 catalogs without changing Rust | Adds a few small Svelte components and a narrow classification seam | Medium |
| B. Inline all diagnostics in `App.svelte` | Fewest files and direct access to the report | Couples filtering/lifecycle/diagnostics into one growing component; harder to review and test | Low–Medium |
| C. Change core/bindings to add UI-oriented diagnostic categories | Removes frontend string classification | Broadens a completed core contract and requires Rust tests/binding regeneration for a presentation-only phase | High |

## Recommendation

Choose **Approach A**: add small presentational report-diagnostic components (or one bounded diagnostics panel) and an explicit embedded badge in the inventory row, driven exclusively by the current `ScanReport`. Keep classification based on existing structured fields where possible: `location.origin === "embedded"`, `root.status === "notFound"`, `issue.path !== null`, and `issue.severity`. For absence of a client, use the currently established core diagnostic reason only behind one isolated presentation helper and lock its expected input in tests; do not broaden core unless that helper cannot reliably distinguish supported-client absence.

The proposal should scope the successful-report screen as: inventory list + independent diagnostic panels, not a mutually exclusive error state. It should also explicitly retain the existing hard command-failure surface.

## Risks

- **Unstructured absent-client reason:** the generated report has no typed client-missing discriminator; coupling UI directly to English reason text is fragile. Contain it in one helper and state the fallback behavior, or escalate a minimal typed-core change only if proposal/design confirms necessity.
- **Diagnostic duplication:** absent roots are represented by both `rootsScanned.status` and warning issues. Define one display source per condition to avoid duplicate cards.
- **False embedded classification:** `path === null` alone is not the semantic discriminator; use `origin === "embedded"`.
- **Scope creep:** do not add update/uninstall controls, persistence, new IPC commands, or localization of diagnostic payloads.
- **Coverage gap:** node-only helper tests cannot prove visible CA-11/12/13 behavior; retain jsdom `App.test.ts` coverage and add manual `tauri dev` confirmation.

## Ready for Proposal

**Yes.** Use change name `add-error-and-empty-states` as proposed. The proposal should trace T13 to CA-11, CA-12, and CA-13, preserve the existing T11 lifecycle surfaces, and define strict RED → GREEN → REFACTOR tests for missing-client, unreadable-component, absent-root, and embedded-component rendering.