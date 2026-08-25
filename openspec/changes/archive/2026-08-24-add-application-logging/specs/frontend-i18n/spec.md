# Delta for Frontend i18n

## ADDED Requirements

### Requirement: Log-Path Label Is Fully Localized

The frontend MUST provide complete `en` and `es` catalog entries for the `scan` route's log-path
label introduced by `add-application-logging`. The displayed absolute path value itself MUST NOT be
localized or altered — it is opaque passthrough data, consistent with the existing boundary that
diagnostic reasons and reference-version strings are rendered verbatim regardless of locale.

#### Scenario: The Spanish catalog stays complete for the log-path label

- GIVEN the `en` catalog defines a key for the scan route's log-path label
- WHEN the `es` catalog is inspected
- THEN a corresponding Spanish translation exists for that key

#### Scenario: The log path value itself is never translated

- GIVEN the log-path element renders a specific absolute path
- WHEN the active locale is switched between English and Spanish
- THEN the path value is byte-identical in both renders, while the surrounding label is localized
