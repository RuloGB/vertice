# Delta for Scan Orchestration

## MODIFIED Requirements

### Requirement: In-Memory Read-Only Result

The scan operation MUST keep its result in memory. It MUST NOT introduce SQLite, persistence, provenance/history storage, IPC/UI behavior, or any mutation of scanned roots. CA-16 proof MUST compare before/after snapshots for the full reference fixture tree, covering files plus directory entries and relevant metadata needed to detect content, truncation, permission, rename, create, delete, or modified-timestamp mutations. Runtime symlink preservation MUST NOT be claimed unless a fixture actually contains symlink entries; link mutation APIs remain covered by static audit. The audit policy MUST cover the full filesystem mutation surface, including generic `Write`-based writes and metadata-changing operations, while stating that static audit evidence supports but does not by itself prove absence of indirect writes. Verification and archive evidence MUST record the automated fixture proof and the audit scope used for this guarantee.
(Previously: the requirement required file-hash and `mtime` proof plus a write-surface audit, but it did not require full tree coverage or complete mutation-surface scope.)

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
