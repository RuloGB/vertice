# Delta for Inventory UI

## ADDED Requirements

### Requirement: Inventory Route Removal

The frontend MUST NOT expose an `inventory` route. Removal MUST be a deletion, not a redirect to `agents`, `skills`, or `scan` — no navigation entry, page component, or `nav.inventory`/`area.inventory` catalog key MUST remain reachable.

#### Scenario: Inventory route no longer exists
- GIVEN the application is running with this change applied
- WHEN the navigation model and route table are inspected
- THEN no route id `inventory` exists
- AND no automatic redirect from a former `inventory` reference is performed

### Requirement: Home CTA Reflects Multiple Live Sections

The Home page CTA MUST link to a route that still reflects the live startup scan (`agents`, `skills`, or `scan`) and its copy MUST NOT claim that a single named section is the only place live scan data appears, since the scan now backs three routes.

#### Scenario: CTA copy matches the split
- GIVEN Home renders its call-to-action
- WHEN the CTA body text is inspected
- THEN it does not claim exclusivity over one section as the sole live-scan view
- AND activating the CTA navigates to a route backed by the startup scan

### Requirement: Per-Kind Agents and Skills Pages

The frontend MUST render two dedicated routes, `agents` and `skills`, each listing only the components from the single startup `ScanReport` whose `kind` matches that route (`agent` or `skill` respectively). Neither route MUST trigger its own scan; both MUST consume the report already held by the shell.

#### Scenario: Agents page shows only agents
- GIVEN a loaded report with both `agent` and `skill` components
- WHEN the user navigates to the `agents` route
- THEN only components with `kind === "agent"` are listed
- AND no additional `scan` or `rescan` invocation occurs

#### Scenario: Skills page shows only skills
- GIVEN a loaded report with both `agent` and `skill` components
- WHEN the user navigates to the `skills` route
- THEN only components with `kind === "skill"` are listed
- AND no additional `scan` or `rescan` invocation occurs

### Requirement: Independent Per-Page Search State

The Agents page and Skills page MUST each hold their own search query state, independent of one another. Navigating between them or typing a query on one page MUST NOT alter the other page's query or filtered results.

#### Scenario: Query does not leak across pages
- GIVEN the user enters a search query on the Agents page
- WHEN the user navigates to the Skills page
- THEN the Skills page search field is unaffected by the Agents query
- AND the Skills page shows its own unfiltered or independently-filtered list

### Requirement: Full Scan Report Route

The frontend MUST provide a `scan` route rendering the complete `ScanReport`: every entry of `rootsScanned` regardless of `status` (including `notFound`), the detected installations/components summary, `durationMs`, and all `issues`. This route MUST render a result for a healthy scan (zero issues, all roots found) as well as for a scan with issues, never an empty surface.

#### Scenario: Healthy scan is visible
- GIVEN a successful report with all roots found and zero issues
- WHEN the user navigates to the `scan` route
- THEN the roots, installations, and duration are rendered
- AND a healthy/clean verdict is shown instead of a blank panel

#### Scenario: Scan with issues
- GIVEN a successful report with one or more `issues`
- WHEN the user navigates to the `scan` route
- THEN the roots, installations, duration, and every issue are rendered

### Requirement: Incident Indicator on List Pages

The Agents and Skills pages MUST each show a discreet incident indicator when the current report has a non-empty `issues` list OR at least one entry in `rootsScanned` with `status === "notFound"`. Activating the indicator MUST navigate to the `scan` route. The indicator MUST NOT appear when `issues` is empty and every root has a status other than `notFound`.

#### Scenario: Indicator from issues
- GIVEN a report with a non-empty `issues` list and all roots found
- WHEN the Agents or Skills page is rendered
- THEN the incident indicator is shown on that page
- AND activating it navigates to the `scan` route

#### Scenario: Indicator from a not-found root with zero issues
- GIVEN a report with `issues: []` and one root in `rootsScanned` with `status === "notFound"`
- WHEN the Agents or Skills page is rendered
- THEN the incident indicator is still shown on both pages
- AND activating it navigates to the `scan` route

#### Scenario: No indicator on a fully healthy report
- GIVEN a report with `issues: []` and every root found
- WHEN the Agents or Skills page is rendered
- THEN no incident indicator is shown

### Requirement: Home Scan-Status Block

The Home page MUST render a scan-status block reflecting one of three states derived from the last scan attempt: healthy (successful, no issues, no not-found roots), completed with issues (successful, but issues present or a root not found), or failed (the scan/rescan invocation rejected). The failed state MUST offer a retry action that invokes `rescan`. Home MUST NOT show a pending placeholder once a scan attempt has settled, whether it succeeded or failed.

#### Scenario: Healthy status
- GIVEN the startup scan succeeded with no issues and all roots found
- WHEN Home renders
- THEN the scan-status block shows the healthy state
- AND no retry action is offered

#### Scenario: Completed-with-issues status
- GIVEN the startup scan succeeded but reported issues or a not-found root
- WHEN Home renders
- THEN the scan-status block shows the completed-with-issues state with issue count and duration
- AND a link to the `scan` route is offered

#### Scenario: Failed status with retry
- GIVEN the startup scan invocation rejected
- WHEN Home renders
- THEN the scan-status block shows the failed state, never a pending placeholder
- AND a retry action is offered
- AND activating retry invokes `rescan`

## MODIFIED Requirements

### Requirement: Startup Inventory Rendering

On startup, the frontend MUST invoke `scan` without requiring configuration. The resulting `ScanReport` MUST be held once by the shell and MUST be the sole source of data for the Agents page, the Skills page, and the `scan` route — no page MUST perform its own scan. The Agents page MUST render one row per `ScanReport.components` entry with `kind === "agent"`; the Skills page MUST render one row per entry with `kind === "skill"`. Each row SHALL expose the component name, kind, description when present, and its origin locations.

