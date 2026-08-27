# Delta for Frontend i18n

## MODIFIED Requirements

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
