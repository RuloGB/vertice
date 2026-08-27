# Inventory UI Specification

## Purpose

Define the read-only frontend inventory presentation over the existing typed `scan` and `rescan` commands. This is a new capability for T11, covering CA-1 and the visual half of CA-3. `add-client-version-freshness` (2026-08-24) added a freshness badge to the clients view and the rule that `Outdated` is never an incident.

## Requirements

### Requirement: Detail Pages Summarize AI Clients By Location Ownership

The AgentDetail, SkillDetail, and McpDetail pages MUST consume each
`Location.client` value and render a deduplicated client summary. Groups MUST
appear in fixed order: `claudeCode`, `openCode`, `codex`, then shared (`null`),
and MUST include the number of locations in each group. Groups with no
locations MUST NOT be rendered.

#### Scenario: Locations are deduplicated, ordered, and counted

- GIVEN a component with locations in arbitrary order, including repeated clients
- WHEN any detail page computes its AI-client groups
- THEN each distinct client appears once with its location count
- AND the order is Claude Code, OpenCode, Codex, then Shared

#### Scenario: Shared locations use localized common-noun copy

- GIVEN a component has a location whose `client` is `null`
- WHEN its AI-client group is rendered
- THEN the group label comes from the i18n key `aiClients.shared`
- AND the English and Spanish values are “Shared” and “Compartido” respectively

#### Scenario: Client names remain proper nouns

- GIVEN a group whose client is `claudeCode`, `openCode`, or `codex`
- WHEN its label is rendered in either supported locale
- THEN it is the hardcoded proper noun “Claude Code”, “OpenCode”, or “Codex”
- AND no client display name is looked up as an i18n key

#### Scenario: Existing empty state is preserved

- GIVEN an Agent, Skill, or MCP component has no locations
- WHEN its detail page renders the AI Clients section
- THEN the existing localized empty-state message is shown
- AND no client group row is fabricated

### Requirement: Startup Inventory Rendering

On startup, the frontend MUST invoke `scan` without requiring configuration. The resulting `ScanReport` MUST be held once by the shell and MUST be the sole source of data for the Agents page, the Skills page, and the `scan` route — no page MUST perform its own scan. The Agents page MUST render one row per `ScanReport.components` entry with `kind === "agent"`; the Skills page MUST render one row per entry with `kind === "skill"`. Each row SHALL expose the component name, kind, description when present, and its origin locations.

#### Scenario: Successful startup scan feeds both pages
- GIVEN the application starts with a successful report containing both agent and skill components
- WHEN the startup scan completes
- THEN the Agents page displays one row per agent component and the Skills page displays one row per skill component
- AND each row displays its name, kind, description, and origin locations

### Requirement: Duplicate Rows and Complete Paths

The UI MUST mark a component as duplicated only when the same AI client can
consume both a shared-root copy and that client’s specific-root copy. Copies
that exist only across distinct client-specific roots MUST NOT be marked as
duplicated. The UI MUST disclose every location path, including nullable paths,
and MUST NOT regroup components by name or compare file contents.

#### Scenario: Shared plus consuming client-specific copy is duplicated
- GIVEN a component has at least one shared location and at least one location
  for the same client
- WHEN its row is rendered and its locations are disclosed
- THEN the row shows a duplicate mark
- AND all location entries remain visible

#### Scenario: Distinct client-specific copies are not duplicates
- GIVEN a component has only client-specific locations for different clients
- WHEN its row is rendered
- THEN the row does not show a duplicate mark
- AND all location entries remain visible

#### Scenario: Nullable location path
- GIVEN a component contains a location whose path is null
- WHEN its locations are rendered
- THEN the row remains renderable and represents that path safely without inventing a filesystem action

### Requirement: View-Only Filtering and Search

Each of the Agents page and Skills page MUST provide a name search over its own kind-filtered subset of the in-memory report components. Search MUST NOT invoke another scan and MUST NOT expose a kind selector, since each page already scopes to one kind.

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

