# Delta for Frontend i18n

## MODIFIED Requirements

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
