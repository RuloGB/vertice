# Client Installation Detector Specification

## Purpose

Defines the contract for detecting which AI clients are installed on the user's machine, on Windows: three independent probe slots (Claude Code npm, Claude Code desktop, OpenCode npm), each yielding a separately reported `ClientInstallation` with its own version, plus a typed per-slot `ClientPresence` status record (`detected`/`notDetected`) published through `model/`. Traces to T7 of `internal-docs/plan-desarrollo-poc.md`; closes CA-7 (the two Claude Code installations are detected separately, each with its version) and CA-11 (an absent client is reported as *not detected*, never as an error and never as an unexplained empty list); bound by CA-16 (read-only) and CA-17 (fixture-based, machine-independent tests on a new, non-reused fixture tree). Core (Rust) only, Windows only — macOS/Linux path tables are T16. **`report-client-presence-as-status` (2026-08-23) reversed the original decision recorded here**: the not-detected representation was originally closed on the `ScanIssue` carrier with a typed carrier explicitly rejected (`client-installation-detection` design §2), but that design's own §2 named its own retrofit condition — once T10/T11's consumer existed, a typed carrier would be "the retrofit, and it is cheap" — and T11 completing met that condition. `domain-model` is now a Modified Capability of this line of work, with `ClientPresence`/`ClientPresenceStatus` published through `model/` and carried on `ScanReport.client_presence`; the not-detected `ScanIssue` `Warning` is removed accordingly.

## Requirements

### Requirement: Windows Probe Paths Are Hardcoded, Never OS-Convention-Derived

The scanner MUST probe three slots under `home: &Path`: Claude Code npm
(`AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/`), OpenCode npm
(`AppData/Roaming/npm/node_modules/opencode-ai/`), and the Claude Code
runtime bundled in Claude Desktop. The npm slots MUST be `home` plus
hardcoded segments only, no `dirs`/`directories` crate, no environment read.
The bundled slot is the sole exception: its candidate roots MAY come from a
bounded, read-only, one-level-deep, `Claude_*`-prefix-filtered listing of
`home/AppData/Local/Packages`, plus the hardcoded legacy path
`home/AppData/Roaming/Claude/claude-code/`. No publisher hash MAY be
hardcoded anywhere.

#### Scenario: The two npm slots resolve with no enumeration

- GIVEN a fixture `home`
- WHEN the npm slots are probed
- THEN each path is `home` plus its fixed segments, no enumeration involved

### Requirement: Claude Code npm And Bundled Are Never Merged

A home carrying both a Claude Code npm install and one or more bundled
Claude Code installs MUST produce separate `ClientInstallation` values with
`client: ClaudeCode`, never collapsed on account of shared client kind (CA-7).

#### Scenario: npm and bundled installs with different versions never merge

- GIVEN a fixture home with a Claude Code npm install (`1.2.0`) and a
  bundled install (`1.3.0`)
- WHEN the scanner runs
- THEN two `ClientInstallation` values are produced, versions `1.2.0` and
  `1.3.0`, neither merged

#### Scenario: A present OpenCode npm install is reported alongside Claude Code

- GIVEN a fixture home carrying an OpenCode npm installation and one Claude Code installation
- WHEN the scanner runs
- THEN the result contains one `ClientInstallation` with `client: OpenCode` and one with `client: ClaudeCode`, each independently reported

### Requirement: Version Is Extracted From The Correct Source Per Slot

For the two npm slots, the scanner MUST extract `version` from the `"version"` key of that slot's `package.json`, parsed through the existing `jsonc.rs` seam — no second JSON dependency. For the desktop slot, the scanner MUST extract `version` from the name of its single versioned subdirectory, a path-segment read, never a file read.

#### Scenario: An npm slot's version comes from package.json

- GIVEN a fixture Claude Code npm install whose `package.json` declares `"version": "1.4.2"`
- WHEN the scanner runs
- THEN the resulting `ClientInstallation` has `version: "1.4.2"`

#### Scenario: The desktop slot's version comes from the directory name

- GIVEN a fixture Claude Code desktop install whose single versioned subdirectory is named `1.5.0`
- WHEN the scanner runs
- THEN the resulting `ClientInstallation` has `version: "1.5.0"` and its `path` points at that versioned directory

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

### Requirement: A Malformed Or Unreadable package.json Produces An Error, Never A Phantom Installation

A slot whose `package.json` fails to parse MUST produce exactly one `ScanIssue` at `IssueSeverity::Error` carrying that file's path and MUST NOT produce any `ClientInstallation` for that slot. A `package.json` that parses but has no `"version"` key MUST likewise produce no `ClientInstallation` and one `ScanIssue`, never an entry with an empty version string.

#### Scenario: Malformed package.json yields an Error issue and no installation

- GIVEN a fixture Claude Code npm install whose `package.json` is malformed
- WHEN the scanner runs
- THEN exactly one `ScanIssue` at `IssueSeverity::Error` carrying that file's path is produced, and no `ClientInstallation` is produced for that slot

