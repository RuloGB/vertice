# MCP Scanner Specification

## Purpose

Defines the contract for discovering MCP (Model Context Protocol) server
`Component` values configured in Claude Code, OpenCode and Codex —
user-level configuration only. This is an inventory-and-location capability:
which MCP servers are installed, and where each is configured. It is
explicitly **not** a credential audit, **not** a reproduction of launch
commands, and **not** a runtime probe — no MCP server is ever connected to,
launched, or introspected.

The central contract is that **redaction is structural**: no value from a
server's `env` map, `headers` map, `args` list, or a remote `url`'s
userinfo/query/fragment may ever reach `Component`, `Location`,
`ScanReport`, the IPC payload, or the application log. Parsing reuses the
existing `jsonc.rs` (Claude Code, OpenCode) and `toml.rs` (Codex) seams —
this capability introduces no new format seam and no new dependency.

## Requirements

### Requirement: Only Key Names Cross From env And headers, Never Values

For a stdio server, the adapter MUST capture the `env` map's keys as
`env_keys: Vec<String>` and MUST NOT bind any value from that map to a
variable, field, or log call that outlives the parse. For a remote server,
the same rule applies to `headers` as `header_keys: Vec<String>`. No test,
fixture assertion, or code path MAY read a captured value back from either
map.

#### Scenario: A fake token in env never reaches the serialized report

- GIVEN a stdio fixture whose `env` carries a realistic fake token, e.g. `GITHUB_TOKEN=ghp_FAKE1234567890`
- WHEN the scanner runs and the resulting `ScanReport` is serialized
- THEN `env_keys` contains `"GITHUB_TOKEN"` and the token value string appears nowhere in the serialized output

#### Scenario: A bearer header value never reaches the serialized report

- GIVEN a remote fixture whose `headers` carry `Authorization: Bearer sk-FAKE...`
- WHEN the scanner runs and the resulting `ScanReport` is serialized
- THEN `header_keys` contains `"Authorization"` and the bearer value appears nowhere in the serialized output

#### Scenario: A secret-bearing scan leaves no trace in the application log

- GIVEN a fixture home whose MCP configuration carries the fake token above
- WHEN a full scan runs and the report is logged
- THEN the token string appears nowhere in the application log file

### Requirement: A Remote URL Is Stripped Of Userinfo, Query, And Fragment

A captured `Remote.url` MUST NOT include userinfo, a query string, or a
fragment. Only scheme, host, port, and path MAY survive. If the sanitization
rule (closed in design) cannot safely process a given URL, the adapter MUST
omit the URL rather than emit it verbatim; it MUST NOT fall back to emitting
the original string.

#### Scenario: Userinfo and a credential query parameter are both stripped

- GIVEN a remote fixture whose `url` is `https://user:tok_FAKE@host/mcp?apiKey=tok_FAKE`
- WHEN the scanner runs
- THEN the emitted transport's `url`, if present, contains neither `user:tok_FAKE@` nor `apiKey=tok_FAKE`

#### Scenario: An unparseable URL is omitted, never emitted verbatim

- GIVEN a remote fixture whose `url` cannot be safely sanitized by the chosen rule
- WHEN the scanner runs
- THEN the location either carries no `url` or the entry degrades per the `ScanIssue` taxonomy below, and the original string is never emitted unmodified

#### Scenario: A URL whose authority boundary is ambiguous is rejected, never truncated

- GIVEN a remote fixture whose `url` places a userinfo-delimiting `@` after a `/`, `?`, or `#` that a naive authority cut would land on first, e.g. `https://tok_FAKE/@host.example.test/mcp`
- WHEN the scanner runs
- THEN no fragment of the userinfo (e.g. `tok_FAKE`) appears anywhere in the emitted transport, the serialized report, or the application log — the entry degrades per the `ScanIssue` taxonomy below rather than emitting a truncated authority

### Requirement: args Values Are Never Captured, Only Their Count

`McpTransport::Stdio` MUST carry `arg_count: usize`, never an `args` list of
values. No adapter code path MAY bind an individual argument string to any
field, log call, or `ScanIssue`.

#### Scenario: A stdio server with a token-bearing argument exposes only a count

- GIVEN a stdio fixture whose `args` includes `--token=ghp_FAKE...`
- WHEN the scanner runs and the resulting `ScanReport` is serialized
- THEN `arg_count` reflects the argument count and the token string appears nowhere in the serialized output

### Requirement: Transport Lives On Location, One Component Per Server Name

`Location.mcp_transport: Option<McpTransport>` MUST be populated for every
MCP location and `None` for every skill or agent location. A server name
configured in more than one in-scope client MUST consolidate into one
`Component` (per the existing `(kind, name)` identity rule) with N
`Location` entries, each retaining its own `McpTransport` — consolidation
MUST NOT discard or merge transport data across locations.

#### Scenario: One server name in three clients yields one component, three transports

- GIVEN a server named `github` configured in Claude Code, OpenCode, and Codex, each with a different `command`
- WHEN the scanner runs
- THEN the result contains exactly one `Component` with three `Location` entries, each carrying its own `McpTransport`

#### Scenario: A skill or agent location never carries a transport

- GIVEN any `Location` produced by the skill or agent adapters
- WHEN it is inspected
- THEN `mcp_transport` is `None`

### Requirement: Component Identity Is The Config Key, Unchanged Rule

`ComponentId` for an MCP component MUST derive from `ComponentId::derive(ComponentKind::Mcp, server_key)`, where `server_key` is the config object's server-name key alone — never from `Location`, file path, or client. No change to the identity function is introduced by this capability.

