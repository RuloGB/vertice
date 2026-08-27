# Delta for Desktop Shell

## MODIFIED Requirements

### Requirement: Minimal Scan Command Surface

The shell SHALL expose the existing six inventory commands plus four typed prompt commands for listing, creating, updating, and deleting prompts. Prompt commands MUST be thin async pass-throughs to the prompt repository, MUST contain no search or clipboard business logic, and MUST preserve the existing scan, freshness, settings, and log-path behavior unchanged.
(Previously: the surface contained exactly six inventory-oriented commands and no prompt commands.)

#### Scenario: Prompt commands extend without changing scan behavior
- GIVEN the shell registers prompt commands alongside the inventory commands
- WHEN the frontend invokes `scan` or `rescan`
- THEN the returned scan behavior is unchanged by prompt support

#### Scenario: Prompt mutations stay typed
- GIVEN the frontend creates, updates, or deletes a prompt
- WHEN the shell command resolves
- THEN it returns typed prompt data or typed success without stringly payloads

### Requirement: Minimal Capability Grant

The shell capability declaration SHALL still grant `core:default` only. Prompt support MUST NOT add filesystem, shell, dialog, or clipboard capability grants, and every prompt write MUST stay confined to the application data directory through the sanctioned prompt persistence module.
(Previously: the audited shell named only the freshness cache and settings store as sanctioned write-capable command paths.)

#### Scenario: Prompt support adds no new capability grant
- GIVEN the prompt commands are registered
- WHEN the capability declaration is reviewed
- THEN it remains `core:default` only

#### Scenario: Prompt writes stay app-data-only
- GIVEN a prompt create, update, or delete command succeeds
- WHEN its filesystem side effects are traced
- THEN any write occurs only inside the application data directory

### Requirement: The Read-Only Audit Recognizes A Fourth Write Exception

`crates/vertice-app/tests/read_only_audit.rs` MUST name the prompt persistence module as a fourth sanctioned write exception, and that module MUST be proved individually as app-data-only with an allow-list limited to the atomic JSON write path it uses.
(Previously: the audit recognized exactly three sanctioned write-exception modules and had no prompt persistence exception.)

#### Scenario: The prompt store exception is proved on its own merits
- GIVEN the prompt persistence module source
- WHEN the read-only audit runs
- THEN it independently verifies app-data path derivation and rejects forbidden absolute-path or direct-env access

#### Scenario: The audit exception count becomes four
- GIVEN the read-only audit's sanctioned writer list after this change
- WHEN it is inspected
- THEN it contains exactly four entries, including the prompt persistence module