#### Scenario: Reload from either page refreshes shared state
- GIVEN a previously loaded report
- WHEN the user activates reload from the Agents page
- THEN `rescan` is invoked and a loading state is shown on the Agents page
- AND the new report replaces the prior report for the Skills page and the `scan` route as well

### Requirement: Minimal Lifecycle States

The Agents page, Skills page, and `scan` route MUST each distinguish loading, hard scan failure, and successful empty results using the shell's single scan/rescan lifecycle. A hard failure MUST show a minimal non-blank failure surface on the active page; an empty successful kind subset MUST show an empty list region distinct from failure. Those lifecycle surfaces MUST render through frontend i18n messages while preserving raw diagnostic passthrough data when present.

#### Scenario: Hard failure on a list page
- GIVEN `scan` or `rescan` rejects with a scan error
- WHEN the invocation settles while the Agents or Skills page is active
- THEN that page leaves loading and shows a minimal failure state without crashing

#### Scenario: Empty successful kind subset
- GIVEN a successful report with zero components of one kind
- WHEN the corresponding page is rendered
- THEN that page shows an empty list region distinct from failure

### Requirement: Localized Inventory Chrome

The Agents page, Skills page, and `scan` route MUST render all user-facing chrome through the active frontend i18n catalog, including toolbar labels, placeholders, loading, empty, failure, duplicate, title, aria, null-path copy, diagnostic labels, incident-indicator copy, embedded/non-actionable status, and the supported-clients table chrome (title, per-status labels, unavailable-version copy, unsupported-platform message). It MUST update that chrome when the active locale changes and MUST NOT localize payload fields from the scan report or client slot labels.

#### Scenario: Chrome follows locale changes

- GIVEN a loaded Agents page in English
- WHEN the active locale changes to Spanish
- THEN all Agents-page chrome, including the incident indicator, is re-rendered in Spanish
- AND component payload fields remain unchanged

#### Scenario: Supported-clients table chrome is localized, labels are not

- GIVEN the `scan` route renders the supported-clients table in English
- WHEN the active locale changes to Spanish
- THEN the table title, status words, and unavailable-version copy re-render in Spanish
- AND each row's client label text stays byte-identical in both locales

### Requirement: Read-Only Frontend Boundary

The inventory UI MUST use only the existing typed scan wrapper and report data. It MUST NOT access the filesystem, write data, install components, add watchers, schedule interval refreshes, or auto-refresh.

#### Scenario: No watcher or filesystem behavior
- GIVEN the inventory UI is running
- WHEN no user invokes reload
- THEN no watcher, timer-based refresh, filesystem API, or write operation is performed

### Requirement: Always-Visible Supported Clients Table Replaces The Detected-Installations Panel

The `scan` route MUST render a "Supported clients" table with exactly one row per entry in `ScanReport.clientPresence` when it is non-`null` — client label, status (`detected`/`notDetected`, localized chrome word), and version(s) when the record has at least one installation. This table REPLACES the prior "Detected installations" panel entirely; the two MUST NOT be rendered together, since doing so would render the same fact twice on the one route whose defect was user confusion. The table MUST NOT render a `path` column; each version cell MUST instead carry that installation's path as a `title` tooltip, so CA-7's per-installation path remains inspectable. A `Detected` row with an empty `installations` MUST render the localized `scan.clientVersionUnavailable` copy in place of a version, never a blank cell and never `notDetected` styling. A row's version cell MUST show every entry in that record's `installations`, never merged or reduced to one, when there are two or more (CA-7).

#### Scenario: Three-row table on Windows, mixed status

- GIVEN a Windows report with three `clientPresence` entries, two `Detected` (each with one installation) and one `NotDetected`
- WHEN the `scan` route is rendered
- THEN the table shows exactly three rows with client label, localized status, and version(s) where present
- AND no `path` column is present, and no separate "Detected installations" panel is rendered

#### Scenario: Two coexisting versions render in one row with path tooltips

