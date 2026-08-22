# Delta for Desktop Shell

## MODIFIED Requirements

### Requirement: Minimal Capability Grant

The shell capability declaration SHALL grant `core:default` only: no filesystem plugin, no filesystem scopes, no shell or dialog permissions. The audited desktop surface MUST show that the webview has zero filesystem mutation capability over scanned roots, including content writes, truncation, creation, deletion, rename/link creation, permission changes, and equivalent indirect mutation paths. The audit policy MUST cover the capability file plus the command-exposed desktop surface and MUST avoid claiming that text inspection alone proves all transitive write absence. Verification evidence MUST name the audited capability and command surfaces used to support CA-16.
(Previously: the requirement required explicit audit evidence for the write-capability boundary, but it did not require full mutation-surface coverage or the limitation on static-text proof claims.)

#### Scenario: Capabilities grant nothing beyond core default

- GIVEN the shell capability declaration
- WHEN it is reviewed or audited
- THEN it grants only `core:default`
- AND it contains no filesystem, shell, or dialog permission or scope

#### Scenario: Webview has no filesystem mutation surface over scanned roots

- GIVEN the audited capability declaration and scan command surface
- WHEN the desktop shell read-only audit runs
- THEN no webview-exposed filesystem mutation capability exists over scanned roots
- AND the audit records the capability and command surfaces it reviewed
