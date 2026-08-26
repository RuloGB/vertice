# Delta for Domain Model

`ComponentKind` gains a third variant, `Mcp`. `SearchRootKind` gains the
mirroring `Mcp` variant, restoring the documented 1:1 rationale for three
kinds. A new closed type, `McpTransport`, joins the public model surface,
and `Location` gains one new optional, kind-conditional field,
`mcp_transport: Option<McpTransport>`, widening `Location`'s responsibility
beyond "where is the file" — documented on the type itself. `provenance_hint`'s
opacity requirement is re-affirmed for the new kind, not relaxed.
`FreshnessSubject` is unchanged (see `mcp-scanner`'s requirement that no MCP
component ever appears in a `FreshnessCheck`).

## ADDED Requirements

### Requirement: McpTransport Is A Closed, Value-Free Enum

`McpTransport` MUST be a closed enum (no `#[non_exhaustive]`) with exactly
two variants: `Stdio { command: String, arg_count: usize, env_keys: Vec<String> }`
and `Remote { url: String, header_keys: Vec<String> }`. Neither variant MAY
declare a field capable of holding an individual argument value, an `env`
value, or a `headers` value — the type itself MUST make that redaction
structurally unreachable, not conventional.

#### Scenario: McpTransport has no field capable of holding a secret value

- GIVEN the `McpTransport` type definition
- WHEN its fields are inspected
- THEN no field is a value-carrying map or a per-argument string; only key names, a command string, a URL, and a count are representable

#### Scenario: McpTransport is exhaustively matchable

- GIVEN a `match` over every `McpTransport` variant with no wildcard arm
- WHEN the code is compiled
- THEN compilation succeeds, proving the enum is closed

### Requirement: SearchRootKind Mirrors ComponentKind 1:1, Now For Three Kinds

`SearchRootKind` MUST admit exactly one variant per `ComponentKind` variant,
preserving the documented rationale that clients organize search roots per
component kind. Adding `ComponentKind::Mcp` MUST be accompanied by adding
`SearchRootKind::Mcp` in the same change; the mirror MUST NOT be left
broken, and an MCP root MUST NOT be labeled `Agent` or any other existing
variant.

#### Scenario: SearchRootKind has exactly as many variants as ComponentKind

- GIVEN `ComponentKind` and `SearchRootKind`
- WHEN their variant sets are compared
- THEN both admit exactly three variants, one per kind, with matching names

#### Scenario: An MCP search root carries SearchRootKind::Mcp, not Agent

- GIVEN a `SearchRoot` resolved for an MCP configuration source
- WHEN its `kind` field is inspected
- THEN it is `SearchRootKind::Mcp`, never `SearchRootKind::Agent`

### Requirement: Location Carries An Optional, Kind-Conditional Transport

`Location` MUST gain `mcp_transport: Option<McpTransport>`. It MUST be
`None` for every `Location` produced by a skill or agent adapter. It MUST be
`Some(_)` for every `Location` produced by an MCP adapter from an entry the
adapter fully understood. This is the only placement for connection detail;
`Component` MUST NOT gain any `command`/`args`/`env`/`url`/`headers` field,
and no sibling payload keyed by `ComponentId` MUST be introduced for this
purpose.

**Degraded entries are the one exception, and it is deliberate.** An MCP
entry that the adapter could not fully understand — wrong-typed, matching
neither the stdio nor the remote shape, or carrying a URL that the
sanitization rule refuses — MUST still yield a `Location`, with
`mcp_transport: None` and an accompanying `IssueSeverity::Warning`. `None`
on an MCP location therefore means "this server is configured here, but its
connection detail could not be captured safely", never "this is not an MCP
location". Dropping the entry instead would hide a configured server from an
inventory, and emitting a partially-understood transport would risk emitting
an unredacted value; reporting the location without the detail is the only
option that does neither.

Consumers MUST NOT infer a location's kind from `mcp_transport`. The kind is
carried by `Component::kind` and by `SearchRoot::kind`, which stay
authoritative.

#### Scenario: A skill or agent location carries no transport

- GIVEN any `Location` produced by the skill or agent adapters
- WHEN it is inspected
- THEN `mcp_transport` is `None`

#### Scenario: An understood MCP entry carries its own transport

- GIVEN a `Location` produced by an MCP adapter from an entry it fully understood
- WHEN it is inspected
- THEN `mcp_transport` is `Some(_)`, populated with that location's own `McpTransport`

#### Scenario: A wrong-typed MCP entry still yields a location, without a transport

- GIVEN an MCP config entry whose value is wrong-typed, or matches neither the stdio nor the remote shape
- WHEN the adapter processes it
- THEN a `Location` is still produced for that entry, with `mcp_transport` set to `None`
- AND exactly one `ScanIssue` at `IssueSeverity::Warning` accompanies it
- AND the entry is not dropped from the report

#### Scenario: A URL the sanitization rule refuses degrades the same way

- GIVEN a remote MCP entry whose `url` cannot be safely reduced to scheme, host and port
- WHEN the adapter processes it
- THEN `mcp_transport` is `None` and a `Warning` is emitted
- AND neither the original URL nor any part of it appears anywhere in the report or the log

## MODIFIED Requirements

### Requirement: ComponentKind Is a Closed Enumeration

`ComponentKind` MUST be a closed enum (no `#[non_exhaustive]`) admitting
exactly the variants `Skill`, `Agent`, and `Mcp`.
(Previously: admitted exactly `Skill` and `Agent`.)

#### Scenario: ComponentKind is exhaustively matchable

