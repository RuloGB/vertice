# Delta for Scan Orchestration

The orchestrator's adapter list gains a fourth adapter class: MCP servers,
one adapter per in-scope client (Claude Code, OpenCode, Codex). The "one bad
adapter does not abort the scan" property and the `NotFound`-root warning
behavior, already proven for skill and agent adapters, MUST hold for MCP
adapters as well. No new Tauri command is introduced; MCP scanning folds
into the existing public scan operation.

## MODIFIED Requirements

### Requirement: Complete Consolidated Scan Report

The core SHALL expose one public scan operation that invokes the existing
skills, Claude-agent, OpenCode-agent, Codex-agent, client-installation, and
MCP adapters (one per in-scope client: Claude Code, OpenCode, Codex) for the
registered user roots. It MUST return an in-memory `ScanReport` containing
the consolidated components, detected installations, every scanned root,
accumulated issues, and a measured duration. It MUST apply the existing
duplicate-consolidation behavior without changing adapter parsing,
registered roots, installation probes, or consolidation semantics.
(Previously: "invokes the existing skills, Claude-agent, OpenCode-agent,
Codex-agent, and client-installation adapters" — no MCP adapter existed.)

#### Scenario: Complete fixture scan

- GIVEN versioned fixtures for all registered roots and supported client installations
- WHEN the public core scan operation runs
- THEN it returns one `ScanReport` with consolidated components, installations, scanned roots, and all adapter issues

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
(Previously: identical text, without the MCP freshness-exclusion and
secret-in-log restatements, because no MCP adapter existed.)

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
