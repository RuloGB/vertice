# Delta for Workspace Architecture

The seam inventory's size is unchanged: no new parser-owning module is
added. `jsonc.rs` and `toml.rs`'s sole-importer containment now covers a
third class of consumer — MCP adapters — alongside the existing skill/agent
adapters. This capability adds no new dependency and requires no `deny.toml`
change.

## ADDED Requirements

### Requirement: MCP Adapters Reuse Existing Seams, Introducing No New Dependency

MCP adapters MUST parse configuration exclusively through the existing
`jsonc.rs` (Claude Code, OpenCode) or `toml.rs` (Codex) seams. No MCP
adapter MAY import a JSON or TOML parsing crate directly, and this
capability MUST NOT add a new dependency to `Cargo.toml`. `cargo deny check
bans licenses` MUST continue to pass with `deny.toml` byte-identical to its
pre-change state.

#### Scenario: The sole-importer containment tests stay green with MCP adapters present

- GIVEN the source of every module under `crates/vertice-core/src/` after MCP adapters are added
- WHEN `tests/yaml_seam_invariant.rs`-style containment checks run for `jsonc.rs` and `toml.rs`
- THEN no module other than the seam itself imports the underlying parsing crate, including every new MCP adapter module

#### Scenario: No new dependency is introduced

- GIVEN `Cargo.toml`, `Cargo.lock`, and `deny.toml` after this change
- WHEN they are compared to their pre-change state
- THEN they are byte-identical, and `cargo deny check bans licenses` passes

### Requirement: No Secret-Bearing Value May Cross The Core's Public Surface

No value read from an MCP server's `env` map, `headers` map, `args` list, or
a remote URL's userinfo/query/fragment MUST ever cross `vertice-core`'s
public surface — `Component`, `Location`, `ScanReport`, or any value
returned by a `pub` function. Redaction MUST happen inside the adapter
layer, before a value is ever bound to a variable that outlives the parse;
it MUST NOT be enforced at the model layer or at the IPC boundary as an
afterthought.

#### Scenario: No public core function can return a raw secret value

- GIVEN every `pub` function in `vertice-core` reachable from an MCP fixture carrying a fake secret
- WHEN their return values are inspected after a scan
- THEN the fake secret value appears in none of them; only key names, a command string, a stripped URL, and counts are present
