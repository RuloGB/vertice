# Design: Split Inventory into Agents, Skills, and Scan Pages

Frontend-only. No Rust, no IPC, no binding, no capability change. Svelte 5 runes, Tailwind 4,
existing project idiom (explicit props, no rest-spread pages, no component-level test harness).

## Technical Approach

`App.svelte` stays the single owner of the scan lifecycle and of the one `ScanReport`. It gains two
independent query values and derives the diagnostics partition once, then fans the report out to
three routes. One internal `ComponentKindPage.svelte` is parameterized by `kind: ComponentKind`;
`AgentsPage.svelte` / `SkillsPage.svelte` are literal wrappers. `filterComponents`,
`ComponentList`, `ComponentRow`, `LocationList` are reused with behavior unchanged. `ScanPage.svelte`
composes the full report and always renders a verdict.

## Architecture Decisions

### Decision: Shared kind page with typed props exported from the module block

**Choice**: `frontend/src/lib/pages/ComponentKindPage.svelte`, props:

```ts
// <script module lang="ts"> in ComponentKindPage.svelte
export type KindPageProps = {
  status: "idle" | "loading" | "ready" | "failed";
  report: ScanReport | null;
  failureMessage: string | null;
  query: string;
  incidents: number;                       // pre-computed by the shell
  onQueryChange: (query: string) => void;
  onReload: () => void;
  onNavigate: (route: RouteId) => void;    // indicator target: "scan"
};
```

The instance block adds `kind: ComponentKind` and derives everything else:

```ts
const KIND_ROUTE = { agent: "agents", skill: "skills" } as const satisfies Record<ComponentKind, RouteId>;
const visible = $derived(filterComponents(report?.components ?? [], { kind, query }));
```

Wrappers are two lines: `let props: KindPageProps = $props();` then
`<ComponentKindPage kind="agent" {...props} />`. Spread is used *only* in the wrappers, where the
prop set is provably identical.

**Alternatives considered**: (a) no wrappers, one `App.svelte` branch handling both routes with a
ternary `kind`; (b) explicit forwarding of all 8 props in each wrapper; (c) props type in a separate
`componentKindPage.ts`.
**Rationale**: (a) collapses the route table into a conditional and was excluded by the approved
approach. (b) is 16 lines of noise that drift silently. `ComponentKind` being a closed
`"skill" | "agent"` makes `KIND_ROUTE` exhaustiveness-checked at compile time. If `svelte-check`
rejects the type import from the module block, fall back to (c) — no other change.

### Decision: Rename the view components, keep `lib/inventory.ts`

| Old | New | Why |
|---|---|---|
| `lib/InventoryList.svelte` | `lib/ComponentList.svelte` | named after the retired route surface |
| `lib/InventoryRow.svelte` | `lib/ComponentRow.svelte` | idem |
| `lib/InventoryToolbar.svelte` | `lib/ComponentToolbar.svelte` | already being edited (kind `<select>` removed) |
| catalog `inventory.*` | catalog `components.*` | 5 keys, 2 locales, 4 call sites — mechanical |
| `lib/pages/InventoryPage.svelte` | deleted | replaced by the kind page |
| `lib/inventory.ts` (`isDuplicate`) | **unchanged** | see below |

**Alternatives considered**: leave all `Inventory*` names in place; or purge the word everywhere
including `lib/inventory.ts` and the `inventory-ui` capability.
**Rationale**: what is retired is the *route/view* named Inventory, not the domain word. The
capability stays `inventory-ui` (proposal decision 6) and the product tagline is still "AI component
inventory". Renaming the three `.svelte` files costs ~6 import lines and zero test-string churn
because those tests assert rendered text, not module names — cheap enough that leaving stale names
is not worth it. `lib/inventory.ts` is a pure domain predicate with no view coupling and a paired
`inventory.test.ts`; renaming it moves two files for no reader benefit, so it stays.

### Decision: `incidentCount` is a pure function next to `partitionDiagnostics`

```ts
// frontend/src/lib/scanDiagnostics.ts
export function incidentCount(diagnostics: Diagnostics): number {
  return (
    diagnostics.unavailableRoots.length +
    diagnostics.missingClientIssues.length +
    diagnostics.remainingRecoverableIssues.length
  );
}
```

