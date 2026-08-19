# Client Installation Detector Specification

## Purpose

Defines the contract for detecting which AI clients are installed on the user's machine, on Windows: three independent probe slots (Claude Code npm, Claude Code desktop, OpenCode npm), each yielding a separately reported `ClientInstallation` with its own version, or an explicit "not detected" signal when absent. Traces to T7 of `internal-docs/plan-desarrollo-poc.md`; closes CA-7 (the two Claude Code installations are detected separately, each with its version) and CA-11 (an absent client is reported as *not detected*, never as an error and never as an unexplained empty list); bound by CA-16 (read-only) and CA-17 (fixture-based, machine-independent tests on a new, non-reused fixture tree). Core (Rust) only, Windows only — macOS/Linux path tables are T16. No `domain-model` requirement is added or modified by this capability: `sdd-design` closed the not-detected representation on the `ScanIssue` carrier and explicitly rejected a typed carrier (design §2), so `model/` and the generated TypeScript bindings are unchanged and `domain-model` is not a Modified Capability.

## Requirements

### Requirement: Windows Probe Paths Are Hardcoded, Never OS-Convention-Derived

The scanner MUST probe exactly three fixed, hardcoded paths relative to a passed-in `home: &Path`: the Claude Code npm install (`AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code/`), the Claude Code desktop install (`AppData/Roaming/Claude/claude-code/<version>/`), and the OpenCode npm install (`AppData/Roaming/npm/node_modules/opencode-ai/`). No probed path MAY be produced by a `dirs`/`directories` crate or by reading an environment variable; every path MUST be composed from `home` plus a hardcoded relative segment, mirroring `roots.rs`'s existing convention.

#### Scenario: All three probe paths resolve under the passed-in home

- GIVEN a scanner invocation with a fixture `home` path
- WHEN the three slots are probed
- THEN each probed path is `home` concatenated with its fixed relative segment, computed with no `dirs`/`directories` import and no environment variable read

### Requirement: Claude Code npm And Desktop Are Never Merged

A home carrying both the Claude Code npm installation and the Claude Code desktop installation MUST produce two separate `ClientInstallation` values with `client: ClaudeCode`, each carrying its own `version` and its own `path`. The scanner MUST NOT collapse them into one entry on account of sharing the same client kind (CA-7).

#### Scenario: Both Claude Code installs with different versions yield two entries

- GIVEN a fixture home with a Claude Code npm install at version `1.2.0` and a Claude Code desktop install at version `1.3.0`
- WHEN the scanner runs
- THEN the result contains exactly two `ClientInstallation` values with `client: ClaudeCode`, one with `version: "1.2.0"` and one with `version: "1.3.0"`, and neither is merged into the other

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

### Requirement: An Absent Slot Is Reported As An Explicit "Not Detected" Signal

When a slot's probed path does not exist on disk, the scanner MUST produce zero `ClientInstallation` for that slot and MUST emit an explicit signal that names the client, the install kind, and the concrete probed path. This signal MUST be observably distinguishable from a parse-failure signal for the same slot and MUST NOT be represented as a silent omission from the result (CA-11). The carrier for this signal is a `ScanIssue`, closed in `sdd-design` (design §2) together with its severity and `reason` format; this spec asserts the naming and distinguishability guarantees above, and does not restate that carrier's concrete severity or string format.

#### Scenario: An absent slot yields no installation and a signal naming the probed path

- GIVEN a fixture home where the OpenCode npm path does not exist
- WHEN the scanner runs
- THEN no `ClientInstallation` with `client: OpenCode` is produced, and a signal is emitted that names the OpenCode client, its npm install kind, and the exact path that was probed

#### Scenario: A not-detected signal is distinguishable from a parse-error signal

- GIVEN one fixture home where a slot's path does not exist and another where the same slot's path exists but its `package.json` is malformed
- WHEN the scanner runs over each home
- THEN the two runs produce signals that can be told apart as "not detected" versus "parse error", never the same shape

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

### Requirement: Each Desktop Version Directory Is Its Own Installation

A desktop install directory present with zero versioned subdirectories MUST produce no `ClientInstallation` and one `ScanIssue`. A desktop install directory carrying one or more versioned subdirectories MUST produce one `ClientInstallation` per versioned subdirectory, each with the directory name as `version` and the subdirectory itself as `path`. These installations MUST NOT be merged into one entry and MUST NOT be reported as a `ScanIssue` anomaly on account of there being more than one — a client installed twice is reported as two `ClientInstallation` values, mirroring `ClientInstallation`'s own contract for a client installed twice (`crates/vertice-core/src/model/installation.rs:8-10`).

#### Scenario: A desktop directory with no versioned subdirectory yields no installation

- GIVEN a fixture Claude Code desktop directory containing no versioned subdirectory
- WHEN the scanner runs
- THEN no `ClientInstallation` is produced for the desktop slot, and one `ScanIssue` references it

#### Scenario: A desktop directory with two versioned subdirectories yields two installations, never merged

- GIVEN a fixture Claude Code desktop directory containing two versioned subdirectories with different names
- WHEN the scanner runs
- THEN exactly two `ClientInstallation` values are produced, both `client: ClaudeCode`, one per subdirectory with its own `version` and `path`, no `ScanIssue` is produced for the desktop slot on account of the count, and neither installation is merged into the other

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

Each requirement above MUST be exercised by a fixture committed under `crates/vertice-core/tests/fixtures/installations/`, a tree distinct from and never reused from any T4/T5/T6 fixture tree. At minimum, the fixture set MUST cover: both Claude Code installs present with different versions (CA-7); OpenCode npm present; a slot whose path does not exist (CA-11); a malformed `package.json`; a `package.json` missing `"version"`; a desktop directory with no versioned subdirectory; a desktop directory with more than one versioned subdirectory, yielding one installation per subdirectory; and per-slot failure isolation. No test MAY read the author's machine, set an environment variable, or invoke `claude`/`opencode`.

#### Scenario: Fixture set covers every documented case

- GIVEN this spec's full list of requirements
- WHEN the `crates/vertice-core/tests/fixtures/installations/` directory is enumerated
- THEN each requirement above has at least one fixture proving its behavior
