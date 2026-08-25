# Delta for Scan Orchestration

## MODIFIED Requirements

### Requirement: Visible and Isolated Diagnostics

The scan operation MUST accumulate diagnostics for non-parseable components with their paths,
undetected supported clients, and absent roots. It MUST NOT omit those conditions silently. A
failure from one adapter MUST NOT abort the remaining adapters; their available results and
diagnostics MUST remain represented in the report. A failed or degraded freshness lookup MUST NOT
produce a `ScanIssue` under any circumstance; it is represented exclusively as `Freshness::Unknown`
in the separate freshness report, never in `ScanReport.issues`. The application log introduced by
`add-application-logging` is an orthogonal sink over these same diagnostics: it MUST NOT change
`ScanReport` or `ScanIssue` semantics, MUST NOT add a new diagnostic field or variant to either type,
and observing a report for logging purposes MUST NOT alter the report returned to the caller.
(Previously: did not address the relationship between `ScanReport`/`ScanIssue` and the application
log, because no log existed.)

#### Scenario: Unreadable component does not interrupt the scan

- GIVEN a registered root contains an unreadable or corrupt component and another adapter has valid fixture input
- WHEN the public core scan operation runs
- THEN the report includes a non-parseable issue with the component path
- AND it includes the valid result from the other adapter

#### Scenario: Root and client are unavailable

- GIVEN a registered root is absent and a supported client installation is not detected
- WHEN the public core scan operation runs
- THEN the report includes diagnostics for the absent root and the undetected client

#### Scenario: Adapter failure is isolated

- GIVEN one adapter fails while other adapters have valid fixture input
- WHEN the public core scan operation runs
- THEN it returns a report containing the available results and accumulated diagnostics from the remaining adapters

#### Scenario: A malformed Codex agent file does not abort the scan

- GIVEN a fixture home where one Codex agent `.toml` file is malformed while every other adapter's fixture input is well-formed
- WHEN the public core scan operation runs
- THEN the report includes an `Error` `ScanIssue` for the malformed Codex agent file, and every other adapter's valid results are still present in the report

#### Scenario: A failed freshness lookup never becomes a ScanIssue

- GIVEN every reference-version lookup fails for the current set of subjects
- WHEN the scan operation and the freshness evaluation both run
- THEN `ScanReport.issues` contains zero entries attributable to the freshness failure
- AND the degradation is represented only as `Freshness::Unknown` in the freshness report

#### Scenario: Logging a report does not mutate ScanReport or ScanIssue

- GIVEN a completed `ScanReport` is observed by the logging sink for missing-root and
  undetected-client events
- WHEN the report already returned to the frontend is compared before and after that observation
- THEN it is unchanged — no field, issue, or status is added, removed, or altered by logging