**Alternatives considered**: recompute `issues.length + rootsScanned.filter(notFound).length` at each
call site; a boolean `hasIncidents(report)` taking the raw report.
**Rationale**: the naive sum double-counts, because `partitionDiagnostics` deliberately drops the
`search root {id} was not found` warnings from `remainingRecoverableIssues`. The three partition
lengths are exactly "issues plus not-found roots, de-duplicated". Proof that it matches the spec
predicate: a warning is only dropped when a matching `notFound` root exists, so `issues` non-empty
⇒ count > 0, and any `notFound` root ⇒ count ≥ 1, and clean ⇒ 0. Home needs the same number for its
issue count, so a boolean would force a second traversal. The shell derives it once and passes
`incidents: number` down; no page re-partitions.

### Decision: `ScanDiagnostics` is narrowed and wrapped, not extended

`ScanDiagnostics.svelte` → `ScanIssueList.svelte`: keeps its `diagnostics: Diagnostics` prop and its
"render nothing when there is nothing" guard, but the guard drops `unavailableRoots` and it renders
only the missing-client and recoverable sections. `ScanPage.svelte` is the wrapper that supplies
everything else.

**Alternatives considered**: extend `ScanDiagnostics` to render verdict + roots + installations +
duration + issues and invert its guard.
**Rationale**: extending makes it a god-component and destroys the one thing it does well. The
`unavailableRoots` section moves into the roots table because `ScanPage` must list *every* root with
its status — keeping both would be exactly the duplicate root diagnostic the spec forbids.

## Data Flow

    scan()/rescan()  ──→  App.svelte  ──┬─→ HomePage        (status, failureMessage, incidents, onRetry)
                          report        ├─→ AgentsPage ─┐
                          diagnostics   ├─→ SkillsPage ─┴→ ComponentKindPage → ComponentToolbar
                          incidents     │                                     → ComponentList → ComponentRow
                                        │                                     → IncidentIndicator ──┐
                                        └─→ ScanPage → ScanVerdict / roots / installations /        │
                                                       duration / ScanIssueList                     │
                                             ▲──────── onNavigate("scan") ─────────────────────────┘

## File Changes

| File | Action | Description |
|---|---|---|
| `frontend/src/lib/pages/ComponentKindPage.svelte` | Create | Shared per-kind page (title, toolbar, indicator, lifecycle, list) |
| `frontend/src/lib/pages/AgentsPage.svelte` | Create | `kind="agent"` wrapper |
| `frontend/src/lib/pages/SkillsPage.svelte` | Create | `kind="skill"` wrapper |
| `frontend/src/lib/pages/ScanPage.svelte` | Create | Verdict, all roots with status, installations, duration, `ScanIssueList` |
| `frontend/src/lib/IncidentIndicator.svelte` | Create | Discreet button, `data-testid="incident-indicator"`, hidden when `incidents === 0` |
| `frontend/src/lib/pages/InventoryPage.svelte` | Delete | Replaced |
| `frontend/src/lib/ScanDiagnostics.svelte` | Rename → `ScanIssueList.svelte` | Roots section removed; guard on the two issue arrays |
| `frontend/src/lib/InventoryList.svelte` | Rename → `ComponentList.svelte` | Keys `inventory.empty` → `components.empty` |
| `frontend/src/lib/InventoryRow.svelte` | Rename → `ComponentRow.svelte` | `inventory.embedded/duplicate/duplicateTitle` → `components.*` |
| `frontend/src/lib/InventoryToolbar.svelte` | Rename → `ComponentToolbar.svelte` | `kind` prop, `onKindChange` prop and the `<select>` removed |
| `frontend/src/lib/scanDiagnostics.ts` | Modify | Add `incidentCount` |
| `frontend/src/lib/navigation.ts` | Modify | `inventory` → `scan` in `ROUTE_IDS`/`NAV_GROUPS.data`; `ROUTES_WITH_CONTENT` = home, agents, skills, scan, subscriptions; update the module doc |
| `frontend/src/lib/NavIcon.svelte` | Modify | `inventory` branch → `scan` (magnifier: `<circle cx="11" cy="11" r="7"/><path d="m16.5 16.5 4 4"/>`) |
| `frontend/src/App.svelte` | Modify | Two queries, `diagnostics`/`incidents` derived, new branch table, `loadInventory` → `runScan`, drop `kind` state and the `ComponentFilter` import |
| `frontend/src/lib/pages/HomePage.svelte` | Modify | Scan-status block + retry + link to `scan`; CTA retargeted to `agents` |
| `frontend/src/lib/i18n/catalogs.ts` | Modify | Key migration below |
| `frontend/src/lib/{navigation,scanDiagnostics}.test.ts`, `i18n/locale.test.ts`, `src/App.test.ts` | Modify | See Testing Strategy |

