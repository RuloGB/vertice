# Skill Scanner Specification

## Purpose

Defines the contract for discovering skill `Component` values under the four fixed user roots on disk. Traces to T4 of `internal-docs/plan-desarrollo-poc.md`, extended by T7's `add-codex-client-support`; closes CA-6 (no plugin skill appears), CA-8 partial (`_shared` is an ordinary skill), CA-9 (absent/empty root produces no issue and no component), CA-14 (no project-scope component); contributes to CA-12 partial (unreadable file is reported, scan continues); bound by CA-16 (read-only). Core (Rust) only — no IPC or frontend surface in this change; regenerated bindings are a byproduct of the `domain-model` delta.

## Requirements

### Requirement: User Root Set Is Fixed and Hardcoded

The scanner MUST resolve exactly four user roots by concatenating the resolved home directory with a hardcoded, per-client relative suffix: `.claude/skills/`, `.agents/skills/`, `.config/opencode/skills/`, and `.codex/skills/`. The singular `.config/opencode/skill/` MUST be treated as the same OpenCode root as its plural form, matching the glob `{skill,skills}/**/SKILL.md`. Root paths MUST NOT be derived from any OS config-directory convention (e.g. `%APPDATA%` on Windows); they are computed from the home directory alone. The `codex-skills` root MUST be appended after the three pre-existing roots, never inserted before or between them, so that canonical root order for those three roots — and therefore first-non-empty field precedence for any component already merged across them — is unchanged by this addition.

#### Scenario: OpenCode root resolves under the home directory on every OS

- GIVEN the scanner runs on any supported platform
- WHEN the OpenCode root is resolved
- THEN it is `<home>/.config/opencode/skills/` (or its singular alias), never a platform config-dir path

#### Scenario: Singular and plural OpenCode roots are the same root

- GIVEN fixtures for both `.config/opencode/skill/` and `.config/opencode/skills/`
- WHEN the scanner resolves the OpenCode root
- THEN both are scanned as one logical root, not two

#### Scenario: The Codex root resolves under the home directory on every OS

- GIVEN the scanner runs on any supported platform
- WHEN the Codex skill root is resolved
- THEN it is `<home>/.codex/skills/`, never a platform config-dir path

#### Scenario: A Codex SKILL.md with vendor-specific extra keys still parses

- GIVEN a fixture Codex skill root containing a `SKILL.md` whose frontmatter declares `name`, `description`, and the Codex-specific keys `disable-model-invocation`, `user-invocable`, `license`, and `metadata.*`
- WHEN the scanner walks that root
- THEN a `Component` is produced for it, with the unmodelled keys silently ignored rather than causing a parse failure — the same permissive behavior the frontmatter reader already applies to the other three roots

#### Scenario: The Codex root is appended last, not inserted mid-order

- GIVEN the four resolved skill roots
- WHEN their order is inspected
- THEN `.codex/skills/` is the fourth entry, and the relative order of `.claude/skills/`, `.agents/skills/`, and `.config/opencode/skills/` is identical to their order before this root was added

### Requirement: SKILL.md Presence Is the Sole Detection Rule

A directory MUST be treated as a skill if and only if it directly contains a `SKILL.md` file. The scanner MUST NOT apply any name-based heuristic (e.g. excluding directories named `_shared` or prefixed with `_`) to decide whether a directory is a skill.

#### Scenario: A directory named `_shared` containing SKILL.md is a skill

- GIVEN a fixture directory named `_shared` containing a `SKILL.md` file
- WHEN the scanner walks the root containing it
- THEN a `Component` is produced for it exactly as for any other skill directory

### Requirement: Traversal Is Recursive

The scanner MUST walk each root recursively, matching every `SKILL.md` at any depth, not only at a fixed depth of one.

#### Scenario: A SKILL.md nested below the top level is discovered

- GIVEN a fixture with `SKILL.md` two or more directories below a root
- WHEN the scanner walks that root
- THEN a `Component` is produced for it

