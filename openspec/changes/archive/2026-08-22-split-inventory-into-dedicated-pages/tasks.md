# Tasks: Split Inventory into Agents, Skills, and Scan Pages

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 500-650 |
| 400-line budget risk | High |
| Chained PRs recommended | No (explicit user exception) |
| Suggested split | Single PR (PR-size exception granted) |
| Delivery strategy | exception-ok |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Full change (all phases below) | PR 1 | User-approved `size:exception`; single PR delivery, no chaining. |

## Phase 1: Navigation Model (RED → GREEN)

- [x] 1.1 RED: edit `frontend/src/lib/navigation.test.ts` — `isRouteId("scan") === true`, `isRouteId("inventory") === false`; `hasContent` true for home/agents/skills/scan/subscriptions, false for mcp/prompts; group coverage assertions updated.
- [x] 1.2 GREEN: edit `frontend/src/lib/navigation.ts` — replace `inventory` with `scan` in `ROUTE_IDS`/`NAV_GROUPS.data`; update `ROUTES_WITH_CONTENT` to home, agents, skills, scan, subscriptions; update module doc.
- [x] 1.3 Edit `frontend/src/lib/NavIcon.svelte` — replace `inventory` icon branch with `scan` (magnifier glyph per design.md).

## Phase 2: incidentCount Unit (RED → GREEN)

- [x] 2.1 RED: edit `frontend/src/lib/scanDiagnostics.test.ts` — add cases: clean report → `0`; zero `issues` + one `notFound` root → `1` (correctness-critical scenario, decision 3); one `notFound` root + its de-duplicated warning + one real issue → `2` not `3`; missing-client only → `1`.
- [x] 2.2 GREEN: add `incidentCount(diagnostics: Diagnostics): number` to `frontend/src/lib/scanDiagnostics.ts` summing `unavailableRoots.length + missingClientIssues.length + remainingRecoverableIssues.length`.

## Phase 3: i18n Catalog Migration (RED → GREEN)

- [x] 3.1 RED: edit `frontend/src/lib/i18n/locale.test.ts` — assert absence of `nav.inventory`, `area.inventory`, `toolbar.allKinds`, `toolbar.kindAriaLabel`, `diagnostics.unavailableRoots`, and the whole `inventory.*` namespace in both catalogs; assert presence of `components.*`, `scan.*`, `incident.*`, `home.scan*` in both `en` and `es` with parity.
- [x] 3.2 GREEN: edit `frontend/src/lib/i18n/catalogs.ts` (both locales) — remove retired keys; rename `inventory.loading|empty|duplicate|duplicateTitle|embedded` → `components.*`; reword `failure.title`, `home.ctaTitle`, `home.ctaBody`, `home.ctaAction`; add `nav.scan`/`area.scan`, `scan.verdictHealthy`, `scan.verdictIssues`, `scan.rootsTitle`, `scan.rootFound`/`scan.rootNotFound`, `scan.installationsTitle`, `scan.installationsEmpty`, `scan.durationLabel`/`scan.durationValue`, `incident.label`/`incident.count`/`incident.action`, `home.scanTitle`/`home.scanHealthy`/`home.scanIssues`/`home.scanFailed`/`home.scanRetry`/`home.scanOpen`/`home.scanPending` — per design.md's exact key/value table, both locales in lockstep.

## Phase 4: Component Renames (mechanical, tests stay green)

- [x] 4.1 Rename `frontend/src/lib/InventoryList.svelte` → `frontend/src/lib/ComponentList.svelte`; update `inventory.empty` reference to `components.empty`.
- [x] 4.2 Rename `frontend/src/lib/InventoryRow.svelte` → `frontend/src/lib/ComponentRow.svelte`; update `inventory.embedded/duplicate/duplicateTitle` references to `components.*`.
- [x] 4.3 Rename `frontend/src/lib/InventoryToolbar.svelte` → `frontend/src/lib/ComponentToolbar.svelte`; remove `kind` prop, `onKindChange` prop, and the kind `<select>` element.
- [x] 4.4 Update all import sites of the three renamed components (search-and-replace `Inventory{List,Row,Toolbar}` → `Component{List,Row,Toolbar}`). Only remaining reference was in `InventoryPage.svelte`, which Phase 5 deletes.

## Phase 5: Shared Kind Page + Wrappers (RED → GREEN, leaves App.test.ts red until Phase 7)

- [x] 5.1 Create `frontend/src/lib/pages/ComponentKindPage.svelte` per design.md's `KindPageProps` contract (module-block exported type; instance `kind: ComponentKind`; `$derived` filtered `visible` list via `filterComponents`; renders `ComponentToolbar`, `IncidentIndicator`, `ComponentList`).
- [x] 5.2 Create `frontend/src/lib/IncidentIndicator.svelte` — discreet button, `data-testid="incident-indicator"`, hidden when `incidents === 0`, `onclick` navigates to `"scan"`.
- [x] 5.3 Create `frontend/src/lib/pages/AgentsPage.svelte` — two-line wrapper, `kind="agent"`, spreads `KindPageProps`.
- [x] 5.4 Create `frontend/src/lib/pages/SkillsPage.svelte` — two-line wrapper, `kind="skill"`, spreads `KindPageProps`.
- [x] 5.5 Delete `frontend/src/lib/pages/InventoryPage.svelte` (replaced by 5.1/5.3/5.4).

