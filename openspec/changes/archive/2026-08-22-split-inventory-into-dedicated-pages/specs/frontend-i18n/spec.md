# Delta for Frontend i18n

## MODIFIED Requirements

### Requirement: Catalog Completeness and Boundary

The frontend MUST provide complete `en` and `es` catalogs for Agents page, Skills page, and `scan` route chrome, including labels, placeholders, loading, empty, failure, title, aria, duplicate, null-path copy, full scan-report labels (roots found/not found, installations, duration, issues), incident-indicator copy, embedded/non-actionable status, and the Home scan-status block (healthy/completed-with-issues/failed, retry action). Existing `inventory.*`, `toolbar.*`, and `diagnostics.*` keys MUST be reused or renamed for the new per-page and scan-route chrome rather than duplicated per page; keys scoped only to the removed combined `inventory` route (including `nav.inventory` and `area.inventory`) MUST be retired. The frontend MUST NOT localize payload fields or diagnostic passthrough values such as component names, paths, `ScanIssue.reason`, or `ScanError.detail.reason`.

(Previously: catalog coverage applied to a single combined `inventory` route and did not include a full scan-report route, an incident indicator, or a Home scan-status block; a kind-selector key existed on the combined toolbar.)

#### Scenario: Payload stays verbatim
- GIVEN a scan error includes a diagnostic reason from core
- WHEN the UI renders the failure surface on any of the three routes
- THEN localized chrome surrounds the message
- AND the diagnostic reason remains verbatim passthrough data

#### Scenario: Successful-report diagnostics use catalog chrome
- GIVEN a successful report contains a recoverable issue with a reason value
- WHEN the `scan` route renders its diagnostic
- THEN the diagnostic label is rendered in the active locale
- AND the reason value remains verbatim

#### Scenario: No duplicated keys across Agents and Skills
- GIVEN the Agents page and Skills page both render toolbar and lifecycle chrome
- WHEN the catalog is inspected
- THEN both pages consume the same shared toolbar/lifecycle keys
- AND no per-page duplicate of an identical string exists

#### Scenario: Inventory-only keys are retired
- GIVEN the catalog is inspected after this change
- WHEN searching for `nav.inventory`, `area.inventory`, or the removed kind-selector key
- THEN none of them exist in either the `en` or `es` catalog

#### Scenario: Spanish catalog stays complete
- GIVEN the `en` catalog defines a key for the scan route, incident indicator, or Home scan-status block
- WHEN the `es` catalog is inspected
- THEN a corresponding Spanish translation exists for that key
