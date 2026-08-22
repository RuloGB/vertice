# Delta for Inventory UI

## ADDED Requirements

### Requirement: Non-Blocking Successful Scan Diagnostics

For a successful scan report, the UI MUST retain the inventory and MAY render concise diagnostics for unavailable scan roots, missing supported clients, and recoverable scan issues. Root availability MUST derive only from `rootsScanned`; recoverable issues MUST derive only from `issues`; and a missing-client notice MUST be discreet. The UI MUST NOT duplicate a root diagnostic, replace the inventory with diagnostics, or treat diagnostics as a hard scan failure.

#### Scenario: Mixed successful report
- GIVEN a successful report has components, an unavailable root, a missing client, and a recoverable issue
- WHEN the report is rendered
- THEN the inventory remains visible
- AND each applicable diagnostic is rendered without a duplicate root warning

#### Scenario: Successful report without diagnostics
- GIVEN a successful report has no unavailable roots, missing clients, or recoverable issues
- WHEN the report is rendered
- THEN no diagnostic panel or missing-client notice is shown

### Requirement: Embedded Component State

The UI MUST visibly mark a component as embedded and non-actionable when at least one of its locations has `origin` equal to `embedded`. It MUST determine this state from location origin, not from whether a path is null, and MUST NOT invent an action for the component.

#### Scenario: Embedded component with a path
- GIVEN a component has an embedded location with a non-null path
- WHEN its inventory row is rendered
- THEN the row shows embedded/non-actionable status
- AND no action control is introduced

#### Scenario: Nullable non-embedded path
- GIVEN a component has a non-embedded location with a null path
- WHEN its inventory row is rendered
- THEN the row remains safely renderable
- AND it is not marked embedded solely because its path is null

## MODIFIED Requirements

### Requirement: Localized Inventory Chrome

The inventory UI MUST render all user-facing chrome through the active frontend i18n catalog, including toolbar labels, placeholders, kind labels, loading, empty, failure, duplicate, title, aria, null-path copy, diagnostic labels, and embedded/non-actionable status. It MUST update that chrome when the active locale changes and MUST NOT localize payload fields from the scan report.

(Previously: Inventory chrome included lifecycle and location labels but not diagnostic or embedded-state chrome.)

#### Scenario: Chrome follows locale changes
- GIVEN a loaded inventory in English
- WHEN the active locale changes to Spanish
- THEN all inventory UI chrome is re-rendered in Spanish
- AND component payload fields remain unchanged
