# Agent Scanner Specification

## Purpose

Defines the contract for discovering Claude Code agent `Component` values: resolving the single `~/.claude/agents/` user root, a flat (non-recursive) walk over the `.md` files directly beneath it, agent frontmatter parsing via the shared reader, and the fixed set of embedded agents that ship inside Claude Code itself with no backing file. Traces to T5 of the completed PoC roadmap; closes CA-5 partial (the 17 on-disk agents of the reference installation appear, asserted over equivalent fixtures) and CA-13 core half (embedded components are marked and distinguishable); contributes to CA-12 partial (a corrupt file yields a `ScanIssue` carrying its path and interrupts nothing); bound by CA-16 (read-only) and CA-17 (fixture-based, machine-independent tests). Core (Rust) only — no IPC or frontend surface in this change; no `domain-model` requirement is added, modified, or exercised differently by this capability.

## Requirements

### Requirement: Agent Root Resolves Under The Home Directory

The scanner MUST resolve exactly one Claude Code agent root by concatenating the resolved home directory with the hardcoded relative suffix `.claude/agents/`, with `kind: SearchRootKind::Agent`. The root path MUST NOT be derived from any OS config-directory convention; it is computed from the home directory alone, mirroring the convention `skill-scanner` already established for its three roots.

#### Scenario: The agent root resolves under the home directory on every OS

- GIVEN the scanner runs on any supported platform
- WHEN the Claude Code agent root is resolved
- THEN it is `<home>/.claude/agents/`, never a platform config-dir path

#### Scenario: The resolved root carries SearchRootKind::Agent

- GIVEN the resolved Claude Code agent root
- WHEN its `kind` field is inspected
- THEN it is `SearchRootKind::Agent`, distinguishing it from a skill root of the same client

### Requirement: A Direct `.md` File Under The Root Is An Agent, Detected Flat

A file MUST be treated as an agent candidate if and only if it is a `.md` file located directly under the resolved agent root. The scanner MUST NOT walk into subdirectories of the agent root; traversal is a single, flat `read_dir` pass, never recursive.

#### Scenario: A `.md` file directly under the root is discovered

- GIVEN a fixture agent root containing one `valid-agent.md` file directly beneath it
- WHEN the scanner walks that root
- THEN a `Component` is produced for it

#### Scenario: A file nested in a subdirectory of the root is not discovered

- GIVEN a fixture agent root containing a subdirectory that itself contains a well-formed agent `.md` file
- WHEN the scanner walks that root
- THEN no `Component` is produced from the nested file, and no `ScanIssue` references it

#### Scenario: A non-`.md` file directly under the root is ignored

- GIVEN a fixture agent root containing a file without a `.md` extension
- WHEN the scanner walks that root
- THEN no `Component` and no `ScanIssue` reference that file

### Requirement: Absent and Empty Agent Roots Produce No Issue and No Component

An agent root that does not exist on disk, and one that exists but contains no `.md` file directly beneath it, MUST each produce zero components with `origin: File` and zero `ScanIssue` values. This requirement binds file-backed components only. Embedded components are governed independently by the embedded-agent requirement below, so an assertion for this requirement MUST filter on `origin == File` and MUST NOT assert that the component set is empty — an empty-set assertion would forbid the CA-13 behavior specified below. These two states MUST remain distinguishable from each other in the reported root status, matching the guarantee `skill-scanner` already provides for its own roots.

#### Scenario: An absent agent root yields nothing and no issue

- GIVEN an agent root path that does not exist on disk
- WHEN the scanner runs
- THEN no `Component` with `origin: File` and no `ScanIssue` reference that root

#### Scenario: A present, empty agent root yields nothing and no issue

- GIVEN an agent root path that exists on disk and contains zero `.md` files directly beneath it
- WHEN the scanner runs
- THEN no `Component` with `origin: File` and no `ScanIssue` reference that root

#### Scenario: Absent and present-empty are distinguishable in the result

- GIVEN one agent root that does not exist and another that exists and is empty
- WHEN the scan result is inspected
- THEN the two roots are reported in states that can be told apart

### Requirement: Agent Frontmatter Data Contract

The scanner MUST parse each discovered agent file's frontmatter into a shape exposing `name: String` (required), `description: Option<String>`, `model: Option<String>`, and `tools: Option<String>`. `tools` MUST be a scalar `String`, matching the real Claude Code file format (`tools: Read, Grep, Glob, Bash`), never a sequence type. Parsing MUST be delegated to the existing generic `frontmatter::read<T: DeserializeOwned>` entry point without modification to that function or to `crate::yaml`.

