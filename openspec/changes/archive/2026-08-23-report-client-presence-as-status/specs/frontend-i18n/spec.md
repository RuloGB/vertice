# Delta for Frontend i18n

Key inventory below matches `design.md` §6 exactly: five new keys, three removed keys (their only consumers — `diagnostics.missingClient`'s section, and the "Detected installations" panel — are deleted by this change).

## MODIFIED Requirements

### Requirement: Catalog Completeness and Boundary

The frontend MUST provide complete `en` and `es` catalogs for Agents page, Skills page, and `scan` route chrome, including labels, placeholders, loading, empty, failure, title, aria, duplicate, null-path copy, full scan-report labels (roots found/not found, duration, issues), incident-indicator copy, embedded/non-actionable status, the Home scan-status block (healthy/completed-with-issues/failed, retry action), and the supported-clients table chrome: `scan.clientsTitle`, `scan.clientDetected`, `scan.clientNotDetected`, `scan.clientVersionUnavailable`, and `scan.clientsUnsupportedPlatform`. Existing `inventory.*`, `toolbar.*`, and `diagnostics.*` keys MUST be reused or renamed for the new per-page and scan-route chrome rather than duplicated per page; keys scoped only to the removed combined `inventory` route (including `nav.inventory` and `area.inventory`) MUST be retired. `diagnostics.missingClient`, `scan.installationsTitle`, and `scan.installationsEmpty` MUST also be retired: the supported-clients table replaces the "Detected installations" panel and the missing-client notice entirely, so these keys have no remaining consumer. The frontend MUST NOT localize payload fields or diagnostic passthrough values such as component names, paths, `ScanIssue.reason`, `ScanError.detail.reason`, or a `ClientPresence.label` value — the last is a product proper noun (e.g. `"Claude Code CLI (npm)"`), not translatable prose, and MUST render identically regardless of active locale.
(Previously: did not name the supported-clients table chrome or `ClientPresence.label` in the non-localization list, since neither existed as a first-class UI field; enumerated `scan.installationsTitle`/`scan.installationsEmpty` as required keys rather than retired ones; did not mention `diagnostics.missingClient`'s retirement.)

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

- GIVEN the `en` catalog defines a key for the scan route, incident indicator, Home scan-status block, or the supported-clients table
- WHEN the `es` catalog is inspected
- THEN a corresponding Spanish translation exists for that key

#### Scenario: Client presence labels are not catalog entries

- GIVEN a `ClientPresence.label` value such as `"OpenCode (npm)"`
- WHEN the `en` and `es` catalogs are searched for a matching translatable key
- THEN no such key exists — the label is rendered verbatim from the report, identical in both locales

#### Scenario: The retired installations-panel keys are gone

- GIVEN the catalog is inspected after this change
- WHEN searching for `diagnostics.missingClient`, `scan.installationsTitle`, or `scan.installationsEmpty`
- THEN none of them exist in either the `en` or `es` catalog, and no test still asserts their presence
