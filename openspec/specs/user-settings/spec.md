# User Settings Specification

## Purpose

Define the durable, whole-document user configuration this change introduces: `settings.json`
inside the application data directory, its three fields (`locale`, `enabled`, `disclosure_seen`),
its read/partial-patch-write IPC command pair (`user_settings` / `set_user_settings`, a rename and
repurposing of the former `freshness_settings` / `set_freshness_settings` pair per the
`desktop-shell` capability's six-command-surface constraint), its write durability, and — the
subtle part — the asymmetric fallback each field takes depending on a three-way load outcome
(missing, unreadable, loaded). This document replaces `enabled` and `disclosure_seen`'s previous
home inside the disposable `component-freshness` cache and gives the frontend-i18n locale choice its
first durable home.

## Requirements

### Requirement: A Single Durable Settings Document Holds Locale And Opt-Out State

`vertice-app` MUST persist exactly one settings document, `settings.json`, inside the application
data directory (CA-16), holding three fields: `locale` (the persisted UI language choice, optional —
absent means no explicit choice has been made), `enabled` (the freshness check opt-out), and
`disclosure_seen` (whether the freshness disclosure has been shown). `vertice-core`'s `model/` MUST
define the plain, I/O-free `UserSettings` type consumed by this document; `vertice-app` MUST own all
file I/O against it. No other module MUST write to `settings.json`.

#### Scenario: The settings document lives inside the app data directory

- GIVEN a completed settings write
- WHEN the filesystem is inspected for the write this capability introduces
- THEN `settings.json` is located inside the application data directory and nowhere else

#### Scenario: The settings type is defined in core without I/O

- GIVEN the `UserSettings` type consumed by the IPC contract
- WHEN its defining module is inspected
- THEN it is defined in `vertice-core`'s `model/`, contains no filesystem or I/O call, and derives
  `ts_rs::TS` for binding generation

### Requirement: A Read Command And A Partial-Patch Write Command Expose The Settings Document

The shell MUST expose exactly two commands for this document — `user_settings` (read) and
`set_user_settings` (write) — counted within, not in addition to, the `desktop-shell` capability's
six-command surface; they are the rename and repurposing of the prior `freshness_settings` /
`set_freshness_settings` pair, not new commands. `user_settings` MUST read the current settings and
MUST create no file as a side effect of reading. `set_user_settings` MUST accept `locale`, `enabled`,
and `disclosure_seen` as three independently optional fields and MUST treat an omitted (`None`)
field as "leave this field's persisted value unchanged" — never as a request to reset it to a
default. This is a deliberate departure from the project's prior always-send-full-state convention:
two independent frontend owners write this document (the application shell for `locale`, the clients
page for `enabled` and `disclosure_seen`), and a full-state write built from either owner's stale
in-memory copy would silently overwrite the other's just-written field — for `enabled` specifically,
a stale full-state write could silently re-enable outbound network requests the user had just turned
off, which is exactly the failure the asymmetric fallback below exists to prevent. `set_user_settings`
MUST return the settings actually persisted after applying the patch, so the caller never has to
guess whether the write landed. Both commands MUST be async and MUST offload their blocking file I/O
onto the async runtime's blocking facility, consistent with every other command in the shell.

#### Scenario: Reading settings does not create the file

- GIVEN `settings.json` does not yet exist
- WHEN `user_settings` is invoked
- THEN it returns the documented first-run defaults
- AND no file is created as a side effect of the read

#### Scenario: A patch changes only the fields it names

- GIVEN a persisted document with `locale: "es"`, `enabled: false`, `disclosure_seen: true`
- WHEN `set_user_settings` is invoked with `locale: Some("en")`, `enabled: None`, `disclosure_seen: None`
- THEN the persisted `locale` becomes `"en"`
- AND `enabled` remains `false` and `disclosure_seen` remains `true`, unchanged by the patch
- AND the command's own return value already reflects the new `locale` alongside the unchanged fields

#### Scenario: One writer's patch does not clobber a field only the other writer owns

- GIVEN the application shell issues `set_user_settings(locale: Some("es"), enabled: None, disclosure_seen: None)`
- AND the clients page had previously persisted `enabled: false`
- WHEN the locale patch is applied
- THEN the persisted `enabled` remains `false`
- AND no test asserting this may pass by asserting a full-state write instead of a true partial patch

#### Scenario: Both commands run off the main thread

- GIVEN either settings command is invoked
- WHEN it performs its file I/O
- THEN the work is offloaded to the async runtime's blocking facility and the main thread remains
  responsive

### Requirement: An Explicit User Choice Survives A Full Application Restart

Once `set_user_settings` persists a value for `locale`, `enabled`, or `disclosure_seen`, that value
MUST be read back unchanged after a full application restart, including on a machine where the
application data directory did not previously exist — the directory MUST be created before the
write is attempted, following the same sanctioned-exception pattern as the freshness cache module.
The write MUST be staged to a temporary file in the same directory and committed via an atomic
rename, so a write that is interrupted before the rename leaves the previously-persisted document
intact rather than a torn, half-written `settings.json`.

#### Scenario: A first-ever write survives a restart with no prior app data directory

- GIVEN a machine where the application data directory has never existed
- WHEN `set_user_settings` persists `locale: Some("es")`
- THEN the parent directory is created before the write, the write succeeds, and a restarted
  application's `user_settings` call reads back `locale: "es"`

#### Scenario: An opt-out survives a restart

- GIVEN the user disables the freshness check via `set_user_settings(enabled: Some(false))`
- WHEN the application is fully restarted
- THEN `user_settings` reports `enabled: false`
- AND no outbound freshness request is made before the user re-enables it

#### Scenario: An interrupted write leaves the previous document intact

- GIVEN a settings write that stages its temporary file but is interrupted before the commit rename
- WHEN `user_settings` is subsequently invoked
- THEN it returns the settings that were persisted before the interrupted write began, not a
  partially-written or corrupt value

### Requirement: The Load Outcome Is A Three-Way Classification That Drives An Asymmetric Fallback

Reading `settings.json` MUST classify into exactly three outcomes, evaluated in this order, before
any field-level fallback is applied: `Missing` (the file does not exist — a genuine first run, or
`fs::read_to_string` fails with a not-found error); `Unreadable` (the file exists but cannot be
trusted — a non-not-found I/O error reading it, its contents being empty or consisting only of
whitespace after trimming, or its contents failing to parse as the documented schema); and `Loaded`
(the file exists, is non-empty, and parses successfully, in which case each field's stored value is
used, falling back per-field to the ordinary defaults below only for a field genuinely absent from
an otherwise-valid document). An empty or whitespace-only file MUST be classified as `Unreadable`,
not `Missing`, because the write path never produces an empty committed file — its existence is
evidence of an anomaly (for example an interrupted process leaving a zero-byte artifact through a
means other than this capability's own atomic-rename write), not of a first run.

The three fields MUST NOT share one uniform fallback across these outcomes. `locale` and
`disclosure_seen` MUST fall back to their ordinary documented defaults (no explicit choice, so the
frontend-i18n browser-precedence resolution applies, for `locale`; `false` for `disclosure_seen`)
identically for both `Missing` and `Unreadable` — the distinction is immaterial for these two fields
because neither failure mode has a harmful direction. `enabled` MUST NOT follow that same uniform
rule: for `Missing`, `enabled` MUST resolve to its documented first-run default of `true`. For
`Unreadable`, `enabled` MUST resolve to `false` instead of `true` — the resolution MUST distinguish
"no document" from "an unreadable document" for this field specifically, because collapsing them
into one default would silently resume outbound network requests the user had explicitly turned
off. This asymmetry MUST be covered by dedicated tests for the `Missing` and `Unreadable` branches
separately, including a dedicated case for an empty file and a dedicated case for a whitespace-only
file; a single test asserting one uniform default, or a single `Unreadable` test covering only one
of its producing conditions, is insufficient to demonstrate compliance.

#### Scenario: A genuine first run yields the ordinary defaults for locale and disclosure, and true for enabled

- GIVEN `settings.json` has never existed on this machine
- WHEN `user_settings` is invoked
- THEN `enabled` resolves to `true`, `disclosure_seen` resolves to `false`, and `locale` resolves to
  absent (so frontend-i18n's browser-precedence resolution applies)

#### Scenario: A corrupt existing document yields false for enabled, ordinary defaults for the rest

- GIVEN `settings.json` exists on disk but its contents fail to parse
- WHEN `user_settings` is invoked
- THEN `enabled` resolves to `false`, `disclosure_seen` resolves to `false`, and `locale` resolves
  to absent
- AND no further outbound freshness request is made until the user explicitly re-enables the check

#### Scenario: An empty file is classified as Unreadable, not Missing

- GIVEN `settings.json` exists on disk with zero bytes of content
- WHEN `user_settings` is invoked
- THEN the load outcome is `Unreadable`, and `enabled` resolves to `false`, not to the `Missing`
  branch's `true`

#### Scenario: A whitespace-only file is classified as Unreadable, not Missing

- GIVEN `settings.json` exists on disk containing only whitespace characters
- WHEN `user_settings` is invoked
- THEN the load outcome is `Unreadable`, and `enabled` resolves to `false`

#### Scenario: The Missing and Unreadable branches are independently tested

- GIVEN the settings-load test suite
- WHEN it is inspected
- THEN it contains a dedicated case for a never-existing file, a dedicated case for an empty file, a
  dedicated case for a whitespace-only file, and a dedicated case for a file with invalid contents,
  and at least the never-existing case and one `Unreadable`-producing case assert different `enabled`
  outcomes

### Requirement: No Migration Path From The Former Freshness-Cache Location

This capability MUST NOT read or migrate `enabled` or `disclosure_seen` from the pre-existing
freshness response cache document. A machine carrying a pre-existing cache document with those
fields still present MUST have `settings.json`'s fields resolve exactly as if the cache document did
not exist — the `Missing`-outcome defaults apply, not a value carried over from the cache.

#### Scenario: A pre-existing cache document with legacy fields is not consulted

- GIVEN a `freshness-cache.json` file on disk still containing `enabled` or `disclosure_seen` fields
  from before this change, and no `settings.json` file
- WHEN `user_settings` is invoked
- THEN it returns the `Missing`-outcome defaults, ignoring any value present in the cache document

### Requirement: The Settings Write Path Is A Sanctioned, Individually-Proved Exception To The Read-Only Audit

The settings write path MUST be confined to one dedicated module (`settings/store.rs`), distinct
from the freshness cache module — the two documents have deliberately different write semantics
(cheap whole-file write for the disposable cache; stage-and-rename for the durable document), so
the settings module's audit exception is not a re-point of the freshness cache module's existing
exception but a genuinely new, third one. `crates/vertice-app/tests/read_only_audit.rs` MUST name it
as an independently-proved sanctioned exception: its path derivation MUST reference
`app_data_dir()`, MUST contain no literal absolute path, and MUST NOT read `std::env::` directly.
Its allow-list MUST be pinned to exactly the operations its write path performs (`fs::write`,
`create_dir_all`, `fs::rename`) and MUST deny every other filesystem mutation primitive denied
elsewhere in the audit. This module joins, rather than replaces, the existing sanctioned exceptions,
bringing the audit's total to three.

#### Scenario: The settings module is proved on its own merits

- GIVEN the settings module's source, stripped of `#[cfg(test)]` bodies
- WHEN the read-only audit runs
- THEN it independently verifies the module references `app_data_dir()`, contains no literal
  absolute path, and reads no environment variable directly

#### Scenario: The settings write path stays confined to the app data directory

- GIVEN a completed settings write
- WHEN the filesystem is inspected outside the application data directory
- THEN no write occurred there as a result of `set_user_settings`

#### Scenario: The settings module's allow-list does not extend beyond its three operations

- GIVEN the settings module's source
- WHEN the read-only audit inspects it for filesystem mutation primitives
- THEN it finds only `fs::write`, `create_dir_all`, and `fs::rename`
- AND `remove_file`, `remove_dir`, `OpenOptions`, `File::create`, `.write_all(`, `.set_len(`,
  `set_permissions`, `hard_link`, and `symlink_*` are all absent from the module