## Phase 6: Scan Page (leaves App.test.ts red until Phase 7)

- [x] 6.1 Rename `frontend/src/lib/ScanDiagnostics.svelte` → `frontend/src/lib/ScanIssueList.svelte` — keep `diagnostics: Diagnostics` prop and empty guard, but drop the `unavailableRoots` section; render only missing-client and recoverable sections.
- [x] 6.2 Create `frontend/src/lib/pages/ScanPage.svelte` — composes verdict (`scan.verdictHealthy`/`scan.verdictIssues`), full roots table (found/not found via `rootFound`/`rootNotFound`), installations summary, duration, and `ScanIssueList`; never renders blank.

## Phase 7: App.svelte Shell Wiring (GREEN — restores App.test.ts to green after edits)

- [x] 7.1 Edit `frontend/src/App.svelte` — replace single `query`/`kind` state with `agentsQuery` and `skillsQuery` (`$state`, independent, never reset on navigation); remove `kind` state and the `ComponentFilter`/kind-select wiring.
- [x] 7.2 Edit `frontend/src/App.svelte` — add `diagnostics = $derived(partitionDiagnostics(...))` and `incidents = $derived(...)` per design.md's Interfaces section; rename `loadInventory` → `runScan`.
- [x] 7.3 Edit `frontend/src/App.svelte` — rewrite the route branch table: `home` → `HomePage` with `status`/`failureMessage`/`incidents`/`onNavigate`/`onRetry`; `agents` → `AgentsPage`; `skills` → `SkillsPage`; `scan` → `ScanPage`; `subscriptions` unchanged; `{:else if !hasContent(route)}` → `PlaceholderPage`.

## Phase 8: Home Page (RED → GREEN)

- [x] 8.1 RED: extend `frontend/src/App.test.ts` (or a dedicated Home case) asserting Home's healthy/completed-with-issues/failed scan-status states, retry action, and CTA retarget to `agents` — see Phase 9 integration cases 1-2, 4-5, 8.
- [x] 8.2 GREEN: edit `frontend/src/lib/pages/HomePage.svelte` — add `status`, `failureMessage`, `incidents`, `onRetry` props; derive healthy/completed-with-issues/failed/pending state per design.md; render retry (failed) and link to `scan` (issues); retarget CTA to `agents` route with reworded `home.ctaBody`/`home.ctaAction`.

## Phase 9: Integration Tests — App.test.ts (RED → GREEN, full suite)

- [x] 9.1 Retarget every `navigateTo("Inventory")`/`navigateTo("Inventario")` call site (~10) to `"Skills"`, `"Agents"`, or `"Scan"`; update title assertions (`"— Inventory"` → `"— Skills"`/`"— Agents"`/`"— Scan"`, Spanish equivalents); update failure-copy assertions (`"Inventory scan failed."` → `"Scan failed."`).
- [x] 9.2 Update the sidebar-labels test to `["Home","Agents","Skills","MCP","Prompts","Scan","AI Subscriptions"]`; narrow the placeholder loop to `["MCP","Prompts"]`.
- [x] 9.3 Add: Agents route lists only `kind === "agent"`, Skills only `kind === "skill"`; `scan` called once, `rescan` never.
- [x] 9.4 Add: a query typed on Agents survives Home → Agents; Skills search field stays empty with its own unfiltered list.
- [x] 9.5 Add: neither page renders `select[aria-label]` other than the language selector.
- [x] 9.6 Add (**correctness-critical**): `issues: []` with one `rootsScanned` entry `status: "notFound"` → `[data-testid="incident-indicator"]` present on Agents AND Skills; clicking it yields `document.title === "Vertice v0.1.0 — Scan"`.
- [x] 9.7 Add: non-empty `issues`, all roots found → indicator on both pages; fully clean report → indicator absent on both.
- [x] 9.8 Add: Scan route, clean report — roots, installations, duration, `scan.verdictHealthy` all rendered, panel never blank.
- [x] 9.9 Add: Scan route, `mixedReportFixture` — each diagnostic rendered exactly once, `search root claude-skills was not found` never appears.
- [x] 9.10 Add: `scan` rejects → Home shows failed state and retry, `home.statsPending` (`"—"`) absent; clicking retry calls `rescan` once.
- [x] 9.11 Add: locale switch on Agents re-renders indicator copy in Spanish while component payloads stay verbatim.
- [x] 9.12 Replace "opens the inventory from the greeting page call to action" with "opens the agents page from the greeting call to action" (clicks `"Open agents"`, asserts `"— Agents"`); replace "renders no diagnostics for a clean report" with the healthy-scan-route test (9.8); replace "keeps the inventory filter when navigating away and back" with the query-independence test (9.4).

## Phase 10: Final Gate

- [x] 10.1 From `frontend/`, run `npm run lint && npm run check && npm run test && npm run build` — all four MUST pass. Run vitest from `frontend/`, never from `src/` (stray `node_modules` gotcha).
- [x] 10.2 Confirm no hand-edits landed in `frontend/src/bindings/` and no Rust files changed (frontend-only change; bindings/CI bindings-in-sync step must stay green).