#### Scenario: A comma-separated tools scalar deserializes successfully

- GIVEN a fixture agent file whose frontmatter declares `tools: Read, Grep, Glob, Bash`
- WHEN the file is parsed
- THEN it returns `Ok` with `tools` set to that value as one `String`, not a list

#### Scenario: A missing model and missing tools field is not a failure

- GIVEN a fixture agent file whose frontmatter declares `name` and `description` but no `model` key and no `tools` key
- WHEN the file is parsed
- THEN it returns `Ok` with `model == None` and `tools == None`, and a `Component` is still produced

#### Scenario: A folded block-scalar description is parsed in full

- GIVEN a fixture agent file whose frontmatter declares `description: >` spanning multiple lines
- WHEN the file is parsed
- THEN it returns `Ok` with the complete, un-truncated description, parsed through the shared YAML seam and never through a regular expression

### Requirement: On-Disk Agent Component Assembly

Every agent successfully parsed from a `.md` file under the agent root MUST be assembled into a `Component` with `kind: ComponentKind::Agent`, `scope: Scope::User`, and exactly one `Location` where `path: Some(path)`, `origin: LocationOrigin::File`, and `root` set to the resolved agent root's id. `Component.id` MUST be derived the same way every other adapter derives it: from `(kind, normalized name)` alone.

#### Scenario: A valid on-disk agent produces a correctly shaped Component

- GIVEN a fixture agent root containing one well-formed agent `.md` file
- WHEN the scanner runs
- THEN it produces one `Component` with `kind: Agent`, `scope: User`, and one `Location { path: Some(_), origin: File }`

### Requirement: Embedded Agents Are Emitted From A Fixed, Named List

The scanner MUST emit a component for each of a fixed set of six embedded Claude Code agents (`Explore`, `Plan`, `general-purpose`, `statusline-setup`, `claude`, `claude-code-guide`) whenever the Claude Code client directory `<home>/.claude` is present, independent of what the agent root beneath it contains. When `<home>/.claude` is absent, the scanner MUST emit zero embedded components: the client is not installed, and reporting six of its agents would be a false positive. Presence of `<home>/.claude` is a directory-presence probe, not installation detection; refining it is T7's concern and MUST NOT change this component contract. Each MUST have `kind: ComponentKind::Agent`, `scope: Scope::User`, and exactly one `Location` where `path: None` and `origin: LocationOrigin::Embedded`. Every embedded component's `Location` MUST still carry a valid, non-panicking `SearchRootId` value in its `root` field — the domain model does not make `Location.root` optional, and this capability introduces no model change to make it so. The specific id chosen is a design decision; this requirement only binds that the value is present and well-formed.

#### Scenario: The six embedded agents appear even when the agent root is absent

- GIVEN a home where `<home>/.claude` exists but `<home>/.claude/agents/` does not
- WHEN the scanner runs
- THEN it still produces exactly six components with `origin: Embedded` and `path: None`

#### Scenario: No embedded agents are emitted when the client directory is absent

- GIVEN a home where `<home>/.claude` does not exist
- WHEN the scanner runs
- THEN it produces zero components, embedded or on-disk, and no `ScanIssue`

#### Scenario: Embedded and on-disk agents are distinguishable by origin and path alone

- GIVEN a scan producing both embedded and on-disk agent components
- WHEN each `Component`'s single `Location` is inspected
- THEN embedded components have `origin: Embedded` and `path: None`, on-disk components have `origin: File` and `path: Some(_)`, and no filename or naming heuristic is needed to tell them apart

#### Scenario: An embedded component's Location.root is a valid value

- GIVEN a scan that produces an embedded agent component
- WHEN its `Location.root` field is inspected
- THEN it holds a valid `SearchRootId`, never an omitted or panic-inducing value

### Requirement: A User Agent File Shadowing An Embedded Agent Name Produces Two Components

When an on-disk agent file's `name` collides with one of the six embedded agents' names, the scanner MUST emit both as separate components rather than merging, suppressing, or overwriting either. Consolidating same-identity components across origins is explicitly out of scope for this capability and deferred to T8.

#### Scenario: An on-disk agent named `Plan` coexists with the embedded `Plan`

- GIVEN a fixture agent root containing a file whose frontmatter `name` is `Plan`
- WHEN the scanner runs
- THEN the result contains two components with identity derived from `(Agent, "Plan")` — one with `origin: Embedded, path: None` and one with `origin: File, path: Some(_)` — and neither replaces the other