- GIVEN a `clientPresence` entry whose `installations` has two `ClientInstallation` values at different versions and different paths
- WHEN that row is rendered
- THEN both versions are shown inside that single row, neither merged nor reduced to one
- AND each version's cell carries its own installation path as a `title` tooltip

#### Scenario: Detected but broken renders an unavailable-version row, and still counts as an incident

- GIVEN a `clientPresence` entry with `status: Detected` and empty `installations`, alongside an `Error` `ScanIssue` for that same slot
- WHEN the `scan` route is rendered
- THEN that row shows the `Detected` status with the localized unavailable-version copy, never `notDetected`
- AND the incident indicator still lights on the Agents and Skills pages, because the `Error` issue is present

### Requirement: Freshness Badge On The Clients View

The view listing detected client installations MUST render a freshness badge beside each installation's version, driven by the freshness report, with exactly four visual states: up to date, outdated, unknown, and pending (shown while the freshness lookup for that entry is in flight and no verdict has arrived yet). Before the freshness report arrives, the badge MUST show the pending state rather than an empty or misleading state. The badge MUST render for `Unknown` as a first-class, non-error state — never hidden and never rendered as if it were a failure of the view itself.

#### Scenario: Pending state before the report arrives

- GIVEN the scan has rendered and the freshness report has not yet resolved
- WHEN the clients view is rendered
- THEN each installation's badge shows the pending state, not an empty cell

#### Scenario: Four distinct states render correctly

- GIVEN a freshness report containing one `UpToDate`, one `Outdated`, and one `Unknown` verdict across three installations
- WHEN the clients view renders after the report resolves
- THEN each installation's badge shows the state matching its verdict, with three visually distinct states plus the pending state used only before resolution

#### Scenario: Unknown renders as a first-class state, not an error

- GIVEN a freshness verdict of `Unknown` for an installation
- WHEN its badge is rendered
- THEN it shows the unknown state distinctly from up-to-date and outdated
- AND the view does not present it as a scan failure or a broken row

### Requirement: An Outdated Verdict Is Never An Incident

A `Freshness::Outdated` verdict MUST NOT be counted toward `incidentCount`, MUST NOT light the incident indicator, and MUST NOT move the Home scan-status block out of its healthy state. An out-of-date client is informational, not a fault.

#### Scenario: An outdated client does not trigger the incident indicator

- GIVEN a report with zero `issues` and a freshness report containing one `Outdated` verdict
- WHEN the Agents or Skills page is rendered
- THEN no incident indicator is shown

#### Scenario: An outdated client does not affect the Home scan-status block

- GIVEN the startup scan succeeded with `issues: []` and a freshness report containing one `Outdated` verdict
- WHEN Home renders
- THEN the scan-status block shows the healthy state

### Requirement: No Probe Table Renders As An Explicit Unsupported State, Never Fabricated Rows

When `ScanReport.clientPresence` is `null` (no probe table for this platform), the `scan` route MUST render an explicit "not supported on this platform" message using the localized `scan.clientsUnsupportedPlatform` copy, and MUST NOT render the supported-clients table at all. It MUST NOT fabricate `notDetected` rows for clients Vertice did not probe.

#### Scenario: Null clientPresence shows the unsupported-platform message, not fabricated rows

- GIVEN a report where `clientPresence` is `null`
- WHEN the `scan` route is rendered
- THEN the localized unsupported-platform message is shown
- AND no client row of any status is rendered

### Requirement: Scan Route Rescan Control

The `scan` route MUST offer a rescan control mirroring `ComponentToolbar`'s pattern, reusing the `toolbar.reload`/`toolbar.reloading` catalog keys. `App.svelte` MUST thread `onReload` and lifecycle `status` into `ScanPage` so the control is wired to the same `rescan` invocation used by the Agents and Skills pages, and the resulting report MUST replace the shell's single held report.

#### Scenario: Rescan from the scan route refreshes shared state

