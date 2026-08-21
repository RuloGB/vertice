# Inventory UI Specification

## Purpose

Define the read-only frontend inventory presentation over the existing typed `scan` and `rescan` commands. This is a new capability for T11, covering CA-1 and the visual half of CA-3.

## Requirements

### Requirement: Startup Inventory Rendering

On startup, the frontend MUST invoke `scan` without requiring configuration and MUST render one row for each consolidated `ScanReport.components` entry. Each row SHALL expose the component name, kind, description when present, and its origin locations.

#### Scenario: Successful startup scan
- GIVEN the application starts with a successful report containing components
- WHEN the startup scan completes
- THEN the inventory displays one unified row per report component
- AND each row displays its name, kind, description, and origin locations

### Requirement: Duplicate Rows and Complete Paths

The UI MUST mark a component as duplicated if and only if `locations.length > 1`. It MUST disclose every location path, including nullable paths, and MUST NOT regroup components by name or compare file contents.

#### Scenario: Multi-location component
- GIVEN a component has three locations
- WHEN its row is rendered and its locations are disclosed
- THEN the row shows a clear duplicate mark
- AND all three location entries are shown

#### Scenario: Nullable location path
- GIVEN a component contains a location whose path is null
- WHEN its locations are rendered
- THEN the row remains renderable and represents that path safely without inventing a filesystem action

### Requirement: View-Only Filtering and Search

The UI MUST provide a kind filter for `skill` and `agent` and a name search. Filtering and search MUST operate only on the in-memory report components and MUST NOT invoke another scan.

#### Scenario: Combined view filters
- GIVEN a loaded report containing both kinds and multiple names
- WHEN the user selects a kind and enters a name query
- THEN only matching components are shown
- AND the report remains unchanged

### Requirement: Reload Lifecycle

The UI MUST provide reload that invokes `rescan`, replacing the displayed report with the resulting successful report. It MUST show a loading state from invocation start until settlement.

#### Scenario: Reload obtains fresh data
- GIVEN a previously loaded inventory
- WHEN the user activates reload
- THEN `rescan` is invoked and a loading state is shown
- AND the new report replaces the prior inventory on success

### Requirement: Minimal Lifecycle States

The UI MUST distinguish loading, hard scan failure, and successful empty results. A hard failure MUST show a minimal non-blank failure surface; an empty successful report MUST show an empty list region distinct from failure. Those lifecycle surfaces MUST render through frontend i18n messages while preserving raw diagnostic passthrough data when present.

#### Scenario: Hard failure
- GIVEN `scan` or `rescan` rejects with a scan error
- WHEN the invocation settles
- THEN the UI leaves loading and shows a minimal failure state without crashing

#### Scenario: Empty successful report
- GIVEN a successful report with `components: []`
- WHEN it is rendered
- THEN the UI shows an empty inventory region distinct from failure

### Requirement: Localized Inventory Chrome

The inventory UI MUST render all user-facing chrome through the active frontend i18n catalog, including toolbar labels, placeholders, kind labels, loading, empty, failure, duplicate, title, aria, and null-path copy. It MUST update that chrome when the active locale changes and MUST NOT localize payload fields from the scan report.

#### Scenario: Chrome follows locale changes
- GIVEN a loaded inventory in English
- WHEN the active locale changes to Spanish
- THEN all inventory UI chrome is re-rendered in Spanish
- AND component payload fields remain unchanged

### Requirement: Read-Only Frontend Boundary

The inventory UI MUST use only the existing typed scan wrapper and report data. It MUST NOT access the filesystem, write data, install components, add watchers, schedule interval refreshes, or auto-refresh.

#### Scenario: No watcher or filesystem behavior
- GIVEN the inventory UI is running
- WHEN no user invokes reload
- THEN no watcher, timer-based refresh, filesystem API, or write operation is performed
