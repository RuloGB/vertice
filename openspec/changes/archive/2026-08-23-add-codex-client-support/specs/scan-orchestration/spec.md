# Delta for Scan Orchestration

The orchestrator's adapter list gains a fourth agent adapter (Codex, joining
the Claude-agent and OpenCode-agent adapters) and the existing skill adapter
now walks a fourth root (`codex-skills`, via the `skill-scanner` delta). The
"one bad adapter does not abort the scan" property, already proven for the
existing adapters, MUST hold for the Codex agent adapter as well. Duplicate
consolidation itself is unchanged (`duplicate-consolidation` carries no delta
for this proposal); the scenario below exercises its existing, unmodified
merge rule against a same-named skill sourced from a Codex root and a Claude
Code root, which is the identity decision this proposal records and pins.

## MODIFIED Requirements

### Requirement: Complete Consolidated Scan Report

The core SHALL expose one public scan operation that invokes the existing
skills, Claude-agent, OpenCode-agent, Codex-agent, and client-installation
adapters for the registered user roots. It MUST return an in-memory
`ScanReport` containing the consolidated components, detected installations,
every scanned root, accumulated issues, and a measured duration. It MUST
apply the existing duplicate-consolidation behavior without changing adapter
parsing, registered roots, installation probes, or consolidation semantics.
(Previously: "invokes the existing skills, Claude-agent, OpenCode-agent, and
client-installation adapters" — no Codex-agent adapter existed.)

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
- THEN the report contains exactly one `Component` for that identity, carrying two `Location` entries, one per root — the identity decision recorded in the parent proposal: no client discriminator is introduced, and consolidation behavior is unmodified

### Requirement: Visible and Isolated Diagnostics

The scan operation MUST accumulate diagnostics for non-parseable components
with their paths, undetected supported clients, and absent roots. It MUST
NOT omit those conditions silently. A failure from one adapter MUST NOT
abort the remaining adapters; their available results and diagnostics MUST
remain represented in the report.
(Previously: identical text; the Codex agent adapter is now one of the
adapters this guarantee covers.)

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
