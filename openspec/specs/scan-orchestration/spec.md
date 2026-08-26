# Scan Orchestration Specification

## Purpose

Define T9's public core scan workflow, which produces one complete in-memory `ScanReport` from the existing adapters and duplicate consolidation. `add-client-version-freshness` (2026-08-24) added the constraint that freshness is explicitly outside this operation: never invoked by the scan, never counted toward the CA-15 budget, and a failed lookup is never represented as a `ScanIssue`. `add-application-logging` (2026-08-24) added the constraint that the application log is an orthogonal observer over a completed `ScanReport`: it never mutates the report or its issues. `add-mcp-scanning` (2026-08-26) added a fourth adapter class, MCP servers (one adapter per in-scope client), folded into the same public scan operation with no new Tauri command.

## Requirements

### Requirement: Complete Consolidated Scan Report

The core SHALL expose one public scan operation that invokes the existing
skills, Claude-agent, OpenCode-agent, Codex-agent, client-installation, and
MCP adapters (one per in-scope client: Claude Code, OpenCode, Codex) for the
registered user roots. It MUST return an in-memory `ScanReport` containing
the consolidated components, detected installations, every scanned root,
accumulated issues, and a measured duration. It MUST apply the existing
duplicate-consolidation behavior without changing adapter parsing,
registered roots, installation probes, or consolidation semantics.

#### Scenario: Complete fixture scan

- GIVEN versioned fixtures for all registered roots and supported client installations
- WHEN the public core scan operation runs
- THEN it returns one `ScanReport` with consolidated components, installations, scanned roots, and all adapter issues
- AND its duration is measured for that scan

#### Scenario: Components from multiple adapters overlap

- GIVEN adapters produce components with the same existing consolidation identity
- WHEN the public core scan operation runs
- THEN the report represents those components according to existing duplicate-consolidation behavior

#### Scenario: A same-named skill from a Codex root and a Claude Code root consolidates into one component

- GIVEN a fixture home with a skill of the same name present under both `.codex/skills/` and `.claude/skills/`
- WHEN the public core scan operation runs
- THEN the report contains exactly one `Component` for that identity, carrying two `Location` entries, one per root — no client discriminator is introduced, and consolidation behavior is unmodified

#### Scenario: A same-named MCP server across all three clients consolidates into one component with three transports

- GIVEN a fixture home with an MCP server named `github` configured in Claude Code, OpenCode, and Codex, each with a different command
- WHEN the public core scan operation runs
- THEN the report contains exactly one `Component { kind: Mcp }` carrying three `Location` entries, each retaining its own `McpTransport`, and `total_location_count_is_conserved` stays green

### Requirement: Visible and Isolated Diagnostics

The scan operation MUST accumulate diagnostics for non-parseable components
with their paths, undetected supported clients, and absent roots. It MUST
NOT omit those conditions silently. A failure from one adapter MUST NOT
abort the remaining adapters; their available results and diagnostics MUST
remain represented in the report. A failed or degraded freshness lookup MUST
NOT produce a `ScanIssue` under any circumstance; it is represented
exclusively as `Freshness::Unknown` in the separate freshness report, never
in `ScanReport.issues`. No MCP component MUST ever appear as a subject in
any `FreshnessCheck`. The application log introduced by
`add-application-logging` is an orthogonal sink over these same
diagnostics: it MUST NOT change `ScanReport` or `ScanIssue` semantics, MUST
NOT add a new diagnostic field or variant to either type, and observing a
report for logging purposes MUST NOT alter the report returned to the
caller; this includes never logging a value from any MCP `env`, `headers`,
or `args` field.

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

#### Scenario: A malformed MCP config in one client does not abort the scan

- GIVEN a fixture home where one client's MCP configuration file is malformed while every other adapter's fixture input is well-formed
- WHEN the public core scan operation runs
- THEN the report includes an `Error` `ScanIssue` for the malformed MCP config, and every other adapter's valid results, including the other two clients' MCP servers, are still present in the report

#### Scenario: A failed freshness lookup never becomes a ScanIssue

- GIVEN every reference-version lookup fails for the current set of subjects
- WHEN the scan operation and the freshness evaluation both run
- THEN `ScanReport.issues` contains zero entries attributable to the freshness failure
- AND the degradation is represented only as `Freshness::Unknown` in the freshness report

#### Scenario: Logging a report does not mutate ScanReport or ScanIssue, and never leaks an MCP secret

- GIVEN a completed `ScanReport` from a fixture home carrying a realistic fake MCP token is observed by the logging sink
- WHEN the report already returned to the frontend is compared before and after that observation, and the application log file is inspected
- THEN the returned report is unchanged — no field, issue, or status is added, removed, or altered by logging
- AND the fake token string appears nowhere in the log file

### Requirement: Measured Reference-Volume Performance

The scan operation MUST measure elapsed scan duration and place that measured value in the report. On the versioned reference-volume fixture, the complete scan MUST finish in under two seconds, satisfying CA-15. The scan operation MUST NOT invoke, await, or otherwise depend on any freshness/reference-version lookup; the measured duration MUST reflect only the scan's own adapters and consolidation, never any network-bound freshness work, regardless of whether the freshness check is enabled or disabled.

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

### Requirement: In-Memory Read-Only Result

The scan operation MUST keep its result in memory. It MUST NOT introduce SQLite, persistence, provenance/history storage, IPC/UI behavior, or any mutation of scanned roots. CA-16 proof MUST compare before/after snapshots for the full reference fixture tree, covering files plus directory entries and relevant metadata needed to detect content, truncation, permission, rename, create, delete, or modified-timestamp mutations. Runtime symlink preservation MUST NOT be claimed unless a fixture actually contains symlink entries; link mutation APIs remain covered by static audit. The audit policy MUST cover the full filesystem mutation surface, including generic `Write`-based writes and metadata-changing operations, while stating that static audit evidence supports but does not by itself prove absence of indirect writes. Verification and archive evidence MUST record the automated fixture proof and the audit scope used for this guarantee.

#### Scenario: Reference fixture tree remains unchanged after scan

- GIVEN the versioned reference fixture and its pre-scan full-tree snapshot
- WHEN the public core scan operation completes
- THEN the post-scan snapshot matches for all fixture files, directories, and tracked metadata
- AND the returned report still exists only in memory

#### Scenario: Audit policy covers filesystem mutation classes

- GIVEN the scanner modules and the CA-16 app-data exception
- WHEN the read-only audit reviews filesystem mutation capabilities
- THEN the audit covers write, truncate, create, delete, rename, link, and metadata-changing operations
- AND its evidence does not claim static checks alone prove absence of indirect writes

#### Scenario: Manual proof remains supplemental

- GIVEN verification runs on the reference machine
- WHEN CA-16 evidence is recorded
- THEN the verify or archive artifact documents the automated fixture proof and audit scope
- AND any manual/system-level evidence is supplemental rather than substitutive
