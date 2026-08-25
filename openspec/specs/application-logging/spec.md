# Application Logging Specification

## Purpose

Define the persistent, human-readable diagnostic log this change introduces: where it lives, its
line format, the four event classes it MUST record, its rotation/retention bound, and the two
distinct failure classes that govern its own reliability. The log is a local, user-owned file the
maintainer asks a user to send; it is not telemetry (design principle 8) and nothing transmits it.

## Requirements

### Requirement: Log Sink Location Inside The App Data Directory

The log file MUST be located inside `app_data_dir()` (Tauri `Manager::path().app_data_dir()`) and
MUST NOT be written to any other location, including a literal path under the OS home directory.
Exactly one module in `vertice-app` MUST own opening, writing, and rotating the log file; no other
module MAY do so. That module's path derivation MUST reference `app_data_dir()`, MUST contain no
literal absolute path, and MUST NOT read `std::env::` directly.

#### Scenario: The sink creates its own directory on first use

- GIVEN `app_data_dir()` resolves to a directory that does not yet exist
- WHEN the application starts and the sink initializes
- THEN the sink module creates that directory before opening the log file
- AND no other module in the workspace performs the directory creation

#### Scenario: The sink module derives its path exclusively from app_data_dir

- GIVEN the sink module's source
- WHEN it is inspected
- THEN it references `app_data_dir()`, contains no literal absolute path, and reads no environment
  variable directly

### Requirement: Fixed-Column Plain-Text Line Format

Each log line MUST be plain text, one event per line, in fixed column order: a timestamp carrying
the date, the time to at least second precision, and an explicit UTC offset; the level; the
emitting source file and line number; and the message. The column order MUST NOT drift silently.

#### Scenario: A logged line carries source file, timestamp, and offset

- GIVEN any event this spec requires to be logged
- WHEN the corresponding line is written
- THEN it contains a date, a time to at least second precision, an explicit UTC offset, the
  emitting source file, and the message, in the fixed column order

### Requirement: The Four Required Event Classes Are Recorded

The application MUST log: (1) application startup; (2) scan start and scan end, with the scan's
measured duration; (3) every search root reported `SearchRootStatus::NotFound` and every AI client
reported `ClientPresenceStatus::NotDetected` for a completed scan; (4) every freshness check that
resolves to `Freshness::Unknown`, together with its `reason`. No other event class is required by
this change.

#### Scenario: Startup is logged once

- GIVEN the application launches and the sink initializes successfully
- WHEN `run()` completes its startup sequence
- THEN exactly one startup line is written to the log

#### Scenario: A scan logs its start, end, and duration

- GIVEN a scan or rescan invocation completes
- WHEN the returned `ScanReport` is available
- THEN the log contains a start line and an end line for that invocation, and the end line carries
  `ScanReport.duration_ms`

#### Scenario: A missing root and an undetected client are both logged

- GIVEN a `ScanReport` whose `roots_scanned` contains a `SearchRootStatus::NotFound` entry and
  whose `client_presence` contains a `ClientPresenceStatus::NotDetected` entry
- WHEN the scan completes
- THEN the log contains one line for the missing root and one line for the undetected client

#### Scenario: A freshness-unknown verdict is logged with its reason

- GIVEN a `FreshnessReport` whose `checks` contains an entry with `Freshness::Unknown { reason }`
- WHEN the freshness command completes
- THEN the log contains a line for that entry carrying the `reason` value verbatim

### Requirement: Size-Bounded Rotation With One Retained Predecessor

Before writing a line, if the current log file is at or above 1 MiB, the sink MUST rotate: the
current file is renamed over the single predecessor slot (overwriting any existing predecessor),
and a new, empty current file is started. The on-disk footprint MUST NOT exceed two files.

#### Scenario: Writing at or above the size threshold triggers rotation

- GIVEN the current log file is at or above 1 MiB
- WHEN the sink is about to write the next line
- THEN the current file is renamed over the predecessor slot and a new empty current file receives
  the line
- AND at most two log files exist on disk afterward

#### Scenario: A fresh install has no predecessor file

- GIVEN the application has never rotated its log
- WHEN the log directory is inspected
- THEN exactly one log file exists

### Requirement: A Per-Line Write Failure Is Silent And Never Fails A Scan

A failure to write a single log line MUST NOT cause `scan`, `rescan`, `freshness`, or the
settings-write path to fail, MUST NOT change their returned result, and MUST NOT be surfaced to the
caller.

#### Scenario: A log write failure does not fail or alter a scan's result

- GIVEN the log sink cannot write a line during a scan
- WHEN the scan command completes
- THEN it resolves with the same `ScanReport` it would have produced had the write succeeded
- AND the write failure is not surfaced to the command caller

### Requirement: Sink Initialisation Failure Is Reported Once, On Stderr

If the sink cannot create the log directory or open the log file at startup, the application MUST
still start and MUST still perform all other functions normally, but MUST emit exactly one
diagnostic message to stderr describing the initialisation failure. No further per-line failure
after a failed initialisation MUST produce additional stderr output.

#### Scenario: Sink initialisation failure is reported exactly once

- GIVEN the log directory cannot be created or the log file cannot be opened at startup
- WHEN the application starts
- THEN exactly one message is written to stderr describing the failure
- AND the application continues to start and function normally

#### Scenario: A failed initialisation does not repeat the stderr message

- GIVEN sink initialisation already failed and reported the one stderr message
- WHEN subsequent scan/rescan/freshness operations attempt to log
- THEN no additional stderr output is produced for those per-line failures