`frontend/src/lib/{filterComponents,inventory,scan,appTitle,subscriptions}.ts`, `LocationList.svelte`,
`Sidebar.svelte`, `PlaceholderPage.svelte`, `SubscriptionsPage.svelte` are untouched.

## Interfaces / Contracts

`App.svelte`:

```ts
let agentsQuery = $state("");
let skillsQuery = $state("");            // never shared, never reset on navigation
const diagnostics = $derived(partitionDiagnostics(report?.rootsScanned ?? [], report?.issues ?? []));
const incidents = $derived(report === null ? 0 : incidentCount(diagnostics));
```

Branch table: `home` → `HomePage {report} {status} {failureMessage} {incidents} onNavigate
onRetry={() => void runScan("reload")}`; `agents` → `AgentsPage query={agentsQuery}
onQueryChange={(v) => (agentsQuery = v)} …`; `skills` → `SkillsPage query={skillsQuery} …`; `scan` →
`ScanPage {status} {report} {failureMessage} {diagnostics} {incidents}`; `subscriptions` →
`SubscriptionsPage`; `{:else if !hasContent(route)}` → `PlaceholderPage`.

`HomePage` gains `status`, `failureMessage`, `incidents`, `onRetry`. Its state is derived, not
passed: `status === "failed"` → failed (retry, never `home.statsPending`); `status === "ready" &&
incidents === 0` → healthy; `status === "ready"` → completed-with-issues (count + `durationMs` +
link to `scan`); otherwise pending.

## i18n Key Migration (`en` and `es`, both locales in lockstep)

**Removed**: `nav.inventory`, `area.inventory` (fall out automatically from `Record<RouteId, string>`),
`toolbar.kindAriaLabel`, `toolbar.allKinds`, `diagnostics.unavailableRoots`.

**Renamed (key changes, value kept)**: `inventory.loading|empty|duplicate|duplicateTitle|embedded`
→ `components.*`.

**Re-worded (key kept, value changes)**: `failure.title` → "Scan failed." / "Falló el escaneo.";
`home.ctaTitle` → "Browse your components" / "Explora tus componentes"; `home.ctaBody` → "Agents and
Skills are backed by the startup scan." / "Agents y Skills se apoyan en el escaneo de arranque.";
`home.ctaAction` → "Open agents" / "Abrir agentes".

**Added**:

| Key | en | es |
|---|---|---|
| `nav.scan` / `area.scan` | Scan | Escaneo |
| `scan.verdictHealthy` | Scan completed with no incidents. | El escaneo terminó sin incidencias. |
| `scan.verdictIssues` | Scan completed with {count} incidents. | El escaneo terminó con {count} incidencias. |
| `scan.rootsTitle` | Scan roots | Raíces de escaneo |
| `scan.rootFound` / `scan.rootNotFound` | Found / Not found | Encontrada / No encontrada |
| `scan.installationsTitle` | Detected installations | Instalaciones detectadas |
| `scan.installationsEmpty` | No supported client installation was detected. | No se detectó ninguna instalación de cliente compatible. |
| `scan.durationLabel` / `scan.durationValue` | Duration / {ms} ms | Duración / {ms} ms |
| `incident.label` | Scan incidents | Incidencias del escaneo |
| `incident.count` | {count} scan incidents | {count} incidencias del escaneo |
| `incident.action` | Open the scan report | Abrir el informe del escaneo |
| `home.scanTitle` | Last scan | Último escaneo |
| `home.scanHealthy` | Healthy — no incidents. | Correcto: sin incidencias. |
| `home.scanIssues` | Completed with {count} incidents in {ms} ms. | Terminó con {count} incidencias en {ms} ms. |
| `home.scanFailed` | The scan failed. | El escaneo falló. |
| `home.scanRetry` | Retry scan | Reintentar escaneo |
| `home.scanOpen` | Open scan report | Abrir informe del escaneo |
| `home.scanPending` | Scanning... | Escaneando... |

`kind.skill` / `kind.agent` stay — `ComponentRow` still renders the kind badge.
`diagnostics.title|missingClient|recoverableIssues` stay, consumed by `ScanIssueList`.

