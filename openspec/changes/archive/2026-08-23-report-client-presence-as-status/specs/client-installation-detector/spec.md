# Delta for Client Installation Detector

Core (Rust) requirements below; the removed frontend requirement is the string-coupling rule it retires. Absence gains a typed carrier (`ClientPresence`, defined in the `domain-model` delta) and loses a parsed English string; `InstallSlot::label()` keeps producing `Error`-reason text (`installations.rs:120-121`) — labels do not leave the `ScanIssue` channel entirely, only the not-detected `Warning` does. Field names and semantics below match `design.md` §2, §4, and §7 exactly.

## ADDED Requirements

### Requirement: Every Resolved Probe Slot Always Emits A Typed Presence Record

For each platform with a real probe table, `scan_for` MUST return `Some(Vec<ClientPresence>)` with exactly one record per probe slot, regardless of whether any `ClientInstallation` was resolved for that slot. On Windows this MUST always be exactly three records (Claude Code npm, Claude Code bundled, OpenCode npm), in deterministic order. A slot's `status` MUST be `Detected` when at least one candidate root for that slot exists on disk, and `NotDetected` when none does — `status` MUST NOT be derived from whether `installations` is non-empty. A slot resolving to more than one installation MUST list all of them inside that single record's `installations`, never merged or reduced to one (CA-7). Emitting a presence record MUST NOT itself push a `ScanIssue`.

`ScanReport.installations` MUST remain a derived flattening of every record's `installations`, in record order, computed by a single function; it MUST NOT be independently accumulated anywhere else.

#### Scenario: A machine with no clients yields three notDetected records and zero issues

- GIVEN the `nothing` fixture home
- WHEN the scanner runs
- THEN `client_presence` is `Some` with exactly three records, all `status: NotDetected` with empty `installations`
- AND zero `ScanIssue` values are produced

#### Scenario: A bundled slot with two coexisting versions keeps both in one record

- GIVEN the `packaged-and-legacy` fixture home, where the bundled slot resolves an MSIX package and the legacy path at different versions
- WHEN the scanner runs
- THEN the bundled slot's `ClientPresence` record has `status: Detected` and `installations.len() == 2`, both versions present and neither merged

#### Scenario: A candidate root that exists but yields nothing is Detected, not NotDetected

- GIVEN the `npm-dir-no-package-json` fixture home, where an npm slot's directory exists but its `package.json` is missing
- WHEN the scanner runs
- THEN that slot's `ClientPresence` record has `status: Detected` and empty `installations`
- AND the existing `Error` `ScanIssue` naming that slot's path is still produced, unchanged
- AND that slot's record is never `status: NotDetected`, since the candidate root exists on disk

#### Scenario: ScanReport.installations equals the flattened presence records

- GIVEN the `packaged-and-legacy` fixture home
- WHEN the scanner runs
- THEN `ScanReport.installations`, element-for-element in order, equals the concatenation of every `ClientPresence` record's `installations`

### Requirement: An Unsupported Platform Reports No Probe Attempt, Not Absence

On `HostPlatform::Unsupported`, `scan_for` MUST set `client_presence: None` — never `Some(vec![])` and never three `NotDetected` records. `None` MUST be read as "this platform has no probe table; client detection was not attempted", distinct from a slot that was looked for and not found. The scanner MUST continue to emit exactly one `Warning` `ScanIssue` with `path: None` stating that client installation detection is not implemented on this platform, byte-identical to current behavior. `ScanReport.installations` MUST be empty in this case.

#### Scenario: Unsupported platform yields None, not empty records, and the existing single warning

- GIVEN `HostPlatform::Unsupported`
- WHEN the scanner runs
- THEN `client_presence` is `None`
- AND `ScanReport.installations` is empty
- AND exactly one `Warning` `ScanIssue` with `path: None` is produced, as before this change

## REMOVED Requirements

### Requirement: An Absent Slot Is Reported As An Explicit "Not Detected" Signal

(Reason: absence now has a typed carrier — `ClientPresence.status == NotDetected` — so the parsed `"{label} not detected"` `Warning` `ScanIssue` is redundant and is removed. `Error`-severity issues per slot are unaffected and continue exactly as before.)
(Migration: consumers checking for an absent slot MUST read `ScanReport.clientPresence[].status` instead of matching `ScanIssue.reason`, after first checking `clientPresence` is not `null`. `crates/vertice-core/tests/client_installations.rs` and `scan.rs:129-154` are rewritten to assert on presence records rather than `reason.ends_with("not detected")`.)

### Requirement: Frontend Reason-String Matching Tracks The New Label Vocabulary (TypeScript)

(Reason: this requirement *is* the string coupling the change removes. `MISSING_CLIENT_REASONS` and `isMissingClientIssue` are deleted from `frontend/src/lib/scanDiagnostics.ts`; nothing replaces string-matching because nothing needs to match a string anymore.)
(Migration: the frontend reads `ScanReport.clientPresence` directly, typed, with no reason-string parsing. `frontend/src/lib/scanDiagnostics.test.ts:6-10` is rewritten accordingly.)
