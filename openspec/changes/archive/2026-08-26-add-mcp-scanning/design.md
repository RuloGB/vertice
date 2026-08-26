# Design: Add MCP Server Scanning (backend)

> Spec trace: closes the proposal's "Committed to resolving in `sdd-design`" list (`proposal.md:288-298`). Bound by the four spec files in `specs/` — `mcp-scanner`, `domain-model`, `scan-orchestration`, `workspace-architecture` — which are approved and are **not** modified here. This document writes **no spec and no task**.
> Inherits, unchanged: `archive/2026-08-23-add-codex-client-support/design.md` (**CXD**) §5.1-§5.3 (the `toml_seam` alias and the seam's read-only public surface), §8 (permissive DTOs — unmodelled keys are ignored, never an error), §10.2 (no symlink in any fixture, ever), §10.3 (every fixture directory carries at least one file); `archive/opencode-agent-adapter` (**OAD**) §5.3/§6.2 (components from config object keys; ordered deep merge, last-wins at the leaf), §5.6 (no `escalate` function — every `ScanIssue` is built where the context is in hand).
> Invariants this design may not weaken: **CA-16** (read-only), **CA-17** (versioned fixtures only), core purity (no `tauri`), the format-seam sole-importer rule, `model/`'s import allow-list, and the proposal's central commitment that **redaction is structural**.

## 0. What is verified, what is corroborated, and what is still an assumption

This section closes — or explicitly fails to close — the proposal's decision 7 (`proposal.md:77-86`), the item that gates every fixture.

**Methodological statement, recorded because it changes how the rest of this document must be read.** This phase had no network access, so **no upstream schema could be fetched**. What closed the gap instead was a **shape-only inspection of the real machine, performed on 2026-08-25 under explicit user authorization** (§0.5): key names and value *types* only, every scalar rendered as `str(len=N)` / `number` / `bool`. **No value was read, printed, or transcribed, and none appears anywhere in this document.** That is a weaker oracle than an upstream schema — it proves what one machine has, not what the format permits — and it is labelled as such throughout. Nothing below is promoted to fact without a citation, and the four rows it could not settle remain assumptions with their blast radius stated.

### 0.1 Verified from this repository

| # | Statement | Basis |
|---|---|---|
| **V1** | `jsonc::parse` returns a seam-owned `JsonValue` whose `Object` is a `BTreeMap<String, JsonValue>`, so **object key order is sorted by construction and duplicate keys are already resolved to the last occurrence by the parser** | `jsonc.rs:19-34` |
| **V2** | The TOML seam exposes **only** `from_str::<T: DeserializeOwned>` and one error type. There is **no** `TomlValue` tree — the JSONC adapters' value-level walk (`opencode_agents.rs:276-286`) has no TOML analogue | `toml.rs:12-22` |
| **V3** | `roots::probe` returns `NotFound` only for `ErrorKind::NotFound`; any other outcome is `Found`. The `NotFound` **Warning** is emitted once per distinct root id by the orchestrator, never by an adapter | `roots.rs:219-225`, `scan.rs:61-72` |
| **V4** | `opencode_agent_root` is the alias-grouping precedent: two files under **one** root id, `SearchRoot.path` = the merge base, `scan_paths` = base then overlay **in merge order**, `status: Found` if either exists | `roots.rs:173-198`, pinned by `roots.rs:253-268` |
| **V5** | `consolidate::ROOT_ORDER` has **8** entries and is pinned to `skill_roots ++ agent_roots ++ opencode_agent_root ++ codex_agent_root`, in that concatenation order, by a test that builds the expectation from `roots.rs` itself | `consolidate.rs:21-30`, `consolidate.rs:189-206` |
| **V6** | `location_key` is `(root_rank, root_id, path)`; `merge_into` concatenates locations with `extend` and never deduplicates; the final component sort is `(name, id)` | `consolidate.rs:48-54`, `consolidate.rs:104`, `consolidate.rs:132-136` |
| **V7** | `ComponentKind` is matched exhaustively in exactly **one** place in `src/`: `identity_prefix` (`identity.rs:41-47`). Every adapter *constructs* `Component` rather than matching on kind, so the blast radius of a third variant is one `match` arm plus the bindings | `identity.rs:41-47`; grep of `ComponentKind::` across `crates/` |
| **V8** | `Location` is constructed as a struct literal at **6** sites in `src/` and **7** in `tests/` (all in `tests/model_contract.rs` — `tests/agent_scanner.rs`, `tests/codex_agent_scanner.rs` and `tests/opencode_agent_scanner.rs` reference `Location` fields but never construct one). A new field breaks every one of them at compile time — loud, never silent | **Correction found by adversarial review, 2026-08-25.** A first pass used a naive `grep -rn "Location {" crates/vertice-core` and reported 8+7=15. Read by hand, two of those eight `src/` matches are not construction sites: `model/location.rs:14` is the type's own `pub struct Location {` **definition**, and `consolidate.rs:150` is a test-helper **function signature** (`fn location(...) -> Location {`) matched only because of the `-> Location {` return-type brace — the real literal it precedes is `consolidate.rs:151`, already counted separately. The true construction sites: `agents.rs` ×2 (187, 210), `codex_agents.rs` ×1 (180), `consolidate.rs` ×1 (151), `opencode_agents.rs` ×1 (194), `skills.rs` ×1 (134) in `src/`; `model_contract.rs` ×7 in `tests/`. Total **13** |
| **V9** | `scan_for`'s `complete` fixture asserts `roots_scanned.len() == 8`, `components.len() == 12`, and **`report.issues.is_empty()`**; `missing-root-client` asserts `roots_scanned.len() == 8` and exactly **8** path-less `Warning`s | `scan.rs:98-108`, `scan.rs:138-150` |
| **V10** | `reference-volume` asserts only `duration_ms < 2000`, `!components.is_empty()`, and tree-snapshot equality — it does **not** pin an issue count | `scan.rs:213-223` |
| **V11** | `tests/fixtures/roots/reference/` is reached exclusively by `skills::scan`; no MCP root resolves inside it, so the 69/25/22/3 pins are structurally untouched by this change | `skill_scanner.rs` fixture wiring; MCP roots resolve to `.claude.json` / `.config/opencode/` / `.codex/`, none of which `skills::scan` reads |

### 0.2 Corroborated by committed fixtures (stronger than web-sourced, weaker than an upstream schema)

| # | Statement | Basis |
|---|---|---|
| **C1** | OpenCode's user config is `~/.config/opencode/opencode.json` with a `~/.config/opencode/opencode.jsonc` overlay, in that merge order | `roots.rs:173-198` — shipped and pinned by a previous cycle |
| **C2** | OpenCode's MCP root key is **`mcp`**, and an entry is an object keyed by server name | `tests/fixtures/roots/opencode-agents/reference/.config/opencode/opencode.json:3-5`; `.../no-agent-key/.config/opencode/opencode.json:3` |
| **C3** | An OpenCode entry carries `type: "local"` and — **decisively different from every other client** — `command` as a **JSON array**, not a string plus a separate `args` list: `{ "type": "local", "command": ["echo"] }` | Same two fixtures, committed during `opencode-agent-adapter` alongside a `$schema` reference to `https://opencode.ai/config.json` |

C2/C3 were the only per-client MCP schema facts this repository could corroborate before §0.3's inspection. C3 matters disproportionately, because it alone invalidates a single shared "`command` + `args`" mapping across the three clients (§6.3) — and §0.3 **M9** now confirms it independently, on a real machine, from a different source than the committed fixture.

### 0.3 Verified by shape inspection of the real machine, 2026-08-25

**Basis for every row: a shape-only inspection performed under explicit user authorization** (§0.5) — key names and value types only (`str(len=N)` / `number` / `bool`), **no value read**. This is a *one-machine* oracle: it proves the shape a real installation has, not the full space the format permits. It is therefore strong evidence for what the adapters must handle and weak evidence about what they may *assume absent*, which is why every adapter stays permissive (CXD §8) and every unmodelled key is ignored rather than rejected.

| # | Statement | Corrects |
|---|---|---|
| **M1** | **Claude Code has TWO user-level MCP sources, not one**: `~/.claude.json` **and** `~/.claude/settings.json`, each carrying a top-level **`mcpServers`** object | **This design's §5.1, which specified a single-file `claude-mcp` root. Corrected in §5.1/§5.2.** A1/A2 closed |
| **M2** | A `~/.claude.json` entry has shape `{ type: str, command: str, args: [str…] }`; a `~/.claude/settings.json` entry has shape `{ command: str, args: [str…] }` — **no `type` key at all**. `type` is therefore **optional**, and stdio must be the inferred default when it is absent | A3 closed, and it **forces structural transport discrimination** (§6.3) |
| **M3** | `~/.claude.json` also holds `projects.<path>` entries whose keys include **`disabledMcpjsonServers`** / **`enabledMcpjsonServers`**. Claude Code's enable/disable state therefore lives **outside the server entry and is project-scoped** | Proposal decision 6 and the user-scope-only rule are now **evidence-backed**: this state is out of scope because it is project data, not because it was hard to model. `projects.*` is never read |
| **M4** | `~/.claude.json` is **51 KB** on this machine | §9.3's multi-megabyte performance risk is **downgraded to Low likelihood**, not removed |
| **M5** | Codex's root table is **`mcp_servers`** (snake_case); `mcpServers` is confirmed **absent**. `command` is a **string**, `args` an **array of strings**, and an **empty `args` array was observed** — so `arg_count == 0` is a real case, not a hypothetical. `env` is a nested table of string → string | A7 closed |
| **M6** | **Codex HAS a remote transport.** One entry has shape `{ url: str }` **and nothing else** — no `command`, and **no `type` discriminator anywhere in the Codex schema** | **A8 closed affirmatively** — the row this design flagged as least certain. It also forces structural discrimination for Codex (§6.3) |
| **M7** | Codex entries also carry **`enabled: bool`** and `startup_timeout_sec: number` | A9 closed for Codex. Read-never stands (proposal decision 6); the flag's *existence* is now fact, so the disabled-server fixture uses a real shape (§10.4) |
| **M8** | **Both** `~/.config/opencode/opencode.json` **and** `~/.config/opencode/opencode.jsonc` exist on this machine and **both carry a top-level `mcp` key** | C1/C2 promoted from repo-corroborated to observed. The two-file merge root is evidence-backed |
| **M9** | An OpenCode entry's `command` is an **array of strings with no separate `args` key at all**; `type` is observed with lengths 5 and 6, consistent with `"local"`/`"remote"`; remote entries carry **`url`**; entries carry **`enabled: bool`** | C3 confirmed independently. A5 closed for `url` |

### 0.4 Residual assumptions — four rows, all with the same benign failure mode

| # | Assumption | Why still open | Blast radius if wrong |
|---|---|---|---|
| **A4** | OpenCode's stdio environment map key is **`environment`** (not `env`) | This machine's OpenCode config contains **no environment key of either spelling**, so local evidence cannot settle it | One string constant. An absent key yields **no keys and no issue** (§7), never an error. It cannot leak a value either way |
| **A5′** | OpenCode's remote header map key is **`headers`** | No remote entry on this machine carried headers | Same: absent ⇒ empty `header_keys`, no issue |
| **A8′** | Codex's remote header map key is **`http_headers`** | M6's remote entry carried **`url` and nothing else** | Same |
| **A10** | For a server declared in **both** Claude Code files, `~/.claude/settings.json` wins at the leaf (§5.2) | Claude Code's own precedence between the two files could not be observed — it needs a server declared in both with *different* values, which this machine does not have | **Cosmetic only.** It changes which `command` string is displayed for a server configured twice, differently, by the same user. Both locations are emitted either way (§5.2) |

**Consequence, stated plainly.** All four residual rows are *map-key-name* or *display-precedence* questions. **None can fail in a way that emits a secret**, because redaction is enforced by the shape of what is extracted (§3, §6.2), not by correctly guessing a schema; and none can fail in a way that hides a server, because an absent map key is not an error and the entry is emitted regardless. The failure mode of every one of them is **an empty key list plus a correct component** — which is exactly the degradation §7 already specifies.

### 0.5 The authorization, recorded

The inspection in §0.3 was performed **only** after explicit user authorization, and **only** in shape-only form: names and types, never values. This design and every artifact derived from it therefore contain **no credential, no value, and no verbatim line from any real configuration file**. The rule that produced that outcome is retained as a standing constraint in §10.5: **no fixture may be authored from, or diffed against, a real configuration file** — fixtures are synthetic, and their secrets are `FAKE` by construction (§10.2).

### 0.6 A conflict between two approved spec files, which this design must resolve

`specs/domain-model/spec.md:57-64` requires `Location.mcp_transport` to "be `Some(_)` for every `Location` produced by an MCP adapter".
`specs/mcp-scanner/spec.md:126-133` requires a present-but-wrong-typed **entry** to "degrade to `None` plus an `IssueSeverity::Warning`, never dropping the entry silently".

For an entry whose transport cannot be determined, these cannot both hold unless the adapter either invents a sentinel transport (`Remote { url: "" }`) or drops the entry. The model rejects sentinels by policy (`component.rs:28-31`, `location.rs:8-10`), and dropping the entry is the under-reporting failure this feature exists to prevent (proposal decision 6).

**Resolution: the specific rule wins.** A degraded MCP entry yields a `Component` with a `Location` carrying **`mcp_transport: None`** plus one `Warning`. `domain-model`'s requirement is read as its evident intent — *skills and agents never carry a transport, MCP locations do* — and needed a carve-out for the degraded case.

**RESOLVED, 2026-08-25.** The carve-out was written into `specs/domain-model/spec.md` (requirement "Location Carries An Optional, Kind-Conditional Transport"), and it matches this resolution: `Some(_)` is required only for an entry the adapter **fully understood**; a wrong-typed entry, an entry matching neither shape, and a URL the sanitization rule refuses all yield a `Location` with `mcp_transport: None` plus a `Warning`, and are never dropped. The spec additionally records that `None` on an MCP location means "configured here, detail not safely capturable", never "not an MCP location", and forbids consumers from inferring kind from the field. **This item no longer blocks.** Every other reading was rejected: `Remote { url: String::new() }` is a sentinel in the one type whose entire purpose is that a bad value is unrepresentable; a location-less `Component` answers "which server" while discarding "where", i.e. half the product goal (proposal decision 9).

## 1. Technical approach

Four additive slices over one closed enum, one new value type, one new field, and three new roots. Nothing existing is redesigned; exactly one existing function pair is **moved** (§4.3).

```
                                   vertice-core                    (no tauri; NO new dependency)
 frontend ──IPC──> vertice-app ──> ├── model/component   + ComponentKind::Mcp          (§2)
 future vertice-cli ──────────>    ├── model/location    + SearchRootKind::Mcp
                                   │                     + Location.mcp_transport      (§2)
                                   ├── model/mcp     NEW  McpTransport (value-free)    (§2)
                                   ├── model/identity + one match arm ("mcp")          (V7)
                                   ├── roots         + claude-mcp, opencode-mcp,
                                   │                   codex-mcp                       (§5)
                                   ├── jsonc / toml  UNCHANGED — reused, not extended
                                   ├── json_merge    MOVED out of opencode_agents      (§4.3)
                                   ├── mcp           NEW  sanitize_url, KeyNames,
                                   │                      ArgCount, McpScan  ← the ONLY
                                   │                      redaction primitives (§3, §4)
                                   ├── mcp_claude    NEW  jsonc,  1 file               (§6.3)
                                   ├── mcp_opencode  NEW  jsonc,  2 files, deep merge  (§6.3)
                                   ├── mcp_codex     NEW  toml,   1 file               (§6.3)
                                   ├── consolidate   ROOT_ORDER 8 -> 11; NO logic change (§8)
                                   └── scan          + three adapters in the concatenation

 per adapter, three strictly ordered phases — the shape that makes redaction reviewable:
   read    : path -> String -> seam -> parsed document      (values exist here, and ONLY here)
   redact  : parsed entry -> RedactedEntry                  (the single choke point per client)
   assemble: RedactedEntry -> Component + Location + issues (no raw value is in scope)
```

**CLI isolation is unchanged.** Every new entry point takes `home: &Path` and reads no environment; `roots::home_dir` (`roots.rs:30-32`) remains the crate's sole ambient read. **`Cargo.toml`, `Cargo.lock` and `deny.toml` are expected byte-identical** — the two format seams already in the crate cover both formats, and every redaction primitive is built from `std` and `serde`, which are already dependencies.

## 2. Core data model changes

| Type | Change |
|---|---|
| `ComponentKind` | **`Mcp` variant added** (`component.rs:40-43`). The enum's own doc (`component.rs:34-36`) anticipates exactly this reviewed breaking change. `identity_prefix` gains `ComponentKind::Mcp => "mcp"` (`identity.rs:41-47`) — the only exhaustive match in `src/` (V7). The doc comment on `Component` (`component.rs:9-12`) updates from "a skill or an agent" to name three kinds |
| `SearchRootKind` | **`Mcp` variant added** (`location.rs:72-75`), restoring the documented 1:1 mirror (`location.rs:66-68`) |
| `Location` | **`mcp_transport: Option<McpTransport>` added**, breaking all 13 struct-literal sites at compile time (V8) |
| `McpTransport` | **New file `model/mcp.rs`**, re-exported from `model/mod.rs:35-46` |
| `FreshnessSubject`, `ComponentId`, `Scope`, `SearchRoot`, `ScanIssue`, `IssueSeverity`, `ScanReport`, `ClientKind`, `ClientInstallSlot` | **Unchanged.** A diff in any of their bindings means something leaked |

```rust
/// Connection detail for one MCP `Location`. Closed, never
/// `#[non_exhaustive]`, and deliberately INCAPABLE of holding a secret:
/// there is no field for an `env` value, a `headers` value, or an
/// individual argument. Redaction is therefore a property of this type,
/// not a rule an adapter author can forget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum McpTransport {
    Stdio {
        /// The executable only. Never an argument, never an env value.
        command: String,
        /// How many arguments were configured. A count cannot carry a value.
        arg_count: usize,
        /// Key NAMES from the environment map, in the map's own sorted
        /// order. Never a value.
        env_keys: Vec<String>,
    },
    Remote {
        /// The sanitized ORIGIN of the configured endpoint —
        /// `scheme://host[:port]`. Userinfo, query, fragment AND path are
        /// removed before construction (§3.2); this field is never built
        /// from a raw configured string.
        url: String,
        /// Key NAMES from the headers map. Never a value.
        header_keys: Vec<String>,
    },
}
```

`model/mcp.rs` imports nothing outside the module's allow-list (`model/mod.rs:8-15`): `serde`, `ts_rs`. No `std::fs`, no `std::io`, no clock. **All sanitization and key extraction happen in the adapter layer** (`src/mcp.rs`), never here — the model is the *shape* of the guarantee, `src/mcp.rs` is its *enforcement*.

Consequences stated rather than inherited:

- **`Location`'s documented responsibility widens.** Its doc comment (`location.rs:8-10`) must say so explicitly: it now answers "where is this" *and*, for one kind only, "how is it reached". The alternative placements were rejected in the proposal (`proposal.md:37-43`); this design does not reopen them.
- **`arg_count` is `usize`, and `Vec<String>` key lists preserve the parsed map's order**, which is sorted-by-construction for JSONC (V1) and for the TOML `BTreeMap` DTO (§6.3) — determinism is a property of the container, not of a sort call someone might omit.
- **No client field anywhere.** `SearchRoot` is untouched (`location.rs:43-51`); the client remains recoverable by convention from the root id, logged as P2 in `internal-docs/pendientes-desarrollo.md:49-76`.

**Accepted limitation, recorded explicitly so no reviewer mistakes it for a structural guarantee.** `Stdio.command` is captured verbatim into a `String`. Unlike `env_keys`, `header_keys`, `arg_count`, and the sanitized `url`, nothing about `command`'s *type* prevents an adapter author from accidentally routing a value into it — its safety rests on the convention that `command` is "the executable path", which every observed shape agrees on (M2, M5, M9), not on any structural constraint the way the rest of `McpTransport` is built to be value-free. This is a deliberate, proportionate trade — the executable path is display-relevant and not a secret by any observed client convention — but it is a convention-based residual, not a structural one, and it is named as such here rather than left to be assumed.

## 3. Decision — the URL sanitization rule

Closes the proposal's second open item (`proposal.md:291`).

### 3.1 The rule

A pure, I/O-free, dependency-free, **regex-free** function in `src/mcp.rs`:

```rust
/// Reduce a configured remote URL to the endpoint origin, or refuse.
/// Purely subtractive: every character in the output is copied verbatim
/// from `raw`. Nothing is normalized, lowercased, decoded or invented.
pub(crate) fn sanitize_url(raw: &str) -> Option<String>;
```

**Correction found by adversarial review, 2026-08-25.** The original four-step
version of this rule cut the authority at the first `/`, `?` or `#` and only
*afterward* looked for `@`. For `https://tok3n/@host.example/mcp`, that cuts
at the `/` **inside** the userinfo before the `@` is ever seen, leaving
`authority = "tok3n"` — no `@`, so the userinfo step is a no-op — and
`"tok3n"` then passes the host check unchanged, because it is non-empty and
contains none of `@ / \ ? # [ ]`. The function returned
`Some("https://tok3n")`: a verbatim fragment of the credential, into
`McpTransport::Remote.url` and from there into `ScanReport`, IPC and the log
file. This is not contrived — base64 tokens use `/` in their standard
alphabet, so a Basic-Auth-style credential URL hits it routinely, and it is
exactly the leak class this whole design exists to prevent.