### Requirement: Symbolic Links Are Not Followed

The scanner MUST NOT follow symbolic links while traversing a root.

#### Scenario: A symlinked directory is not traversed into

- GIVEN a root containing a symbolic link to a directory holding a `SKILL.md`
- WHEN the scanner walks that root
- THEN no `Component` is produced from following that symlink

### Requirement: Absent and Empty Roots Produce No Issue and No Component

A root that does not exist on disk, and a root that exists but contains no `SKILL.md`, MUST each produce zero components and zero `ScanIssue` values. These two states MUST remain distinguishable from each other in the scan result.

#### Scenario: An absent root yields nothing and no issue

- GIVEN a root path that does not exist on disk
- WHEN the scanner runs
- THEN no `Component` and no `ScanIssue` reference that root

#### Scenario: A present, empty root yields nothing and no issue

- GIVEN a root path that exists on disk and contains zero `SKILL.md` files
- WHEN the scanner runs
- THEN no `Component` and no `ScanIssue` reference that root

#### Scenario: Absent and present-empty are distinguishable in the result

- GIVEN one root that does not exist and another that exists and is empty
- WHEN the scan result is inspected
- THEN the two roots are reported in states that can be told apart

### Requirement: Every Skill Component Has Scope::User

Every `Component` produced by this scanner MUST have `scope: Scope::User`. The scanner MUST NOT construct any root or component associated with `Scope::Project` or `Scope::Local`.

#### Scenario: All discovered skills are User-scoped

- GIVEN a full scan across the four roots
- WHEN the produced `Component` values are inspected
- THEN every one has `scope == Scope::User`

#### Scenario: A project-shaped tree outside the four roots yields nothing

- GIVEN a fixture `.claude/skills/` directory located outside the four resolved roots
- WHEN the scanner runs
- THEN no `Component` is produced from it

### Requirement: No Plugin-Provided Skill Appears In The Result

The scan result MUST NOT contain any component sourced from a plugin-provided location. This MUST hold because the scanner only ever walks the four fixed roots — no plugin-exclusion filter is required or permitted as a substitute for root scoping.

#### Scenario: A plugin-shaped fixture outside the four roots is absent from the result

- GIVEN a fixture tree resembling a plugin skill location, located outside the four resolved roots
- WHEN the scanner runs
- THEN no `Component` in the result traces back to that fixture

### Requirement: Per-File Parsing Failures Do Not Abort The Scan

Each discovered `SKILL.md` MUST be parsed via the frontmatter reader (`frontmatter::read`). A file that the reader reports as a failure MUST produce a `ScanIssue` carrying that file's path, and MUST NOT prevent any other file in the same or a different root from being discovered and parsed.

#### Scenario: One corrupt SKILL.md yields an issue and does not stop the walk

- GIVEN a root containing one corrupt `SKILL.md` and two well-formed sibling `SKILL.md` files
- WHEN the scanner walks that root
- THEN one `ScanIssue` carrying the corrupt file's path is produced, and both sibling skills are still discovered as components

### Requirement: Scanner Performs No Writes

The scanner MUST perform no filesystem write of any kind — no file creation, no file modification, no directory creation — anywhere in its resolution or traversal logic.

#### Scenario: A full scan run leaves the fixture tree byte-for-byte unchanged

- GIVEN a fixture tree with a known state before a scan
- WHEN the scanner runs a full scan over it
- THEN the fixture tree's contents are unchanged afterward

### Requirement: Reference Fixture Set Produces Exactly 69 On-Disk Entries

Run over the fixture tree reproducing the reference installation, the scanner MUST produce exactly 69 on-disk skill entries, un-consolidated (duplicates across roots are each counted). Consolidating these into unique identities is explicitly out of scope for this capability.

#### Scenario: The reference fixture tree yields 69 entries

- GIVEN the committed fixture tree under `crates/vertice-core/tests/fixtures/roots/`
- WHEN the scanner performs a full scan across all three roots
- THEN the total count of produced components plus per-file parsing issues equals 69
