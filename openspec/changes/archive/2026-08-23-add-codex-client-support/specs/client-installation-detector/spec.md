# Delta for Client Installation Detector

A fourth Windows probe slot, Codex standalone, joins the existing three
(Claude Code npm, Claude Code bundled, OpenCode npm). Neither existing
`VersionSource` fits Codex's release-directory-plus-triple naming, so a new
`VersionSource` variant is added with its own resolver; `~/.codex/version.json`
is explicitly and permanently excluded as a version source. Whether the
resolver follows the `bin` -> `current` -> `releases/<version>-<triple>`
symlink chain or enumerates `releases/` directly, and the exact
version-string extraction rule, are `sdd-design` decisions — this delta
specifies only the observable outcome, consistent with the existing
present-but-broken-vs-absent distinction this capability already draws for
the bundled Claude Code slot.

## MODIFIED Requirements

### Requirement: Windows Probe Paths Are Hardcoded, Never OS-Convention-Derived

The scanner MUST probe four slots under `home: &Path`: Claude Code npm
(`AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/`), OpenCode npm
(`AppData/Roaming/npm/node_modules/opencode-ai/`), the Claude Code runtime
bundled in Claude Desktop, and Codex standalone. The npm slots MUST be
`home` plus hardcoded segments only, no `dirs`/`directories` crate, no
environment read. The bundled slot's candidate roots MAY come from a
bounded, read-only, one-level-deep, `Claude_*`-prefix-filtered listing of
`home/AppData/Local/Packages`, plus the hardcoded legacy path
`home/AppData/Roaming/Claude/claude-code/`. The Codex slot's candidate roots
are drawn from `home/.codex/packages/standalone/releases/`, composed from
`home` plus hardcoded segments only, no `dirs`/`directories` crate and no
environment read — mirroring `plan-desarrollo-poc.md:179`'s constraint for
every other path in this capability. No publisher hash MAY be hardcoded
anywhere. Whether the Codex resolver additionally follows the
`%LOCALAPPDATA%\Programs\OpenAI\Codex\bin` symlink chain, or resolves
candidate roots solely from `releases/`, is a design-phase decision; either
satisfies this requirement as long as no environment-derived or
publisher-hashed path is introduced.
(Previously: exactly three slots — the two npm slots and the bundled slot —
with no Codex slot.)

#### Scenario: The two npm slots resolve with no enumeration

- GIVEN a fixture `home`
- WHEN the npm slots are probed
- THEN each path is `home` plus its fixed segments, no enumeration involved

#### Scenario: The Codex slot's candidate roots are composed from home alone

- GIVEN a fixture `home` with a Codex installation under `.codex/packages/standalone/releases/`
- WHEN the Codex slot is probed
- THEN every candidate path is `home` plus hardcoded segments, with no `dirs`/`directories` import and no environment variable read anywhere in its resolution

### Requirement: Every Resolved Probe Slot Always Emits A Typed Presence Record

For each platform with a real probe table, `scan_for` MUST return
`Some(Vec<ClientPresence>)` with exactly one record per probe slot,
regardless of whether any `ClientInstallation` was resolved for that slot.
On Windows this MUST always be exactly four records (Claude Code npm, Claude
Code bundled, OpenCode npm, Codex standalone), in deterministic order. A
slot's `status` MUST be `Detected` when at least one candidate root for that
slot exists on disk, and `NotDetected` when none does — `status` MUST NOT be
derived from whether `installations` is non-empty. A slot resolving to more
than one installation MUST list all of them inside that single record's
`installations`, never merged or reduced to one (CA-7). Emitting a presence
record MUST NOT itself push a `ScanIssue`.

`ScanReport.installations` MUST remain a derived flattening of every
record's `installations`, in record order, computed by a single function; it
MUST NOT be independently accumulated anywhere else.
(Previously: exactly three records, with no Codex slot.)

#### Scenario: A machine with no clients yields four notDetected records and zero issues

- GIVEN the `nothing` fixture home
- WHEN the scanner runs
- THEN `client_presence` is `Some` with exactly four records, all `status: NotDetected` with empty `installations`
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

#### Scenario: A home with no Codex installation yields a NotDetected Codex record

- GIVEN a fixture home with no `.codex/` directory at all
- WHEN the scanner runs
- THEN the Codex slot's `ClientPresence` record has `status: NotDetected`, empty `installations`, and contributes zero `ScanIssue` values (CA-11)

## ADDED Requirements

