# Delta for Desktop Shell

## MODIFIED Requirements

### Requirement: Minimal Scan Command Surface

The shell SHALL expose exactly six commands: `scan` and `rescan`, both implemented in `vertice-app`
as identical thin async pass-throughs to the core scan operation; a separate freshness command
returning the freshness report; two settings commands, one reading the persisted freshness settings
and one writing them; and a read-only command returning the absolute path of the application log
file as a plain `String`. The two settings commands are not optional conveniences: the confirmed
default posture requires a visible opt-out and a first-run disclosure, and neither can function
without a way to read and mutate that persisted state. The settings-write command SHALL be the only
command in the shell permitted to cause a write, and only to the sanctioned settings/cache location
inside the app data directory. The log-path command MUST NOT create, write, or otherwise mutate the
log file or its directory — it MUST only compute and return the path. Commands MUST contain no
business logic — no filtering, no transformation of the report, no caching beyond the sanctioned
freshness response cache, no state. `vertice-core` MUST remain unchanged by `scan`/`rescan`. Because
no cache exists for the scan itself, `rescan` remains semantically identical to `scan`. The
freshness command MUST be invokable independently of `scan`/`rescan`, MUST NOT be awaited by them,
and its failure or slowness MUST NOT affect the outcome or timing of a scan invocation.
(Previously: the surface was exactly five commands — `scan`, `rescan`, `freshness`,
`freshness_settings`, `set_freshness_settings` — with no log-path command.)

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

#### Scenario: The log-path command returns the path without touching the file

- GIVEN the log sink has or has not yet been initialized
- WHEN the frontend invokes the log-path command
- THEN it returns the absolute path derived from `app_data_dir()` as a plain string
- AND no file or directory is created, opened, or modified as a side effect of the invocation

### Requirement: Minimal Capability Grant

The shell capability declaration SHALL grant `core:default` only: no filesystem plugin, no
filesystem scopes, no shell or dialog permissions. This MUST remain true after the freshness
command and the log-path command are added — a direct Rust-side HTTP client and a `String`-returning
path computation both require no Tauri capability grant. The audited desktop surface MUST show that
the webview has zero filesystem mutation capability over scanned roots, including content writes,
truncation, creation, deletion, rename/link creation, permission changes, and equivalent indirect
mutation paths. The audit policy MUST cover the capability file plus the command-exposed desktop
surface, including the freshness command and the log-path command, and MUST avoid claiming that text
inspection alone proves all transitive write absence. Verification evidence MUST name the audited
capability and command surfaces used to support CA-16.
(Previously: covered the freshness command as the most recent addition; did not yet name the
log-path command.)

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

#### Scenario: The log-path command adds no new capability grant

- GIVEN the log-path command is registered
- WHEN the capability declaration is reviewed
- THEN it remains `core:default` only — no filesystem, shell, or dialog permission is added for a
  command that only returns a computed string

## ADDED Requirements

### Requirement: The Read-Only Audit Recognizes A Second Write Exception

`crates/vertice-app/tests/read_only_audit.rs` MUST name exactly two sanctioned write-exception
modules: the existing freshness-cache module and the logging sink module. Each exception MUST be
proved individually — path derived from `app_data_dir()`, no literal absolute path, no `std::env::`
read — rather than accepted merely by presence in the exception list.

#### Scenario: The audit proves the logging sink exception on its own merits

- GIVEN the logging sink module's source, stripped of `#[cfg(test)]` bodies
- WHEN the read-only audit runs
- THEN it independently verifies the module references `app_data_dir()`, contains no literal
  absolute path, and reads no environment variable directly
- AND a module outside the two named exceptions containing a forbidden mutation pattern still fails
  the audit
