# Delta for inventory-ui

## ADDED Requirements

### Requirement: Localized Inventory Chrome

The inventory UI MUST render all user-facing chrome through the active frontend i18n catalog, including toolbar labels, placeholders, kind labels, loading, empty, failure, duplicate, title, aria, and null-path copy. It MUST update that chrome when the active locale changes and MUST NOT localize payload fields from the scan report.

#### Scenario: Chrome follows locale changes
- GIVEN a loaded inventory in English
- WHEN the active locale changes to Spanish
- THEN all inventory UI chrome is re-rendered in Spanish
- AND component payload fields remain unchanged

## MODIFIED Requirements

### Requirement: Minimal Lifecycle States

The UI MUST distinguish loading, hard scan failure, and successful empty results. A hard failure MUST show a minimal non-blank failure surface; an empty successful report MUST show an empty list region distinct from failure. Those lifecycle surfaces MUST render through frontend i18n messages while preserving raw diagnostic passthrough data when present.
(Previously: Rich diagnostics, client/installation/root messaging, and i18n were out of scope.)

#### Scenario: Hard failure
- GIVEN `scan` or `rescan` rejects with a scan error
- WHEN the invocation settles
- THEN the UI leaves loading and shows a minimal failure state without crashing

#### Scenario: Empty successful report
- GIVEN a successful report with `components: []`
- WHEN it is rendered
- THEN the UI shows an empty inventory region distinct from failure