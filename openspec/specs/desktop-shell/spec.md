# Desktop Shell Specification

## Purpose

Define the Tauri 2 desktop shell IPC surface that exposes the core scan to the frontend: the command contract, non-blocking execution, the capability (ACL) posture, the content security policy, and the frontend filesystem boundary. `add-client-version-freshness` (2026-08-24) grew the command surface from two commands to five: a freshness command, and two settings commands (read/write) required by the confirmed enabled-by-default-with-opt-out posture.

## Requirements

### Requirement: Minimal Scan Command Surface

The shell SHALL expose exactly five commands: `scan` and `rescan`, both implemented in `vertice-app` as identical thin async pass-throughs to the core scan operation; a separate freshness command returning the freshness report; and two settings commands, one reading the persisted freshness settings and one writing them. The two settings commands are not optional conveniences: the confirmed default posture requires a visible opt-out and a first-run disclosure, and neither can function without a way to read and mutate that persisted state. The settings-write command SHALL be the only command in the shell permitted to cause a write, and only to the sanctioned settings/cache location inside the app data directory. Commands MUST contain no business logic — no filtering, no transformation of the report, no caching beyond the sanctioned freshness response cache, no state. `vertice-core` MUST remain unchanged by `scan`/`rescan`. Because no cache exists for the scan itself, `rescan` remains semantically identical to `scan`. The freshness command MUST be invokable independently of `scan`/`rescan`, MUST NOT be awaited by them, and its failure or slowness MUST NOT affect the outcome or timing of a scan invocation.

#### Scenario: Successful scan returns the consolidated report

- GIVEN registered user roots with discoverable components
- WHEN the frontend invokes `scan`
- THEN the command returns the core scan's `ScanReport` — components, installations, scanned roots, issues, and measured duration — unmodified

#### Scenario: Rescan behaves identically to scan

- GIVEN any number of prior invocations
- WHEN the frontend invokes `rescan`
- THEN it performs a full fresh core scan exactly as `scan` does
- AND no cached or stored result is read, reused, or invalidated for the scan itself

#### Scenario: Scan issues surface without command failure

- GIVEN a scanned root contains an unreadable component
- WHEN `scan` is invoked
- THEN the command resolves successfully with a report containing the corresponding issue entries
- AND per-component issues do not become command errors

#### Scenario: The freshness command is independent of the scan command

- GIVEN a scan has already completed and rendered
- WHEN the freshness command is invoked separately
- THEN it returns the freshness report without re-running or blocking on `scan` or `rescan`
- AND a slow or failing freshness lookup does not delay or fail a concurrent or subsequent scan invocation

### Requirement: Non-Blocking Command Execution

All five commands SHALL be async and MUST offload their blocking work onto the Tauri async runtime's blocking facility (`spawn_blocking`) or an equivalent non-blocking mechanism. A scan taking up to the CA-15 two-second budget MUST NOT block the main thread or freeze the UI. The freshness command's network-bound work MUST likewise never block the main thread, and its latency MUST NOT be included in, or count against, the CA-15 scan budget.

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

The shell capability declaration SHALL grant `core:default` only: no filesystem plugin, no filesystem scopes, no shell or dialog permissions. This MUST remain true after the freshness command is added — a direct Rust-side HTTP client requires no Tauri capability grant. The audited desktop surface MUST show that the webview has zero filesystem mutation capability over scanned roots, including content writes, truncation, creation, deletion, rename/link creation, permission changes, and equivalent indirect mutation paths. The audit policy MUST cover the capability file plus the command-exposed desktop surface, including the new freshness command, and MUST avoid claiming that text inspection alone proves all transitive write absence. Verification evidence MUST name the audited capability and command surfaces used to support CA-16.

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

#### Scenario: The freshness command adds no new capability grant

- GIVEN the freshness command is registered
- WHEN the capability declaration is reviewed
- THEN it remains `core:default` only, with no filesystem, shell, network, or dialog permission added for the new command

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
