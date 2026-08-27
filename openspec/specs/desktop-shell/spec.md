# Desktop Shell Specification

## Purpose

Define the Tauri 2 desktop shell IPC surface that exposes the core scan to the frontend: the command contract, non-blocking execution, the capability (ACL) posture, the content security policy, and the frontend filesystem boundary. `add-client-version-freshness` (2026-08-24) grew the command surface from two commands to five: a freshness command, and two settings commands (read/write) required by the confirmed enabled-by-default-with-opt-out posture. `add-application-logging` (2026-08-24) grew the surface to six commands with a read-only log-path command, and extended the read-only audit to recognize a second sanctioned write exception (the logging sink). `add-locale-persistence` (2026-08-25) renamed the settings command pair to `user_settings`/`set_user_settings`, moved locale and opt-out state into a durable `settings.json`, and grew the read-only audit to a third sanctioned write exception.

## Requirements

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
### Requirement: Non-Blocking Command Execution

All six commands SHALL be async and MUST offload their blocking work onto the Tauri async runtime's
blocking facility (`spawn_blocking`) or an equivalent non-blocking mechanism. A scan taking up to the
CA-15 two-second budget MUST NOT block the main thread or freeze the UI. The freshness command's
network-bound work MUST likewise never block the main thread, and its latency MUST NOT be included
in, or count against, the CA-15 scan budget. The log-path command performs no I/O and is exempted
from the `spawn_blocking` offload for that reason alone; it remains `async` for interface
consistency with the other five commands.
(Previously: stated "All five commands," which had already drifted from the six-command surface
`add-application-logging` introduced, and made no exception for the log-path command, which
`commands.rs` already implements without `spawn_blocking` because it performs no I/O. This corrects
both pre-existing wording errors; neither is a behavior change caused by this change.)

#### Scenario: UI remains responsive during a slow scan

- GIVEN a scan that takes up to the CA-15 two-second budget
- WHEN the command is awaiting the core scan
- THEN the scan runs off the main thread
- AND the window event loop remains responsive for the whole duration

#### Scenario: A slow freshness lookup does not block the UI or the scan budget

- GIVEN the freshness command is performing a live network lookup that takes longer than the CA-15 scan budget
- WHEN it is in flight
- THEN the main thread and UI remain responsive
- AND the elapsed time is not attributed to, or measured as part of, the scan's CA-15 duration

### Requirement: Typed IPC Contract

Commands MUST return typed results directly, using the generated types exactly as serialized for the TypeScript bindings, including the freshness command's typed report. The shell MUST NOT introduce hand-written DTOs or string error payloads. A failure of an offloaded task itself (join failure) MUST map to the appropriate typed error variant — transport mapping, not business logic. A freshness-lookup failure MUST surface as `Freshness::Unknown` within the typed report, never as a rejected command invocation for an otherwise-successful report.

#### Scenario: Core error crosses IPC as the typed payload

- GIVEN the core scan fails because no roots are configured
- WHEN `scan` is invoked
- THEN the invocation rejects with the serde-tagged `ScanError` payload matching the generated binding (kind `noRootsConfigured`)

#### Scenario: Offloaded task failure maps to the internal variant

- GIVEN the offloaded scan task fails to complete
- WHEN the command observes the task failure
- THEN the invocation rejects with the existing internal `ScanError` variant carrying a reason detail

#### Scenario: A degraded freshness lookup still resolves the command successfully

- GIVEN every reference-version lookup for the current report fails or times out
- WHEN the freshness command is invoked
- THEN it resolves successfully with a typed report whose entries are `Freshness::Unknown`, not a rejected invocation

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
### Requirement: Hardened Content Security Policy

The shell configuration SHALL declare a CSP of at least `default-src 'self'` plus `object-src 'none'` and `base-uri 'none'`. It MUST NOT allow remote content, and the production policy MUST NOT contain `unsafe-inline`.

#### Scenario: Production window loads under the hardened policy

- GIVEN the packaged application
- WHEN the window loads its content
- THEN the effective CSP is at least `default-src 'self'; object-src 'none'; base-uri 'none'`
- AND no content is loaded from any remote origin

### Requirement: Frontend Filesystem Boundary

The frontend SHALL invoke scan commands only through a typed wrapper module importing its types exclusively from the generated bindings. The frontend MUST NOT use any Tauri filesystem plugin or otherwise touch the filesystem; all scan data MUST arrive via command invocation.

#### Scenario: Frontend has no filesystem plugin available

- GIVEN the running application
- WHEN frontend code executes in the webview
- THEN no filesystem plugin API is available to it
- AND scan results reach it only as typed command responses

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
