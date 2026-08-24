# OpenCode Agent Scanner Specification

## Purpose

Defines the contract for discovering OpenCode agent `Component` values from the `agent` object embedded in two configuration files, `opencode.json` and `opencode.jsonc`, merged per key. Traces to T6 of the completed PoC roadmap; closes the OpenCode half of CA-5 (an agent defined only in `opencode.jsonc` appears alongside `opencode.json`-only agents); contributes to CA-12 (a malformed file yields a `ScanIssue` carrying its path and interrupts nothing); bound by CA-16 (read-only) and CA-17 (fixture-based, machine-independent tests, no reuse of another scanner's fixture tree). Core (Rust) only — no IPC or frontend surface in this change; no `domain-model` requirement is added, modified, or exercised differently by this capability.

This capability differs from `agent-scanner` (T5, Claude Code) in cardinality: an OpenCode agent is an entry inside a JSON object, not a file on a walked directory tree. One resolved root produces N components from parsing and merging exactly two files, never from directory traversal.

## Requirements

### Requirement: OpenCode Agent Root Resolves Under The Home Directory To Two Config Files

The scanner MUST resolve exactly one `SearchRoot` with id `opencode-agents` and `kind: SearchRootKind::Agent`, reporting the canonical probed config file path. The two candidate config file paths, `<home>/.config/opencode/opencode.json` and `<home>/.config/opencode/opencode.jsonc`, MUST both be present in `ResolvedRoot.scan_paths`. Neither path MAY be derived from `XDG_CONFIG_HOME` or any other OS config-directory convention; both are computed from the home directory alone, hardcoded, mirroring the convention `skill-scanner` and `agent-scanner` already established. Which of the two files is designated canonical for `SearchRoot.path` is a design-phase decision; either choice satisfies this requirement as long as both paths appear in `scan_paths`.

#### Scenario: The OpenCode agent root resolves under the home directory on every OS

- GIVEN the scanner runs on any supported platform
- WHEN the OpenCode agent root is resolved
- THEN both `<home>/.config/opencode/opencode.json` and `<home>/.config/opencode/opencode.jsonc` are present in `scan_paths`, never a platform config-dir path

#### Scenario: The resolved root carries the fixed id and SearchRootKind::Agent

- GIVEN the resolved OpenCode agent root
- WHEN its `id` and `kind` fields are inspected
- THEN `id == SearchRootId("opencode-agents")` and `kind == SearchRootKind::Agent`

### Requirement: Root Status Is Found If Either Config File Exists

The resolved OpenCode agent root's `status` MUST be `SearchRootStatus::Found` if at least one of `opencode.json` or `opencode.jsonc` exists on disk, and `SearchRootStatus::NotFound` only if neither exists. A root reported as `NotFound` MUST remain distinguishable from a root reported as `Found` that yielded zero components, so that "looked in the wrong place" is never confusable with "found nothing".

#### Scenario: Only opencode.json exists

- GIVEN a fixture home where `opencode.json` exists and `opencode.jsonc` does not
- WHEN the scanner resolves the root
- THEN `status == Found`

#### Scenario: Only opencode.jsonc exists

- GIVEN a fixture home where `opencode.jsonc` exists and `opencode.json` does not
- WHEN the scanner resolves the root
- THEN `status == Found`

#### Scenario: Neither config file exists

- GIVEN a fixture home where neither `opencode.json` nor `opencode.jsonc` exists
- WHEN the scanner resolves the root
- THEN `status == NotFound`, and the root's reported `path` is still populated with the canonical probed path

#### Scenario: A NotFound root is distinguishable from a Found root with zero components

- GIVEN one fixture home where the config directory is entirely absent, and another where a healthy config file exists but its `agent` object is empty
- WHEN the scan result is inspected
- THEN the first root reports `status: NotFound` and the second reports `status: Found`, even though both produce zero components

### Requirement: JSON And JSONC Parsing Happen Per File Behind One Seam

`opencode.json` MUST be parsed as strict JSON. `opencode.jsonc` MUST be parsed as JSONC, tolerating `//` line comments, `/* */` block comments, and trailing commas. Both parsers MUST be invoked from a single named module-internal seam, mirroring how `crate::yaml` is the crate's only YAML entry point. No regular expression MAY be used to strip comments or otherwise pre-process either file before parsing.

#### Scenario: A JSONC file with comments and a trailing comma parses successfully

- GIVEN a fixture `opencode.jsonc` containing a `//` line comment, a `/* */` block comment, and a trailing comma in the `agent` object
- WHEN the file is parsed
- THEN parsing succeeds and every declared agent is extracted

#### Scenario: A strict JSON file rejects a trailing comma

- GIVEN a fixture `opencode.json` containing a trailing comma
- WHEN the file is parsed
- THEN parsing fails and is treated as malformed input by the malformed-file-isolation requirement below

### Requirement: An Agent Entry's Body Can Never Prevent The Agent From Being Reported

The presence of a key in the merged `agent` object is the sole detection rule. No property of that key's value — an unmodelled field, an unexpected type, or a missing field — MAY prevent the corresponding `Component` from being produced. `description` is the only field read; it is surfaced only when it is present and is a string, and MUST otherwise degrade to absent rather than discarding the entry. No field beyond `description` MAY be surfaced on the resulting `Component`.

This requirement is stated in terms of observable behavior, not mechanism: it does not mandate or forbid `serde` deserialization, a DTO, or value-level extraction. Any implementation satisfying the scenarios below conforms. (The mechanism is a design decision; see `design.md` §5.4.)

Rationale: OpenCode's real entries carry `mode`, `prompt`, `tools`, `hidden` and `permission`, and `tools` is an **object** where Claude Code's frontmatter `tools` is a scalar. Making an agent's existence depend on correctly typing a body Vertice does not display would couple inventory completeness to schema guesses about a client that is free to add fields.

#### Scenario: An agent entry carrying every observed real-world field produces a component

- GIVEN a fixture `agent` entry with `description`, `mode`, `prompt`, `tools` (an object of tool-name to boolean), `hidden`, and `permission`
- WHEN the scanner runs
- THEN a `Component` is produced whose data derives only from the entry's key and its `description`, and no other field is surfaced

#### Scenario: An agent entry with an empty body still produces a component

- GIVEN a fixture `agent` entry whose value is an empty object
- WHEN the scanner runs
- THEN a `Component` is produced for that key, carrying no description

#### Scenario: An unexpected type in the body degrades the field, never the component

- GIVEN a fixture `agent` entry whose `description` is a number, an object, or `null`, and whose `tools` is a string rather than an object
- WHEN the scanner runs
- THEN a `Component` is still produced for that key, carrying no description, and no `ScanIssue` claims the agent was skipped

#### Scenario: An unmodelled future field does not disturb the result

- GIVEN a fixture `agent` entry carrying a field name this capability does not model
- WHEN the scanner runs
- THEN a `Component` is produced for that key exactly as if the field were absent

### Requirement: hidden Is Never A Filtering Signal

An agent entry whose `hidden` field is `true` MUST still produce a `Component`. The scanner MUST NOT read `hidden`, or any other field, as a reason to exclude an entry from the result. `hidden` governs OpenCode's own agent picker UI, not whether the component is installed, and Vertice inventories what is installed.

#### Scenario: An agent entry with hidden: true still produces a component

- GIVEN a fixture `agent` entry whose value includes `"hidden": true`
- WHEN the scanner runs
- THEN a `Component` is produced for that entry, identical in shape to one produced from an entry with `hidden: false` or no `hidden` field at all

### Requirement: The agent Object Is Merged Per Key Across Both Files, Last-Wins Only On Conflicting Keys

The scanner MUST parse `opencode.json` and `opencode.jsonc` independently into their own `agent` objects, then merge those two objects per key: a key present in only one file's `agent` object MUST survive unchanged into the merged result, and a key present in both files MUST resolve by taking `opencode.jsonc`'s value for that key, applied at the level of the individual entry — not by discarding `opencode.json`'s entire object and replacing it with `opencode.jsonc`'s. The merge MUST NOT be implemented as parsing a concatenated or textually-merged document; each file MUST be parsed to a value first, and the merge MUST operate on the two already-parsed values.

#### Scenario: A key present in only opencode.json survives

- GIVEN `opencode.json` declares agent key `alpha` and `opencode.jsonc` does not declare it
- WHEN the scanner runs
- THEN the result contains one component derived from `alpha`, sourced from `opencode.json`

#### Scenario: A key present in only opencode.jsonc survives (CA-5)

- GIVEN `opencode.jsonc` declares agent key `beta` and `opencode.json` does not declare it
- WHEN the scanner runs
- THEN the result contains one component derived from `beta`, sourced from `opencode.jsonc`, alongside every component sourced from `opencode.json`

#### Scenario: A key present in both files with a partial override yields one component whose non-overridden field survives

- GIVEN `opencode.json` declares agent key `gamma` with `description: "from json"` and `opencode.jsonc` declares the same key `gamma` with a value that does not set `description`
- WHEN the scanner runs
- THEN the result contains exactly one component derived from `gamma` whose `description` is `"from json"` — the field `opencode.json` set and `opencode.jsonc` did not override

#### Scenario: A key present in both files with a conflicting field takes opencode.jsonc's value

- GIVEN `opencode.json` declares agent key `delta` with `description: "old"` and `opencode.jsonc` declares the same key `delta` with `description: "new"`
- WHEN the scanner runs
- THEN the result contains exactly one component derived from `delta` whose `description` is `"new"`

### Requirement: One File Produces N Components

Parsing a single config file's `agent` object MUST yield one `Component` per key present in that object, not at most one component per file. This inverts the cardinality that `skill-scanner` and `agent-scanner` established, where each discovered filesystem entry produces at most one component.

#### Scenario: A single config file with three agent keys yields three components

- GIVEN `opencode.json` declares three distinct keys in its `agent` object and `opencode.jsonc` is absent
- WHEN the scanner runs
- THEN the result contains exactly three components, one per key

### Requirement: Component Assembly For Every Merged Agent Key

Every key surviving the merge MUST be assembled into a `Component` with `kind: ComponentKind::Agent`, `scope: Scope::User`, and exactly one `Location` where `origin: LocationOrigin::File` and `root` set to the resolved OpenCode agent root's id. `Component.id` MUST be `ComponentId::derive(Agent, name)` where `name` is the merged object's key alone, never derived from which file the surviving value came from.

#### Scenario: A merged agent produces a correctly shaped Component

- GIVEN a fixture home where `opencode.json` declares one agent key
- WHEN the scanner runs
- THEN it produces one `Component` with `kind: Agent`, `scope: User`, one `Location { origin: File }`, and `id == ComponentId::derive(Agent, "<key>")`

#### Scenario: Component identity is independent of source file

- GIVEN the same agent key declared in both `opencode.json` and `opencode.jsonc` with different sub-fields
- WHEN the scanner runs
- THEN the resulting component's `id` is `ComponentId::derive(Agent, "<key>")`, identical to the id it would have if the key had been declared in only one of the two files

### Requirement: Malformed JSON In One File Isolates To That File

A file that fails to parse MUST produce exactly one `ScanIssue` at `IssueSeverity::Error` carrying that file's path, and MUST NOT prevent the other file from being parsed, merged, and having its agents emitted as components. Because parsing happens per file before the merge, a malformed file simply contributes no entries to the merge rather than aborting it.

#### Scenario: Malformed opencode.json does not block opencode.jsonc's agents

- GIVEN `opencode.json` is malformed and `opencode.jsonc` declares two well-formed agent keys
- WHEN the scanner runs
- THEN exactly one `ScanIssue` at `IssueSeverity::Error` carrying `opencode.json`'s path is produced, and both agents declared in `opencode.jsonc` are emitted as components

#### Scenario: Malformed opencode.jsonc does not block opencode.json's agents

- GIVEN `opencode.jsonc` is malformed and `opencode.json` declares two well-formed agent keys
- WHEN the scanner runs
- THEN exactly one `ScanIssue` at `IssueSeverity::Error` carrying `opencode.jsonc`'s path is produced, and both agents declared in `opencode.json` are emitted as components

### Requirement: Absent Files, Absent agent Key, And Empty agent Object Produce No Component And No Issue

Each of the following MUST produce zero components and zero `ScanIssue` values: a config file that does not exist on disk; a config file that exists, parses successfully, and carries no `agent` key; and a config file that exists, parses successfully, and carries an `agent` key whose value is an empty object. Absence is reported exclusively through the root's `status`, never as a `ScanIssue` — the discipline `skill-scanner` established for its own empty and absent roots (CA-9).

#### Scenario: A missing config file yields nothing, no issue

- GIVEN `opencode.json` does not exist and `opencode.jsonc` does not exist
- WHEN the scanner runs
- THEN zero components and zero `ScanIssue`s are produced

#### Scenario: A well-formed file with no agent key yields nothing, no issue

- GIVEN `opencode.json` parses successfully and has no top-level `agent` key
- WHEN the scanner runs
- THEN zero components and zero `ScanIssue`s reference that file

#### Scenario: A well-formed file with an empty agent object yields nothing, no issue

- GIVEN `opencode.json` parses successfully and its `agent` key is `{}`
- WHEN the scanner runs
- THEN zero components and zero `ScanIssue`s reference that file

### Requirement: Out-Of-Scope Top-Level Keys Produce No Component

A config file carrying top-level keys other than `agent` — including `mcp`, `permission`, `share`, and `$schema` — MUST NOT produce any component from those keys. Only the `agent` key is read for component extraction.

#### Scenario: An mcp key produces no component

- GIVEN `opencode.json` declares a top-level `mcp` key alongside a well-formed `agent` key
- WHEN the scanner runs
- THEN every produced component derives from `agent`, and no component or `ScanIssue` references the `mcp` key or its contents

### Requirement: Component And Issue Ordering Is Deterministic

The scanner MUST emit components and `ScanIssue`s in a deterministic order across repeated runs on the same input and across platforms. JSON object key iteration order MUST NOT be relied upon as the ordering source; the scanner MUST impose its own stable ordering (for example, sorting merged keys) before assembling the result.

#### Scenario: Two runs over the same fixture home produce identically ordered results

- GIVEN a fixture home with multiple agent keys across both config files
- WHEN the scanner runs twice over the same input
- THEN the two resulting component lists are in identical order

### Requirement: A Normalization Collision Between Two Agent Keys Is Reported, Not Silently Collapsed

If two distinct keys in the merged `agent` object normalize to the same `ComponentId` (per the crate's existing `(kind, normalized name)` identity rule), the scanner MUST NOT silently drop one of them. Both resulting components MUST be emitted, each carrying the `ComponentId` derived from its own key, consistent with `ComponentId::derive`'s existing collision behavior elsewhere in the crate. This capability introduces no new collision-resolution policy of its own.

#### Scenario: Two agent keys differing only in case both appear in the result

- GIVEN a fixture `agent` object declaring both `Reviewer` and `reviewer` as distinct keys
- WHEN the scanner runs
- THEN both resulting components are emitted, each with the `ComponentId` `ComponentId::derive`'s existing rule produces for its own key — neither is dropped by this capability

### Requirement: Scanner Performs No Writes

The scanner MUST perform no filesystem write of any kind — no file creation, no file modification, no directory creation — anywhere in its resolution, parsing, or merge logic.

#### Scenario: A full scan run leaves the fixture tree byte-for-byte unchanged

- GIVEN a fixture home with a known state before a scan
- WHEN the scanner runs a full scan over it
- THEN the fixture tree's contents are unchanged afterward

### Requirement: This Capability Introduces No Domain Model Change

This capability MUST NOT add, remove, or modify any field on any type under `crates/vertice-core/src/model/`, and MUST NOT require regeneration of any file under `frontend/src/bindings/`. Fields deserialized from an agent entry beyond `description` are consumed internally by this capability only; they MUST NOT be surfaced as new `Component` fields.

#### Scenario: Model and bindings are unchanged after this capability is implemented

- GIVEN the repository before and after this capability is implemented
- WHEN `crates/vertice-core/src/model/` and `frontend/src/bindings/` are compared
- THEN they are byte-identical, and the CI bindings-drift gate reports no diff

### Requirement: Every Case Is Traceable To A Repository Fixture In A New, Non-Reused Tree

Each requirement above MUST be exercised by a fixture committed under `crates/vertice-core/tests/fixtures/roots/opencode-agents/`, as synthetic home directories, a tree distinct from and never reused from `crates/vertice-core/tests/fixtures/roots/agents/` (T5, `agent-scanner`) or any `skill-scanner` fixture tree. At minimum, the fixture set MUST cover: an agent only in `.json`; an agent only in `.jsonc`; the same key in both with a partial-key override; the same key in both with a fully conflicting field; a `.jsonc` file with real `//` comments, a `/* */` comment, and a trailing comma; malformed `.json` with a healthy `.jsonc`; the symmetric malformed `.jsonc` with a healthy `.json`; both files absent; the `agent` key missing; the `agent` key present but empty; a non-`agent` top-level key (`mcp`); an agent entry carrying every observed real-world field including `hidden: true`; a normalization collision between two keys; and a reference fixture pinning the CA-5 assertion.

The `.jsonc`-only-agent case and the comments/trailing-comma case are not observable against the current reference machine's real OpenCode installation — that installation's `opencode.jsonc` carries no `agent` key and no comments — so these two cases are exercised only by committed fixtures, and the manual T16 oracle contrast against `opencode debug config` does not exercise the `.jsonc` agent path. A passing oracle check at T16 MUST NOT be mistaken for coverage of that path; the fixture tests are its only coverage.

#### Scenario: Fixture set covers every documented case

- GIVEN this spec's full list of requirements
- WHEN the `crates/vertice-core/tests/fixtures/roots/opencode-agents/` directory is enumerated
- THEN each requirement above has at least one fixture proving its behavior
