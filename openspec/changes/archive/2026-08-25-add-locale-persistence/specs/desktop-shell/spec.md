# Delta for Desktop Shell

## MODIFIED Requirements

### Requirement: Minimal Scan Command Surface

The shell SHALL expose exactly six commands: `scan` and `rescan`, both implemented in `vertice-app`
as identical thin async pass-throughs to the core scan operation; a separate freshness command
returning the freshness report; two user-settings commands, one reading the persisted durable
settings document (`locale`, `enabled`, `disclosure_seen`) and one writing it; and a read-only
command returning the absolute path of the application log file as a plain `String`. The two
user-settings commands are not optional conveniences: the confirmed default posture requires a
visible opt-out and a first-run disclosure, and the frontend's persisted locale choice requires a
way to read and mutate that persisted state before the first paint. `user_settings` and
`set_user_settings` are a rename and repurposing of the prior `freshness_settings` /
`set_freshness_settings` pair, not an addition — `enabled` and `disclosure_seen` moved out of the
freshness response cache into the durable settings document this pair now owns, and `locale` joined
them, so the command count stays exactly six. `set_user_settings` MUST accept each of `locale`,
`enabled`, and `disclosure_seen` as an independently optional field (a partial patch), where an
omitted field MUST leave that field's persisted value unchanged — not a full-state write of all
three fields on every call. The log-path command MUST NOT create, write, or otherwise mutate the log
file or its directory — it MUST only compute and return the path. Commands MUST contain no business
logic — no filtering, no transformation of the report, no caching beyond the sanctioned freshness
response cache, no state. `vertice-core` MUST remain unchanged by `scan`/`rescan`. Because no cache
exists for the scan itself, `rescan` remains semantically identical to `scan`. The freshness command
MUST be invokable independently of `scan`/`rescan`, MUST NOT be awaited by them, and its failure or
slowness MUST NOT affect the outcome or timing of a scan invocation.
(Previously: the surface was exactly six commands — `scan`, `rescan`, `freshness`,
`freshness_settings`, `set_freshness_settings`, `log_file_path` — where the settings pair only
covered the freshness opt-out and disclosure flag and always sent full state on write. This change
renames that pair to `user_settings` / `set_user_settings`, widens its payload with `locale`, and
changes its write to a partial patch. The command count does not change.)

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

#### Scenario: The command surface stays at six after the rename

- GIVEN the registered `generate_handler!` list after this change
- WHEN it is inspected
- THEN it contains exactly `scan`, `rescan`, `freshness`, `user_settings`, `set_user_settings`, and
  `log_file_path`, and no `freshness_settings` or `set_freshness_settings` entry remains

#### Scenario: A partial patch changes only the fields it names

- GIVEN a persisted document with `locale: "es"`, `enabled: false`, `disclosure_seen: true`
- WHEN `set_user_settings` is invoked with `locale: Some("en")` and `enabled: None` and
  `disclosure_seen: None`
- THEN the persisted `locale` becomes `"en"`
- AND `enabled` and `disclosure_seen` remain unchanged at `false` and `true`

#### Scenario: Two independent writers do not clobber each other's field

- GIVEN the application shell writes `locale` through `set_user_settings` immediately followed by
  the clients page writing `enabled` through a second, concurrent `set_user_settings` invocation
  carrying only `enabled`
- WHEN both invocations have completed
- THEN the persisted document reflects both the new `locale` and the new `enabled` value
- AND neither invocation's omitted fields overwrite the other's just-written field with a stale copy

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

### Requirement: Minimal Capability Grant

