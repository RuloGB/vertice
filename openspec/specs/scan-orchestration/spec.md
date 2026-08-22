# Scan Orchestration Specification

## Purpose

Define T9's public core scan workflow, which produces one complete in-memory `ScanReport` from the existing adapters and duplicate consolidation.

## Requirements

### Requirement: Complete Consolidated Scan Report

The core SHALL expose one public scan operation that invokes the existing skills, Claude-agent, OpenCode-agent, and client-installation adapters for the registered user roots. It MUST return an in-memory `ScanReport` containing the consolidated components, detected installations, every scanned root, accumulated issues, and a measured duration. It MUST apply the existing duplicate-consolidation behavior without changing adapter parsing, registered roots, installation probes, or consolidation semantics.

#### Scenario: Complete fixture scan

- GIVEN versioned fixtures for all registered roots and supported client installations
- WHEN the public core scan operation runs
- THEN it returns one `ScanReport` with consolidated components, installations, scanned roots, and all adapter issues
- AND its duration is measured for that scan

#### Scenario: Components from multiple adapters overlap

- GIVEN adapters produce components with the same existing consolidation identity
- WHEN the public core scan operation runs
- THEN the report represents those components according to existing duplicate-consolidation behavior

### Requirement: Visible and Isolated Diagnostics

The scan operation MUST accumulate diagnostics for non-parseable components with their paths, undetected supported clients, and absent roots. It MUST NOT omit those conditions silently. A failure from one adapter MUST NOT abort the remaining adapters; their available results and diagnostics MUST remain represented in the report.

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

### Requirement: Measured Reference-Volume Performance

The scan operation MUST measure elapsed scan duration and place that measured value in the report. On the versioned reference-volume fixture, the complete scan MUST finish in under two seconds, satisfying CA-15.

#### Scenario: Reference-volume scan meets CA-15

- GIVEN the versioned fixture representing the reference scan volume
- WHEN the complete public core scan operation runs
- THEN the returned report contains its measured duration
- AND the scan completes in less than two seconds

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
