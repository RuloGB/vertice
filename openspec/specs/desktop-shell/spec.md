# Desktop Shell Specification

## Purpose

Define the Tauri 2 desktop shell IPC surface that exposes the core scan to the frontend: the command contract, non-blocking execution, the capability (ACL) posture, the content security policy, and the frontend filesystem boundary.

## Requirements

### Requirement: Minimal Scan Command Surface

The shell SHALL expose exactly two commands, `scan` and `rescan`, both implemented in `vertice-app` as identical thin async pass-throughs to the core scan operation. Commands MUST contain no business logic — no filtering, no transformation of the report, no caching, no state. `vertice-core` MUST remain unchanged. Because no cache exists, `rescan` is semantically identical to `scan`; it exists to keep the IPC contract stable for future cache-invalidation semantics, and because the PoC plan mandates exactly these two commands.

#### Scenario: Successful scan returns the consolidated report

- GIVEN registered user roots with discoverable components
- WHEN the frontend invokes `scan`
- THEN the command returns the core scan's `ScanReport` — components, installations, scanned roots, issues, and measured duration — unmodified

#### Scenario: Rescan behaves identically to scan

- GIVEN any number of prior invocations
- WHEN the frontend invokes `rescan`
- THEN it performs a full fresh core scan exactly as `scan` does
- AND no cached or stored result is read, reused, or invalidated

#### Scenario: Scan issues surface without command failure

- GIVEN a scanned root contains an unreadable component
- WHEN `scan` is invoked
- THEN the command resolves successfully with a report containing the corresponding issue entries
- AND per-component issues do not become command errors

### Requirement: Non-Blocking Command Execution

Both commands SHALL be async and MUST offload the blocking core scan onto the Tauri async runtime's blocking facility (`spawn_blocking`). A scan taking up to the CA-15 two-second budget MUST NOT block the main thread or freeze the UI.

#### Scenario: UI remains responsive during a slow scan

- GIVEN a scan that takes up to the CA-15 two-second budget
- WHEN the command is awaiting the core scan
- THEN the scan runs off the main thread
- AND the window event loop remains responsive for the whole duration

### Requirement: Typed IPC Contract

Commands MUST return `Result<ScanReport, ScanError>` directly, using the T2-generated types exactly as serialized for the TypeScript bindings. The shell MUST NOT introduce hand-written DTOs or string error payloads. A failure of the offloaded task itself (join failure) MUST map to the existing `ScanError` internal variant — transport mapping, not business logic.

#### Scenario: Core error crosses IPC as the typed payload

- GIVEN the core scan fails because no roots are configured
- WHEN `scan` is invoked
- THEN the invocation rejects with the serde-tagged `ScanError` payload matching the generated binding (kind `noRootsConfigured`)

#### Scenario: Offloaded task failure maps to the internal variant

- GIVEN the offloaded scan task fails to complete
- WHEN the command observes the task failure
- THEN the invocation rejects with the existing internal `ScanError` variant carrying a reason detail

### Requirement: Minimal Capability Grant

The shell capability declaration SHALL grant `core:default` only: no filesystem plugin, no filesystem scopes, no shell or dialog permissions. The audited desktop surface MUST show that the webview has zero filesystem mutation capability over scanned roots, including content writes, truncation, creation, deletion, rename/link creation, permission changes, and equivalent indirect mutation paths. The audit policy MUST cover the capability file plus the command-exposed desktop surface and MUST avoid claiming that text inspection alone proves all transitive write absence. Verification evidence MUST name the audited capability and command surfaces used to support CA-16.

#### Scenario: Capabilities grant nothing beyond core default

- GIVEN the shell capability declaration
- WHEN it is reviewed or audited
- THEN it grants only `core:default`
- AND it contains no filesystem, shell, or dialog permission or scope

#### Scenario: Webview has no filesystem mutation surface over scanned roots

- GIVEN the audited capability declaration and scan command surface
- WHEN the desktop shell read-only audit runs
- THEN no webview-exposed filesystem mutation capability exists over scanned roots
- AND the audit records the capability and command surfaces it reviewed

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
