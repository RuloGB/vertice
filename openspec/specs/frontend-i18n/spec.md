# Frontend i18n Specification

## Purpose

Define frontend-only locale resolution, catalogs, and runtime switching for English and Spanish UI chrome.

## Requirements

### Requirement: Supported Locale Resolution

The frontend MUST resolve the active locale at application startup from a persisted, durable user
choice when one exists; otherwise from `navigator.languages` or `navigator.language` via
`resolveLocale`; otherwise `en`. It MUST map `es*` to `es`, `en*` to `en`, and fall back to `en` for
unsupported locales. A persisted choice MUST be read through the `user-settings` capability's
settings-read command, not from an in-memory or session-scoped value, and MUST survive a full
application restart: selecting a language, quitting the application, and relaunching it MUST
resolve the same locale without the user reselecting it. When no settings document has ever been
written, or the settings document exists but its `locale` field is absent, unreadable, or fails to
parse, resolution MUST fall through to the `navigator.languages` / `navigator.language` step exactly
as if no persisted choice existed — a missing or corrupt `locale` field is never treated as an
explicit choice of `en`.
(Previously: resolution was described as falling back to "a manual **session** override", which was
ambiguous and, as implemented, was in-memory only — the choice did not survive a restart because no
persistence path existed at all.)

#### Scenario: Supported browser locale, no persisted choice

- GIVEN no persisted locale choice exists and the browser locale is `es-MX`
- WHEN the application starts and resolves its initial locale
- THEN the active locale is `es`

#### Scenario: Unsupported browser locale, no persisted choice

- GIVEN no persisted locale choice exists and the browser locale is `pt-BR`
- WHEN the application starts and resolves its initial locale
- THEN the active locale is `en`

#### Scenario: A persisted choice survives a full application restart

- GIVEN the user selects Spanish through the language selector while the browser locale is `en-US`
- WHEN the application is fully restarted
- THEN the active locale on the new launch is `es`, read from the persisted settings document
- AND no reselection is required

#### Scenario: A persisted choice takes precedence over the browser locale

- GIVEN a persisted locale choice of `en` and a browser locale of `es-MX`
- WHEN the application starts and resolves its initial locale
- THEN the active locale is `en`, not `es`

#### Scenario: A missing or corrupt persisted locale falls through to browser detection

- GIVEN the settings document does not exist, or exists but its `locale` field is missing or fails
  to parse, and the browser locale is `es-MX`
- WHEN the application starts and resolves its initial locale
- THEN the active locale is `es`, resolved from `navigator.languages` exactly as if no persisted
  choice had ever been made

### Requirement: Reactive UI Locale Switching

The frontend MUST expose a language selector that updates one shared reactive locale source. Changing that locale MUST update all inventory UI chrome in the same session without requiring a reload.

#### Scenario: Manual language change
- GIVEN the inventory UI is rendered in English
- WHEN the user switches the selector to Spanish
- THEN inventory chrome updates to Spanish in the same session
- AND no restart or rescan is required

### Requirement: Catalog Completeness and Boundary

The frontend MUST provide complete `en` and `es` catalogs for the Prompts page chrome in addition to the already-required inventory and scan chrome. Prompts catalog coverage MUST include navigation label, page title, search placeholder, create/edit form labels, save/cancel/delete/copy actions, loading, empty, failure, confirmation, and copy success/failure feedback. The frontend MUST NOT localize user-authored prompt content such as `title`, `body`, `tags`, or `bestForContext` values.
(Previously: catalog completeness covered the inventory, scan, supported-clients, and freshness chrome, but not Prompts-page strings.)

#### Scenario: Spanish catalog stays complete for Prompts
- GIVEN the `en` catalog defines a Prompts-page chrome key
- WHEN the `es` catalog is inspected
- THEN the corresponding Spanish translation exists

#### Scenario: Prompt content stays verbatim across locales
- GIVEN a saved prompt whose text is user-authored
- WHEN the active locale changes between English and Spanish
- THEN the surrounding chrome is localized
- AND the prompt's stored content remains byte-identical
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
