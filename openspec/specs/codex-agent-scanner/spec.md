# Codex Agent Scanner Specification

## Purpose

Defines the contract for discovering Codex agent `Component` values from
flat TOML files under `~/.codex/agents/`. Traces to T7's third-client
extension of the T5/T6 per-client agent-adapter pattern
(`internal-docs/plan-desarrollo-poc.md:132-167`), added by
`add-codex-client-support` (2026-08-23); closes CA-12 (a corrupt file is
reported with its path and does not break the scan) for the new TOML
dialect, and CA-8 (no name-convention filtering) re-affirmed for a third
vendor. Bound by CA-16 (read-only) and CA-17 (versioned fixtures). Core
(Rust) only — no IPC or frontend surface; `ClientKind::Codex` and its
regenerated binding are specified by `domain-model`, not here.

This capability differs from `agent-scanner` only in file format: a Codex
agent is a flat TOML file, not Markdown with YAML frontmatter, so it is
parsed through the `toml.rs` seam (specified by `workspace-architecture`)
rather than through `frontmatter::read`. It is a third standalone adapter
module, not an extension of `agents.rs` or `opencode_agents.rs`, and
introduces no shared abstraction across the three agent adapters.

## Requirements

### Requirement: Codex Agent Root Resolves Under The Home Directory

The scanner MUST resolve exactly one Codex agent root by concatenating the
resolved home directory with the hardcoded relative suffix `.codex/agents/`.
The root path MUST NOT be derived from any OS config-directory convention,
`dirs`/`directories` crate, or environment read; it is computed from the home
directory alone, mirroring the convention `skill-scanner` and `agent-scanner`
already established. The root is represented as a `SearchRoot` with `kind:
SearchRootKind::Agent`, mirroring `opencode_agent_root`.

#### Scenario: The Codex agent root resolves under the home directory on every OS

- GIVEN the scanner runs on any supported platform
- WHEN the Codex agent root is resolved
- THEN it is `<home>/.codex/agents/`, never a platform config-dir path

### Requirement: A Direct `.toml` File Under The Root Is An Agent Candidate, Detected Flat

A file MUST be treated as a Codex agent candidate if and only if it is a
`.toml` file located directly under the resolved Codex agent root. The
scanner MUST NOT walk into subdirectories of the agent root; traversal is a
single, flat `read_dir` pass, never recursive, mirroring `agent-scanner`'s
flat-walk rule for `~/.claude/agents/`.

#### Scenario: A `.toml` file directly under the root is discovered

- GIVEN a fixture Codex agent root containing one well-formed `valid-agent.toml` file directly beneath it
- WHEN the scanner walks that root
- THEN a `Component` is produced for it

#### Scenario: A file nested in a subdirectory of the root is not discovered

- GIVEN a fixture Codex agent root containing a subdirectory that itself contains a well-formed agent `.toml` file
- WHEN the scanner walks that root
- THEN no `Component` is produced from the nested file, and no `ScanIssue` references it

#### Scenario: A non-`.toml` file directly under the root is ignored

- GIVEN a fixture Codex agent root containing a file without a `.toml` extension
- WHEN the scanner walks that root
- THEN no `Component` and no `ScanIssue` reference that file

### Requirement: Absent and Empty Codex Agent Roots Produce No Issue and No Component

A Codex agent root that does not exist on disk, and one that exists but
contains no `.toml` file directly beneath it, MUST each produce zero
components and zero `ScanIssue` values, mirroring the absent/empty-root
guarantee `skill-scanner` and `agent-scanner` already provide.

#### Scenario: An absent Codex agent root yields nothing and no issue

- GIVEN a Codex agent root path that does not exist on disk
- WHEN the scanner runs
- THEN no `Component` and no `ScanIssue` reference that root

#### Scenario: A present, empty Codex agent root yields nothing and no issue

- GIVEN a Codex agent root path that exists on disk and contains zero `.toml` files directly beneath it
- WHEN the scanner runs
- THEN no `Component` and no `ScanIssue` reference that root

### Requirement: TOML Frontmatter Is Parsed Through The Shared Seam, Never By Hand Or By Regex

