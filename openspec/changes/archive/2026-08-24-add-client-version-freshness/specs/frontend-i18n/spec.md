# Delta for Frontend i18n

New `clients.*` keys cover the freshness badge's four states, the first-run disclosure text, and the opt-out setting copy, complete in both `en` and `es` per design principle 7. Freshness payload values (a reference version string, an upstream package or repository name) are passthrough data, not translatable prose, joining the existing proper-noun rule already applied to `ClientPresence.label`.

## MODIFIED Requirements

### Requirement: Catalog Completeness and Boundary

The frontend MUST provide complete `en` and `es` catalogs for Agents page, Skills page, and `scan` route chrome, including labels, placeholders, loading, empty, failure, title, aria, duplicate, null-path copy, full scan-report labels (roots found/not found, duration, issues), incident-indicator copy, embedded/non-actionable status, the Home scan-status block (healthy/completed-with-issues/failed, retry action), the supported-clients table chrome (`scan.clientsTitle`, `scan.clientDetected`, `scan.clientNotDetected`, `scan.clientVersionUnavailable`, `scan.clientsUnsupportedPlatform`), and the freshness chrome on the clients view: `clients.*` keys for each of the four badge states (up to date, outdated, unknown, pending), the first-run disclosure text (stating that public registries are queried and that nothing about the user is sent), and the opt-out setting's label and description. Existing `inventory.*`, `toolbar.*`, and `diagnostics.*` keys MUST be reused or renamed for the new per-page and scan-route chrome rather than duplicated per page; keys scoped only to the removed combined `inventory` route (including `nav.inventory` and `area.inventory`) MUST be retired. `diagnostics.missingClient`, `scan.installationsTitle`, and `scan.installationsEmpty` MUST also be retired. The frontend MUST NOT localize payload fields or diagnostic passthrough values such as component names, paths, `ScanIssue.reason`, `ScanError.detail.reason`, a `ClientPresence.label` value, a freshness verdict's reference version string, or an upstream package/repository name — these are product proper nouns or opaque data, not translatable prose, and MUST render identically regardless of active locale.
(Previously: did not cover the freshness badge chrome, the first-run disclosure, the opt-out setting copy, or the passthrough rule for a freshness verdict's version string and upstream identity.)

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

#### Scenario: Spanish catalog stays complete, including freshness chrome

- GIVEN the `en` catalog defines a key for the scan route, incident indicator, Home scan-status block, the supported-clients table, or any freshness badge state, disclosure text, or opt-out setting copy
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

#### Scenario: A freshness verdict's reference version stays verbatim in both locales

- GIVEN an `Outdated` freshness verdict carrying a reference version string such as `"2.1.241"`
- WHEN its badge is rendered in English and then in Spanish
- THEN the version string is byte-identical in both renders, while the surrounding badge chrome is localized