### Requirement: Each Codex Release Directory Is Its Own Installation

Every resolved candidate root under `home/.codex/packages/standalone/releases/`
MUST produce its own `ClientInstallation`, mirroring the 1..N candidate-root
shape `resolve_bundled_slot` already establishes for the bundled Claude Code
slot. Multiple release directories MUST NOT be merged, deduplicated, or
reduced to a single "highest version wins" entry — every release directory
present on disk is a distinct installation (CA-7).

#### Scenario: Two release directories at different versions yield two installations

- GIVEN a fixture home with two directories under `releases/`, `0.148.0-x86_64-pc-windows-msvc` and `0.149.0-x86_64-pc-windows-msvc`
- WHEN the scanner runs
- THEN two `ClientInstallation` values with `client: Codex` are produced, with distinct versions and distinct paths, neither merged nor reduced to one

### Requirement: Codex Version Is Extracted From The Release Directory Name, Never From version.json

The scanner MUST extract a Codex installation's `version` from its release
directory's name under `releases/<version>-<target-triple>/`, a path-segment
read, never a file read. `~/.codex/version.json` MUST NEVER be read for a
version, under any circumstance: its `latest_version` field is an
update-availability cache, not an installed-version record, and MUST NOT be
treated as a fallback when the directory-name extraction is ambiguous or
fails. The exact algorithm used to split the version prefix from the
target-triple suffix is a design-phase decision, but for a directory named
`<version>-<target-triple>` in the observed shape (e.g.
`0.149.0-x86_64-pc-windows-msvc`), the extracted `version` MUST equal the
version prefix alone (`0.149.0`), with the target-triple suffix never
appearing in the reported version string.

#### Scenario: The Codex slot's version comes from the release directory name

- GIVEN a fixture Codex install whose sole release directory is named `0.149.0-x86_64-pc-windows-msvc`
- WHEN the scanner runs
- THEN the resulting `ClientInstallation` has `version: "0.149.0"` and its `path` points at that release directory

#### Scenario: version.json is present but never consulted

- GIVEN a fixture Codex install with a well-formed `~/.codex/version.json` declaring a `latest_version` that differs from the installed release directory's version
- WHEN the scanner runs
- THEN the resulting `ClientInstallation`'s `version` equals the release directory name's version prefix, never the value from `version.json`

#### Scenario: A prerelease-shaped directory name does not silently corrupt the version

- GIVEN a fixture Codex install whose release directory is named `0.150.0-rc.1-x86_64-pc-windows-msvc`
- WHEN the scanner runs
- THEN the extracted version reflects the intended prerelease version, never a value truncated at the first `-` that discards the `rc.1` component or fuses it with the target triple

### Requirement: A Release Directory Name Outside The Expected Shape Is An Error, Not An Absence

A release directory that exists under `releases/` but whose name does not
fit the expected `<version>-<target-triple>` shape MUST be treated as a
present-but-broken candidate, consistent with this capability's existing
rule that an existing-but-unusable candidate root is `Detected` with an
`Error` `ScanIssue`, never folded into `NotDetected` (mirroring "An existing
candidate root holding no version directory is an error, not an absence").
It MUST produce one `Error` `ScanIssue` carrying that directory's path and
MUST NOT produce a `ClientInstallation` with an empty, truncated, or
otherwise corrupted version string.

#### Scenario: An unparseable release directory name yields an error, not a phantom installation

- GIVEN a fixture home with a release directory whose name does not match the `<version>-<target-triple>` shape
- WHEN the scanner runs
- THEN no `ClientInstallation` is produced for that directory, one `Error` `ScanIssue` carrying its path is produced, and the Codex slot's `ClientPresence` record remains `status: Detected` if any other candidate root exists

### Requirement: A Malformed Codex Candidate Does Not Block Other Slots

A failure while probing the Codex slot (an unparseable release directory
name, or an unreadable `releases/` directory) MUST NOT prevent any other
slot — Claude Code npm, Claude Code bundled, or OpenCode npm — from being
probed, detected, or reported, mirroring the existing per-slot isolation
this capability already guarantees.

#### Scenario: A broken Codex slot does not block the other three

- GIVEN a fixture home where the Codex release directory name is unparseable, while Claude Code npm, Claude Code bundled, and OpenCode npm are all well-formed
- WHEN the scanner runs
- THEN one `ScanIssue` at `IssueSeverity::Error` is produced for the Codex slot, and the other three slots' `ClientInstallation` values are still produced
