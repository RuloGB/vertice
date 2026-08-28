# Delta for Desktop Shell

## MODIFIED Requirements

### Requirement: Minimal Scan Command Surface

The shell SHALL expose the existing six inventory commands, four prompt commands, and four typed subscription commands for listing, creating, updating, and deleting subscriptions. Prompt and subscription commands MUST be thin async pass-throughs to their repositories, MUST contain no frontend business logic, and MUST preserve the existing scan, freshness, settings, log-path, and prompt behavior unchanged.
(Previously: the surface contained six inventory commands and four prompt commands, with no subscription commands.)

#### Scenario: Subscription commands extend without changing scan behavior
- GIVEN the shell registers subscription commands alongside the existing commands
- WHEN the frontend invokes `scan` or `rescan`
- THEN the returned scan behavior is unchanged

#### Scenario: Subscription mutations stay typed
- GIVEN the frontend creates, updates, or deletes a subscription
- WHEN the shell command resolves
- THEN it returns typed subscription data or typed success without stringly payloads

### Requirement: Minimal Capability Grant

The shell capability declaration SHALL still grant `core:default` only. Subscription support MUST NOT add filesystem, shell, dialog, or clipboard capability grants, and subscription writes MUST stay confined to the application data directory through the sanctioned subscription persistence module.
(Previously: only prompt writes were named alongside the existing sanctioned persistence paths.)

#### Scenario: Subscription support adds no capability grant
- GIVEN the subscription commands are registered
- WHEN the capability declaration is reviewed
- THEN it remains `core:default` only

#### Scenario: Subscription writes stay app-data-only
- GIVEN a subscription create, update, or delete command succeeds
- WHEN its filesystem side effects are traced
- THEN any write occurs only inside the application data directory

### Requirement: The Read-Only Audit Recognizes A Fifth Write Exception

`crates/vertice-app/tests/read_only_audit.rs` MUST name the subscription persistence module as a fifth sanctioned write exception, and that module MUST be proved individually as app-data-only with an allow-list limited to its atomic JSON write path.
(Previously: the audit recognized exactly four sanctioned writer modules, including prompt persistence.)

#### Scenario: The subscription store exception is proved on its own merits
- GIVEN the subscription persistence module source
- WHEN the read-only audit runs
- THEN it independently verifies app-data path derivation and rejects forbidden absolute-path or direct-env access

#### Scenario: The audit exception count becomes five
- GIVEN the read-only audit's sanctioned writer list after this change
- WHEN it is inspected
- THEN it contains exactly five entries, including the subscription persistence module