Each discovered `.toml` file MUST be parsed exclusively through the crate's
`toml.rs` seam (`toml::from_str`), never through a hand-rolled parser and
never through a regular expression, so that a `developer_instructions` value
written as a triple-quoted, multiline `"""…"""` string is returned complete
and byte-exact, including embedded newlines — no truncation at the first
quote or the first newline. `CodexAgentDocument` maps `name: String`,
`description: Option<String>`, and `developer_instructions:
Option<String>`; a file missing `name` is an `Error` for the whole file,
never a fallback to the file stem, and `developer_instructions` is parsed
and pinned by tests but deliberately dropped, never mapped onto `Component`.

#### Scenario: A genuine multiline developer_instructions value is returned complete

- GIVEN a fixture Codex agent `.toml` file whose `developer_instructions` key is a triple-quoted `"""…"""` string spanning multiple lines with embedded blank lines
- WHEN the file is parsed
- THEN the resulting value is the complete, unmodified multiline string, with no truncation and no loss of embedded newlines

#### Scenario: No regular expression is involved in parsing any Codex agent file

- GIVEN the Codex agent adapter's source
- WHEN it is inspected
- THEN it contains no regular-expression-based parsing of `.toml` content

### Requirement: On-Disk Codex Agent Component Assembly

Every Codex agent successfully parsed from a `.toml` file under the Codex
agent root MUST be assembled into a `Component` with `kind:
ComponentKind::Agent`, `scope: Scope::User`, and exactly one `Location` where
`path: Some(path)`, `origin: LocationOrigin::File`. `Component.id` MUST be
derived the same way every other adapter derives it: from `(kind, normalized
name)` alone, with no client discriminator (per the identity decision
recorded in `add-codex-client-support`'s proposal).

#### Scenario: A valid on-disk Codex agent produces a correctly shaped Component

- GIVEN a fixture Codex agent root containing one well-formed agent `.toml` file
- WHEN the scanner runs
- THEN it produces one `Component` with `kind: Agent`, `scope: User`, and one `Location { path: Some(_), origin: File }`

### Requirement: Per-File Parsing Failures Do Not Abort The Walk

Each discovered `.toml` file that fails to parse, or that fails required-field
validation (a missing `name`), MUST produce exactly one `ScanIssue` at
`IssueSeverity::Error` carrying that file's path, mirroring `agent-scanner`'s
and `skill-scanner`'s escalation rule, and MUST NOT prevent any sibling
`.toml` file under the same root from being discovered and parsed (CA-12).

#### Scenario: One malformed Codex agent file yields an issue and does not stop the walk

- GIVEN a Codex agent root containing one malformed `.toml` file and two well-formed sibling agent `.toml` files
- WHEN the scanner walks that root
- THEN one `ScanIssue` at `IssueSeverity::Error` carrying the malformed file's path is produced, and both sibling agents are still discovered as components

### Requirement: Scanner Performs No Writes

The scanner MUST perform no filesystem write of any kind — no file creation,
no file modification, no directory creation — anywhere in its resolution,
parsing, or assembly logic (CA-16).

#### Scenario: A full scan run leaves the fixture tree byte-for-byte unchanged

- GIVEN a fixture Codex agent root with a known state before a scan
- WHEN the scanner runs a full scan over it
- THEN the fixture tree's contents are unchanged afterward

### Requirement: Every Case Is Traceable To A Repository Fixture In A New, Non-Reused Tree

Each requirement above MUST be exercised by a fixture committed under a new
`crates/vertice-core/tests/fixtures/roots/codex-agents/` tree, distinct from
and never reused from `crates/vertice-core/tests/fixtures/roots/agents/` (T5)
or `crates/vertice-core/tests/fixtures/roots/opencode-agents/` (T6). At
minimum, the fixture set MUST cover: a valid agent; an agent with a genuine
multiline `"""…"""` `developer_instructions`; a malformed `.toml` file; a
nested subdirectory under the root; a non-`.toml` sibling file; an absent
root; and an empty root.

#### Scenario: Fixture set covers every documented case

- GIVEN this spec's full list of requirements
- WHEN the `crates/vertice-core/tests/fixtures/roots/codex-agents/` directory is enumerated
- THEN each requirement above has at least one fixture proving its behavior