### Requirement: Per-File Parsing Failures Do Not Abort The Walk

Each discovered agent `.md` file MUST be parsed via `frontmatter::read`. A file the reader reports as a failure MUST produce a `ScanIssue` at `IssueSeverity::Error` carrying that file's path, mirroring `skill-scanner`'s escalation rule, and MUST NOT prevent any sibling file under the same root from being discovered and parsed.

#### Scenario: One corrupt agent file yields an issue and does not stop the walk

- GIVEN an agent root containing one corrupt agent `.md` file and two well-formed sibling agent files
- WHEN the scanner walks that root
- THEN one `ScanIssue` at `IssueSeverity::Error` carrying the corrupt file's path is produced, and both sibling agents are still discovered as components

#### Scenario: Every reader failure class is escalated to Error

- GIVEN a `ScanIssue` returned by `frontmatter::read` at any severity
- WHEN the agent scanner processes that failure
- THEN the resulting `ScanIssue` has `severity: Error`, with `path` and `reason` unchanged

### Requirement: Non-UTF-8 Discovered Paths Are Guarded

A discovered path that is not representable as UTF-8 MUST yield a `ScanIssue` at `IssueSeverity::Error` with `path: None` and a lossy rendering of the path in `reason`, and the walk MUST continue to sibling entries, mirroring `skill-scanner`'s guard.

#### Scenario: A non-UTF-8 path is reported without aborting the walk

- GIVEN a discovered agent file path that is not valid UTF-8
- WHEN the scanner processes that entry
- THEN it produces one `ScanIssue` with `path: None` and continues to any remaining entries under the same root

### Requirement: Scanner Performs No Writes

The scanner MUST perform no filesystem write of any kind — no file creation, no file modification, no directory creation — anywhere in its resolution or traversal logic.

#### Scenario: A full scan run leaves the fixture tree byte-for-byte unchanged

- GIVEN a fixture agent root with a known state before a scan
- WHEN the scanner runs a full scan over it
- THEN the fixture tree's contents are unchanged afterward

### Requirement: This Capability Introduces No Domain Model Change

This capability MUST NOT add, remove, or modify any field on any type under `crates/vertice-core/src/model/`, and MUST NOT require regeneration of any file under `frontend/src/bindings/`. `AgentFrontmatter`'s `model` and `tools` fields are consumed by this capability only as parsed data; they MUST NOT be surfaced as new `Component` fields.

#### Scenario: Model and bindings are unchanged after this capability is implemented

- GIVEN the repository before and after this capability is implemented
- WHEN `crates/vertice-core/src/model/` and `frontend/src/bindings/` are compared
- THEN they are byte-identical, and the CI bindings-drift gate reports no diff

### Requirement: Reference Fixture Set Produces Exactly 17 On-Disk Agent Components

Run over a fixture tree reproducing the reference installation's agent root, the scanner MUST produce exactly 17 on-disk agent components.

#### Scenario: The reference fixture agent root yields 17 on-disk components

- GIVEN a committed fixture tree of 17 well-formed agent `.md` files directly under a fixture agent root
- WHEN the scanner performs a full scan of that root
- THEN it produces exactly 17 `Component` values with `kind: Agent`, `origin: File`

### Requirement: Every Case Is Traceable To A Repository Fixture

Each requirement above MUST be exercised by a fixture committed under `crates/vertice-core/tests/fixtures/roots/agents/`, as synthetic home directories. This capability walks a tree rather than addressing a single file, so its fixtures belong beside the other walked-tree fixtures under `fixtures/roots/` — the addressed-file fixtures under `fixtures/frontmatter/` are a different shape and MUST NOT be reused. No test MAY read the author's machine or set an environment variable, and no test MAY reuse a fixture authored for `skill-scanner`. At minimum, the fixture set MUST cover: a valid agent, a broken-frontmatter agent, an agent missing `model` and `tools`, an agent with a folded block-scalar `description`, an agent whose `tools` value is a comma-separated scalar, an agent name shadowing an embedded agent, a nested subdirectory under the root, an absent root, an empty root, an absent `<home>/.claude` client directory, and the 17-file reference set.

#### Scenario: Fixture set covers every documented case

- GIVEN this spec's full list of requirements
- WHEN the `crates/vertice-core/tests/fixtures/roots/agents/` directory is enumerated
- THEN each requirement above has at least one fixture proving its behavior
