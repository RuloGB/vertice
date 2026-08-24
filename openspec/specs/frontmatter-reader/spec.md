# Frontmatter Reader Specification

## Purpose

Defines the contract for reading YAML frontmatter from a single `SKILL.md`-shaped file: isolating the `---`-fenced block, delegating YAML deserialization to the shared seam, and converting every documented failure class into a `ScanIssue` value. Traces to T3 of the completed PoC roadmap; closes CA-10 (complete multi-line description) and CA-12 (partial: corrupt file carries its path). Core (Rust) only — no frontend or IPC surface.

## Requirements

### Requirement: Single-File Input Only

The reader MUST accept a single `&Path` pointing at one file and MUST NOT perform directory walking, root discovery, or glob expansion.

#### Scenario: Reader touches only the given path
- GIVEN a path to one Markdown file with frontmatter
- WHEN the reader is invoked with that path
- THEN it reads only that file and discovers no other path

### Requirement: One Outcome Per File

The reader MUST return exactly one outcome per file, `Result<T, ScanIssue>` — never a partial value alongside a separate issue list.

#### Scenario: Success returns Ok only
- GIVEN a file with valid frontmatter
- WHEN parsed
- THEN it returns `Ok(value)` with no accompanying `ScanIssue`

#### Scenario: Failure returns Err only
- GIVEN a file meeting any documented failure class
- WHEN parsed
- THEN it returns `Err(ScanIssue)` with no partially parsed value alongside it

### Requirement: Generic Over the Deserialization Target

The reader MUST be generic over its output type, accepting any `T: DeserializeOwned`, so a future caller supplies its own frontmatter shape without modifying the reader.

#### Scenario: Reader is reused for a second target type
- GIVEN a second, non-skill struct implementing `DeserializeOwned`
- WHEN the reader is invoked with that type as its target
- THEN it deserializes into that type via the same read/split/error path, unchanged

### Requirement: Skill Frontmatter Data Contract

For the skill use case, the target MUST expose `name: String` (required) and `description: Option<String>` (optional), mirroring `Component.name`/`Component.description` optionality.

#### Scenario: Absent description is a value, not a failure
- GIVEN frontmatter with `name` present and no `description` key
- WHEN parsed
- THEN it returns `Ok` with `description == None`

### Requirement: Fence Splitting Is Line-Based and Regex-Free

The reader MUST isolate the frontmatter block by detecting exact `---` fence lines, before any YAML deserialization, and MUST NOT use a regular expression for this step.

#### Scenario: A folded description is never truncated
- GIVEN frontmatter with a `description: >` folded block scalar spanning multiple lines
- WHEN the fence is split
- THEN no line of the block scalar's content is altered or dropped

### Requirement: YAML Parsing Is Delegated to the Shared Seam

The reader MUST delegate deserialization of the isolated block to `vertice_core::yaml::from_str` and MUST NOT import the YAML parsing crate directly.

#### Scenario: A parse failure surfaces through the shared seam
- GIVEN a fenced block with malformed YAML
- WHEN the reader delegates parsing to the shared seam
- THEN the seam's parse failure is converted into `ScanIssue.reason`

### Requirement: Successful Parse Returns Complete, Correctly-Typed Data

A valid file MUST yield `Ok` with every field set to its exact, complete value.

#### Scenario: Single-line description
- GIVEN valid frontmatter with `name` and a single-line `description`
- WHEN parsed
- THEN it returns `Ok` with both fields set to their exact values

#### Scenario: Folded multi-line description is complete (CA-10)
- GIVEN valid frontmatter with a folded `description: >` scalar spanning multiple lines
- WHEN parsed
- THEN it returns `Ok` with the full, correct description asserted in its entirety, not a prefix

#### Scenario: Absent description succeeds
- GIVEN valid frontmatter with `name` present and no `description` key
- WHEN parsed
- THEN it returns `Ok` with `description == None`

### Requirement: Every Failure Class Yields a ScanIssue, Never a Panic

The reader MUST NOT panic for any input. Each failure class below MUST produce `Err(ScanIssue)`. This is a leaf-level, per-file guarantee only; that one failing file does not abort a larger scan is established at the orchestration level (T9), not here.

#### Scenario: Corrupt YAML carries its path (CA-12 partial)
- GIVEN a fixture whose fenced block contains malformed YAML
- WHEN parsed
- THEN it returns `Err(ScanIssue)` with `path: Some(path)` and `reason` describing the parse failure

#### Scenario: Non-UTF-8 content carries its path, never None
- GIVEN a fixture at a valid, readable path whose bytes are not valid UTF-8
- WHEN parsed
- THEN it returns `Err(ScanIssue)` with `path: Some(path)` — distinct from a non-UTF-8 *path*, T4's concern and out of scope here

Additional failure classes, each producing `Err(ScanIssue)` without panicking:

| Case | Given |
|---|---|
| Absent frontmatter | Markdown body, no `---` fence at all |
| Empty file | Zero-byte input, distinct from absent frontmatter |
| Missing name | `description` present, `name` key absent |
| Type mismatch | A scalar-typed field given a YAML list or mapping |
| Unterminated fence | Opening `---` with EOF before a closing `---` |
| I/O failure | Path that cannot be read (missing or unreadable) |

### Requirement: Every Case Is Traceable to a Repository Fixture

Each requirement above MUST be exercised by a fixture committed under `crates/vertice-core/tests/fixtures/`. Fixtures MUST be self-contained within the repository; no test MAY depend on a machine-specific or out-of-repository path. The locked set is ten fixtures: baseline, folded-multiline, absent-description, absent-frontmatter, empty-file, corrupt-yaml, missing-name, type-mismatch, unterminated-fence, non-utf8-content. The I/O-failure case is the sole exception and MUST NOT have a fixture file — it is exercised through a defined repository-relative path that does not exist on disk.

#### Scenario: Fixture set covers every documented case
- GIVEN this spec's full list of success and failure requirements
- WHEN the fixture directory is enumerated
- THEN each requirement above has at least one fixture proving its behavior, the I/O-failure case excepted, which is proven through a repository-relative path that does not exist on disk

### Requirement: Non-UTF-8 Fixture Bytes Survive Checkout Unmodified

The non-UTF-8-content fixture's exact byte sequence MUST be preserved unmodified across checkout on all three CI platforms. The mechanism is a design decision.

#### Scenario: Byte content is stable across platforms
- GIVEN the non-UTF-8-content fixture committed to the repository
- WHEN checked out on any of the three CI platforms
- THEN its bytes are identical to the committed original and still trigger a UTF-8 decode failure, not an accidental successful parse

### Requirement: Core-Only Capability, No Frontend Surface

This capability MUST expose no IPC command, no Tauri command registration, and no generated TypeScript binding.

#### Scenario: No frontend artifact is produced
- GIVEN this capability as delivered
- WHEN the frontend/IPC surface is inspected
- THEN no new command, binding, or frontend-visible type exists as a result of it