The shell capability declaration SHALL grant `core:default` only: no filesystem plugin, no
filesystem scopes, no shell or dialog permissions. This MUST remain true after the freshness
command, the user-settings command pair, and the log-path command — a direct Rust-side HTTP client,
a read-modify-write against a local file, and a `String`-returning path computation all require no
Tauri capability grant. Exactly two command paths in the shell can cause a filesystem write, and
each is confined to one sanctioned module and one document: the `freshness` command persists
refreshed reference-lookup cache entries via the freshness cache module (`freshness/cache.rs`,
`freshness-cache.json`), and `set_user_settings` persists the durable settings document via the
settings store module (`settings/store.rs`, `settings.json`). No other command path — `scan`,
`rescan`, `user_settings`, or `log_file_path` — causes any write. The audited desktop surface MUST
show that the webview has zero filesystem mutation capability over scanned roots, including content
writes, truncation, creation, deletion, rename/link creation, permission changes, and equivalent
indirect mutation paths. The audit policy MUST cover the capability file plus the command-exposed
desktop surface, including the freshness command, the user-settings command pair, and the log-path
command, and MUST avoid claiming that text inspection alone proves all transitive write absence.
Verification evidence MUST name the audited capability and command surfaces used to support CA-16.
(Previously: claimed the settings-write command was "the only command in the shell permitted to
cause a write," which had already drifted — the freshness command persists cache entries via
`build_report` independently of the settings-write path. This corrects that pre-existing wording
error by naming both write-capable command paths and the module each one writes through; it is not
a behavior change caused by this change.)

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

#### Scenario: The renamed user-settings pair adds no new capability grant

- GIVEN `user_settings` and `set_user_settings` are registered in place of the former
  `freshness_settings` / `set_freshness_settings` pair
- WHEN the capability declaration is reviewed
- THEN it remains `core:default` only — the rename and payload widening add no filesystem, shell,
  network, or dialog permission

#### Scenario: Exactly two command paths can cause a write, each to its own document

- GIVEN the full registered command surface after this change
- WHEN each command is traced for filesystem side effects
- THEN only `freshness` (writing `freshness-cache.json` via `freshness/cache.rs`) and
  `set_user_settings` (writing `settings.json` via `settings/store.rs`) cause a write
- AND `scan`, `rescan`, `user_settings`, and `log_file_path` cause none

### Requirement: The Read-Only Audit Recognizes A Third Write Exception

`crates/vertice-app/tests/read_only_audit.rs` MUST name exactly three sanctioned write-exception
modules: the freshness cache module (`freshness/cache.rs`), the logging sink module, and the
settings store module (`settings/store.rs`) introduced by this change. Each exception MUST be proved
individually — path derived from `app_data_dir()`, no literal absolute path, no `std::env::` read —
rather than accepted merely by presence in the exception list. The settings store module's allow-list
MUST be pinned to exactly the filesystem operations its temp-file-plus-rename write path performs
(`fs::write`, `create_dir_all`, `fs::rename`) and MUST deny every other mutation primitive denied
elsewhere in the audit (`remove_file`, `remove_dir`, `OpenOptions`, `File::create`, `.write_all(`,
`.set_len(`, `set_permissions`, `hard_link`, `symlink_*`) inside that same module.
(Previously: "The Read-Only Audit Recognizes A Second Write Exception" named exactly two sanctioned
write-exception modules — the freshness cache module and the logging sink module — because the
freshness cache module's TTL'd write and the durable settings write had not yet been split apart.)

#### Scenario: The audit proves the logging sink exception on its own merits

- GIVEN the logging sink module's source, stripped of `#[cfg(test)]` bodies
- WHEN the read-only audit runs
- THEN it independently verifies the module references `app_data_dir()`, contains no literal
  absolute path, and reads no environment variable directly
- AND a module outside the three named exceptions containing a forbidden mutation pattern still fails
  the audit

#### Scenario: The audit proves the settings store exception on its own merits

- GIVEN the settings store module's source, stripped of `#[cfg(test)]` bodies
- WHEN the read-only audit runs
- THEN it independently verifies the module references `app_data_dir()`, contains no literal
  absolute path, and reads no environment variable directly

#### Scenario: The settings store's allow-list does not extend beyond its own three operations

- GIVEN the settings store module's source
- WHEN the read-only audit inspects it for filesystem mutation primitives
- THEN it finds only `fs::write`, `create_dir_all`, and `fs::rename`
- AND `remove_file`, `remove_dir`, `OpenOptions`, `File::create`, `.write_all(`, `.set_len(`,
  `set_permissions`, `hard_link`, and `symlink_*` are all absent from the module

#### Scenario: The audit's exception count is exactly three

- GIVEN the read-only audit's list of sanctioned writer modules
- WHEN it is inspected after this change
- THEN it contains exactly three entries, and no fourth exception has been introduced
