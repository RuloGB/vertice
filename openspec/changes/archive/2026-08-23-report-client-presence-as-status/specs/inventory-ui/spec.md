# Delta for Inventory UI

Field names and semantics below match `design.md` §6 and §7 exactly: `ClientPresence.status: Detected` means a candidate root exists on disk, not that a version was extracted; a `Detected` row with zero `installations` renders an unavailable-version state while its `Error` issue still counts as an incident.

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Non-Blocking Successful Scan Diagnostics

The `scan` route MUST render the full report — roots scanned (found and not found), the supported-clients presence table (or the unsupported-platform message when `clientPresence` is `null`), duration, and every `ScanIssue` — for both healthy and unhealthy successful scans. Root availability MUST derive only from `rootsScanned`; a `notFound` root MUST render neutrally, without danger colouring; recoverable issues MUST derive only from `issues`. The route MUST NOT duplicate a root diagnostic and MUST NOT treat a report with diagnostics as a hard scan failure. The Agents and Skills pages MUST NOT render this full-report diagnostics panel themselves; they only surface the incident indicator that links to the `scan` route.
(Previously: rendered a separate "Detected installations" panel and a missing-client notice from `ScanIssue.reason`; the panel is replaced by the supported-clients table and absence now appears only in that table, never in `issues`.)

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

### Requirement: Incident Indicator on List Pages

The Agents and Skills pages MUST each show a discreet incident indicator when the current report has a non-empty `issues` list, excluding any issue classified as the synthetic echo of a `notFound` search root. A `notFound` entry in `rootsScanned`, alone, MUST NOT trigger the indicator: it renders neutrally and is excluded from the incident count, the same neutral treatment as a `notDetected` or `Detected`-but-broken client slot is given in the supported-clients table (though a broken client's `Error` issue still counts, per its own rule). Activating the indicator MUST navigate to the `scan` route. The indicator MUST NOT appear when `issues` (after excluding root echoes) is empty, regardless of `rootsScanned` status.
(Previously: also fired on any `rootsScanned` entry with `status === "notFound"`, with zero issues.)

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
(Previously: completed-with-issues also fired on a not-found root with zero issues; healthy required no not-found roots.)

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

### Requirement: Full Scan Report Route

The frontend MUST provide a `scan` route rendering the complete `ScanReport`: every entry of `rootsScanned` regardless of `status` (including `notFound`), the supported-clients presence table (`clientPresence`, or the unsupported-platform message when it is `null`), `durationMs`, and all `issues`. This route MUST NOT render a separate installations list alongside the supported-clients table — the table is the sole presentation of installation data on this route, with per-installation paths available as tooltips. This route MUST render a result for a healthy scan (zero issues, all roots found) as well as for a scan with issues, never an empty surface.
(Previously: rendered a "Detected installations" list independent of any client-presence concept, since `clientPresence` did not exist.)

#### Scenario: Healthy scan is visible

- GIVEN a successful report with all roots found and zero issues
- WHEN the user navigates to the `scan` route
- THEN the roots, the supported-clients table, and duration are rendered
- AND a healthy/clean verdict is shown instead of a blank panel

#### Scenario: Scan with issues

- GIVEN a successful report with one or more `issues`
- WHEN the user navigates to the `scan` route
- THEN the roots, the supported-clients table, duration, and every issue are rendered

### Requirement: Localized Inventory Chrome

The Agents page, Skills page, and `scan` route MUST render all user-facing chrome through the active frontend i18n catalog, including toolbar labels, placeholders, loading, empty, failure, duplicate, title, aria, null-path copy, diagnostic labels, incident-indicator copy, embedded/non-actionable status, and the supported-clients table chrome (title, per-status labels, unavailable-version copy, unsupported-platform message). It MUST update that chrome when the active locale changes and MUST NOT localize payload fields from the scan report or client slot labels.
(Previously: did not name the supported-clients table chrome or client slot labels, since neither existed as first-class UI fields; also implicitly covered the now-removed "Detected installations" panel chrome.)

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