**The invariant the rule must hold, stated explicitly:** no input may produce
an output containing any byte that came from the userinfo component; when
the authority's structure is ambiguous, the function MUST reject rather than
guess. The steps below are total under that invariant — every input is
either accepted with a userinfo-free authority, or rejected outright, never
truncated into something that merely *looks* clean.

1. Reject if `raw` is empty or contains any ASCII control character or any whitespace.
2. Split once on `"://"`. Reject if absent. The scheme must be non-empty, must start with an ASCII letter, and may contain only ASCII alphanumerics, `+`, `-`, `.`; otherwise reject.
3. Take the **candidate authority** as everything up to the first `/`, `?` or `#`; call everything from that delimiter onward the **tail**. (Naively taking "the last `@` in the whole remainder" instead of bounding the search to the candidate authority is rejected: it mis-parses `https://host/path@foo` into host `foo`.)
4. **Reject if the tail contains any `@`.** An `@` in the discarded tail means the true authority boundary is ambiguous — the cut in step 3 may have landed inside userinfo, exactly as in the failing case above. The rule never guesses in this situation; it refuses.
5. **Userinfo:** if the candidate authority contains `@`, keep only what follows the **last** `@` within it. (Reached only once step 4 has confirmed the tail is `@`-free, so this is now unambiguous.)
6. **Host and port:** if what remains starts with `[`, it must contain `]`, the bracketed body must be non-empty and consist only of hex digits, `:` and `.` (IPv6), and what follows `]` must be either empty or `:` plus at least one ASCII digit and nothing else. Otherwise, split at the **last** `:`, if any: the port must be non-empty and all ASCII digits, and the host must be non-empty and contain none of `@ / \ ? # [ ]`. Reject on any violation.
7. Emit `scheme://host[:port]`.

### 3.2 Two decisions inside that rule

**The port is PRESERVED.** `http://localhost:3000` and `http://localhost:8080` are different servers and rendering them identically would make the inventory wrong. A port is a routing datum, not a credential surface — no known configuration convention hides a secret in it. Rejected: dropping it (collapses distinct local servers into indistinguishable rows) and normalizing the default port away (an invented value; the rule stays subtractive).