(Previously: startup scan populated a single combined `inventory` route showing all components regardless of kind.)

#### Scenario: Successful startup scan feeds both pages
- GIVEN the application starts with a successful report containing both agent and skill components
- WHEN the startup scan completes
- THEN the Agents page displays one row per agent component and the Skills page displays one row per skill component
- AND each row displays its name, kind, description, and origin locations

### Requirement: View-Only Filtering and Search

Each of the Agents page and Skills page MUST provide a name search over its own kind-filtered subset of the in-memory report components. Search MUST NOT invoke another scan and MUST NOT expose a kind selector, since each page already scopes to one kind.

(Previously: a single combined view exposed both a kind `<select>` filter and a name search over all components.)

#### Scenario: Search within one kind
- GIVEN the Agents page shows multiple agent components
- WHEN the user enters a name query
- THEN only matching agent components are shown
- AND the underlying report remains unchanged

#### Scenario: No kind selector is present
- GIVEN either the Agents page or the Skills page is rendered
- WHEN the toolbar is inspected
- THEN no kind `<select>` control exists on either page

### Requirement: Reload Lifecycle

Each of the Agents page and Skills page MUST provide reload that invokes `rescan`, replacing the shell's held report with the resulting successful report; both pages and the `scan` route MUST reflect the new report on the next render. Each page MUST show a loading state from invocation start until settlement.

(Previously: reload lived on the single combined `inventory` route.)

#### Scenario: Reload from either page refreshes shared state
- GIVEN a previously loaded report
- WHEN the user activates reload from the Agents page
- THEN `rescan` is invoked and a loading state is shown on the Agents page
- AND the new report replaces the prior report for the Skills page and the `scan` route as well

### Requirement: Minimal Lifecycle States

The Agents page, Skills page, and `scan` route MUST each distinguish loading, hard scan failure, and successful empty results using the shell's single scan/rescan lifecycle. A hard failure MUST show a minimal non-blank failure surface on the active page; an empty successful kind subset MUST show an empty list region distinct from failure. Those lifecycle surfaces MUST render through frontend i18n messages while preserving raw diagnostic passthrough data when present.

(Previously: lifecycle states applied to the single combined `inventory` route.)

#### Scenario: Hard failure on a list page
- GIVEN `scan` or `rescan` rejects with a scan error
- WHEN the invocation settles while the Agents or Skills page is active
- THEN that page leaves loading and shows a minimal failure state without crashing

#### Scenario: Empty successful kind subset
- GIVEN a successful report with zero components of one kind
- WHEN the corresponding page is rendered
- THEN that page shows an empty list region distinct from failure

### Requirement: Localized Inventory Chrome

The Agents page, Skills page, and `scan` route MUST render all user-facing chrome through the active frontend i18n catalog, including toolbar labels, placeholders, loading, empty, failure, duplicate, title, aria, null-path copy, diagnostic labels, incident-indicator copy, and embedded/non-actionable status. It MUST update that chrome when the active locale changes and MUST NOT localize payload fields from the scan report.

(Previously: chrome applied to the single combined `inventory` route and did not cover an incident indicator.)

#### Scenario: Chrome follows locale changes
- GIVEN a loaded Agents page in English
- WHEN the active locale changes to Spanish
- THEN all Agents-page chrome, including the incident indicator, is re-rendered in Spanish
- AND component payload fields remain unchanged

### Requirement: Non-Blocking Successful Scan Diagnostics

The `scan` route MUST render the full report — roots scanned (found and not found), detected installations, duration, and every issue — for both healthy and unhealthy successful scans. Root availability MUST derive only from `rootsScanned`; recoverable issues MUST derive only from `issues`; a missing-client notice MUST be discreet. The route MUST NOT duplicate a root diagnostic and MUST NOT treat a report with diagnostics as a hard scan failure. The Agents and Skills pages MUST NOT render this full-report diagnostics panel themselves; they only surface the incident indicator that links to the `scan` route.

(Previously: diagnostics rendered inline on the combined `inventory` route and only for non-empty diagnostic cases, so a fully clean scan rendered nothing.)

#### Scenario: Mixed successful report on the scan route
- GIVEN a successful report has components, an unavailable root, a missing client, and a recoverable issue
- WHEN the `scan` route is rendered
- THEN all roots (found and not found), installations, duration, and each applicable diagnostic are rendered without a duplicate root warning

#### Scenario: Clean successful report on the scan route
- GIVEN a successful report has no unavailable roots, missing clients, or recoverable issues
- WHEN the `scan` route is rendered
- THEN the roots, installations, and duration are still rendered
- AND a healthy verdict is shown instead of an empty panel

### Requirement: Embedded Component State

The Agents page and Skills page MUST each visibly mark a component as embedded and non-actionable when at least one of its locations has `origin` equal to `embedded`. This MUST be determined from location origin, not from whether a path is null, and MUST NOT invent an action for the component.

(Previously: applied to rows on the single combined `inventory` route.)

#### Scenario: Embedded component with a path
- GIVEN an agent component has an embedded location with a non-null path
- WHEN its row is rendered on the Agents page
- THEN the row shows embedded/non-actionable status
- AND no action control is introduced

#### Scenario: Nullable non-embedded path
- GIVEN a skill component has a non-embedded location with a null path
- WHEN its row is rendered on the Skills page
- THEN the row remains safely renderable
- AND it is not marked embedded solely because its path is null

Note: "Duplicate Rows and Complete Paths" and "Read-Only Frontend Boundary" are unaffected by this change — their behavior is unchanged (both apply, unmodified, to the Agents and Skills pages via the reused `InventoryRow`/`LocationList` components) and they are intentionally omitted from this delta.
