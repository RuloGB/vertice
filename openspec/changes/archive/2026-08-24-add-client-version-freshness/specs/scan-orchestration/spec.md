# Delta for Scan Orchestration

Freshness is explicitly outside the scan operation this spec defines: it is not invoked by the scan, does not count toward its measured duration or CA-15 budget, and a failed lookup is never represented as a `ScanIssue`.

## MODIFIED Requirements

### Requirement: Measured Reference-Volume Performance

The scan operation MUST measure elapsed scan duration and place that measured value in the report. On the versioned reference-volume fixture, the complete scan MUST finish in under two seconds, satisfying CA-15. The scan operation MUST NOT invoke, await, or otherwise depend on any freshness/reference-version lookup; the measured duration MUST reflect only the scan's own adapters and consolidation, never any network-bound freshness work, regardless of whether the freshness check is enabled or disabled.
(Previously: did not state a relationship to freshness, since the capability did not exist.)

#### Scenario: Reference-volume scan meets CA-15

- GIVEN the versioned fixture representing the reference scan volume
- WHEN the complete public core scan operation runs
- THEN the returned report contains its measured duration
- AND the scan completes in less than two seconds

#### Scenario: The scan operation does not await freshness

- GIVEN the freshness check is enabled
- WHEN the public core scan operation runs
- THEN it completes without invoking or waiting on any freshness/reference-version lookup
- AND its measured duration is unaffected by freshness lookup latency

### Requirement: Visible and Isolated Diagnostics

The scan operation MUST accumulate diagnostics for non-parseable components with their paths, undetected supported clients, and absent roots. It MUST NOT omit those conditions silently. A failure from one adapter MUST NOT abort the remaining adapters; their available results and diagnostics MUST remain represented in the report. A failed or degraded freshness lookup MUST NOT produce a `ScanIssue` under any circumstance; it is represented exclusively as `Freshness::Unknown` in the separate freshness report, never in `ScanReport.issues`.
(Previously: did not address freshness, since the capability did not exist.)

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

#### Scenario: A failed freshness lookup never becomes a ScanIssue

- GIVEN every reference-version lookup fails for the current set of subjects
- WHEN the scan operation and the freshness evaluation both run
- THEN `ScanReport.issues` contains zero entries attributable to the freshness failure
- AND the degradation is represented only as `Freshness::Unknown` in the freshness report
