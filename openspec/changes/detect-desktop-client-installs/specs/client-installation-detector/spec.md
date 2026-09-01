# Delta for Client Installation Detector

## MODIFIED Requirements

### Requirement: Windows Probe Paths Are Hardcoded, Never OS-Convention-Derived

The scanner MUST probe five slots under `home: &Path`: Claude Code npm
(`AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/`), OpenCode npm
(`AppData/Roaming/npm/node_modules/opencode-ai/`), the Claude Code runtime
bundled in Claude Desktop, OpenCode desktop, and Codex standalone. The npm
slots MUST be `home` plus hardcoded segments only, no `dirs`/`directories`
crate, no environment read. The bundled slot's candidate roots MAY come from
a bounded, read-only, one-level-deep, `Claude_*`-prefix-filtered listing of
`home/AppData/Local/Packages`, plus the hardcoded legacy path
`home/AppData/Roaming/Claude/claude-code/`. The OpenCode desktop slot's
candidate root is the single hardcoded path
`home/AppData/Local/Programs/@opencode-aidesktop` (literal folder name,
leading `@` included), `home` plus fixed segments only. The Codex slot's
candidate roots are drawn from `home/.codex/packages/standalone/releases/`,
composed from `home` plus hardcoded segments only. No path in this
requirement MAY be built from `dirs`/`directories` or an environment read,
and no publisher hash MAY be hardcoded anywhere.
(Previously: four slots; adds the OpenCode desktop slot's single hardcoded path.)

#### Scenario: The two npm slots resolve with no enumeration

- GIVEN a fixture `home`
- WHEN the npm slots are probed
- THEN each path is `home` plus its fixed segments, no enumeration involved

#### Scenario: The Codex slot's candidate roots are composed from home alone

- GIVEN a fixture `home` with a Codex installation under `.codex/packages/standalone/releases/`
- WHEN the Codex slot is probed
- THEN every candidate path is `home` plus hardcoded segments, with no `dirs`/`directories` import and no environment variable read anywhere in its resolution

#### Scenario: The OpenCode desktop slot's path is composed from home alone

- GIVEN a fixture `home`
- WHEN the OpenCode desktop slot is probed
- THEN its sole candidate path is `home/AppData/Local/Programs/@opencode-aidesktop`, with no `dirs`/`directories` import and no environment variable read

### Requirement: Version Is Extracted From The Correct Source Per Slot

For the two npm slots, the scanner MUST extract `version` from the
`"version"` key of that slot's `package.json`, parsed through the existing
`jsonc.rs` seam. For the bundled slot, the scanner MUST extract `version`
from the name of its single versioned subdirectory, a path-segment read. For
the OpenCode desktop slot, the scanner MUST extract `version` from the
archived `package.json` inside `app.asar`: read the header length from the
archive's fixed-offset binary prefix, refuse to parse a header exceeding a
hardcoded byte-size ceiling (`HEADER_MAX_BYTES = 4 MiB`), otherwise parse the
header JSON through the `jsonc.rs` seam, resolve `package.json`'s offset and
size from it, read exactly those bytes, and extract `version`. There is NO
wall-clock time-budget ceiling: a synchronous parse cannot be safely aborted
mid-flight without either a worker thread (which leaks a still-running parse
on timeout) or a hand-written incremental scanner (rejected — this project
has no dependency-free incremental JSON API and a bespoke parser for a
format this crate already has a seam for is exactly what `AGENTS.md` bans
for frontmatter). The byte-size ceiling is a refusal threshold for the
pathological case, not a time guard; the measured real-world cost is
**30.9 ms average / 43.6 ms worst-of-20** on a synthetic header matching the
real archive's measured 1.73 MiB size (`BENCH-1`, measured 2026-09-01) — a
known scan-time regression on machines with the OpenCode desktop app
installed (against a whole scan that otherwise takes 12-45 ms), tracked in
`internal-docs/pendientes-desarrollo.md` (entry P17) rather than mitigated
by a runtime guard. EVERY failure at any step — an oversized header, a
malformed prefix, an absent `package.json` entry, malformed header JSON, an
offset or shape that fails the D1/D2/D3 defense-in-depth checks, a missing
or empty `version`, or an unreadable archive — MUST degrade that slot to
`ClientPresenceStatus::Detected` with empty `installations`, MUST NOT panic,
and MUST NOT fail the scan. An oversized header (the byte-size ceiling
firing) MUST produce exactly one `Warning`-severity `ScanIssue` naming the
declared size and the ceiling — a deliberate "we chose not to look", not a
defect. Every OTHER failure mode MUST produce exactly one `Error`-severity
`ScanIssue`. Detection MUST NOT depend on version extraction succeeding.
(Previously: covered only the two npm slots and the bundled slot; adds the OpenCode desktop asar-header source and its full degradation contract. Also corrects an earlier draft of this same delta, which had claimed a time-budget ceiling and blanket non-`Error` severity for every failure — withdrawn during design per §3.1/§3.4 and §5.2's severity taxonomy, replaced by the byte-size-only ceiling and the `Warning`-for-oversized-header/`Error`-for-everything-else split above.)

#### Scenario: An npm slot's version comes from package.json

- GIVEN a fixture Claude Code npm install whose `package.json` declares `"version": "1.4.2"`
- WHEN the scanner runs
- THEN the resulting `ClientInstallation` has `version: "1.4.2"`

#### Scenario: The bundled slot's version comes from the directory name

- GIVEN a fixture Claude Code desktop install whose single versioned subdirectory is named `1.5.0`
- WHEN the scanner runs
- THEN the resulting `ClientInstallation` has `version: "1.5.0"` and its `path` points at that versioned directory

#### Scenario: OpenCode desktop version extraction succeeds

- GIVEN a fixture OpenCode desktop install whose `app.asar` header resolves `package.json`'s offset and size, with extracted `version` `"0.4.0"`
- WHEN the scanner runs
- THEN exactly one `ClientInstallation` with `client: OpenCode` and `version: "0.4.0"` is produced for that slot

#### Scenario: Every version-extraction failure mode degrades to Detected with no installations, never a NotDetected verdict

- GIVEN a fixture OpenCode desktop install whose `app.asar` triggers one of: an oversized header beyond the ceiling, a malformed prefix, a header JSON with no `package.json` entry, malformed header JSON, an offset/shape that fails the D1/D2/D3 defense-in-depth checks, a `package.json` entry with a missing or empty `version`, or an unreadable archive
- WHEN the scanner runs
- THEN that slot's `ClientPresence` record has `status: Detected` and empty `installations`
- AND no panic occurs and the scan completes successfully

#### Scenario: An oversized header degrades with a Warning, every other failure with an Error

- GIVEN a fixture OpenCode desktop install whose `app.asar` header exceeds `HEADER_MAX_BYTES`
- WHEN the scanner runs
- THEN exactly one `Warning`-severity `ScanIssue` is produced, naming the declared size and the ceiling
- GIVEN instead a fixture OpenCode desktop install whose `app.asar` fails for any OTHER reason in the list above
- WHEN the scanner runs
- THEN exactly one `Error`-severity `ScanIssue` is produced

### Requirement: Every Resolved Probe Slot Always Emits A Typed Presence Record

For each platform with a real probe table, `scan_for` MUST return
`Some(Vec<ClientPresence>)` with exactly one record per probe slot,
regardless of whether any `ClientInstallation` was resolved for that slot.
On Windows this MUST always be exactly five records (Claude Code npm, Claude
Code bundled, OpenCode npm, OpenCode desktop, Codex standalone), in
deterministic order. A slot's `status` MUST be `Detected` when at least one
candidate root for that slot exists on disk, and `NotDetected` when none
does — `status` MUST NOT be derived from whether `installations` is
non-empty. A slot resolving to more than one installation MUST list all of
them inside that single record's `installations`, never merged or reduced
to one (CA-7). Emitting a presence record MUST NOT itself push a
`ScanIssue`.

`ScanReport.installations` MUST remain a derived flattening of every
record's `installations`, in record order, computed by a single function.
(Previously: exactly four records; adds the OpenCode desktop slot, bringing the count to five.)

#### Scenario: A machine with no clients yields five notDetected records and zero issues

- GIVEN the `nothing` fixture home
- WHEN the scanner runs
- THEN `client_presence` is `Some` with exactly five records, all `status: NotDetected` with empty `installations`
- AND zero `ScanIssue` values are produced

#### Scenario: The scan always emits exactly five presence records on Windows

- GIVEN any fixture home, whether every slot is detected, none is, or a mix
- WHEN the scanner runs
- THEN `client_presence` is `Some` with exactly five records, one per defined slot, in deterministic order

#### Scenario: OpenCode desktop root existing yields Detected; absent yields NotDetected

- GIVEN two fixture homes: one with `AppData/Local/Programs/@opencode-aidesktop` present, one without
- WHEN the scanner runs over each
- THEN the OpenCode desktop slot's record is `status: Detected` for the first home and `status: NotDetected` for the second

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
