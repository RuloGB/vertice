# Frontend i18n Specification

## Purpose

Define frontend-only locale resolution, catalogs, and runtime switching for English and Spanish UI chrome.

## Requirements

### Requirement: Supported Locale Resolution

The frontend MUST resolve the active locale from a manual session override when present; otherwise from `navigator.languages` or `navigator.language`. It MUST map `es*` to `es`, `en*` to `en`, and fallback to `en` for unsupported locales.

#### Scenario: Supported browser locale
- GIVEN no manual override and the browser locale is `es-MX`
- WHEN the app resolves its initial locale
- THEN the active locale is `es`

#### Scenario: Unsupported browser locale
- GIVEN no manual override and the browser locale is `pt-BR`
- WHEN the app resolves its initial locale
- THEN the active locale is `en`

### Requirement: Reactive UI Locale Switching

The frontend MUST expose a language selector that updates one shared reactive locale source. Changing that locale MUST update all inventory UI chrome in the same session without requiring a reload.

#### Scenario: Manual language change
- GIVEN the inventory UI is rendered in English
- WHEN the user switches the selector to Spanish
- THEN inventory chrome updates to Spanish in the same session
- AND no restart or rescan is required

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
