# Delta for Component Freshness

## ADDED Requirements

### Requirement: The Application Data Directory Is Created Before The Sanctioned Path Writes To It

The sanctioned settings/cache write path MUST create its parent directory, derived exclusively from
`app_data_dir()`, before attempting to write to it. This directory creation MUST remain confined to
the existing sanctioned exception module (`freshness/cache.rs`); it MUST NOT be performed by any
other module, and it MUST NOT require adding a new named exception to the read-only audit. On a
machine where the application data directory has never existed, the freshness response cache MUST
persist across restarts and a freshness opt-out setting written via the settings-write command MUST
survive an application restart, instead of silently reverting.

#### Scenario: The cache write succeeds when the app data directory does not yet exist

- GIVEN `app_data_dir()` resolves to a directory that has never been created
- WHEN the freshness response cache is written
- THEN the parent directory is created first, inside the sanctioned exception module, and the write
  succeeds

#### Scenario: Disabling the freshness check survives a restart (regression)

- GIVEN a machine where the application data directory did not previously exist
- WHEN the user disables the freshness check via the settings-write command, then restarts the
  application
- THEN the setting reads back as disabled after restart, and no further outbound freshness request
  is made — the setting does not silently revert

### Requirement: Freshness-Unknown Verdicts Are Also Recorded In The Application Log

Every `Freshness::Unknown { reason }` verdict produced by the freshness command MUST also be written
to the application log, per the `application-logging` specification's event-coverage requirement.
This is an orthogonal sink: it does not change `Freshness`'s three-valued vocabulary, the comparison
rules, or any other requirement in this specification.

#### Scenario: A freshness-unknown verdict appears in both the report and the log

- GIVEN a subject whose freshness evaluation resolves to `Freshness::Unknown { reason }`
- WHEN the freshness command completes
- THEN the returned `FreshnessReport` contains that verdict as before
- AND the application log contains a corresponding line carrying the same reason