- GIVEN the `scan` route is rendered with a previously loaded report
- WHEN the user activates the rescan control
- THEN `rescan` is invoked, a loading state is shown until settlement using `toolbar.reloading` copy
- AND the resulting report replaces the prior report for Home, Agents, and Skills as well

### Requirement: The Not-Found-Root Incident Suppression Is Pinned By A Behavioral Contract

The frontend classifies a `ScanIssue` as the synthetic echo of a `notFound` search root by reconstructing the exact reason string `` `search root ${root.id} was not found` `` (produced by `crates/vertice-core/src/scan.rs`) and comparing it against `issue.reason`. This string reconstruction is a known, load-bearing coupling — out of scope to remove in this change — because it is the sole mechanism that keeps a `notFound` root out of `incidentCount`. A test MUST exist asserting that a `ScanIssue` built from that exact reason string is classified as the root echo and excluded from `incidentCount`, and that any other issue with a different `reason` is NOT excluded, so a future drift between the two string literals is caught by a failing test rather than by a silently re-lit incident badge.

#### Scenario: The exact root-not-found reason string is excluded from incidentCount

- GIVEN a `ScanIssue` with `reason: "search root skills-user was not found"` and no other issues
- WHEN `incidentCount` is computed over the resulting `Diagnostics`
- THEN it is `0`

#### Scenario: A similar but non-matching reason string is not excluded

- GIVEN a `ScanIssue` with `reason: "search root skills-user is not found"` (a one-word drift from the exact vocabulary)
- WHEN `incidentCount` is computed over the resulting `Diagnostics`
- THEN it is `1`, proving the classification is exact-string, not fuzzy

### Requirement: Non-Blocking Successful Scan Diagnostics

The `scan` route MUST render the full report — roots scanned (found and not found), the supported-clients presence table (or the unsupported-platform message when `clientPresence` is `null`), duration, and every `ScanIssue` — for both healthy and unhealthy successful scans. Root availability MUST derive only from `rootsScanned`; a `notFound` root MUST render neutrally, without danger colouring; recoverable issues MUST derive only from `issues`. The route MUST NOT duplicate a root diagnostic and MUST NOT treat a report with diagnostics as a hard scan failure. The Agents and Skills pages MUST NOT render this full-report diagnostics panel themselves; they only surface the incident indicator that links to the `scan` route.

#### Scenario: Mixed successful report on the scan route

- GIVEN a successful report has components, an unavailable root, a `notDetected` client slot, and a recoverable `Error` issue
- WHEN the `scan` route is rendered
- THEN all roots (found and not found, the not-found one neutral), the supported-clients table, duration, and the `Error` issue are rendered without a duplicate root warning
- AND the `notDetected` slot appears only in the supported-clients table, never as an issue

#### Scenario: Clean successful report on the scan route

- GIVEN a successful report has no unavailable roots or recoverable issues
- WHEN the `scan` route is rendered
- THEN the roots, the supported-clients table, and duration are still rendered
- AND a healthy verdict is shown instead of an empty panel

### Requirement: Embedded Component State

The Agents page and Skills page MUST each visibly mark a component as embedded and non-actionable when at least one of its locations has `origin` equal to `embedded`. This MUST be determined from location origin, not from whether a path is null, and MUST NOT invent an action for the component.

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

The frontend MUST provide a `scan` route rendering the complete `ScanReport`: every entry of `rootsScanned` regardless of `status` (including `notFound`), the supported-clients presence table (`clientPresence`, or the unsupported-platform message when it is `null`), `durationMs`, and all `issues`. This route MUST NOT render a separate installations list alongside the supported-clients table — the table is the sole presentation of installation data on this route, with per-installation paths available as tooltips. This route MUST render a result for a healthy scan (zero issues, all roots found) as well as for a scan with issues, never an empty surface.

#### Scenario: Healthy scan is visible

- GIVEN a successful report with all roots found and zero issues
- WHEN the user navigates to the `scan` route
- THEN the roots, the supported-clients table, and duration are rendered
- AND a healthy/clean verdict is shown instead of a blank panel