## Testing Strategy

Strict TDD (`openspec/config.yaml: strict_tdd: true`). RED before GREEN, in this order. There is no
component-level test harness in this project — every UI assertion goes through `App.test.ts` mounting
the real shell with `./lib/scan` mocked. Do not introduce `@testing-library/svelte`.

| # | Layer | File | First assertions (RED) |
|---|---|---|---|
| 1 | Unit | `lib/navigation.test.ts` | `isRouteId("scan") === true`, `isRouteId("inventory") === false`; `hasContent` true for home/agents/skills/scan/subscriptions, false for mcp/prompts; group coverage unchanged |
| 2 | Unit | `lib/scanDiagnostics.test.ts` | `incidentCount`: clean → `0`; **zero `issues` + one `notFound` root → `1`**; one `notFound` root + its de-duplicated warning + one real issue → `2`, not `3`; missing-client only → `1` |
| 3 | Unit | `lib/i18n/locale.test.ts` | retired keys absent from both catalogs (`nav.inventory`, `area.inventory`, `toolbar.allKinds`, `toolbar.kindAriaLabel`, `diagnostics.unavailableRoots`, the whole `inventory` namespace); `components.duplicateTitle` interpolation replaces the `inventory.duplicateTitle` case; spot checks on `scan.*`, `incident.*`, `home.scan*`. Existing en↔es parity and non-blank tests need no edit and guard the Spanish additions |
| 4 | Integration | `src/App.test.ts` | see below |

`App.test.ts` churn — every `navigateTo("Inventory")` / `navigateTo("Inventario")` call site
(~10) retargets to `"Skills"` (the `componentFixture` is `skill:formatter`), `"Agents"` (the
agent-kind fixtures), or `"Scan"`. Title assertions `"— Inventory"` / `"— Inventario"` become
`"— Skills"` / `"— Skills"`, `"— Scan"` / `"— Escaneo"`. `"Inventory scan failed."` /
`"escaneo del inventario."` become `"Scan failed."` / `"Falló el escaneo."`. The sidebar-labels test
expects `["Home","Agents","Skills","MCP","Prompts","Scan","AI Subscriptions"]`. The placeholder loop
narrows to `["MCP","Prompts"]`. "opens the inventory from the greeting page call to action" becomes
"opens the agents page from the greeting call to action", clicking `"Open agents"` and asserting
`"— Agents"`. "renders no diagnostics for a clean report" is replaced by the healthy-scan-route test
below. "keeps the inventory filter when navigating away and back" becomes the query-independence test.

New integration cases:

1. Agents route lists only `kind === "agent"`, Skills only `kind === "skill"`; `scan` called once,
   `rescan` never.
2. A query typed on Agents survives Home → Agents, and the Skills search field is still empty with
   its own unfiltered list.
3. Neither page renders `select[aria-label]` other than the language selector.
4. **Correctness-critical**: `issues: []` with one `rootsScanned` entry `status: "notFound"` →
   `[data-testid="incident-indicator"]` present on Agents *and* on Skills; clicking it yields
   `document.title === "Vertice v0.1.0 — Scan"`.
5. Non-empty `issues`, all roots found → indicator on both pages. Fully clean report → indicator
   absent on both.
6. Scan route, clean report: roots, installations, duration and `scan.verdictHealthy` all rendered;
   the panel is never blank.
7. Scan route, `mixedReportFixture`: each diagnostic rendered exactly once and
   `search root claude-skills was not found` never appears (kept from the current suite, retargeted).
8. `scan` rejects → Home shows the failed state and a retry, `home.statsPending` (`"—"`) absent;
   clicking retry calls `rescan` once.
9. Locale switch on Agents re-renders the indicator copy in Spanish while component payloads stay
   verbatim.

Gates: `npm run lint && npm run check && npm run test && npm run build` from `frontend/`
(run vitest from `frontend/`, never from `src/`). No Rust gate is affected; bindings are untouched
so the CI bindings-in-sync step stays green.

## Migration / Rollout

No data migration. Route `inventory` is deleted with no redirect (spec: Inventory Route Removal);
route state is in-memory only, so no persisted value can reference it. Single PR, single-layer
revert.

## Open Questions

None. All product decisions were settled in the proposal's `## Decisions` section; the CTA target
(`agents`) and the scan icon glyph are design-level choices inside the latitude the spec grants.