**Accepted limitation, cosmetic only.** The rule is purely subtractive (§3.1's doc comment) and does not lowercase the scheme or the host, so `HTTPS://Host` and `https://host` yield different output strings even though they name the same endpoint. This is deliberate, not an oversight: lowercasing would be a normalization, and normalization is exactly what the rule promises not to do ("nothing is normalized, lowercased, decoded or invented"). The consequence is display duplication in an edge case no observed client config exercises, never a redaction gap.

**The PATH is DROPPED — a deliberate deviation from the proposal's prose.** `proposal.md:35` says "What survives is enough to identify the endpoint (scheme, host, port, path)". `specs/mcp-scanner/spec.md:49-55` is the normative text and says only that path **MAY** survive, so dropping it is spec-compliant. It is dropped because **the token-in-path pattern is real and common** in hosted MCP aggregators (`https://<host>/api/mcp/s/<TOKEN>/mcp`), and a rule that strips userinfo and query while leaving a path-borne token is precisely the "works usually" redaction the proposal already rejected once, for `args`, in decision 8 (`proposal.md:93-96`). The trade is asymmetric: leaking writes a live credential to a log file on disk; dropping costs one line of display detail that the **server name** and **`Location.path`** already cover. Consistency with decision 8 decides it.

| Input | Output |
|---|---|
| `https://mcp.example.test/mcp` | `https://mcp.example.test` |
| `http://localhost:3000/sse` | `http://localhost:3000` |
| `https://u:tok@mcp.example.test:8443/mcp/tok?apiKey=tok#f` | `https://mcp.example.test:8443` |
| `https://[::1]:8080/mcp` | `https://[::1]:8080` |
| `mcp.example.test/mcp` (no scheme) | **`None`** → §7 degradation |
| `https:// mcp.example.test` (whitespace) | **`None`** |
| `https://@/x` (empty host) | **`None`** |
| `https://tok3n/@host.example/mcp` (`/` inside userinfo — the verified leak) | **`None`** — tail `/@host.example/mcp` contains `@`; ambiguous, rejected |
| `https://tok3n?x@host.example/mcp` (`?` inside userinfo) | **`None`** — tail `?x@host.example/mcp` contains `@`; ambiguous, rejected |
| `https://tok3n#x@host.example/mcp` (`#` inside userinfo) | **`None`** — tail `#x@host.example/mcp` contains `@`; ambiguous, rejected |
| `https://host/path@foo` (naive "last `@` in the remainder" would mis-parse this to host `foo`) | **`None`** — tail `/path@foo` contains `@`; ambiguous, rejected |
| `https://tok3n%40host.example/mcp` (percent-encoded `%40` in place of a literal `@`) | `https://tok3n%40host.example` — passes through unmodified. **Not a leak**: the rule is byte-level and never decodes percent-encoding (step 6's host check rejects only the literal bytes `@ / \ ? # [ ]`, and `%` is none of them), and real clients split the authority on the literal `@` byte, so a `%40` never functions as a userinfo delimiter — there is no userinfo here to strip |

### 3.3 What happens when the rule refuses

`None` means **the entry is still emitted, with `mcp_transport: None` and one `Warning`** (§0.6, §7). The raw string is **never** emitted, never logged, and — critically — **never interpolated into the `ScanIssue.reason`** (§7.2). Rejected: emitting `Remote { url: String::new() }` (a sentinel, §0.6) and skipping the entry (under-reporting).

Rejected alternatives to the whole rule: **a URL-parsing crate** (`url`) — forbidden by the proposal and by `specs/workspace-architecture/spec.md:11-18`, and it would import a public-suffix/IDN surface for a job that is three `split` calls; **a regular expression** — forbidden by `AGENTS.md`, and it would be strictly harder to audit than an explicit character-class walk; **strip only what looks like a credential** — the heuristic class the proposal already ruled out.

## 4. Decision — module layout

Closes the proposal's third open item (`proposal.md:293`).

### 4.1 Three adapters, plus one shared redaction module

| Option | Consequence | Decision |
|---|---|---|
| One `mcp.rs` with three per-client functions | ~450 lines and three unrelated schemas in one file; the house shape is one module per adapter (`agents.rs`, `opencode_agents.rs`, `codex_agents.rs`), and it makes per-client rollback a surgical edit rather than a file deletion | **Rejected** |
| Three fully standalone modules, each with its own sanitizer and key extraction | Faithful to `agents.rs:8-11`'s "deliberately separate, not a shared abstraction" — but it puts **three copies of the security-critical code** in the tree. The whole premise of this feature is that redaction must be impossible to get wrong; three implementations is three chances | **Rejected** |
| **`src/mcp.rs` (shared redaction primitives + `McpScan`) + `mcp_claude.rs` + `mcp_opencode.rs` + `mcp_codex.rs`** | Each client keeps its own module, its own schema, its own fixtures and its own revert. The primitives that must never diverge — `sanitize_url`, `KeyNames`, `ArgCount` — exist **once**, in one small, heavily unit-tested, I/O-free file that is the single thing a security reviewer must read | **Chosen** |

The precedent argument (`agents.rs:8-11`) is honoured where it applies and set aside where it does not: it argues against extracting a **shared traversal abstraction over adapters that differ**, which this design does not do. It says nothing about sharing a pure, value-level function. `frontmatter.rs`, `jsonc.rs` and `identity.rs` are all already shared by every adapter for exactly this reason.

`src/mcp.rs` contains **no I/O whatsoever** — no `std::fs`, no path probing — so every one of its behaviours is unit-testable without a fixture, exactly like `split_release_dir_name` (CXD §3.2).

### 4.2 The shared result type

```rust
/// Owned result of one client's MCP scan. A distinct type per the house
/// rule (OAD §5.5) — but ONE type shared by the three MCP adapters, since
/// all three produce exactly one root, N components and N issues.
#[derive(Debug, Clone, PartialEq)]
pub struct McpScan {
    /// Always exactly one root.
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}
```

Sharing one result type across the three MCP adapters — where `SkillScan`, `AgentScan`, `OpenCodeAgentScan` and `CodexAgentScan` are four distinct types — is justified because those four describe **four different discovery shapes**, while these three describe **one shape applied to three files**. If a fourth client ever needs a different shape, it gets its own type; nothing here forces the reuse.

### 4.3 One move: `merge_all` / `merge_two` leave `opencode_agents.rs`

The OpenCode MCP root reads the same two files in the same merge order as the OpenCode agent root (V4/C1), so it needs the same ordered deep merge, last-wins-at-the-leaf, that `opencode_agents.rs:220-252` already implements and pins with **ten** unit tests: `base_only_key_survives`, `overlay_only_key_survives`, `shared_key_partial_override_merges_per_field_not_per_object`, `array_vs_anything_overlay_replaces_wholesale`, `scalar_vs_object_overlay_replaces`, `object_vs_scalar_overlay_replaces`, `overlay_null_replaces_and_does_not_delete`, `fold_over_zero_inputs_yields_nothing`, `fold_over_one_input_yields_identity`, `keys_differing_only_by_case_are_not_normalized_before_merging`. A shallow "last file wins per server key" would lose the base entry's `command` when the overlay overrides only one sibling field — a real bug the agent adapter already avoided.

| Option | Consequence | Decision |
|---|---|---|
| Duplicate ~30 lines into `mcp_opencode.rs` | Two implementations of a merge whose semantics the OpenCode capability already pinned; they will drift | **Rejected** |
| `pub(crate)` the two functions in place and call `crate::opencode_agents::merge_all` from the MCP adapter | A two-word diff, but it states a false dependency: the MCP adapter would import the *agent* adapter. Future readers will infer a relationship that does not exist | **Rejected** |
| Move them into `jsonc.rs` | The seam's contract is *parse*, nothing else. `specs/workspace-architecture/spec.md:1-8` keeps the seam inventory fixed; widening a seam's job to keep a file count down is the wrong trade | **Rejected** |
| **Move both functions and their ten unit tests verbatim into a new `pub(crate)` module `src/json_merge.rs`; `opencode_agents.rs` calls it** | A mechanical move with **zero behaviour change**, proven by the existing `opencode-agents` fixture suite staying byte-identical and green. Names the thing for what it is: a `JsonValue` deep merge, owned by neither consumer | **Chosen** |

This is a **delta against the proposal's Affected Areas table** (`proposal.md:227-249`), which did not anticipate a new module here, and it is recorded as such rather than slipped in. It is explicitly **not** the refactor the proposal ruled out (`proposal.md:208`, "any refactor unifying the existing agent adapters behind a shared trait") — no trait is introduced and no adapter changes shape.

## 5. Decision — roots, ids, `scan_paths` grouping, and `ROOT_ORDER`

Closes the proposal's fourth open item (`proposal.md:294`).

### 5.1 Three roots, three ids

**Corrected by M1.** This design originally specified `claude-mcp` as a single-file root. The machine inspection found `mcpServers` in **two** user-level Claude Code files, so `claude-mcp` becomes a **two-path root with a stated merge order** — the same alias-grouping shape as `opencode-mcp`, and for the same reason. This is the one root-layout decision the verification overturned, and it is recorded rather than quietly amended.

| Root id | `SearchRoot.path` (the merge base) | `scan_paths`, in merge order | Grouping rationale |
|---|---|---|---|
| `claude-mcp` | `<home>/.claude.json` | `[<home>/.claude.json, <home>/.claude/settings.json]` (**M1**) | **Two user-level files under one logical root** — V4's alias precedent, applied a second time. `status: Found` if **either** exists. Merge order and its rationale in §5.2 |
| `opencode-mcp` | `<home>/.config/opencode/opencode.json` | `[opencode.json, opencode.jsonc]` (**M8**) | The same shape, with the merge order this repository already shipped and pinned for the agent root (V4, `roots.rs:189-197`). The MCP root and the agent root read the same two files under two different ids — correct, because they are two different `SearchRootKind`s over the same bytes |
| `codex-mcp` | `<home>/.codex/config.toml` | `[<home>/.codex/config.toml]` (**M5**) | One file, `resolve_single` shape (`roots.rs:116-133`), like `codex_agent_root` (`roots.rs:205-212`) |

All three carry `kind: SearchRootKind::Mcp`. All three are `home` plus hardcoded relative segments pushed one at a time — no `dirs`/`directories`, no environment read, no platform branch (dotfiles are dotfiles on all three OSes; CXD §9.2).

`resolve_single` takes `suffix: [&str; 2]` (`roots.rs:116`), which fits neither `<home>/.claude.json` (one segment) nor a two-path root. **Decision: generalize `resolve_single` to `suffix: &[&str]`** — one signature change, four existing call sites updated mechanically, no behavioural change — and build the two multi-path roots from a small `resolve_pair(home, id, kind, base, overlay)` helper that is `resolve_opencode`/`opencode_agent_root`'s status fold (`roots.rs:150-153`, `roots.rs:184-187`) named once instead of written a third and fourth time. Rejected: a second near-identical single-segment resolver, and copying the status fold per root.

**On the id names.** The house convention is `{client}-{plural component kind}`: `claude-skills`, `codex-agents`. `mcp` does not pluralize into the kind's own name — `claude-mcp-servers` names *servers*, while `SearchRootKind::Mcp` names the kind — so the convention cannot be followed literally. `claude-mcp` reads as "the Claude MCP root", stays short in `ROOT_ORDER`, in every `Location.root`, and in the frontend's root grouping. **This is a close call and is recorded as one**; `claude-mcp-servers` was rejected only for verbosity and for naming a different noun than the kind.

### 5.2 Multi-file roots: merge order, and what a `Location` then means

Two of the three roots now read two files, so this needs a rule rather than an accident. **Both multi-file roots use the ordered deep merge, last-wins-at-the-leaf, that `json_merge` already implements** (§4.3) — the semantics the OpenCode agent capability shipped and pinned.

| Root | Base | Overlay (wins at the leaf) | Basis |
|---|---|---|---|
| `claude-mcp` | `~/.claude.json` | `~/.claude/settings.json` | **A10 — unverified, cosmetic blast radius.** `~/.claude.json` is the machine-written store (its entries carry `type`, M2) and is therefore the base and the displayed `SearchRoot.path`; `~/.claude/settings.json` is the hand-authored settings surface (its entries omit `type`, M2), and deliberate hand-authored intent is given the last word |
| `opencode-mcp` | `opencode.json` | `opencode.jsonc` | **Shipped and pinned** for the agent root (V4, `roots.rs:253-268`). Diverging from it for the MCP root would mean one client, two files, two orders |

**The consequence this forces, which is a genuine decision and not a detail.** When one server is declared in **both** files of one root, the adapter emits **one `Location` per declaring file** (mirroring `opencode_agents.rs:191-199`), and **both carry the same, merged, *effective* transport** — not each file's fragment.

| Option | Consequence | Decision |
|---|---|---|
| Each location carries only what **its own file** declares | An overlay that overrides one field would produce a location whose transport is a fragment — no `command`, hence a degraded `None` + `Warning` — for a server that is perfectly well configured. Vertice would report a fault the client does not have | **Rejected** |
| Merge, and emit **one** location, at the last declaring file | Answers "which server" but discards half of "where it is configured" — the product goal for this cycle (proposal decision 9) | **Rejected** |
| **Merge, and emit one location per declaring file, all sharing the effective transport** | Reports what the client will actually use, and reports every file that declares it. Exactly `opencode_agents.rs`'s existing treatment of `description`, applied to a new field | **Chosen** |

Stated so it is auditable: **within one client, transport is a property of the client's resolved configuration; `Location` answers which files declare the server.** Across clients, transports genuinely differ per location — which is the case `specs/mcp-scanner/spec.md:81-94` legislates, and it is unaffected.

### 5.3 `ROOT_ORDER`: 8 → 11, genuinely appended this time

```rust
const ROOT_ORDER: [&str; 11] = [
    "claude-skills", "agents-skills", "opencode-skills", "codex-skills", // skill_roots
    "claude-agents", "claude-embedded-agents",                            // agent_roots
    "opencode-agents", "codex-agents",                                    // single agent roots
    "claude-mcp", "opencode-mcp", "codex-mcp",                            // MCP roots (new)
];
```

The pinning test (`consolidate.rs:189-206`) gains three `expected.push(...)` lines after the `codex_agent_root` push, and nothing else. **It must be updated in the same commit as `roots.rs`, never after.**

Unlike the Codex cycle — whose "appended last" claim CXD §0 had to correct, because `codex-skills` landed at index 3 inside `skill_roots` — these three ids **are** genuinely last, because they are pushed after every existing family in the same concatenation the test builds (V5). Field precedence for every existing skill and agent is therefore unchanged both positionally and by the stronger argument CXD §6.2 gives: `ComponentId` embeds `ComponentKind`, so an `Mcp` component's locations can only ever come from MCP roots, and no rank is ever compared across kinds.

Precedence **within** the MCP family is `claude-mcp < opencode-mcp < codex-mcp`. Since `description` and `provenance_hint` are always `None` for MCP components (§6.4) and `scope` is always `User`, `merge_into` (`consolidate.rs:91-105`) has **no field left to race over** — the only thing precedence decides for MCP is the `name` casing that survives when two clients spell the key differently (`GitHub` vs `github`), and the location display order. Recorded so it is auditable: **Claude Code's spelling wins.**

## 6. Decision — the adapters

### 6.1 The three-phase shape, and why it is a shape

```
read     read_to_string  ->  jsonc::parse | toml::from_str   ->  document
extract  document        ->  the MCP root object/table       ->  entries
redact   entry           ->  RedactedEntry                   ← THE choke point
assemble RedactedEntry   ->  Component + Location + issues
```

`RedactedEntry` is a private per-client struct holding only `Option<McpTransport>` and the issues that producing it raised. **Nothing downstream of `redact` can see a raw value, because nothing downstream is handed one.** The parsed document is a local, dropped before the adapter returns.

**An honest asymmetry between the two formats, recorded rather than glossed.** On the TOML path, redaction is enforced by the *deserializer*: `KeyNames` and `ArgCount` (§6.2) never allocate a value at all. On the JSONC path it cannot be, because `jsonc::parse` returns a fully materialized tree (V1, `jsonc.rs:79-96`) — the values exist in memory before the adapter sees them. There, the guarantee is (a) the three-phase shape above, which bounds the values' visibility to one small function; (b) the fact that `McpTransport` has nowhere to put them (§2); and (c) the `FAKE` invariant test (§10.2), which is mechanical and covers every present and future case. Extending `jsonc.rs` with a streaming or key-only API was rejected: it would change the seam's contract for one consumer, and (b)+(c) already make a leak non-shippable.

**This asymmetry is about value *materialization*, not about the diagnostic-path hazard §7.2 closes.** Both seams' parser-error `Display` can embed source-text fragments — `toml`'s error quotes the offending source line, and `jsonc_parser`'s `ParseError` does the same at least once, in `ParseStringErrorKind::InvalidUnicodeEscapeSequence(String)`, which renders as `"Invalid unicode escape sequence. '{value}' is not a valid UTF8 character"` where `value` is a fragment captured from inside a malformed `\u` escape in a JSON string. **Verified against the pinned dependency**: `jsonc-parser 0.33.1` per `Cargo.lock` is the version this claim is checked against, and its `Display` for that variant does embed the source-derived fragment — this carries the same evidentiary weight as the `V`-prefixed file:line claims in §0.1, not an unchecked assertion. A malformed escape adjacent to or inside a secret string in `~/.claude.json` or `opencode.json` hits this exactly as a malformed line in `~/.codex/config.toml` hits `toml`'s. §7.2's blanket no-interpolation rule is what closes this for **both** seams — it is not a TOML-specific patch for a TOML-specific hazard.

### 6.2 The two TOML-side redaction primitives

`toml.rs` offers only `from_str::<T: DeserializeOwned>` (V2), so the Codex adapter must express redaction in its DTO. Two types in `src/mcp.rs`:

```rust
/// Deserializes ANY map, keeping only its key NAMES. Every value is
/// consumed with `serde::de::IgnoredAny`, so no value is ever allocated,
/// bound, or formatted. There is no constructor that accepts a value.
pub(crate) struct KeyNames(Vec<String>);

/// Deserializes ANY sequence, keeping only its LENGTH. Every element is
/// consumed with `IgnoredAny`. A count cannot carry a value.
pub(crate) struct ArgCount(usize);
```

Both are implemented with a hand-written `Deserialize` visitor over `serde`, already a dependency. This is the strongest form of the proposal's central claim: on the Codex path, **an argument or env value is not merely unused — it is never constructed.**

**Per-field leniency.** `serde` is all-or-nothing per document: one wrong-typed field would fail the whole `from_str` and turn a per-entry `Warning` into a file-level `Error`, contradicting `specs/mcp-scanner/spec.md:126-133`. TOML is self-describing, so the fix is a third primitive:

```rust
/// A field that degrades instead of failing the document. Implemented via
/// a visitor that accepts every TOML type and records a mismatch, so a
/// single wrong-typed field can never escalate to a file-level Error.
pub(crate) enum Lenient<T> { Value(T), WrongType }
```

Rejected: adding a `TomlValue` tree to `toml.rs` mirroring `JsonValue` (changes a seam's public surface, duplicates the JSONC value tree, and is a far larger diff than three visitors); accepting the escalation (contradicts the spec, and makes one malformed `startup_timeout_sec` hide every MCP server the user has).

### 6.3 Per-client extraction

| | Claude Code (**M1, M2**) | OpenCode (**M8, M9**; A4, A5′) | Codex (**M5, M6**; A8′) |
|---|---|---|---|
| Seam | `jsonc::parse` | `jsonc::parse` | `toml::from_str` |
| Files | **2**, deep-merged in `scan_paths` order (§5.2) | **2**, deep-merged in `scan_paths` order | 1 |
| Root key | `mcpServers` | `mcp` | `mcp_servers` (snake_case; `mcpServers` confirmed absent) |
| `type` field | **optional** — present in `~/.claude.json`, absent in `settings.json` (M2) | present, `"local"`/`"remote"` (M9) | **does not exist** (M6) |
| Transport discriminator | **structural** — see below | **structural** | **structural** |
| Env map key | `env` | `environment` (**A4 — the one open row**) | `env`, a nested table (M5) |
| Headers map key | `headers` | `headers` (**A5′**) | `http_headers` (**A8′**) |

#### Transport is discriminated STRUCTURALLY, in all three clients. `type` is never read.

M2 and M6 force this, and it is a simplification, not a workaround: Claude Code's `type` is **optional** (absent in every `~/.claude/settings.json` entry), and Codex has **no `type` field at all** — its remote entry is `{ url }` and nothing else. Any design that read `type` would need the structural fallback anyway for two of the three clients, leaving the *fallback* as the least-tested path in the most security-sensitive adapter.

**The rule is a total function over one 3 × 3 matrix, enumerated here ONCE and nowhere else.**
It is stated on **usability**, never on mere key presence.

- `command` is **usable** when it is present and is neither wrong-typed, nor an empty string, nor an empty array. Otherwise it is **unusable**.
- `url` is **valid** when it is present and survives §3's sanitization. Otherwise it is **unsanitizable**.

| | `url` absent | `url` valid | `url` unsanitizable |
|---|---|---|---|
| **`command` absent** | `None`, **1** Warning: *declares neither a command nor a URL* | `Remote`, **0** Warnings | `None`, **1** Warning: *URL could not be reduced to a safe endpoint* |
| **`command` usable** | `Stdio`, **0** Warnings | `Stdio`, **1** Warning: *declares both; the command was used* | `Stdio`, **0** Warnings — see the note below |
| **`command` unusable** | `None`, **1** Warning: *has no readable command* | `Remote`, **1** Warning: *no readable command; the URL was used instead* | `None`, **1** Warning: *has no readable command* |

**Every other statement of this rule in this document is derived from this table and MUST NOT
restate it independently.** §7.1's taxonomy table lists the same nine outcomes as reason
strings; if the two ever disagree, this matrix wins and §7.1 is the defect. `sdd-tasks` pins
one unit test per cell, named `matrix_command_{absent|usable|unusable}_url_{absent|valid|unsanitizable}`.

**The `command` usable + `url` unsanitizable cell, resolved explicitly (2026-08-25).** It emits
`Stdio` with **no** Warning. The usable command was selected on its own merits and nothing was
lost; the entry's surplus URL was never needed, so refusing it is not a degradation worth
reporting. Emitting the "URL could not be reduced" Warning here would report a fault the
inventory does not have. Note this is the one cell where an unsanitizable URL is silent — it is
silent precisely because it was never a transport candidate.

**Correction found by adversarial review, 2026-08-25 (round 3).** The previous prose form of this
rule left this cell matching three different rows across §6.3 and §7.1 with three mutually
exclusive outcomes. Two consecutive rounds of patching individual rows each produced a new
uncovered cell — first `command` unusable + `url` valid, then its mirror. The matrix above
replaces row-patching with a single total enumeration, which is why it is stated once and
referenced everywhere else.

**Correction found by adversarial review, 2026-08-25.** An earlier version of this rule discriminated on key **existence** ("`command` present → `Stdio`. Else `url` present → `Remote`. Both present → `Stdio`, plus a `Warning`."), and §7.1 independently degraded any wrong-typed or empty `command` to `None`. For an entry with a wrong-typed `command` **and** a valid `url`, those two rules contradicted each other — the existence rule picked `Stdio` from an unusable command, while §7.1's usability rule discarded the whole entry to `None`, silently throwing away a perfectly capturable `Remote` endpoint. **Resolution: discrimination is on usability throughout, never on mere presence**, and an unusable `command` with a valid `url` falls back to `Remote` rather than degrading the entry — it loses no information the entry actually offers, and discarding a usable endpoint in favor of an unusable one would contradict decision 6's under-reporting concern for no benefit. Pinned by `*/entry-unusable-command-valid-url` (§10.1, §10.4) and its RED test (§12).

Rejected: reading `type` — three unrelated vocabularies (`stdio`/`http`/`sse`, `local`/`remote`, and nothing at all), one of them optional, to answer a question the entry's own shape already answers unambiguously. It would also make Vertice brittle against an upstream `type` value it has not heard of (`"streamable-http"`), turning a well-formed server into a degraded row for no reason. **`type` joins the never-read list below.**

#### `command` and `args`: the asymmetry, in one place so it cannot be missed

**Two different shapes across the three clients** (M2, M5, M9). This table is normative.

| Client | Declared shape | → `McpTransport::Stdio.command` | → `arg_count` |
|---|---|---|---|
| **Claude Code** | `command: str` **plus** `args: [str…]` | `command` verbatim | `args.len()`; `args` absent ⇒ **0** |
| **Codex** | `command: str` **plus** `args: [str…]` | `command` verbatim | `args.len()`; `args` absent ⇒ **0**. An **empty array was observed** (M5), so `0` is a real, fixture-pinned case |
| **OpenCode** | `command: [str…]` — **one array, and no `args` key exists at all** (M9) | `command[0]` | `command.len().saturating_sub(1)` |

Edge rules, specified rather than left implicit:

- **OpenCode `command: []` (empty array)** ⇒ **degraded**: `mcp_transport: None` + `Warning`. There is no executable, so there is no stdio transport to report. `saturating_sub` keeps the arithmetic total, but the empty case never reaches it.
- **OpenCode `command[0]` not a string** ⇒ degraded, same treatment.
- **OpenCode elements past index 0** are **counted, never inspected** — including non-string elements. A count cannot carry a value, so their type is irrelevant to redaction and irrelevant to the count.
- **Claude Code / Codex `command: ""`** (empty string) ⇒ degraded. An empty command is the sentinel the model rejects everywhere else.
- **Claude Code / Codex `args` present but not an array** ⇒ transport still `Some`, `arg_count: 0`, plus a `Warning` (§7).
- **Claude Code / Codex `args` element present but not a string** (e.g. `args: ["--flag", 42]`) ⇒ **counted, never inspected** — the same rule as OpenCode's array-command tail above. `arg_count` is `args.len()` unconditionally; a non-string element does not degrade the entry and does not raise a `Warning`, because a count cannot carry a value regardless of what type the discarded element had. Pinned by `codex/args-non-string-element` (§10.1, §10.4) — distinct from `codex/entry-field-wrong-type`, which pins a different case (a wrong-typed *scalar* field degrading via `Lenient`, not a mixed-type `args` array).
- **An absent env or headers map is not a failure**: no keys, **no issue**. Only a *present-but-wrong-typed* map warns. This is what makes A4/A5′/A8′ safe to ship unconfirmed (§0.4) — guessing a map key wrong is indistinguishable from the user not having configured one.

**Never read, by any adapter:** every field not named above, now including **`type`**. `enabled` (verified present on Codex and OpenCode entries — M7, M9), `disabled`, `startup_timeout_sec`, `tool_timeout_sec`, `bearer_token_env_var`, and Claude Code's project-scoped `projects.<path>.disabledMcpjsonServers` / `enabledMcpjsonServers` (M3) are not inspected, not matched, and cannot change any result — the same discipline `opencode_agents.rs:269-275` applies to `hidden`. This is what makes proposal decision 6 ("a disabled server is still emitted") structural rather than a promise: **there is no code path that could filter on it.** M3 also makes the user-scope-only rule evidence-backed for Claude Code: its enable/disable state is not on the entry at all, it is project data, and `projects.*` is never opened. DTOs stay permissive (CXD §8): an unmodelled key is ignored, never an error.

### 6.4 Field mapping onto `Component`

Closes the proposal's sixth open item (`proposal.md:296`).

| Source | `Component` field |
|---|---|
| The server-name config key, **verbatim and un-normalized** | `name` (mirrors `opencode_agents.rs:203`, which preserves the raw key while identity normalizes a copy) |
| The same key, normalized | `id: ComponentId::derive(ComponentKind::Mcp, key)` — trim → NFC → lowercase (`identity.rs:55-57`), unchanged |
| — | `kind: ComponentKind::Mcp` |
| — | `scope: Scope::User`, always |
| — | **`description: None`, always** |
| — | **`provenance_hint: None`, always** |
| One per **declaring** file | `locations: Vec<Location>` with `path: Some(file)`, `root: <the root id>`, `origin: LocationOrigin::File`, `mcp_transport: Some(_) \| None` |

**`description` is always `None`, and that is a decision, not an omission.** The shape inspection enumerated the entry keys of all three clients (M2, M5, M6, M9) and **found no description field in any of them** — this is now an observation, not an absence of evidence. Rejected: synthesizing one from `command` (that is connection detail wearing a display field's clothes, and it re-opens a redaction surface — a `command` can be `npx -y @scope/pkg`, and once a *derived* string is allowed into `description`, the next contributor derives it from `args`); from the transport type (`"stdio"` is already `McpTransport`'s discriminant); or from the client name (that is P2's job). If a schema verifiably grows a description field, adding it is a one-line change in one `redact` function.

**`provenance_hint` is always `None`.** It MUST stay opaque (`specs/domain-model/spec.md:98-112`), it MUST NOT carry disabled state, and it MUST NOT carry the client — `Location.root` already answers that, and putting it here would be the client discriminator P2 explicitly deferred. Both existing config-key adapters pass `None` (`opencode_agents.rs:208`), so this is the precedent, not a new rule.

**An empty server key** (`"": { ... }`) yields a `Component` with `name: ""` and `id: "mcp:"`, emitted, **with no issue** — the same behaviour `agents.rs` and `opencode_agents.rs` already have for a blank name, and CXD §6.3 explicitly declined to add a per-client validation rule that the other adapters do not have. This change does not introduce one either. Two clients each declaring `""` consolidate into one component, which is correct under the existing identity rule and is pinned by a fixture so it is deliberate.

**A non-string server key is unrepresentable, not handled.** JSON object keys arrive as `String` by construction (`jsonc.rs:33`) and TOML table keys are strings by the format's grammar. There is no such case to write code for, and writing defensive code for it would imply the opposite.

## 7. Error paths — the `ScanIssue` taxonomy

Closes the proposal's seventh open item (`proposal.md:297`). **No new severity and no new field on `ScanIssue`**; `IssueSeverity` stays at exactly two variants — that is a review check.

### 7.1 The dividing line

> **`Error` ⇔ the file yielded nothing. `Warning` ⇔ the file was read, but a part of it was not understood.**

Every row below follows from that one sentence, and it is the reading that reconciles `specs/mcp-scanner/spec.md:126-133` with the existing adapters.

| Condition | Severity | `path` | Reason (fixed strings — see §7.2) |
|---|---|---|---|
| Config file absent (`NotFound`) | **none** | — | Absence is `SearchRootStatus::NotFound`; the single `Warning` comes from `scan::append_missing_root_issues` (V3). **CA-11** |
| File unreadable, or not valid UTF-8 | `Error` | file | `could not read the <Client> MCP configuration` |
| Invalid JSONC / invalid TOML | `Error` | file | `could not parse the <Client> MCP configuration` |
| Document root is not an object/table | `Error` | file | `the <Client> MCP configuration is not a JSON object` |
| MCP root key absent | **none** | — | Absence is never a failure (`opencode_agents.rs:130-131`) |
| MCP root key present but empty | **none** | — | An explicitly empty inventory is an answer, not a fault |
| MCP root key present, wrong-typed | `Warning` | file | `the "<key>" key is not a JSON object; no MCP server was read from this file` |
| Entry present, not an object/table | `Warning` | file | `MCP server "<key>" is not a JSON object; its transport was not read` → component emitted, `mcp_transport: None` |
| Entry matches neither stdio nor remote | `Warning` | file | `MCP server "<key>" declares neither a command nor a URL; its transport was not read` → `None` |
| Entry declares both, `command` usable **and `url` valid** | `Warning` | file | `MCP server "<key>" declares both a command and a URL; the command was used` → `Stdio` |
| `command` wrong-typed, empty string, or empty array, **and no valid `url`** | `Warning` | file | `MCP server "<key>" has no readable command` → `None` |
| `command` wrong-typed, empty string, or empty array, **and `url` present and valid** | `Warning` | file | `MCP server "<key>" has no readable command; the URL was used instead` → `Remote` (§6.3's usability fallback, added 2026-08-25) |
| Remote `url` wrong-typed, **and `command` not usable** | `Warning` | file | `MCP server "<key>" has no readable URL` → `None` |
| Remote `url` refused by §3, **and `command` not usable** | `Warning` | file | `MCP server "<key>" has a URL that could not be reduced to a safe endpoint` → `None` |
| `url` wrong-typed or refused by §3, **and `command` usable** | **none** | — | The usable command was selected on its own merits; the surplus URL was never a transport candidate → `Stdio`, silent (§6.3's matrix, `command` usable × `url` unsanitizable) |
| `env`/`environment`/`headers` **absent** | **none** | — | An unconfigured map is not a fault. **This is the row that makes A4/A5′/A8′ safe to ship unconfirmed** (§0.4): a wrong guess at a map key name is indistinguishable from the user not having configured one, and neither is an error |
| `env`/`environment`/`headers` present but not a map | `Warning` | file | `MCP server "<key>" has a non-object <field>; its key names were not read` → transport still `Some`, key list empty |
| `args` present but not an array | `Warning` | file | `MCP server "<key>" has a non-array argument list` → transport `Some`, `arg_count: 0` |

**Divergence from `opencode_agents.rs:139-146`, recorded deliberately.** That adapter treats a wrong-typed root key as an `Error`. Here it is a `Warning`, because `specs/mcp-scanner/spec.md:129-133` pins it and because §7.1's line puts it there: the file parsed, we simply could not read one branch of it. The existing adapter is **not** changed to match — that would be an unrelated behaviour change to a shipped capability.

**Isolation is per entry and per file.** Every arm continues; one unreadable client never costs another client's servers, and one bad entry never costs its siblings — **CA-12**.

### 7.2 The rule that has no precedent in this codebase: reasons never interpolate

`opencode_agents.rs:100-119` and `codex_agents.rs` build reasons with `format!("...: {err}")`. **MCP adapters MUST NOT.**

The reason is concrete, not theoretical, and it is **not TOML-specific — both format seams have it**. A `ScanIssue` crosses IPC and, in the general case, is exactly the kind of value an application logger exists to surface (`proposal.md:21`) — see the honest correction below about what today's logger actually does. A TOML parse error from the `toml` crate quotes the offending source line in its `Display` output; a malformed `~/.codex/config.toml` whose broken line is inside `[mcp_servers.github.env]` would therefore route a live token into whatever consumes that `Display` text, entirely bypassing the model-level redaction the design rests on. `jsonc_parser`'s `ParseError` has the same shape at least once — `ParseStringErrorKind::InvalidUnicodeEscapeSequence(String)` embeds the malformed escape's source fragment in its `Display` (`jsonc-parser 0.33.1`, pinned in `Cargo.lock`, is the version this claim is checked against) — so a malformed `\u` escape adjacent to a secret string in `~/.claude.json` or `opencode.json` hits the identical hazard on the JSONC path. Neither seam is safe by construction on this axis; the blanket no-interpolation rule below is what closes it for **both**, uniformly. The existing adapters (`opencode_agents.rs`, `codex_agents.rs`) are safe today only because the files they read are not credential-bearing; that precedent does not transfer, and this design refuses to inherit it for either seam.

**Correction found by adversarial review, 2026-08-25 — what "reaches the log" means today.** Verified against `crates/vertice-app/src/commands.rs:62-85`: `log_scan_report_with` iterates only `report.roots_scanned` (emitting `root.id` and `root.path` for a `NotFound` root) and `report.client_presence` (emitting `record.label` for `NotDetected`). **It never reads `report.issues` at all**, so `ScanIssue.reason` does not reach the application log through any code path that exists today — the earlier present-tense claim that it does was wrong. `root.path` genuinely does reach the log, so that part of the premise stands. This rule is retained anyway, and is correctly retained, as **defense in depth**: it protects `ScanIssue.reason` against IPC/report exposure regardless of the logger, and against a future logger that starts emitting issues — the exact hedge §10.2 already carries ("including for any future logger that starts emitting `ScanIssue.reason`"). Nothing about the rule itself weakens; only the tense of the claim does.

Therefore, in `mcp.rs`, `mcp_claude.rs`, `mcp_opencode.rs` and `mcp_codex.rs`:

- **No `ScanIssue.reason` may interpolate a parser error, a URL, an argument, an env value, a header value, or any file content.** The only interpolated values permitted are the **server key** and the **client label**, both of which are user-authored identifiers, not values.
- `ScanIssue.path` carries the file, which is where a human goes to see the error. **Diagnostic precision is deliberately traded for containment.**
- Enforced mechanically by `tests/mcp_no_error_interpolation_invariant.rs`, a textual invariant in the style of `tests/codex_version_source_invariant.rs`. **A grep for the literal `{err}` is not enough** — it is defeated by binding the error to any other identifier (`{e}`, `{parse_err}`, `{cause}`) or by interpolating some other in-scope value that carries config content, and neither requires any imagination from a contributor who reasonably copies the neighbouring adapter's `format!("...: {err}")` style. The invariant is therefore **broadened to a structural grep, not a literal one**: scan every `ScanIssue { .. }` / `ScanIssue::new`-style construction site in `mcp.rs`, `mcp_claude.rs`, `mcp_opencode.rs` and `mcp_codex.rs` for a `format!`/`write!` `reason` argument, and reject any interpolated identifier that is not on a fixed, three-entry allow-list: the **server key**, the **client label**, and the **path** (already carried on `ScanIssue.path`, so a `format!` embedding it in `reason` too is still allowed, not just tolerated). Any other interpolated identifier — bound under any name — fails the test. This still cannot catch a value laundered through an intermediate variable the test's parser does not trace (e.g. `let msg = format!("{cause}"); ScanIssue { reason: msg, .. }`), so the residual gap is: **the invariant is sound against direct interpolation in the construction call, not against arbitrary data flow into `reason` before that call.** It is not claimed to be airtight, only to close the specific, realistic defeat (renaming the bound identifier) that the literal-`{err}` grep did not.

Accepted cost, stated: a user debugging a malformed config gets "could not parse" plus a path instead of a line number. That is one `cargo run` away from the real message and is worth a leak class that cannot otherwise be closed.

**Hardening, from adversarial review 2026-08-25 — a third, unenumerated leak vector: panic messages.** §6.1 and this section enumerate and close two paths into the report/log: parser-`Display` quoting, and `format!` interpolation at `ScanIssue` construction. Neither closes a **panic message**. Nothing in the design otherwise forbids `.unwrap()` / `.expect()` / `panic!` on a value bound during the redact phase, and a panic message built with `{:?}` over a raw entry would embed configuration content in whatever observes the panic (stderr, a panic hook) — a channel the §10.2 `FAKE` guard does not observe, since it inspects only the serialized report and the captured log-sink closure. This is theoretical: it requires an implementer to violate the design's own "degrade, never abort" rule (§7.1), which is already the norm every adapter must follow. It is closed as **defense in depth, not as a closed defect**: `mcp.rs`, `mcp_claude.rs`, `mcp_opencode.rs` and `mcp_codex.rs` MUST NOT call `.unwrap()`, `.expect()`, or `panic!` over any value bound during the redact phase, and `tests/mcp_no_error_interpolation_invariant.rs` (or a sibling invariant test in the same style) MUST grep those four modules for `.unwrap(`, `.expect(`, and `panic!(` the same way it already greps for disallowed interpolation.

## 8. Consolidation — `location_key` totality

Closes the proposal's fifth open item (`proposal.md:295`). **Conclusion: `location_key` needs no extension. No change to `consolidate.rs` beyond `ROOT_ORDER` and its pinning test.**

The concern (`proposal.md:262`) is that every MCP server from one client shares one config-file path, so `(root_rank, root_id, path)` (V6, `consolidate.rs:48-54`) is no longer unique. It is not, and it does not need to be:

1. **`location_key` is used only for ordering, never for identity or deduplication.** It appears in exactly two places: sorting one component's own locations (`consolidate.rs:126-130`) and building `member_key` (`consolidate.rs:61-72`). Grouping is by `ComponentId` alone (`consolidate.rs:112-124`); `merge_into` concatenates with `extend` and never compares locations (`consolidate.rs:104`, pinned by `total_location_count_is_conserved`). A duplicate key therefore cannot merge, drop, or hide a location.
2. **Within one component, MCP location keys cannot collide anyway.** An MCP component gets at most one `Location` per declaring file, and the two files under `claude-mcp` and the two under `opencode-mcp` have different paths — so the two-file roots of §5.1 produce two locations with the same `root_id` but distinct `path`s, which is precisely the case `location_key`'s third element already exists to break (`consolidate.rs:44-47`). Two locations sharing a root id and a path would require the same server key twice in one file, which the parser already collapses to one (V1).
3. **Across components, collisions are expected and harmless.** Ten servers in one `~/.claude.json` produce ten components whose single location keys are identical. They are never in the same identity group, so `member_key` is never even called to compare them. Where it is called — two clients declaring the same server name — the two keys differ by `root_rank`.
4. **Every remaining tie is broken deterministically.** `member_key` falls back to `component.name` (`consolidate.rs:71`); the final sort is `(name, id)` (`consolidate.rs:132-136`); adapter output order is `BTreeMap`-sorted (V1) and therefore fixed; and `slice::sort_by` is stable, so even a total tie preserves a deterministic order. `precedence_is_independent_of_input_arrival_order` (`consolidate.rs:285-306`) is the existing pin for exactly this property.

Extending the key with, say, the server name was rejected: it would add a field to a sort key to fix a problem the sort key does not have, and it would couple `consolidate.rs` — which is pure, total and kind-agnostic — to a per-kind concern.

**New pins required** (they assert an existing property against new data, they do not change it): a fixture with several servers in one config file asserting the full component order; and the same server name in all three clients asserting one component, three locations, in `claude-mcp → opencode-mcp → codex-mcp` order, each with its own transport.

## 9. IPC contract surface and paths

### 9.1 IPC

**No new command, no new event, no capability change.** `crates/vertice-app/` and `capabilities/default.json` stay byte-identical; `scan`/`rescan` remain thin pass-throughs. The contract change is entirely inside the existing payload.

| Binding file | Action |
|---|---|
| `ComponentKind.ts` | Modified — `"skill" \| "agent" \| "mcp"` |
| `SearchRootKind.ts` | Modified — third variant |
| `Location.ts` | Modified — `mcpTransport: McpTransport \| null` |
| `McpTransport.ts` | **New** |
| every other `bindings/*.ts` | Unchanged — a diff there means something leaked into `model/` |

Regenerated **only** by `cargo test -p vertice-core`, never hand-edited, landing in the same commit. `frontend/src/` outside `bindings/` is byte-identical. **`ComponentKind` becoming three variants is a genuine breaking change for the frontend's exhaustive handling** — an unhandled `"mcp"` is the expected symptom, and the frontend cycle must be told before it plans (`proposal.md:178`). On revert, `McpTransport.ts` **must be deleted by hand**: `ts_rs` does not remove stale bindings and the CI drift gate cannot see an orphan (a known trap, `proposal.md:356`).

### 9.2 Paths by OS

| Purpose | All three platforms |
|---|---|
| Claude Code MCP | `<home>/.claude.json` **and** `<home>/.claude/settings.json` (**M1**) |
| OpenCode MCP | `<home>/.config/opencode/opencode.json` + `.jsonc` (**M8**) |
| Codex MCP | `<home>/.codex/config.toml` (**M5**) |
| Deliberately never opened | `~/.local/share/opencode/mcp-auth.json` (a credential store); `~/.claude.json`'s `projects.<path>` subtree (**M3** — project-scoped enable/disable state); any project-scope `.mcp.json`, project `opencode.json`, or trusted-project `.codex/config.toml`; any plugin source |

`<home>/.claude/settings.json` sits inside `.claude/`, which is already probed as `claude-embedded-agents` (`roots.rs:97-99`) and walked as `claude-skills`/`claude-agents` (`roots.rs:62-95`). Those adapters walk `.claude/skills/` and `.claude/agents/` subdirectories and never read a file at `.claude/`'s top level, so **no existing root's behaviour changes** and no existing fixture gains or loses a component.

**No platform branch, on purpose.** These are dotfiles under the user's home on every OS, exactly like `.claude`, `.config/opencode` and `.codex` already are (`roots.rs:62-84`, CXD §9.2). All three roots resolve and are read on all three CI legs from day one.

### 9.3 One performance risk, recorded

`~/.claude.json` accumulates per-project state (M3) and was flagged as a possible multi-megabyte read. **M4 downgrades the likelihood: it is 51 KB on the reference machine.** The risk is retained rather than dropped, because 51 KB is one user's history and the file grows monotonically with the number of projects. The adapter reads it whole and parses it whole, because the seam offers no partial or streaming API (V2 for TOML, `jsonc.rs:65-73` for JSONC) — and adding one for a single consumer is out of scope. **CA-15's <2s budget is the guard**, and the `reference-volume` fixture is where it is enforced (V10, `scan.rs:213-223`). §10.3 adds a deliberately oversized `.claude.json` to a budget fixture so the risk is measured rather than assumed. If it ever regresses, the escape hatches — in preference order — are: narrow the file set once A1 is confirmed, or add a key-scoped parse entry point to `jsonc.rs`. Neither is done speculatively.

## 10. Fixtures

Closes the proposal's eighth and ninth open items (`proposal.md:298`, `proposal.md:312-326`).

### 10.1 Layout

New top-level tree, distinct from every existing skill/agent tree as `specs/mcp-scanner/spec.md:197-210` requires:

```
crates/vertice-core/tests/fixtures/mcp/
  claude/{complete, two-files-partial-override, settings-json-only,
          stdio-secret, remote-secret, remote-dirty-url,
          remote-userinfo-ambiguous-url, remote-unparseable-url, malformed,
          malformed-secret-adjacent, root-key-wrong-type, entry-wrong-type,
          entry-unusable-command-valid-url,
          empty-root-key, absent, blank-key}/
  opencode/{complete, two-files-partial-override, empty-command-array,
            stdio-secret, remote-secret, malformed,
            malformed-secret-adjacent, root-key-wrong-type, absent}/
  codex/{complete, empty-args, args-non-string-element, stdio-secret,
         remote-secret, malformed, malformed-secret-adjacent,
         root-key-wrong-type, entry-field-wrong-type, absent}/
  shared/{same-name-three-clients, several-servers-one-file,
          disabled-flagged, no-mcp-anywhere}/
```

Three of those homes exist **because** the machine inspection found something this design had not planned for: `claude/two-files-partial-override` and `claude/settings-json-only` (M1/M2 — Claude Code is a two-file root and its `type` is optional), and `codex/empty-args` (M5 — an empty `args` array is a real observed shape, not a hypothetical).

Each leaf is a synthetic `home`. Inherited without change: **no fixture contains a symlink or a junction, ever** (CXD §10.2), and **every fixture directory carries at least one file**, since git does not track empty directories (CXD §10.3).

### 10.2 The `FAKE` invariant — the headline test

Every fake secret value in every MCP fixture contains the literal substring **`FAKE`**, and **`FAKE` appears nowhere else**: not in a directory name, not in a file name, not in a server key, not in a `command`, not in a host name.

That single rule turns the whole redaction requirement into one assertion:

```rust
// vertice-core: the report that crosses IPC.
assert!(!serde_json::to_string(&report).unwrap().contains("FAKE"));

// vertice-app: everything the logger would have emitted for that report.
let mut emitted = String::new();
log_scan_report_with(&report, |_, message| emitted.push_str(message));
assert!(!emitted.contains("FAKE"));
```

`serde_json` is **already a dev-dependency** of `vertice-core` (`crates/vertice-core/Cargo.toml:19-20`), so the report half adds nothing to the manifest. The log half is a **test-only** addition to `vertice-app`: `log_scan_report_with` already takes the emission closure as a parameter precisely so a test can capture it without touching the process-global `log` sink (`crates/vertice-app/src/commands.rs:59-62`), so `crates/vertice-app/src/` **source** stays byte-identical as §9.1 requires. The two assertions are layered on purpose: the logger's only input is the `ScanReport`, so a clean report makes a clean log **by construction** — the second assertion pins that the construction stays true.

**Stated honestly, corrected 2026-08-25:** verified against `crates/vertice-app/src/commands.rs:62-85`, `log_scan_report_with` today emits only `root.id` / `root.path` (for a `NotFound` root) and `record.label` (for `NotDetected` client presence) — it never reads `report.issues`, so `ScanIssue.reason` does not reach the log through any path that exists today. Against the current logger, the log-half assertion genuinely exercises `root.path`, which the report *can* carry MCP-adjacent content through (a `claude-mcp`/`opencode-mcp`/`codex-mcp` root path), but it does not exercise `ScanIssue.reason` at all — that half of the guard is **forward-looking**: it pins that the construction stays clean *including for any future logger that starts emitting `ScanIssue.reason`* (§7.2's exact hedge), not that today's logger is proven safe against a reason it never touches.

The guard is strictly stronger than per-string assertions: it covers every case in the fixture tree at once, and it automatically covers **future** cases, because a contributor adding a fixture secret inherits the guard for free. Per-value assertions still exist alongside it for the named spec scenarios (`env_keys` **contains** `"GITHUB_TOKEN"`, `header_keys` **contains** `"Authorization"`), because the guard must prove the key survived as well as that the value did not.

The fake-value vocabulary, fixed so a reviewer can grep a diff:

| Where | Value |
|---|---|
| `env` / `environment` | `GITHUB_TOKEN` = `ghp_FAKE0000000000000000000000000000000000` |
| `headers` / `http_headers` | `Authorization` = `Bearer sk-FAKE-0000000000000000000000000` |
| `args` | `--token=ghp_FAKE1111111111111111111111111111111111` |
| remote `url` | `https://u_FAKE:tok_FAKE@mcp.example.test:8443/mcp/tok_FAKE?apiKey=tok_FAKE#f_FAKE` |

That URL is the load-bearing one: it carries a credential in **userinfo, path, query and fragment simultaneously**, and its expected output is exactly `https://mcp.example.test:8443` — proving §3.2's port-preserved, path-dropped rule in one assertion.

**A second load-bearing URL, added by the 2026-08-25 correction (§3.1):** `claude/remote-userinfo-ambiguous-url` uses `https://tok_FAKE/@host.example.test/mcp` — a userinfo containing a `/`, which is exactly the shape that made the original four-step rule leak `tok_FAKE` verbatim into `https://tok_FAKE`. Its expected output is `None`, and the `FAKE` guard proves the fragment reaches neither the serialized report nor the log. **Every dirty-URL fixture before this correction used a colon-only userinfo** (`u_FAKE:tok_FAKE@…`), so the previous fixture set would have shipped this leak with green CI — none of it exercised a userinfo containing `/`, `?` or `#`.

**Reviewer rule, restated from `proposal.md:259`:** any plausible-looking secret in a fixture diff that does **not** contain `FAKE` is a blocker, not a nit.

### 10.3 Existing fixtures, and which pins are genuinely untouchable

**The proposal's eighth question — isolated homes, or `scan-orchestrator/complete` immediately — has a forced answer, and it is worth stating why.** The moment the three adapters are wired into `scan_for`, *every* orchestrator fixture gains three roots. `complete` asserts `report.issues.is_empty()` (V9), so three `NotFound` roots would break it. **`complete` cannot be left untouched; the only real choice is what to put in it.**

So: **both, split by slice.** Per-client behaviour, including every secret-bearing case, lives in isolated `fixtures/mcp/` homes exercised through the adapters directly (slices 2-4). `complete` is touched **only** in the wiring slice, and gains **non-secret** MCP configuration — one plainly-named stdio server per client, no `FAKE` value anywhere — so that orchestration and redaction are proven by different fixtures and a failure in one cannot be mistaken for the other.

| Fixture / test | Change |
|---|---|
| `scan-orchestrator/complete` | Gains `.claude.json` (and, for M1 coverage, `.claude/settings.json`), an `mcp` block in the existing `.config/opencode/opencode.json`, and `.codex/config.toml`, all secret-free. Counts move: roots **8 → 11**, components **12 → 15**, `issues.is_empty()` **restored** |
| `scan-orchestrator/missing-root-client` | No fixture change. Assertions move: roots **8 → 11**, path-less `Warning`s **8 → 11** |
| `scan-orchestrator/corrupt-skill`, `corrupt-codex-agent`, `codex-claude-same-skill` | No fixture change, no assertion change — none pins a root count |
| `scan-orchestrator/mcp-same-name-three-clients` | **New** orchestrator home for `specs/scan-orchestration/spec.md:43-47`: one component, three locations, three transports |
| `scan-orchestrator/reference-volume` | Gains a deliberately oversized, secret-free `.claude.json` for §9.3's budget measurement. Its three assertions (V10) are unchanged and must stay green, including tree-snapshot equality (**CA-16**) |
| `tests/fixtures/roots/reference/` | **Byte-identical.** Structurally safe, not merely observed to be: it is reached only by `skills::scan`, and no MCP root resolves inside it (V11). The 69/25/22/3 pins do not move |
| `tests/model_contract.rs` | `ComponentKind` exhaustive-match test gains `Mcp`; 7 `Location` literals gain `mcp_transport: None` |

### 10.4 Fixture-to-requirement map

| Fixture | Proves |
|---|---|
| `*/stdio-secret` | `env_keys` contains the key name; the token appears nowhere in the serialized report; `arg_count` reflects a `--token=…` argument whose value is absent |
| `*/remote-secret` | `header_keys` contains `Authorization`; no bearer value anywhere |
| `claude/remote-dirty-url` | Userinfo, path, query and fragment all stripped; port preserved (§3.2) |
| `claude/remote-userinfo-ambiguous-url` | A userinfo containing `/` yields `None`, never a truncated authority fragment — the 2026-08-25 correction (§3.1); `FAKE` reaches neither the report nor the log |
| `claude/remote-unparseable-url` | `mcp_transport: None` + one `Warning`; the raw string emitted nowhere, **including in the reason** (§7.2) |
| `*/malformed` | One `Error` with the path, a **fixed** reason with no parser text, siblings unaffected — **CA-12** |
| `codex/malformed-secret-adjacent` | A TOML syntax error on the line **after** a `FAKE` token: the `FAKE` guard proves §7.2's no-interpolation rule empirically |
| `claude/malformed-secret-adjacent`, `opencode/malformed-secret-adjacent` | A malformed `\u` escape adjacent to a `FAKE` token in a JSONC string: the `FAKE` guard proves §7.2's no-interpolation rule on the JSONC path too, empirically rather than by argument from shape (§6.1, §7.2) |
| `*/root-key-wrong-type` | Zero components, one `Warning`, no abort |
| `claude/empty-root-key` | A present-but-empty root key (`"mcpServers": {}`) yields **zero components and zero issues** — §7.1's "MCP root key present but empty" row. An explicitly empty inventory is an answer, not a fault, and is distinguishable from both an absent key and an unreadable file. Traced 2026-08-25 (E4); previously listed in §10.1 with no requirement row and no RED test |
| `*/entry-wrong-type`, `codex/entry-field-wrong-type` | Component emitted, `mcp_transport: None`, one `Warning`; on the Codex path, one wrong-typed field does **not** escalate to a file-level `Error` (§6.2) |
| `claude/entry-unusable-command-valid-url` | An entry with a wrong-typed `command` **and** a valid `url` falls back to `Remote` with exactly one `Warning`, never to `None` — the §6.3/§7.1 usability-discrimination correction (2026-08-25); resolves the contradiction between the two sections |
| `claude/settings-json-only` | A `~/.claude/settings.json` entry with **no `type` key** (M2) still yields `Stdio` — the structural discriminator, and the row that would fail any `type`-reading implementation |
| `codex/remote-secret` | A Codex entry of shape `{ url }` **with no `command` and no `type`** (M6) yields `Remote` — the same structural rule, from the opposite direction |
| `codex/empty-args` | `args: []` (M5) yields `arg_count: 0` and a valid `Stdio`, **not** a degraded entry |
| `codex/args-non-string-element` | `args: ["--flag", 42]` yields `arg_count: 2`, a valid `Stdio`, and **no** `Warning` — a non-string element is counted, never inspected, resolving §6.3's edge-rule table |
| `opencode/empty-command-array` | `command: []` (M9's shape, degenerate) yields `mcp_transport: None` + `Warning` — the boundary between "no arguments" and "no command" |
| `*/two-files-partial-override` | The deep merge survives the move to `json_merge` (§4.3): an overlay overriding one field does not erase the base's `command`. Present for **both** two-file roots (§5.1), and it pins §5.2's rule that both locations carry the merged effective transport |
| `shared/same-name-three-clients` | One `Component`, three `Location`s, three transports, ordered `claude-mcp → opencode-mcp → codex-mcp` |
| `shared/several-servers-one-file` | Deterministic total ordering with a duplicated `location_key` (§8) |
| `shared/disabled-flagged` | The entry **is** emitted, and `provenance_hint` is `None` (proposal decision 6). Uses the **verified** `enabled: bool` shape on a Codex and an OpenCode entry (M7, M9) rather than an invented flag, so the fixture pins real behaviour |
| `shared/no-mcp-anywhere` | Zero MCP components, zero MCP `ScanIssue`s, three `NotFound` root Warnings — **CA-11** |
| `*/blank-key` | `name: ""`, `id: "mcp:"`, emitted, no issue (§6.4) |

### 10.5 The gate — CLEARED, and what remains of it

**G1 is cleared** by the 2026-08-25 shape inspection (§0.3/§0.5). Every file path, root key and entry shape a fixture needs is now observed rather than assumed, and the three corrections it produced (M1, M2, M6) are folded into §5.1, §5.2 and §6.3.

Two constraints survive the gate and are permanent, not phase-scoped:

1. **No fixture may be authored from, or diffed against, a real configuration file.** Fixtures are synthetic and their secrets are `FAKE` by construction (§10.2). The verification produced *shapes*, and shapes are what fixtures are built from — no line of any real file is copied.
2. **The four residual rows (A4, A5′, A8′, A10) ship unconfirmed, deliberately.** Each is a map-key name or a display precedence; each fails to an empty key list or a cosmetic ordering, never to a missing server, a wrong value, or a leak (§0.4, §7). They need **no** pending-task marker and **no** deferred fixture: the adapters' absent-key-is-not-an-error rule makes their behaviour correct either way, and a `tasks.md` entry that cannot fail is noise. If a later sanitized sample settles one, the change is one string constant and one fixture line.

## 11. File changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/model/component.rs` | Modify | `ComponentKind::Mcp`; doc names three kinds |
| `crates/vertice-core/src/model/location.rs` | Modify | `SearchRootKind::Mcp`; `Location.mcp_transport`; widened `Location` doc |
| `crates/vertice-core/src/model/mcp.rs` | **Create** | `McpTransport` (§2) |
| `crates/vertice-core/src/model/mod.rs` | Modify | Re-export `McpTransport` |
| `crates/vertice-core/src/model/identity.rs` | Modify | One match arm: `Mcp => "mcp"` (V7). The derivation rule is unchanged |
| `crates/vertice-core/src/mcp.rs` | **Create** | `sanitize_url`, `KeyNames`, `ArgCount`, `Lenient`, `McpScan` — no I/O (§3, §4.2, §6.2) |
| `crates/vertice-core/src/mcp_claude.rs`, `mcp_opencode.rs`, `mcp_codex.rs` | **Create** | Three adapters (§6) |
| `crates/vertice-core/src/json_merge.rs` | **Create** | `merge_all`/`merge_two` moved verbatim with their tests (§4.3) |
| `crates/vertice-core/src/opencode_agents.rs` | Modify | Calls `json_merge`; the two functions and ten tests move out. **No behaviour change** |
| `crates/vertice-core/src/roots.rs` | Modify | Three MCP roots, **two of them two-path** (§5.1); `resolve_single` takes `&[&str]`; new `resolve_pair` helper naming the existing status fold; doc updates |
| `crates/vertice-core/src/consolidate.rs` | Modify | `ROOT_ORDER` 8 → 11 + three lines in the pinning test. **No logic change** (§8) |
| `crates/vertice-core/src/scan.rs` | Modify | Three adapters wired in; orchestrator test counts (§10.3) |
| `crates/vertice-core/src/lib.rs` | Modify | Four new `pub mod` lines plus `mod json_merge;` |
| `crates/vertice-core/src/jsonc.rs`, `toml.rs`, `yaml.rs`, `frontmatter.rs`, `skills.rs`, `agents.rs`, `codex_agents.rs`, `installations.rs`, `model/freshness.rs` | **Unchanged** | No new seam, no seam widening, no shared adapter abstraction, no `FreshnessSubject` variant |
| `crates/vertice-core/tests/mcp_*.rs`, `mcp_no_error_interpolation_invariant.rs` | **Create** | §7.2, §12 |
| `crates/vertice-core/tests/consolidation.rs`, `model_contract.rs`, `opencode_agent_scanner.rs` | Modify | `Location` literals; `ComponentKind` match; merge tests follow the move |
| `crates/vertice-core/tests/fixtures/mcp/**` | **New** | §10.1 |
| `crates/vertice-core/tests/fixtures/roots/reference/` | **Byte-identical** | V11 |
| `frontend/src/bindings/{ComponentKind,SearchRootKind,Location,McpTransport}.ts` | Regenerated | Never hand-edited |
| `frontend/src/` (source), `crates/vertice-app/`, `capabilities/default.json`, `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Unchanged** | §9.1 |

**CA-16 structurally.** The disk surface added is `symlink_metadata` (via `roots::probe`) and `read_to_string`. No `File::create`, no `OpenOptions`, no `fs::write`, no `create_dir*`, no `remove_*`, no `symlink*` — in source **or** tests.

## 12. Testing strategy (`strict_tdd: true` — RED first)

The load-bearing failing tests, in this order, before any implementation:

1. `fake_token_in_env_never_reaches_the_serialized_report` — the `FAKE` guard (§10.2). The feature's reason for existing.
2. `fake_token_in_env_never_reaches_the_application_log` — the same guard over the log file, which is where a leak lands on disk. **Scoped honestly, corrected 2026-08-25:** `log_scan_report_with` (`crates/vertice-app/src/commands.rs:62-85`) does not read `report.issues` today, so this test does not exercise `ScanIssue.reason` at all against the current logger — it exercises `root.path` (which a `claude-mcp`/`opencode-mcp`/`codex-mcp` root path can carry) and `record.label`, both of which the current logger does emit. Its claim over `reason` is explicitly **forward-looking**: it pins that a clean `ScanReport` makes a clean log by construction, so the guarantee holds automatically for any future logger that starts emitting `ScanIssue.reason`, consistent with §7.2's and §10.2's hedge. It is kept in the RED list because the `root.path` half is real today and the forward-looking half is cheap insurance, not because it currently proves `reason` is safe.
3. `dirty_url_is_reduced_to_scheme_host_and_port` — expects exactly `https://mcp.example.test:8443`. An implementation that keeps the path fails here.
4. `userinfo_containing_a_path_delimiter_is_rejected_not_truncated` — `https://tok3n/@host.example/mcp` (and its `?`/`#` variants) MUST yield `None`, never `Some("https://tok3n")`. The 2026-08-25 correction (§3.1); the direct regression test for the leak.
5. `unparseable_url_yields_no_transport_and_a_warning_without_echoing_the_url`.
6. `token_bearing_argument_yields_only_a_count`.
7. `entry_without_a_type_field_is_discriminated_structurally` — one Claude `settings.json` entry (no `type`, M2) and one Codex `{ url }` entry (no `type` anywhere, M6). Any implementation that reads `type` fails both.
7a. `unusable_command_with_a_valid_url_falls_back_to_remote_not_none` — a Claude entry with a wrong-typed `command` and a valid `url` yields `Remote` with exactly one `Warning`, never `None` — the §6.3/§7.1 usability-discrimination correction (2026-08-25); the direct regression test for the contradiction found by adversarial review.
8. `opencode_array_command_maps_to_command_plus_arg_count` — `["npx", "-y", "pkg"]` ⇒ `command: "npx"`, `arg_count: 2`, against Claude/Codex's string-plus-array shape in the same suite (§6.3).
9. `same_server_name_in_three_clients_yields_one_component_with_three_transports` — **CA-12**-adjacent; `total_location_count_is_conserved` stays green.
10. `malformed_config_yields_one_error_with_a_fixed_reason_and_no_parser_text` — §7.2.
11. `home_without_any_mcp_configuration_yields_no_components_and_no_errors` — **CA-11**.

| Layer | What |
|---|---|
| Unit, no I/O | `sanitize_url` over §3.2's full table including every rejection row; `KeyNames`/`ArgCount` proving keys survive and values are never constructed; `Lenient` proving a wrong-typed field does not fail the document |
| Invariant | `tests/mcp_no_error_interpolation_invariant.rs` (§7.2); the existing `jsonc`/`toml`/`yaml` seam-containment tests, which now cover four more modules for free; `root_order_matches_the_roots_module_in_order` at 11 entries |
| Integration | The §10.4 fixture table, one test per row, through `mcp_claude::scan` / `mcp_opencode::scan` / `mcp_codex::scan` — all three CI legs (**CA-17**) |
| Orchestrator | The §10.3 count deltas; `mcp-same-name-three-clients` |
| Regression | `roots/reference/`'s 69/25/22/3 pins untouched (V11); `reference-volume` read-only + <2s (**CA-15/CA-16**), now with an oversized `.claude.json` (§9.3) |
| Contract | `IssueSeverity` still exactly two variants; `ComponentKind` exhaustive match; **no MCP component appears in any `FreshnessCheck`**; every MCP `Component` is `Scope::User`; every skill/agent `Location` has `mcp_transport: None` |
| Frontend | **No source change and no new test.** Existing Vitest suites must stay green against the regenerated bindings; `npm run check` must pass, since Vitest alone does not typecheck them |

Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, the `msrv` job, bindings-in-sync, and `npm run lint && npm run check && npm run test && npm run build`.

## 13. Slicing and rollback

Four slices, each independently green and independently revertible. The proposal forecasts ~920-1430 lines with **High** 400-line budget risk; this split keeps each PR reviewable and puts the security-critical code in its own slice.

1. **Model + bindings.** `ComponentKind::Mcp`, `identity_prefix`, `SearchRootKind::Mcp`, `McpTransport`, `Location.mcp_transport`, all 13 literal sites, regenerated bindings. Nothing produces an MCP component yet.
2. **`src/mcp.rs` + `src/json_merge.rs`.** Pure, I/O-free, unit-tested to exhaustion: `sanitize_url`, `KeyNames`, `ArgCount`, `Lenient`, and the merge move proven by the untouched OpenCode agent suite. **This is the slice a security reviewer reads.**
3. **OpenCode end to end** — `opencode-mcp` root, adapter, fixtures, `ROOT_ORDER` + pinning test, orchestrator wiring and the `complete`/`missing-root-client` count moves. It goes first now for a different reason than before G1 cleared: it is no longer the *only* corroborated client, but it is still the one that exercises the most machinery in one slice — two-file merge (§5.2), the array-`command` mapping (§6.3), and `json_merge`'s first non-agent consumer.
4. **Claude Code and Codex** — one resolver, one adapter and one fixture tree each, plus their two `ROOT_ORDER` entries. Claude Code is now the *second* two-file root (M1), not a single-file one, so it is no longer the trivial slice this design first assumed; Codex remains single-file but is the only consumer of the `Lenient`/`KeyNames` TOML primitives from slice 2. Splitting them into two PRs is `sdd-tasks`'s call.

Deliberately deferred to `sdd-tasks`, not decided here: the order of the RED tests inside each slice, and whether slice 4 is one PR or two. Final slicing is `sdd-tasks`'s call.

Rollback is the proposal's ordered layers (`proposal.md:351-360`), with three additions this design creates:

- **`json_merge.rs` must be un-moved, not merely deleted** — its functions and tests return to `opencode_agents.rs`, or the OpenCode agent adapter stops compiling.
- **`McpTransport.ts` must be deleted by hand** — `ts_rs` leaves stale bindings and the drift gate cannot see an orphan file (§9.1).
- **`resolve_single`'s signature reverts to `[&str; 2]`**, touching its four existing call sites.

`Cargo.toml`, `Cargo.lock` and `deny.toml` have nothing to revert. `vertice-app` is untouched. **Migration: none** — nothing is persisted; `ScanReport` is rebuilt on every scan.

## 13.5 Adversarial review outcome — ESCALATED, four items carried into `sdd-tasks`

Three rounds of blind dual review ran on 2026-08-25. Round 1 found a **CRITICAL** secret
leak in `sanitize_url` (both judges, independently) — the authority was truncated before
userinfo was stripped, so `https://tok3n/@host.example/mcp` emitted `https://tok3n`, a
verbatim credential fragment. That is fixed and both judges failed to break the replacement
rule in rounds 2 and 3. Two further rounds of fixes corrected the log-exposure claim, the
`Location` counts, the merge-test count, the JSONC parser-error framing, and the
presence-vs-usability discrimination rule.

Judgment Day was then closed as **ESCALATED** by explicit decision: the remaining items are
better resolved against real code than by further document iteration. They are recorded here
so `sdd-tasks` carries them, and each MUST be resolved before its slice is considered done.

**E1 — Confirmed by both judges. The transport discrimination rule is still not total.**
Round 2 closed "unusable `command` + valid `url`"; round 3 found the mirror case, "usable
`command` + present-but-invalid `url`", which currently matches three rows across §6.3 and
§7.1 with three mutually exclusive outcomes. Patching rows has now produced an uncovered
cell twice in a row, so the fix is not another row: **enumerate the full 3 × 3 matrix once**
— `command` {absent, present-usable, present-unusable} × `url` {absent, present-valid,
present-unsanitizable} — with exactly one `(transport, issue count, reason)` triple per cell,
and make §6.3's prose and §7.1's table generated from that single enumeration rather than
stated twice.

**E2 — Confirmed by both judges. The `.unwrap()`/`.expect()`/`panic!` clause (§7.2) is not
enforceable as written**, and both judges reached that from opposite directions. It is
*over*-broad: the precedent it copies (`tests/toml_seam_invariant.rs`) is a whole-file
`content.contains` scan with no `#[cfg(test)]` exclusion, while this crate's convention puts
unit tests inline (`opencode_agents.rs` alone has 11 `.expect(...)` calls in its test
module), so the grep would fail CI on day one. It is also *under*-broad: §6.3's own
normative `command[0]` extraction panics on an out-of-bounds index without emitting any of
the three greppable tokens, and unguarded arithmetic is a second silent surface. Replace the
grep with a deny-lint over the four MCP modules (`clippy::indexing_slicing` and the
`unwrap`/`expect` lints), scoped to exclude test modules, or restate the invariant as "no
panic reachable over a redact-phase value" and name indexing and arithmetic as residual gaps
the way §7.2 already does for laundered variables. This clause was added in round 2 on a
single judge's theoretical finding and was mis-calibrated; do not carry it forward unchanged.

**E3 — One judge. The "identical in all three adapters" claim is fixture-verified for one
adapter.** §6.3 states the usability-fallback rule holds identically across Claude Code,
OpenCode and Codex, but only `claude/entry-unusable-command-valid-url` and its RED test
exist. OpenCode's shape is structurally different (array `command`, no separate `args`), so
an implementer could satisfy every listed fixture and still get the OpenCode and Codex
variants wrong with no failing test. Add the two missing fixtures and RED tests, or narrow
the claim.

**E1 and E4 were amended into this document on 2026-08-25**, after the escalation: §6.3 now
carries the full 3 × 3 matrix as the single normative statement of the discrimination rule,
§7.1's table was aligned to it (including the previously-contradictory `url` rows), and
§10.4 now traces `claude/empty-root-key`. E2 and E3 remain for the implementing slices, where
they are tasked in `tasks.md` (2.6 and 4.3/5.3 respectively).

**E4 — One judge. `claude/empty-root-key` is an orphan fixture.** It appears in §10.1's
layout but has no row in §10.4's requirement map and no RED test names it. Under
`strict_tdd: true` an untraced fixture is either dead weight or a scenario nobody wrote a
failing test for. Trace it or delete it.

**Recorded as an accepted residual, not an action item:** the `%40` pass-through row in §3.2
asserts that real clients split the authority on the literal `@` byte, so a percent-encoded
`%40` never functions as a userinfo delimiter. That claim is about client *parsing
behaviour*, not config *shape*, and it carries no citation — unlike everything else in §0,
which is labelled with its evidentiary basis. It is not a known leak, but it should be
demoted to sit alongside A4/A5′/A8′/A10 as an assumption rather than a settled fact.

## 14. Open questions

- [x] **The URL sanitization rule** — subtractive, scheme + host + **port**, path/query/fragment/userinfo dropped; refusal degrades to `mcp_transport: None` + `Warning`, never a verbatim emission. §3
- [x] **Module layout** — three per-client adapters plus one shared, I/O-free `mcp.rs` owning every redaction primitive. §4.1
- [x] **Root ids and `scan_paths` grouping** — `claude-mcp` (**2 files**, corrected by M1), `opencode-mcp` (2 files), `codex-mcp` (1 file), the two multi-file roots sharing one stated merge order; `ROOT_ORDER` 8 → 11, genuinely appended. §5.1-§5.3
- [x] **What a `Location` means under a multi-file root** — merge, then one location per **declaring** file, all carrying the merged **effective** transport. Forced into the open by M1 and settled with its rejected alternatives. §5.2
- [x] **Transport discrimination** — **structural in all three clients; `type` is never read.** Forced by M2 (Claude's `type` is optional) and M6 (Codex has none). §6.3
- [x] **`command`/`args` mapping** — one normative table covering the string-plus-array shape (Claude, Codex) and the single-array shape (OpenCode), with every edge case specified. §6.3
- [x] **`location_key` totality** — no extension needed; the key orders, it never identifies, and every tie is broken deterministically. §8
- [x] **Field mapping** — key → `name` verbatim and → `id` normalized; `description` and `provenance_hint` **always `None`**; empty key emitted with no issue; a non-string key is unrepresentable. §6.4
- [x] **`ScanIssue` taxonomy** — `Error` ⇔ the file yielded nothing; `Warning` ⇔ part of it was not understood. Plus the new no-interpolation rule, mechanically enforced. §7
- [x] **`scan-orchestrator/complete`** — forced to change by `issues.is_empty()`; it gains **secret-free** MCP configuration, while every secret-bearing case lives in isolated homes. `roots/reference/` stays byte-identical, structurally. §10.3
- [x] **Fixture layout** — `fixtures/mcp/{client,shared}/<case>/`, with the `FAKE` invariant turning the entire redaction requirement into one assertion. §10.1-§10.4
- [x] **The per-client paths, root keys and entry schemas** — **CLOSED by the 2026-08-25 shape inspection** (§0.3), which corrected three of this design's decisions (M1, M2, M6) and confirmed the rest. **G1 is cleared** (§10.5).
- [x] **Codex's remote transport** — **CLOSED affirmatively** (M6): it exists, its entries are `{ url }`, and it carries no `type` discriminator. This was the row flagged as least certain; the remote branch is real code, not dead code.
- [x] **The spec conflict in §0.6 — RESOLVED 2026-08-25.** The degraded-entry carve-out was written into `specs/domain-model/spec.md`, matching this design's resolution: `Some(_)` is required only for a fully understood entry; degraded entries yield a `Location` with `mcp_transport: None` plus a `Warning`, never a drop. **No blocking item remains.**
- [ ] **A4, A5′, A8′** — the three unobserved map-key names (`environment` vs `env` on OpenCode; the remote header key on OpenCode and on Codex). Ship as designed: an absent key yields no keys and **no issue**, so the behaviour is correct either way and a later sample costs one constant. §0.4, §7
- [ ] **A10** — Claude Code's own precedence between `~/.claude.json` and `~/.claude/settings.json`. Cosmetic blast radius only; both locations are emitted regardless. §5.2
- [ ] **`~/.claude.json` size** and the <2s budget — likelihood **downgraded to Low** by M4 (51 KB observed), risk retained because the file grows with project count. Measured by the oversized `reference-volume` fixture; the escape hatches are named but not built. §9.3
- [ ] **Percent-encoded credentials** surviving `sanitize_url` are moot now that the path is dropped, but a credential encoded in a **host label** would survive. Accepted: no convention places one there, and the alternative is emitting no URL at all.
- [ ] **Enabled/disabled state** — still deferred, but its entry condition (proposal decision 6: "at least two of the three in-scope clients verified to expose a disabled flag with consistent semantics") is now **partially met and partially refuted**: Codex and OpenCode both carry a per-entry `enabled: bool` (M7, M9), while Claude Code's equivalent is a **project-scoped list outside the entry** (M3). Two of three agree; the third has a structurally different model. The follow-up cycle should treat that asymmetry as its central problem, not as a detail.
- [ ] **Project scope**, **Copilot**, **`mcp-auth.json`**, and **the client field on `SearchRoot`** (P2) — all deferred with targets by the proposal; nothing here changes their status. M3 does add evidence for the project-scope cycle: Claude Code's per-project MCP enable/disable state is already sitting in `~/.claude.json`'s `projects.<path>` subtree, which this cycle deliberately never opens.