#### Scenario: Scan with issues

- GIVEN a successful report with one or more `issues`
- WHEN the user navigates to the `scan` route
- THEN the roots, the supported-clients table, duration, and every issue are rendered

### Requirement: Incident Indicator on List Pages

The Agents and Skills pages MUST each show a discreet incident indicator when the current report has a non-empty `issues` list, excluding any issue classified as the synthetic echo of a `notFound` search root. A `notFound` entry in `rootsScanned`, alone, MUST NOT trigger the indicator: it renders neutrally and is excluded from the incident count, the same neutral treatment as a `notDetected` or `Detected`-but-broken client slot is given in the supported-clients table (though a broken client's `Error` issue still counts, per its own rule). Activating the indicator MUST navigate to the `scan` route. The indicator MUST NOT appear when `issues` (after excluding root echoes) is empty, regardless of `rootsScanned` status.

#### Scenario: Indicator from issues

- GIVEN a report with a non-empty `issues` list (none of them root echoes) and all roots found
- WHEN the Agents or Skills page is rendered
- THEN the incident indicator is shown on that page
- AND activating it navigates to the `scan` route

#### Scenario: No indicator from a not-found root alone

- GIVEN a report with `issues: []` and one root in `rootsScanned` with `status === "notFound"`
- WHEN the Agents or Skills page is rendered
- THEN no incident indicator is shown on either page

#### Scenario: No indicator on a fully healthy report

- GIVEN a report with `issues: []` and every root found
- WHEN the Agents or Skills page is rendered
- THEN no incident indicator is shown

### Requirement: Home Scan-Status Block

The Home page MUST render a scan-status block reflecting one of three states derived from the last scan attempt: healthy (successful, no issues after excluding `notFound`-root echoes), completed with issues (successful, but issues remain after excluding `notFound`-root echoes), or failed (the scan/rescan invocation rejected). A `notFound` root or a `notDetected`/`Detected`-but-broken-without-Error client slot, alone, MUST NOT move the block out of the healthy state; a genuine `Error` issue from a broken client slot DOES move it to completed-with-issues, since it is not a root echo. The failed state MUST offer a retry action that invokes `rescan`. Home MUST NOT show a pending placeholder once a scan attempt has settled, whether it succeeded or failed.

#### Scenario: Healthy status with a not-found root and a not-detected client

- GIVEN the startup scan succeeded with `issues: []`, one `notFound` root, and one `notDetected` client slot
- WHEN Home renders
- THEN the scan-status block shows the healthy state
- AND no retry action is offered

#### Scenario: Completed-with-issues status

- GIVEN the startup scan succeeded but `issues` (after excluding root echoes) is non-empty
- WHEN Home renders
- THEN the scan-status block shows the completed-with-issues state with issue count and duration
- AND a link to the `scan` route is offered

#### Scenario: Failed status with retry

- GIVEN the startup scan invocation rejected
- WHEN Home renders
- THEN the scan-status block shows the failed state, never a pending placeholder
- AND a retry action is offered
- AND activating retry invokes `rescan`

### Requirement: The Log File Path Is Displayed As Selectable Text On The Scan Route

The `scan` route MUST render the absolute path of the application log file, obtained from the
log-path command, as selectable text alongside a localized label. The element MUST NOT provide a
"reveal in file manager" action or any button that opens the file or its containing folder — it MUST
only display the path for the user to copy.

#### Scenario: The log path is visible and selectable on the scan route

- GIVEN a successful invocation of the log-path command
- WHEN the user navigates to the `scan` route
- THEN the absolute log-file path is rendered as selectable text with a localized label
- AND no reveal-in-file-manager or file-opening action is present

#### Scenario: The rendered path matches what the command returns

- GIVEN the log-path command returns a specific absolute path
- WHEN the `scan` route renders the log-path element
- THEN the displayed text is exactly that path, unmodified
