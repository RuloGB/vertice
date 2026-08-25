# Delta for Component Freshness

## MODIFIED Requirements

### Requirement: The Check Is Enabled By Default, Disclosed, And Fully Stoppable

The freshness check's `enabled` flag and the `disclosure_seen` flag MUST be read from and written to
the durable `user-settings` capability's settings document, not the freshness response cache
document — the cache document is disposable and TTL'd and MUST NOT carry either flag. On first
exposure to the feature, the user MUST be shown a disclosure stating that public registries are
queried and that no information about the user or their machine is sent. A visible, discoverable
setting MUST let the user disable the check, writing through the `user-settings` write command. Once
disabled, the application MUST NOT make any further outbound request for freshness purposes of any
kind, for any subject, until the check is re-enabled. Every outbound request MUST carry no unique
identifier, no machine fingerprint, no inventory data, no component names, and no filesystem paths.

`enabled`'s default is context-dependent and MUST resolve as follows, never as one uniform default:
when the settings document has never existed (a genuine first run), `enabled` MUST resolve to
`true`. When the settings document exists but is unreadable, corrupt, or fails to parse, `enabled`
MUST resolve to `false` — conservatively, so a read failure never silently resumes outbound
requests the user had turned off. `disclosure_seen` MUST fall back to `false` silently in both of
those cases, with no distinction between them, since re-showing the disclosure is safe.
(Previously: `enabled` and `disclosure_seen` were persisted inside the freshness response cache
document (`freshness-cache.json`) via `FreshnessStore`, whose `load` silently treated any missing,
corrupt, or unreadable file as an empty document — collapsing the never-existed and
exists-but-corrupt cases into the same uniform default, which is exactly wrong for `enabled`.)

#### Scenario: Disabling the check stops all further outbound requests

- GIVEN the freshness check is disabled through the setting
- WHEN a subsequent scan or rescan occurs
- THEN no outbound freshness request of any kind is made for any subject

#### Scenario: An outbound request carries no identifying content

- GIVEN a live reference-version request is made
- WHEN its content is inspected
- THEN it contains no unique identifier, no machine fingerprint, no inventory data, no component name, and no filesystem path

#### Scenario: The disclosure is shown before or at the first check

- GIVEN the user has not previously seen the freshness disclosure
- WHEN the feature is first exposed
- THEN the disclosure text, stating that public registries are queried and that nothing about the user is sent, is shown before or alongside the first outbound request

#### Scenario: A genuine first run enables the check by default

- GIVEN the settings document has never existed on this machine
- WHEN the application reads the freshness `enabled` setting
- THEN it resolves to `true`

#### Scenario: An unreadable or corrupt settings document disables the check conservatively

- GIVEN the settings document exists on disk but is corrupt, unreadable, or fails to parse
- WHEN the application reads the freshness `enabled` setting
- THEN it resolves to `false`, not to its normal first-run default of `true`
- AND no outbound freshness request is made until the user explicitly re-enables the check

#### Scenario: A corrupt settings document does not block the disclosure from reappearing

- GIVEN the settings document exists on disk but is corrupt, unreadable, or fails to parse
- WHEN the application reads the `disclosure_seen` setting
- THEN it resolves to `false`
- AND the first-run disclosure is shown again, independently of the `enabled` fallback resolving to `false`

### Requirement: The Response Cache Is The Only New Write, Confined To The App Data Directory

The freshness response cache document (`freshness-cache.json`) MUST contain only TTL'd
reference-lookup response entries, keyed per subject — it MUST NOT contain `enabled` or
`disclosure_seen`, which live exclusively in the durable `user-settings` document. That cache MUST
be located inside the application's own data directory (CA-16) and MUST NOT be written anywhere
else. Its presence or absence MUST NOT change correctness: deleting the cache MUST NOT cause a
crash, only a live lookup or `Unknown`. The cache MUST keep its existing disposable, whole-file write
semantics (no temp-file-plus-rename): a torn write is indistinguishable from, and treated the same
as, a corrupt cache.
(Previously: the same document additionally held `enabled` and `disclosure_seen`, which this change
migrates to the `user-settings` capability's durable document.)

#### Scenario: The cache write stays inside the app data directory

- GIVEN a completed live reference lookup that is cached
- WHEN the filesystem is inspected for new writes introduced by this capability
- THEN the only new write is located inside the application data directory

#### Scenario: A missing cache degrades gracefully

- GIVEN the cache file does not exist
- WHEN freshness is evaluated
- THEN the outcome is a live lookup or `Unknown`, never a crash

#### Scenario: The cache document never carries settings fields

- GIVEN the freshness response cache document on disk
- WHEN it is inspected after this change
- THEN it contains only TTL'd reference-lookup entries keyed per subject
- AND it contains no `enabled` or `disclosure_seen` field