#### Scenario: A package.json missing "version" yields no phantom entry

- GIVEN a fixture OpenCode npm install whose `package.json` has no `"version"` key
- WHEN the scanner runs
- THEN no `ClientInstallation` is produced for that slot, and one `ScanIssue` references it — never an entry with an empty `version`

### Requirement: Each Bundled-Slot Version Directory Is Its Own Installation

Every versioned subdirectory under any resolved candidate root (an MSIX
package's cache path, or the legacy path) MUST produce its own
`ClientInstallation`, named by directory name. Directories MUST NOT merge
across candidate roots even on matching version strings. A `Claude_*` package
with no `claude-code` directory inside is not a candidate root at all: it
contributes zero installations and no issue. A candidate root that DOES exist
but holds zero versioned subdirectories is present-but-broken and MUST emit
its own `Error` `ScanIssue`, preserving the existing empty-directory
behaviour. The overall not-detected `Warning` fires only when no candidate
root exists at all.

#### Scenario: One MSIX package and the legacy path both present, both counted

- GIVEN a fixture home with a `Claude_<hash>` package at version `1.5.0` and
  a legacy install at `1.4.0`
- WHEN the scanner runs
- THEN two `ClientInstallation` values are produced (`1.4.0`, `1.5.0`),
  neither merged

#### Scenario: Multiple packages, and a package missing claude-code, each isolated

- GIVEN a fixture home with two `Claude_*` packages holding distinct
  versions, and a third `Claude_*` package with no `claude-code` directory
- WHEN the scanner runs
- THEN one `ClientInstallation` per versioned subdirectory is produced, and
  the third package contributes nothing and no issue of its own

#### Scenario: An existing candidate root holding no version directory is an error, not an absence

- GIVEN a fixture home with a `Claude_*` package whose `claude-code/`
  directory exists but is empty
- WHEN the scanner runs
- THEN one `Error` `ScanIssue` naming that candidate root is produced
- AND no `"Claude Code (bundled in Claude Desktop) not detected"` `Warning`
  is produced for it

### Requirement: Each Slot Fails Independently

A failure in one slot (a parse error, a missing version, or a desktop directory with no versioned subdirectory) MUST NOT prevent any other slot from being probed, detected, or reported.

#### Scenario: One malformed slot does not block the other two

- GIVEN a fixture home where the Claude Code npm `package.json` is malformed, while the Claude Code desktop and OpenCode npm installs are well-formed
- WHEN the scanner runs
- THEN one `ScanIssue` at `IssueSeverity::Error` is produced for the Claude Code npm slot, and both the Claude Code desktop and OpenCode `ClientInstallation` values are still produced

### Requirement: Only Path Resolution Is Platform-Specific

Per-OS probe-path resolution MUST be confined to a single dispatch point (a Windows-specific probe-table function). Version extraction and `ClientInstallation` assembly MUST be OS-agnostic and contain no `cfg(target_os)` branch, so that adding macOS or Linux probe tables in T16 requires no change to extraction or assembly code.

#### Scenario: Extraction and assembly code contain no OS-conditional branch

- GIVEN the scanner's version-extraction and `ClientInstallation`-assembly code
- WHEN that code is inspected
- THEN it contains no `cfg(target_os)` branch; all platform variation is confined to the probe-table function

### Requirement: Scanner Performs No Writes

The scanner MUST perform no filesystem write of any kind — no file creation, no file modification, no directory creation — anywhere in its probing, parsing, or assembly logic (CA-16).

#### Scenario: A full scan run leaves the fixture tree byte-for-byte unchanged

- GIVEN a fixture home with a known state before a scan
- WHEN the scanner runs a full scan over it
- THEN the fixture tree's contents are unchanged afterward

### Requirement: Installation And Issue Ordering Is Deterministic

The scanner MUST emit `ClientInstallation` and `ScanIssue` values in a deterministic order across repeated runs on the same input.

#### Scenario: Two runs over the same fixture home produce identically ordered results

- GIVEN a fixture home with multiple slots detected and multiple slots absent
- WHEN the scanner runs twice over the same input
- THEN the two resulting installation and issue lists are in identical order

### Requirement: Every Case Is Traceable To A Repository Fixture In A New, Non-Reused Tree

Fixtures MUST live under a new `crates/vertice-core/tests/fixtures/client-installations/`
tree. The existing `fixtures/installations/` tree encodes the superseded path
table and MUST be deleted, not edited in place, so non-reuse (CA-17) is
verifiable in review. Minimum coverage: zero
packages and no legacy path; one package with a versioned directory; a
package with no `claude-code` directory; multiple packages; MSIX and legacy
both present; an absent or unreadable `Packages` directory; plus pre-existing
npm-slot and malformed-`package.json` cases.

#### Scenario: Fixture set covers every bundled-slot case

- GIVEN this spec's bundled-slot requirements
- WHEN the fixture tree is enumerated
- THEN each case above has at least one dedicated fixture