- GIVEN a `match` over every `ComponentKind` variant with no wildcard arm
- WHEN the code is compiled
- THEN compilation succeeds

#### Scenario: Mcp is a fully representable kind

- GIVEN a `Component` constructed with `kind: ComponentKind::Mcp`
- WHEN it is inspected
- THEN it is a valid, representable value, structurally identical in shape to one carrying `Skill` or `Agent`

### Requirement: provenance_hint Is Opaque, Not a Discriminator

`Component.provenance_hint` MUST be typed as `Option<String>`. Consumers
MUST NOT branch on its value to drive behavior; any machine-readable
classification of a location's origin MUST live on `Location.origin`
instead. No MCP-specific state — including a server's enabled/disabled
indicator — MUST ever be encoded in `provenance_hint`; that opacity
requirement applies to every `ComponentKind`, including `Mcp`.

The absence of a provenance hint MUST be represented as `None`, never as an
empty `String`. An empty string is a sentinel value, and the model already
rejects sentinels elsewhere (`Location.path` uses `Option` for the same
reason).
(Previously: identical text, without the explicit MCP/enabled-state
restatement, because `ComponentKind::Mcp` did not exist.)

#### Scenario: provenance_hint is an optional plain string, not an enum

- GIVEN the `Component.provenance_hint` field definition
- WHEN its type is inspected
- THEN it is `Option<String>`, not an enum or tagged union, signaling it is display-only

#### Scenario: a component with no provenance hint is representable without a sentinel

- GIVEN a `Component` whose adapter reported no provenance information
- WHEN it is constructed with `provenance_hint: None`
- THEN it is distinguishable from a component carrying `Some("")` and from one carrying `Some("claude-code")`

#### Scenario: An MCP server's disabled state is never encoded in provenance_hint

- GIVEN an MCP `Component` built from a config entry carrying a disabled indicator
- WHEN `provenance_hint` is inspected
- THEN it contains no representation of that disabled state

### Requirement: Rust Types Generate a Matching TypeScript Contract

All core types — `Component`, `ComponentKind`, `Scope`, `Location`,
`SearchRoot`, `SearchRootKind`, `McpTransport`, `ClientInstallation`,
`ClientPresence`, `ClientPresenceStatus`, `ScanIssue`, `ScanReport`,
`ClientKind`, `Freshness`, `FreshnessSubject`, `FreshnessCheck`,
`FreshnessReport`, `ClientInstallSlot`, and `FreshnessSettings` — and their
nested enums MUST derive `Serialize`, `Deserialize`, and `TS`. Generated
TypeScript bindings MUST be checked into `frontend/src/bindings/` and MUST
structurally mirror the Rust definitions: field names, optionality, and
closed union variants. Every new or modified `.ts` binding, including the
new `McpTransport.ts`, `ComponentKind.ts`'s third variant, `SearchRootKind.ts`'s
third variant, and `Location.ts`'s new `mcpTransport` field, MUST land in
the same commit as its Rust type change; CI's drift gate MUST fail if it is
not regenerated.
(Previously: did not list `SearchRootKind` or `McpTransport`, and
`ComponentKind`/`Location` had no third-kind/`mcp_transport` obligation,
because `ComponentKind::Mcp` did not exist.)

#### Scenario: Struct exports a TypeScript binding

- GIVEN `Component` derives `TS` and its export test runs
- WHEN the generated `.ts` file is inspected
- THEN it declares a type with the same field names as the Rust struct

#### Scenario: Optional path crosses as a nullable string

- GIVEN `Location.path: Option<PathBuf>`
- WHEN its TypeScript binding is generated
- THEN the field's type is `string | null`

#### Scenario: ClientKind's binding reflects three variants

- GIVEN `ClientKind` gains the `Codex` variant and its export test runs
- WHEN `frontend/src/bindings/ClientKind.ts` is inspected
- THEN it declares the union `"claudeCode" | "openCode" | "codex"`, and the CI bindings-drift gate reports no diff between the committed file and a freshly regenerated one

#### Scenario: ComponentKind's binding reflects three variants including mcp

- GIVEN `ComponentKind` gains the `Mcp` variant and its export test runs
- WHEN `frontend/src/bindings/ComponentKind.ts` is inspected
- THEN it declares the union `"skill" | "agent" | "mcp"`

#### Scenario: SearchRootKind's binding mirrors ComponentKind's three variants

- GIVEN `SearchRootKind` gains the `Mcp` variant and its export test runs
- WHEN `frontend/src/bindings/SearchRootKind.ts` is inspected
- THEN it declares a union with exactly three variants, one per `ComponentKind` variant

#### Scenario: McpTransport exports a closed union binding with no value-carrying field

- GIVEN `McpTransport` derives `TS` and its export test runs
- WHEN `frontend/src/bindings/McpTransport.ts` is inspected
- THEN it declares a closed union mirroring `Stdio { command, argCount, envKeys }` and `Remote { url, headerKeys }`, with no field shaped to hold an argument or map value

#### Scenario: Location's binding gains the optional mcpTransport field

- GIVEN `Location` gains `mcp_transport: Option<McpTransport>`
- WHEN `frontend/src/bindings/Location.ts` is regenerated
- THEN it declares `mcpTransport: McpTransport | null` alongside its existing fields

#### Scenario: A forgotten binding fails the CI drift gate

- GIVEN a new or modified public model type from this change is committed without its regenerated `.ts` binding
- WHEN CI's bindings-drift check runs
- THEN it fails, using `git add --intent-to-add` so a brand-new file is also caught