#### Scenario: Identity depends only on kind and server key

- GIVEN two MCP fixtures for the same server key under different clients
- WHEN each is scanned independently
- THEN both produce the same `ComponentId`

### Requirement: A Disabled Server Is Still Emitted, Undifferentiated

The scanner MUST NOT model or read an enabled/disabled flag as a filtering
signal. A server entry carrying any disabled indicator MUST still produce a
`Component`, identical in shape to one with no such indicator. This state
MUST NOT be encoded in `provenance_hint`, which MUST remain opaque per the
domain-model spec.

#### Scenario: A server marked disabled in its config is still emitted

- GIVEN a fixture entry carrying a disabled indicator recognized by its client's schema
- WHEN the scanner runs
- THEN a `Component` is produced for that entry, and `provenance_hint` carries no disabled-state indicator

### Requirement: Malformed Or Wrong-Typed Configuration Degrades, Never Aborts

An unreadable or malformed config file MUST produce exactly one `ScanIssue`
at `IssueSeverity::Error` carrying that file's path and MUST NOT prevent any
other client's adapter from producing its components (CA-12). A root key or
an individual entry that is present but wrong-typed MUST degrade to `None`
plus an `IssueSeverity::Warning`, never dropping the entry silently and
never aborting the scan.

#### Scenario: A malformed config in one client does not block the other two

- GIVEN one client's MCP config file is malformed while the other two are well-formed
- WHEN the scanner runs
- THEN exactly one `ScanIssue` at `Error` carries the malformed file's path, and both other clients' servers are still emitted

#### Scenario: A wrong-typed root key degrades with a warning

- GIVEN a config file whose MCP root key exists but is of the wrong type
- WHEN the scanner runs
- THEN zero MCP components are produced from that file, one `ScanIssue` at `Warning` is produced, and the scan is not aborted

#### Scenario: An unusable command falls back to a valid URL instead of degrading to None

- GIVEN an entry whose `command` is wrong-typed, empty, or otherwise unusable, and whose `url` is present and valid
- WHEN the scanner runs
- THEN the entry's `mcp_transport` is `Some(Remote { .. })`, built from the valid `url`, exactly one `ScanIssue` at `Warning` is produced, and the entry is not degraded to `None`

### Requirement: An Absent Or Empty MCP Root Produces No Component And No Error

A client's MCP configuration root that does not exist on disk, and one that
exists but declares no MCP servers, MUST each produce zero MCP components
and zero `ScanIssue` values — reported only through the existing `NotFound`
root-warning behavior (CA-11), never as silence and never as an error.

#### Scenario: A home with no MCP configuration at all yields nothing

- GIVEN a fixture home with none of the three clients' MCP configuration present
- WHEN the scanner runs
- THEN zero MCP components are produced, zero `ScanIssue`s reference MCP, and the existing `NotFound` root warning behavior applies

### Requirement: Scope Is User-Level Only

Every MCP `Component` MUST be constructed with `scope: Scope::User`. No
project-level, local-level, or plugin-provided MCP configuration source is
read by this capability.

#### Scenario: An MCP component is always User-scoped

- GIVEN any MCP fixture in any in-scope client
- WHEN the scanner runs
- THEN every produced `Component` has `scope: Scope::User`

### Requirement: No MCP Component Ever Appears In A FreshnessCheck

This capability introduces no version-freshness support for MCP servers.
`FreshnessSubject` MUST gain no MCP variant, and no MCP `Component` MUST
ever be included as a subject in any `FreshnessCheck`.

#### Scenario: A full scan and freshness evaluation exclude MCP entirely

- GIVEN a fixture home with MCP servers configured and freshness evaluation enabled
- WHEN a scan and its freshness evaluation both run
- THEN no `FreshnessCheck` entry references any MCP component

### Requirement: Only jsonc.rs And toml.rs Parse MCP Configuration

Every MCP adapter MUST read its client's configuration exclusively through
`jsonc.rs` (Claude Code, OpenCode) or `toml.rs` (Codex). No adapter MAY
import a JSON or TOML parsing crate directly, and no adapter MAY use a
regular expression to parse or pre-process MCP configuration.

#### Scenario: No MCP adapter imports a parsing crate directly

- GIVEN the source of every MCP adapter module
- WHEN it is inspected for imports of a JSON or TOML parsing crate
- THEN none is found; only `jsonc.rs` or `toml.rs` is used

### Requirement: Every Case Is Traceable To A Repository Fixture In A New Tree

Each requirement above MUST be exercised by fixtures committed under new,
per-client trees in `crates/vertice-core/tests/fixtures/`, distinct from
every existing skill/agent fixture tree, and containing fake-but-realistic
secret values (never real credentials). At minimum: a stdio fixture with a
fake token in `env`; a remote fixture with a bearer header; a remote fixture
with userinfo and a credential query parameter; a stdio fixture with a
token-bearing `args` entry; one server name across all three clients; a
malformed config per client format; a wrong-typed root key and a
wrong-typed entry; an absent/empty MCP root per client; a disabled-flagged
entry; and a reference-fixture pin proving MCP scanning holds CA-15/CA-16.
The exact per-client config path, format, and root key used to build these
fixtures are closed in `design.md`, not assumed here.

#### Scenario: Fixture set covers every documented case

- GIVEN this spec's full list of requirements
- WHEN the new MCP fixture trees are enumerated
- THEN each requirement above has at least one fixture proving its behavior
