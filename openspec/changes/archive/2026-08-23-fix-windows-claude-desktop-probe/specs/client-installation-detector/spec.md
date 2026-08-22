# Delta for Client Installation Detector

Core (Rust) requirements below; one frontend (TypeScript) requirement is
appended last, clearly separated, per `openspec/config.yaml` `rules.specs`.

**New reason vocabulary** (exact strings, used throughout):

| Slot | `reason` when not detected |
|------|------|
| Claude Code npm | `"Claude Code CLI (npm) not detected"` |
| OpenCode npm | `"OpenCode (npm) not detected"` (unchanged) |
| Claude Code bundled | `"Claude Code (bundled in Claude Desktop) not detected"` |

## MODIFIED Requirements

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
(Previously: all three slots were fully hardcoded, no enumeration.)

#### Scenario: The two npm slots resolve with no enumeration

- GIVEN a fixture `home`
- WHEN the npm slots are probed
- THEN each path is `home` plus its fixed segments, no enumeration involved

### Requirement: Claude Code npm And Bundled Are Never Merged

A home carrying both a Claude Code npm install and one or more bundled
Claude Code installs MUST produce separate `ClientInstallation` values with
`client: ClaudeCode`, never collapsed on account of shared client kind (CA-7).
(Previously: named the second slot "desktop"; it now MAY yield >1 root.)

#### Scenario: npm and bundled installs with different versions never merge

- GIVEN a fixture home with a Claude Code npm install (`1.2.0`) and a
  bundled install (`1.3.0`)
- WHEN the scanner runs
- THEN two `ClientInstallation` values are produced, versions `1.2.0` and
  `1.3.0`, neither merged

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
(Previously: one fixed desktop path, no multi-root or per-candidate concept.)

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

### Requirement: An Absent Slot Is Reported As An Explicit "Not Detected" Signal

A slot yielding zero `ClientInstallation` MUST emit exactly one `Warning`
`ScanIssue` with `reason` per the vocabulary table above. For the bundled
slot this fires once even after multiple candidate roots were probed and all
failed, with `path` set to the legacy fallback path.
(Previously: reason was `"Claude Code (desktop) not detected"`; one path.)

#### Scenario: No Packages directory and no legacy path yields one signal

- GIVEN a fixture home with neither `AppData/Local/Packages` nor the legacy
  Claude path
- WHEN the scanner runs
- THEN exactly one `ScanIssue` with reason
  `"Claude Code (bundled in Claude Desktop) not detected"` is produced

#### Scenario: An unreadable Packages directory errors but does not block the legacy fallback

- GIVEN a fixture home where `Packages` exists but cannot be listed, and a
  legacy install is present
- WHEN the scanner runs
- THEN one `Error` `ScanIssue` naming the `Packages` path is produced, and
  the legacy install is still reported

### Requirement: Every Case Is Traceable To A Repository Fixture In A New, Non-Reused Tree

Fixtures MUST live under a new `crates/vertice-core/tests/fixtures/client-installations/`
tree. The existing `fixtures/installations/` tree encodes the superseded path
table and MUST be deleted, not edited in place, so non-reuse (CA-17) is
verifiable in review. Minimum coverage: zero
packages and no legacy path; one package with a versioned directory; a
package with no `claude-code` directory; multiple packages; MSIX and legacy
both present; an absent or unreadable `Packages` directory; plus pre-existing
npm-slot and malformed-`package.json` cases.
(Previously: covered only the flat three-slot table.)

#### Scenario: Fixture set covers every bundled-slot case

- GIVEN this spec's bundled-slot requirements
- WHEN the fixture tree is enumerated
- THEN each case above has at least one dedicated fixture

## RENAMED Requirements

### Requirement: Each Desktop Version Directory Is Its Own Installation → Each Bundled-Slot Version Directory Is Its Own Installation

(Reason: "desktop" conflated the runtime with the Claude Desktop application; "bundled" names the runtime bundled inside it.)
(Migration: update test/doc references from "desktop slot" to "bundled slot".)

## ADDED Requirements

### Requirement: Frontend Reason-String Matching Tracks The New Label Vocabulary (TypeScript)

`frontend/src/lib/scanDiagnostics.ts`'s `MISSING_CLIENT_REASONS` MUST contain
exactly the three strings in the vocabulary table above, replacing the old
`"Claude Code (desktop) not detected"` entry.

#### Scenario: A bundled-slot not-detected issue is classified as missing-client

- GIVEN a `ScanIssue` with `severity: "warning"`, non-null `path`, and
  `reason: "Claude Code (bundled in Claude Desktop) not detected"`
- WHEN `isMissingClientIssue` is called with it
- THEN it returns `true`
