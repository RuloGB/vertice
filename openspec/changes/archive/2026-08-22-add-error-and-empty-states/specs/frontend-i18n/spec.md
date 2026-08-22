# Delta for Frontend i18n

## MODIFIED Requirements

### Requirement: Catalog Completeness and Boundary

The frontend MUST provide complete `en` and `es` catalogs for inventory UI chrome, including labels, placeholders, loading, empty, failure, title, aria, duplicate, null-path copy, successful-scan diagnostic labels, and embedded/non-actionable status. It MUST NOT localize payload fields or diagnostic passthrough values such as component names, paths, `ScanIssue.reason`, or `ScanError.detail.reason`.

(Previously: Catalog coverage excluded successful-scan diagnostic and embedded/non-actionable chrome.)

#### Scenario: Payload stays verbatim
- GIVEN a scan error includes a diagnostic reason from core
- WHEN the UI renders the failure surface
- THEN localized chrome surrounds the message
- AND the diagnostic reason remains verbatim passthrough data

#### Scenario: Successful-report diagnostics use catalog chrome
- GIVEN a successful report contains a recoverable issue with a reason value
- WHEN the UI renders its diagnostic
- THEN the diagnostic label is rendered in the active locale
- AND the reason value remains verbatim
