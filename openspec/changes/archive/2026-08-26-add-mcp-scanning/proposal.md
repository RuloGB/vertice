# Proposal: Add MCP Server Scanning (backend)

> Spec trace: this change is scoped from the living specs, not from the completed PoC roadmap. It extends `domain-model` (a new `ComponentKind` variant, a new value type, one new `Location` field), `scan-orchestration` (a fourth class of adapter joining the same infallible concatenation), `skill-scanner`/`agent-scanner` only by analogy, and `duplicate-consolidation` only through `ROOT_ORDER`. It adds one new capability, `mcp-scanner`.
>
> Invariants this change is bound by and MUST NOT weaken: the **read-only invariant (CA-16)** — no `File::create`, no `OpenOptions::write`, nothing written outside the application data directory; **versioned-fixture testing (CA-17)** — every assertion runs against `crates/vertice-core/tests/fixtures/`, never against the author's machine; the **core purity invariant** — `vertice-core` imports nothing from `tauri`; the **format-seam invariant** — `jsonc.rs` and `toml.rs` remain the sole importers of their parser crates; and the **`model/` purity invariant** — no `std::fs`, `std::io`, `std::env` or clock read.
>
> **Backend only.** `crates/vertice-core` and `crates/vertice-app`. No `frontend/src/` **source** change is planned; the frontend appears here exclusively as a binding-contract obligation (`frontend/src/bindings/` regenerates and CI's drift gate fails if it is not committed alongside the Rust change). The frontend work is a separate cycle.
>
> **No new Rust dependency is expected**, and **no new Tauri command**: MCP scanning folds into the existing `scan` / `rescan`.

## Intent

Vertice inventories AI components across AI clients. Today "component" means a skill or an agent. But the thing users increasingly install, forget about, duplicate across clients and cannot audit is the **MCP server** — a process or an endpoint that a client is configured to launch or call, often with credentials attached. A user with `github`, `postgres` and three half-abandoned experiments wired into Claude Code, OpenCode and Codex has no single place that answers "what is actually configured, and where". Vertice's whole premise is to be that place, and it currently has a blind spot exactly where the operational risk is highest.

Three properties make this a well-shaped increment rather than a new subsystem:

1. **The pipeline already has the shape this needs.** `scan.rs` composes independent, infallible adapters, `consolidate.rs` merges by identity, `ScanIssue` carries per-file degradation without aborting, and `commands.rs` is a set of thin pass-throughs with zero business logic. An MCP adapter is a fourth citizen of an existing pattern, not a new pattern.
2. **The correct adapter template already exists.** `opencode_agents.rs` builds components from **object keys inside a config file**, with per-file provenance, deep merge with last-wins-at-the-leaf semantics, and wrong-type degradation to `None` + `Warning` rather than an abort. MCP servers are object keys inside a config file. The precedent is exact.
3. **No new format seam is required.** Claude Code and OpenCode configs go through `jsonc.rs`; Codex goes through `toml.rs`. Both seams exist and are already containment-tested.

The genuinely new problem is **not architectural, it is a data-safety one**. MCP definitions carry `command`/`args`/`env` (stdio) or `url`/`headers` (remote) — precisely where API keys and bearer tokens live. The codebase has **zero** existing redaction tooling: grepping `redact|secret|token|apiKey` across `vertice-core` and `vertice-app` yields only unrelated HTTP plumbing. The read-only invariant does nothing to protect against this: a `ScanReport` is an in-memory value that crosses the IPC boundary and is deliberately logged (`log_scan_report`). A raw credential entering the model would therefore reach a file on disk by design, not by accident.

This proposal's central commitment is that **redaction is structural, not conventional**.

## The decisions this proposal carries as fixed

These were settled by the user before this phase and are **not** re-opened here.

### 1. Redaction: key names only, never values

`env` (stdio) and `headers` (remote) MUST be captured as `Vec<String>` of **key names**. No value from either map MUST ever reach the model, the `ScanReport`, the IPC payload, or the log. This is enforced **by the type**: the model offers no field capable of holding such a value, so the failure mode "someone forgot to redact" is not reachable without a reviewed type change.

This is the same technique `Freshness::Unknown { reason }` uses to make a degraded state a value rather than an error path — the invariant lives in the shape, not in a comment.

**The `url` is governed by the same rule, not exempted from it.** A remote MCP URL can carry a credential in userinfo (`https://user:token@host/…`) or in a query parameter (`?apiKey=…`). Therefore: a captured `url` MUST NOT include userinfo, and MUST NOT include a query string or fragment. What survives is enough to identify the endpoint (scheme, host, port, path) and nothing else. The exact extraction rule is a design decision (see Open Decisions); the **invariant is settled here** and design may not trade it away. If no rule can be implemented safely without a new dependency, the fallback MUST be to omit the URL, never to emit it verbatim.

### 2. Transport lives on `Location`, not on `Component`

`Location.mcp_transport: Option<McpTransport>`, populated only for MCP locations and `None` for every skill and agent.

This is the only placement that survives consolidation without losing information. One server named `github` configured in three clients with three different commands is **one `Component` with three `Location`s**, each keeping its own transport. Verified safe against the existing merge: `merge_into` concatenates locations with `target.locations.extend(other.locations)` (`consolidate.rs:104`) and never deduplicates them, a behavior already pinned by `total_location_count_is_conserved` (`tests/consolidation.rs:113`).

The cost is real and is accepted: `Location` previously answered only "where is this on disk", and now also carries connection detail for one kind. That widening MUST be documented on the type, and the alternative — bolting `command`/`args`/`env`/`url` onto the shared `Component` as `Option` fields that are `None` for every skill and agent, or introducing a sibling payload keyed by `ComponentId` that reintroduces the cross-payload join `model/freshness.rs:38-41` explicitly warns against — is worse on both counts.

### 3. Clients in scope: Claude Code, OpenCode, Codex

**GitHub Copilot is explicitly out of scope and deferred.** `ClientKind::Copilot` does not exist in `model/installation.rs`, and adding it would pull this change into the `client-installation-detector` capability — a different capability from the one this feature nominally touches. Copilot also uses a **different root key** (`servers`, not `mcpServers`), so it is a parsing difference and not merely a path difference. It is a later cycle that first adds the client kind.

## The decisions this proposal settles

These were open after exploration. They are closed here so design can lock type shapes without guessing.

### 4. Freshness is an explicit non-goal for `ComponentKind::Mcp`

There is no upstream version oracle for an arbitrary MCP server: a stdio server is "whatever `npx`/`uvx`/a local binary resolves to at launch time", and a remote server exposes no version at all without connecting to it — which this application MUST NOT do.

Therefore: **`FreshnessSubject` MUST NOT gain an MCP variant in this cycle**, and no MCP component MUST ever appear in a `FreshnessCheck`. This is stated as an explicit non-goal rather than left as a silent absence, so that a future reader does not read the gap as an oversight. `component-freshness` needs **no spec delta** — `FreshnessSubject` is a closed enum with one variant today and stays that way.

### 5. `SearchRootKind` gains an `Mcp` variant

`location.rs:66-68` states the rationale for `SearchRootKind` mirroring `ComponentKind` 1:1: "clients organize search roots per component kind". MCP roots are exactly that — a per-client location whose entire purpose is MCP configuration.

The alternative is to label an MCP root as `Agent` or to leave the mirror broken. Labelling it `Agent` puts a false statement in the data that consumers may reasonably branch on; leaving the mirror broken silently repeals a documented invariant. **`SearchRootKind::Mcp` is added**, keeping the mirror total and the invariant honest. This is a closed enum by design, so the diff is compiler-enforced.

### 6. Enabled/disabled state is NOT modeled in this cycle — and a disabled server is still emitted

Several clients allow disabling a server without removing its entry. The decision is:

- **No `enabled` field is added to the model in this cycle.**
- **A server MUST be emitted regardless of any disabled flag.** Silently dropping it would make Vertice under-report exactly the kind of forgotten configuration this feature exists to surface, and would do so invisibly.
- It **MUST NOT** be smuggled into `provenance_hint`. `domain-model/spec.md:131-133` requires `provenance_hint` to stay opaque and non-behavioral: consumers MUST NOT branch on its value, and any machine-readable classification belongs on a typed field. Using it here would be a direct spec violation.

The reason for deferring rather than modeling: the per-client schemas backing "there is a disabled flag" are **web-sourced and unverified** (see decision 7), and the semantics are not known to be the same across the three clients — a tri-state field whose meaning differs per client is worse than no field. Modeling it is a **follow-up cycle**, whose entry condition is that at least two of the three in-scope clients are verified to expose a disabled flag with consistent semantics.

The consequence is stated plainly so it is auditable: **in this cycle, a disabled server is indistinguishable from an enabled one in Vertice's output.** That is a known, accepted, temporary inaccuracy, and it errs toward over-reporting rather than under-reporting.

### 7. The per-client config paths are ASSUMPTIONS, not verified fact

The config locations, formats and root keys recorded in `exploration.md:138-153` (`~/.claude.json` with `mcpServers`; `~/.config/opencode/opencode.json` with `mcp` and `type: "local"|"remote"`; `~/.codex/config.toml` with `[mcp_servers.<name>]`) are **web-sourced (August 2026)**. They were deliberately **not** verified against the user's real configuration files, precisely to avoid reading live credentials into a persisted document — the same concern this feature exists to address.

**Nothing in this proposal may be read as confirming those paths.** Therefore:

- `sdd-design` MUST close each client's path, format and root key against an authoritative upstream schema or a **sanitized** real sample before any fixture is committed.
- Fixtures MUST be synthetic and sanitized, and MUST contain **fake but realistically-shaped secret values** (e.g. `GITHUB_TOKEN=ghp_FAKE…`) so that the redaction tests prove a value is dropped, rather than proving that an absent value stayed absent.
- If a path turns out to be wrong, the cost is bounded to `roots.rs` and one fixture tree — but a wrong path that ships is a silent empty result, which is the failure mode CA-11 exists to forbid. A root that resolves to nothing MUST produce the existing `NotFound` warning path, not silence.

### 8. `args` values are never captured — only their count

**User decision.** `McpTransport::Stdio` carries `arg_count: usize`, not `args: Vec<String>`.

`args` is the one credential-bearing surface that neither `env` nor `headers` covers: a
`--token=ghp_…` flag or a positional secret is an ordinary way to configure a stdio
server, and capturing arguments verbatim would have reopened the leak that decision 1
closes. The three candidates were verbatim capture (legible, leaky), count only, and a
value-shaped heuristic; the heuristic was rejected outright, because a redaction rule that
works "usually" is worse than one that is structurally impossible to get wrong.

The cost is accepted and stated: an entry can no longer be identified by its arguments, so
`npx -y @modelcontextprotocol/server-github` reads as `npx` with 2 arguments. The product
goal for this cycle is an **inventory** — which MCP servers exist, and where they are
configured — not a reproduction of each server's launch command. `command` remains
captured because it is the recognizable part and is not a place credentials are configured.

`arg_count` is retained rather than dropped entirely because "configured with arguments" is
a meaningful distinction from "configured with none", and a count cannot carry a value.

### 9. Product goal for this cycle: inventory and location

**User statement, recorded so later phases do not over-build.** This cycle answers two
questions and no others: *which MCP servers are installed*, and *where each one is
configured*. Everything beyond that — auditing which servers hold credentials, comparing
divergent configurations of the same server name, or surfacing launch commands — is
explicitly not a goal.

A third question, *which AI client can use a given server*, is **derivable but not
modeled**. `SearchRoot` carries `id`, `path`, `kind` and `status` and deliberately **no
client-label field** (`model/location.rs:84`), so a client is recoverable today only by
convention from the root id string (`claude-*`, `opencode-*`, `codex-*`). Modeling it
properly means changing `SearchRoot`, which affects skills and agents equally and is
therefore its own cycle, to be explored once this one has shipped. This proposal MUST NOT
introduce a client field on `SearchRoot`.

## Approach

### Model: one enum variant, one value type, one field

- **`ComponentKind::Mcp`** joins the existing closed, non-`#[non_exhaustive]` enum (`model/component.rs:40-43`). Its own doc comment (`component.rs:34-36`) anticipates exactly this kind of reviewed breaking change. Blast radius is compiler-enforced and looks small: adapters and `consolidate.rs` construct `Component` rather than matching on kind.
- **`McpTransport`**, a new closed enum in `model/`, re-exported from `model/mod.rs`:
  - `Stdio { command: String, arg_count: usize, env_keys: Vec<String> }`
  - `Remote { url: String, header_keys: Vec<String> }`

  **Key names only, and no argument values at all.** The type has nowhere to put a value, so redaction is not a rule anyone can forget — see decision 8 for `arg_count`.
- **`Location.mcp_transport: Option<McpTransport>`** — `None` for every non-MCP location.
- **`SearchRootKind::Mcp`** — decision 5.

`model/` stays pure: these are plain data with zero I/O, no clock read, and no import outside the declared allow-list. All redaction happens in the **adapter layer, before a value ever reaches the model** — never in `model/`, and never at the IPC boundary.

### Identity: the existing rule, unchanged

`ComponentId::derive(ComponentKind::Mcp, server_key)`, where `server_key` is the config object's server-name key. Identity remains a function of `(kind, name)` alone — never of `Location`, never of file content, never of client. Normalization stays trim → NFC → lowercase.

This is the same move `opencode_agents.rs:159-210` already makes: assembling components from config keys rather than from file names. **No change to `identity.rs` is required.**

The consequence, stated so it is a recorded decision rather than an accident: a server named `github` in Claude Code and a *different* server also named `github` in Codex consolidate into one component with two locations. Both locations stay individually visible, each with its own transport, so nothing is hidden — and the alternative (a client discriminator in identity) would contradict `location.rs:40-42`'s stated intent and would break the "one server configured everywhere is one entry" behavior that makes this feature useful.

### Roots: one MCP root per client, appended to `ROOT_ORDER`

`roots.rs` gains one resolver per in-scope client, built with the existing patterns — `home` plus hardcoded relative segments, no `dirs`/`directories` crate, no environment read beyond the crate's single `home_dir()` call. Where a client has more than one user-level config file in a defined merge order, the resolver follows `opencode_agent_root`'s alias-grouping shape (`roots.rs:139-198`): several `scan_paths` under one logical root id.

`consolidate::ROOT_ORDER` grows from **8 to 11** entries, with `root_order_matches_the_roots_module_in_order` (`consolidate.rs:190-206`) kept synchronized. The new ids are **appended after the existing eight**, so field precedence for every existing skill and agent is provably unchanged, and the ordering among the three MCP roots (Claude Code → OpenCode → Codex) follows the order the existing roots already establish. Failing to update the pinning test is a loud failure, not a silent one, but it MUST be sequenced in the task list.

### Adapters: the `opencode_agents.rs` template, three times

New module(s) under `crates/vertice-core/src/` — either one `mcp.rs` dispatching per client or one module per client; **design's call**, and the project's precedent (`agents.rs`, `opencode_agents.rs`, `codex_agents.rs` are three standalone modules, deliberately not unified behind a trait per `agents.rs:8-11`) argues against extracting a shared abstraction on the first pass.

Each adapter MUST:

- Read only through `jsonc.rs` (Claude Code, OpenCode) or `toml.rs` (Codex). Reaching for `serde_json` or a raw `toml::Value` would violate the sole-importer containment the project already tests for. **No new format seam and no new dependency.**
- Be **infallible from the orchestrator's point of view**: a malformed or unreadable config yields a `ScanIssue` carrying the path, and every other adapter still produces its components. One bad file MUST NOT abort the scan (CA-12).
- Degrade a wrong-typed field to `None` + a `Warning`, following `opencode_agents.rs:166-189`, rather than dropping the entry or aborting.
- Emit `Component { kind: Mcp, scope: Scope::User, … }` with `origin: LocationOrigin::File`.
- **Redact at parse time.** The map's keys are collected; the map's values are never bound to a variable that outlives the parse, never formatted, and never logged.

### Scope of configuration read: user-level only

Only **user-level** configuration is read. Project-level MCP config (`.mcp.json` at a repository root, a project `opencode.json`, a trusted-project `.codex/config.toml`) is **out of scope**: the application emits `Scope::User` only today, has no project-root discovery, and adding one would expand this change into a different problem. `Scope::Project` / `Scope::Local` remain modeled-but-unproduced, exactly as they are for skills and agents.

Plugin-provided MCP servers are likewise out of scope, consistent with the existing plugin exclusion (CA-6/CA-14).

### App layer: nothing

MCP scanning folds into the existing `scan()` / `rescan()`. `crates/vertice-app/src/commands.rs` stays business-logic-free (`commands.rs:1-7`), `capabilities/default.json` stays at `core:default`, and no new IPC command is introduced. All I/O remains in the core.

### The binding contract (explicit obligation)

Adding `ComponentKind::Mcp`, `SearchRootKind::Mcp`, `McpTransport` and `Location.mcp_transport` regenerates files under `frontend/src/bindings/`. Bindings are produced **only** by `cargo test -p vertice-core` and MUST NEVER be hand-edited. CI regenerates them, runs `git add --intent-to-add` first so a *new* uncommitted binding is also caught, and fails on any diff. **The regenerated bindings MUST land in the same commit as the Rust change.**

`ComponentKind` becoming three variants is a **real breaking change for the frontend's exhaustive handling**, even though this cycle changes no frontend source. The frontend cycle MUST be told this in advance; an unhandled `"mcp"` value reaching the UI is the expected failure if it is not.

## Scope

### In Scope

- `model/component.rs`: the `ComponentKind::Mcp` variant.
- `model/location.rs`: `SearchRootKind::Mcp`, and `Location.mcp_transport: Option<McpTransport>` with its documented rationale.
- New `McpTransport` type (`Stdio` / `Remote`, key names only), re-exported from `model/mod.rs`.
- `roots.rs`: one MCP root resolver per in-scope client (Claude Code, OpenCode, Codex), user-level paths only.
- New MCP adapter module(s) in `crates/vertice-core/src/`, parsing through the existing `jsonc.rs` / `toml.rs` seams, with per-file `ScanIssue` isolation and parse-time redaction.
- `consolidate.rs`: `ROOT_ORDER` 8 → 11 (appended), with its pinning test synchronized. **No merge-logic change.**
- `scan.rs`: the new adapters wired into `scan_for`. `append_missing_root_issues` is already generic over `roots_scanned` and needs no change.
- `lib.rs`: the new `pub mod` line(s).
- New sanitized fixture trees per client under `crates/vertice-core/tests/fixtures/`, including fake-but-realistic secret values, a malformed config, a wrong-typed field, an empty/absent config, and a same-name-across-three-clients consolidation home.
- Fixture-first failing tests for every behavior above (`strict_tdd: true`).
- Regenerated `frontend/src/bindings/*.ts`, committed alongside the Rust change.

### Out of Scope

- **GitHub Copilot**, and any client outside the three named. Adding `ClientKind::Copilot` is a separate cycle.
- **Freshness / outdated detection for MCP servers** — explicit non-goal (decision 4). No `FreshnessSubject` variant, no `FreshnessCheck` entry.
- **Enabled/disabled state** as a modeled field (decision 6). Disabled servers are still emitted, undifferentiated.
- **Project-scope and Local-scope MCP configuration**, and any project-root discovery.
- **Plugin-provided MCP servers.**
- **Any frontend source change.** Only `frontend/src/bindings/` moves. The UI for MCP components is a separate cycle.
- **Connecting to, launching, pinging or introspecting any MCP server.** No process is spawned, no request is issued, no tool list is enumerated. This is a configuration inventory, not a runtime probe.
- **Reading any credential store**, including OpenCode's `~/.local/share/opencode/mcp-auth.json`. It is a second sensitive source and is deliberately never opened.
- **Any new Tauri command, capability, or `vertice-app` change.**
- **Any new Rust dependency**, including a URL-parsing crate. No new format seam.
- **Any refactor unifying the existing agent adapters** behind a shared trait.
- **Any write operation.** CA-16 is unchanged.

## Capabilities

### New Capabilities

- **`mcp-scanner`** — discovery of MCP servers configured in Claude Code, OpenCode and Codex: the user-level input paths per client, the root key per client, the mapping from a server-name config key onto `Component { kind: Mcp }`, the `McpTransport` shape, the **redaction requirement** (key names only; no value from `env`, `headers`, userinfo or a query string ever enters the model, the report, the IPC payload or the log), the `ScanIssue` taxonomy for malformed/unreadable/wrong-typed configuration, the "a disabled server is still emitted" rule, and the explicit statement that MCP components are never freshness subjects.

### Modified Capabilities

- **`domain-model`** — `ComponentKind` gains a third variant and stops being a two-valued set; `SearchRootKind` gains the mirroring variant and its 1:1 rationale is restated for three kinds; `Location` gains an optional, kind-conditional field and its documented responsibility widens beyond "where is the file"; the new `McpTransport` type joins the public surface; the generated-TypeScript-contract requirement must reflect the regenerated bindings. The `provenance_hint` opacity requirement is **re-affirmed, not relaxed** — MCP state MUST NOT be encoded there.
- **`scan-orchestration`** — the orchestrator's adapter list gains MCP adapters; the "one bad adapter does not abort the scan" property and the `NotFound`-root warning behavior MUST hold for them.
- **`workspace-architecture`** — the seam inventory is unchanged in count, but the **sole-importer MUST** now covers a third class of consumer, and a new invariant belongs here or in `mcp-scanner`: no secret-bearing value may cross the core's public surface.

**Expected NOT modified**: `duplicate-consolidation` (canonical root order is defined by reference to `roots.rs`, not by enumerating ids, and the merge rule is untouched — but see the `location_key` totality item under Open Decisions), `component-freshness` (decision 4), `skill-scanner`, `agent-scanner`, `opencode-agent-scanner`, `codex-agent-scanner`, `frontmatter-reader`, `client-installation-detector`, `inventory-ui`, `frontend-i18n`, `desktop-shell`, `user-settings`, `application-logging`, `ci-quality-gates`.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/model/component.rs` | Modified | `ComponentKind::Mcp` |
| `crates/vertice-core/src/model/location.rs` | Modified | `SearchRootKind::Mcp`; `Location.mcp_transport` |
| `crates/vertice-core/src/model/mcp.rs` (or equivalent) | **New** | `McpTransport`, key names only |
| `crates/vertice-core/src/model/mod.rs` | Modified | Re-export the new type |
| `crates/vertice-core/src/roots.rs` | Modified | One MCP root per in-scope client |
| `crates/vertice-core/src/mcp*.rs` | **New** | Adapters; parse-time redaction; per-file issue isolation |
| `crates/vertice-core/src/consolidate.rs` | Modified | `ROOT_ORDER` 8→11 + pinning test; **no logic change** |
| `crates/vertice-core/src/scan.rs` | Modified | New adapters wired into the orchestrator |
| `crates/vertice-core/src/lib.rs` | Modified | New `pub mod` line(s) |
| `crates/vertice-core/src/jsonc.rs`, `toml.rs`, `yaml.rs` | **Unchanged** | Reused, not replaced; no new seam |
| `crates/vertice-core/src/model/identity.rs` | **Unchanged** | Identity rule already covers a new kind |
| `crates/vertice-core/src/model/freshness.rs` | **Unchanged** | Explicit non-goal |
| `crates/vertice-core/src/skills.rs`, `agents.rs`, `opencode_agents.rs`, `codex_agents.rs` | **Unchanged** | No shared abstraction extracted |
| `crates/vertice-core/src/installations.rs` | **Unchanged** | No new `ClientKind`, no new install slot |
| `crates/vertice-core/tests/fixtures/` | New trees | Sanitized per-client MCP configs with fake secret values |
| `crates/vertice-core/tests/` | New + Modified | New suites; existing root-count and orchestrator-count assertions updated |
| `frontend/src/bindings/` | Regenerated | `ComponentKind.ts`, `Location.ts`, `SearchRootKind.ts`, new `McpTransport.ts`; never hand-edited |
| `frontend/src/` (source) | **Unchanged** | Separate cycle |
| `crates/vertice-app/` | **Unchanged** | No new command; `commands.rs` stays logic-free |
| `crates/vertice-app/capabilities/default.json` | **Unchanged** | `core:default` only |
| `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Expected unchanged** | No new dependency |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| **A secret value reaches the `ScanReport`, the IPC payload or the log file.** This is the feature's primary risk, not an edge case: `ScanReport` is logged by design, so a leak lands on disk | **High impact, and the whole point of the design** | The model offers **no field** capable of holding a value: `env_keys` / `header_keys` are `Vec<String>` of names. A fixture carrying a realistic fake token, asserting the token string appears **nowhere** in the serialized report, is a mandatory first failing test |
| A credential hides in a **remote URL** (userinfo or query parameter) and slips past the map-oriented redaction | **Med — a real bypass of the obvious rule** | Settled as an invariant, not an exception: userinfo, query and fragment MUST be stripped, or the URL omitted. Extraction rule closed in design; a `https://u:tok@host/mcp?apiKey=…` fixture asserts none of it survives |
| A credential hides in **`args`** (`--token=…`, a positional secret) — a case neither `env` nor `headers` covers | **Closed (user decision 8)** | `args` values are never captured. Only `arg_count: usize` is emitted, so there is no field an argument value could occupy. A fixture with `--token=ghp_…` in `args` asserts the token appears nowhere in the serialized report |
| The web-sourced config paths, formats or root keys are **wrong**, producing a silent empty result | **Med — explicitly unverified** | Decision 7. Design MUST close each against an authoritative schema or a sanitized real sample before fixtures are committed. A missing root still emits the existing `NotFound` warning, so "we did not look" is never silent |
| Fixture authoring reads the author's **real** config and commits a live credential | Med | Fixtures are synthetic and sanitized, with fake-but-realistically-shaped values. No test reads the real machine (CA-17). Reviewers MUST treat any plausible-looking secret in a fixture diff as a blocker |
| `ComponentKind` becoming three variants breaks downstream exhaustive handling, including the frontend once its cycle starts | **Med — a genuine breaking change** | The enum is closed by design and non-`#[non_exhaustive]`, so Rust breakage is compiler-enforced. The frontend cycle MUST be told before it plans; an unhandled `"mcp"` is the expected symptom |
| `Location` widening pollutes every skill and agent location with a `null` field in the TS binding | Med | Accepted and documented on the type. It mirrors the existing "not every kind uses every field" tolerance (`description: Option<String>`), and it is the only placement that survives consolidation without discarding a transport |
| `location_key` (`consolidate.rs:48`) sorts by `(root_rank, root_id, path)`; every MCP server from one client shares **one config file path**, so the key is no longer unique within a root | Med | Sorting stays total and deterministic (ties are stable, nothing panics), but design MUST confirm no behavior depends on uniqueness, or extend the key. A fixture with several servers in one file pins the ordering |
| `ROOT_ORDER` grows without its pinning test being updated | Low — loud failure | The pinning test is the guard; new ids are **appended**, so precedence for existing components is provably unchanged and the untouched reference-fixture pins are the tripwire |
| A contributor reaches for `serde_json` or a raw `toml::Value` inside an MCP adapter | Med | The seam containment tests already grep for the sole importer. Restate the MUST in the `mcp-scanner` capability so it is a spec rule, not folklore |
| The CA-15 <2s scan budget regresses because three more config files are opened on every scan | Low | Three small user-level files, no directory walk. The `reference-volume` fixture already enforces the budget and is the guard |
| The regenerated bindings are forgotten and CI's drift gate fails late | Low | Regenerate via `cargo test -p vertice-core` in the same commit; never hand-edit. `--intent-to-add` catches new files too |
| Reviewers read "disabled servers are shown as if enabled" as a bug | Med | Recorded as decision 6 with its rationale and its follow-up entry condition, and pinned by a fixture so the behavior is deliberate rather than incidental |

## Open Decisions

**Closed in this proposal:**

- **Redaction is key-names-only and structural** (user decision), and the rule extends to URL userinfo, query and fragment.
- **`args` values are never captured** — `arg_count: usize` only (user decision 8).
- **The cycle's goal is inventory and location** (user decision 9); "which client can use it" is derivable from the root id by convention and is deferred to its own cycle, which would change `SearchRoot`.
- **Transport lives on `Location`** (user decision), not on `Component`.
- **Claude Code + OpenCode + Codex only**; Copilot deferred to a cycle that first adds `ClientKind::Copilot` (user decision).
- **Freshness is an explicit non-goal** for `ComponentKind::Mcp`; no `FreshnessSubject` variant.
- **`SearchRootKind` gains a mirroring `Mcp` variant**, keeping the documented 1:1 invariant total.
- **Enabled/disabled state is not modeled this cycle**; a disabled server is still emitted, and it MUST NOT be encoded in `provenance_hint`.
- **Identity is `ComponentId::derive(Mcp, server_key)`** — the existing rule, no client discriminator, no change to `identity.rs`.
- **User-level configuration only**; `Scope::User` only; no project roots, no plugin sources.
- **Reuse `jsonc.rs` / `toml.rs`**; no new seam, no new dependency.
- **Fold into `scan` / `rescan`**; no new Tauri command, no capability change.
- **New root ids are appended last in `ROOT_ORDER`**, so existing field precedence is unchanged.
- **No frontend source change**; bindings regenerate and ship in the same commit.

**Committed to resolving in `sdd-design` — do not guess:**

- **The verified per-client config paths, formats and root keys** for Claude Code, OpenCode and Codex, closed against an authoritative schema or a sanitized real sample (decision 7). This gates every fixture.
- **The URL sanitization rule**: how userinfo, query and fragment are removed with no new dependency, what happens to a URL that does not parse under that rule (omitted, or the location emitted with no transport plus a `Warning`), and whether a port is preserved.
- ~~**`args` handling**~~ — closed by user decision 8: count only, no values.
- **Module layout**: one `mcp.rs` dispatching per client, or one module per client, given the project's three-standalone-adapters precedent.
- **Root id names and `scan_paths` grouping** per client, including whether a client's multiple user-level files share one logical root id in a defined merge order (the `opencode-agents` alias precedent) or become separate roots.
- **`location_key` totality** for MCP locations that share one config-file path, and whether the key must be extended.
- **The field mapping** from a server entry onto `Component`: what becomes `name`, what (if anything) becomes `description`, what becomes `provenance_hint` (which must stay opaque), and what an entry with an empty or non-string key yields.
- **The `ScanIssue` taxonomy**: which conditions are `Error` and which are `Warning` — unreadable file, invalid JSONC/TOML, root key present but wrong-typed, entry present but wrong-typed, entry matching neither stdio nor remote shape.
- **Whether the MCP roots join the `scan-orchestrator/complete` fixture immediately**, which shifts its pinned counts, or land in their own fixture homes first to keep the existing pins as an untouched tripwire.

**Deferred, with target:**

- **GitHub Copilot MCP support** — a later cycle, gated on `ClientKind::Copilot`.
- **Enabled/disabled state** — a follow-up cycle, gated on verified consistent semantics in at least two of the three in-scope clients.
- **Project-scope and Local-scope MCP configuration** — post-cycle, gated on project-root discovery existing at all.
- **Any MCP UI affordance** (transport column, secret-key listing, per-client grouping) — the separate frontend cycle.
- **OpenCode's `mcp-auth.json` and any other credential store** — no target. Deliberately never read.
- **Modeling "which AI client owns a search root"** as a field on `SearchRoot` rather than a root-id naming convention — its own cycle, to be explored after this one ships. It affects skills and agents equally, so it does not belong to an MCP change (decision 9).
- **Capturing `args` values** in any form — no target. Reopening it would reopen decision 1.

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. Sanitized fixtures and failing tests land before implementation, and every assertion runs against `crates/vertice-core/tests/fixtures/` — never the real machine (CA-17). These MUST exist and fail first:

- A stdio server fixture whose `env` carries a realistic fake token, asserting the **token string appears nowhere** in the serialized `ScanReport` while its **key name is present** in `env_keys`.
- A remote server fixture whose `headers` carry `Authorization: Bearer …`, asserting the same: `header_keys` contains `Authorization`, the value is absent everywhere.
- A remote server fixture whose `url` carries userinfo **and** a credential query parameter, asserting neither survives.
- One server name configured in **all three** clients, asserting **one** `Component` with **three** `Location`s, each carrying its **own** `McpTransport` — nothing merged away (`total_location_count_is_conserved`).
- Several servers declared in **one** config file, asserting a deterministic, total ordering of their locations.
- A malformed config per format (invalid JSONC, invalid TOML), asserting one `ScanIssue` carrying the path, the other clients' servers still emitted, and the scan not aborted (CA-12).
- A wrong-typed root key and a wrong-typed entry, asserting degradation to `None` + `Warning` rather than an abort.
- A home with **no** MCP configuration at all, asserting zero MCP components and the existing `NotFound` root-warning behavior — never an error, never an unexplained silence (CA-11).
- A server whose entry carries a disabled flag, asserting it **is** emitted (decision 6).
- An assertion that no MCP component appears in any `FreshnessCheck` (decision 4).
- The existing reference-fixture pins (`tests/fixtures/roots/reference/`) stay **byte-identical and green**, as the regression tripwire.
- The `reference-volume` read-only and <2s budget assertions stay green (CA-15, CA-16).

## Changed-Line Forecast

| Bucket | Est. lines |
|---|---|
| `model/`: `ComponentKind::Mcp`, `SearchRootKind::Mcp`, `McpTransport`, `Location` field, re-exports, docs | 70–110 |
| `roots.rs`: three MCP root resolvers | 60–100 |
| MCP adapter module(s), including redaction and issue taxonomy | 280–420 |
| `consolidate.rs`: `ROOT_ORDER` 8→11 + pinning test | 15–25 |
| `scan.rs` / `lib.rs` wiring | 25–45 |
| Sanitized fixtures (three clients, malformed, wrong-typed, multi-client, secret-bearing) | 120–200 |
| Tests (redaction, adapters, roots, consolidation, orchestrator, updated counts) | 320–470 |
| Regenerated bindings | 30–60 |
| **Total** | **~920–1430** |

**Decision needed before apply: Yes. Chained PRs recommended: Yes. 400-line budget risk: High.**

Natural slices, each independently green and independently revertible:

1. **Model + bindings** — `ComponentKind::Mcp`, `SearchRootKind::Mcp`, `McpTransport`, `Location.mcp_transport`, regenerated bindings, and the type-level redaction tests. No adapter, no root; nothing yet produces an MCP component.
2. **Roots + one client** (the strongest-precedent one) — its resolver, its adapter, its fixtures, `ROOT_ORDER` growth and the pinning test, orchestrator wiring.
3. **The remaining two clients** — one resolver, one adapter and one fixture tree each, reusing the established shape.

Final slicing is `sdd-tasks`'s call.

## Rollback Plan

Additive at every layer. Three-layer revert, in dependency order.

1. **Core (`vertice-core`)** — delete the MCP adapter module(s), the `McpTransport` type and every MCP fixture tree and test; revert the `lib.rs` and `model/mod.rs` lines; drop the three MCP roots from `roots.rs`; revert `ROOT_ORDER` to eight entries and its pinning test; revert the `scan.rs` wiring; remove `ComponentKind::Mcp`, `SearchRootKind::Mcp` and `Location.mcp_transport`. `identity.rs`, `consolidate.rs`'s merge logic, `installations.rs`, `skills.rs`, the three existing agent adapters and all three parser seams have nothing to revert — they were never edited.
2. **Bindings** — `cargo test -p vertice-core` regenerates them from the reverted types. **Never hand-edited, in either direction.** `McpTransport.ts` is a *new* file and MUST be deleted by hand on revert: `ts_rs` does not remove stale bindings, and the CI drift gate cannot see an orphan file. The `--intent-to-add` gate confirms the rest.
3. **Frontend source** — nothing to revert. No source change was made.
4. **Supply chain** — nothing to revert. No dependency is added, so `Cargo.toml`, `Cargo.lock` and `deny.toml` are expected untouched and `cargo deny check bans licenses` returns to its prior state automatically.

**`vertice-app` is untouched**, so the IPC surface and `capabilities/default.json` need no revert. **Migration: none** — nothing is persisted; `ScanReport` is rebuilt on every scan, so an old and a new report never coexist. A partial rollback (core reverted, bindings not) fails at TypeScript compile time or at the CI drift gate, not silently at runtime. Reverting the branch restores the exact pre-change state.

## Dependencies

- **`scan-orchestration`**, **`duplicate-consolidation`**, **`domain-model`** (living specs) — complete. This change extends them by pattern, not by redesign.
- **`opencode-agent-scanner`** (archived, `2026-08-2x`) — complete. Its "components from config object keys" adapter is the template this change follows; nothing in it is modified.
- **`codex-agent-scanner`** / `add-codex-client-support` (archived 2026-08-23) — complete. It shipped the `toml.rs` seam this change reuses, and its proposal already listed "Codex MCP servers" as out of scope. **This proposal supersedes that exclusion for MCP servers, and only for MCP servers** — the Codex config, auth, sessions and prompts trees stay unread.
- **`component-freshness`** (living spec) — complete and **explicitly not extended** (decision 4).
- **`client-installation-detector`** — untouched. This is precisely why Copilot is out of scope.
- **No blocking external dependency.** The frontend MCP cycle depends on this change; this change does not depend on it.

## Success Criteria

- [ ] A fixture whose `env` contains a realistic fake token yields a `Component { kind: Mcp }` whose location carries `McpTransport::Stdio` with the **key name** in `env_keys`, and the **token value appears nowhere** in the serialized `ScanReport`.
- [ ] A fixture whose `headers` contain `Authorization: Bearer …` yields `header_keys` containing `Authorization` and **no** bearer value anywhere in the report.
- [ ] A remote `url` carrying userinfo and a credential query parameter yields **neither** in the emitted transport.
- [ ] No type in `model/` is capable of holding an `env` or `header` **value**; redaction is enforced by shape, not by a call site.
- [ ] Nothing secret-bearing reaches the log: the token/bearer strings appear nowhere in the application log file after a scan of the secret-bearing fixture.
- [ ] One server name configured in Claude Code, OpenCode and Codex yields **one** `Component` with **three** `Location`s, each retaining its own `McpTransport`; `total_location_count_is_conserved` stays green.
- [ ] `ComponentId` for an MCP component derives from `(ComponentKind::Mcp, server_key)` alone — never from `Location`, path, client or file content; `identity.rs` is **unchanged**.
- [ ] A malformed config in one client yields exactly one `ScanIssue` at `IssueSeverity::Error` carrying its path, while every other client's servers are still emitted and the scan completes (**CA-12**).
- [ ] A wrong-typed root key or entry degrades to `None` plus a `Warning`, never an abort.
- [ ] A home with no MCP configuration yields zero MCP components, zero errors, and the existing `NotFound` root warning — never an unexplained empty result (**CA-11**).
- [ ] A server carrying a disabled flag **is** emitted (decision 6), pinned by a fixture.
- [ ] No MCP component appears in any `FreshnessCheck`; `FreshnessSubject` is **unchanged** (decision 4).
- [ ] `SearchRootKind` mirrors `ComponentKind` 1:1 with three variants each (decision 5).
- [ ] No MCP state is written into `provenance_hint`; the opacity requirement (`domain-model/spec.md:131-133`) still holds.
- [ ] `ROOT_ORDER` has eleven entries in `roots.rs` order with the MCP ids **appended**, and `root_order_matches_the_roots_module_in_order` passes.
- [ ] `crates/vertice-core/tests/fixtures/roots/reference/` is **byte-identical** and its existing pinned assertions stay green untouched.
- [ ] Every MCP path is composed from the passed-in `home` plus hardcoded relative segments; no `dirs`/`directories` import and no environment read beyond the crate's single `home_dir()` is introduced.
- [ ] Parsing goes exclusively through `jsonc.rs` and `toml.rs`; the sole-importer containment tests stay green; **no regular expression** parses any MCP config.
- [ ] `Cargo.toml`, `Cargo.lock` and `deny.toml` are byte-identical; `cargo deny check bans licenses` passes; `vertice-core` imports nothing from `tauri`.
- [ ] `crates/vertice-app/` and `capabilities/default.json` are byte-identical; no new IPC command exists.
- [ ] `frontend/src/` outside `bindings/` is byte-identical; bindings are regenerated by `cargo test -p vertice-core`, committed in the same change, and never hand-edited.
- [ ] Every assertion runs against versioned fixtures; no test reads the author's real configuration, sets an environment variable, or launches an MCP server (**CA-17**).
- [ ] No `File::create`, `OpenOptions::write` or equivalent is introduced; nothing is written outside the application data directory (**CA-16**); the `reference-volume` before/after tree hash is unchanged and the <2s budget holds (**CA-15**).
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, and `npm run lint && npm run check && npm run test && npm run build` all pass on the three-platform CI matrix.

## Proposal question round

The interactive question round could not be run from this phase. These are the product questions whose answers would change the proposal, each with the assumption currently written into it. Answer, correct, or skip any of them — a second round is available, and any answer that contradicts an assumption below MUST amend this proposal before `sdd-design` locks the type shapes.

| # | Question | Assumption currently written in |
|---|---|---|
| 1 | ~~Is the product goal an inventory, or a credential audit?~~ | **Answered.** Inventory: which servers are installed and where (decision 9). Key names are captured because they are the safe part of the answer, not because a security audit is being built |
| 2 | Showing the **key names** of secrets (e.g. `GITHUB_TOKEN`, `Authorization`) is itself a mild disclosure. Is that acceptable, or should the model carry only a **count** of secret-bearing keys? | Key names are shown. They are the useful part ("this server needs a GitHub token") and are not themselves credentials |
| 3 | A server configured in three clients with three **different** commands shows as one entry with three locations. Is that the right product answer, or does one row hide a real divergence the user needs to notice? | One entry, three locations, each with its own transport — nothing discarded, but no divergence indicator either |
| 4 | Decision 6 means a **disabled** server looks exactly like an enabled one. Is over-reporting the right error to make for now, or is an unmarked disabled server actively misleading? | Over-report. Silently dropping a forgotten-but-disabled server is the worse failure for an inventory tool |
| 5 | ~~Should a stdio server's `args` be shown at all?~~ | **Answered.** Count only, no values (decision 8). Legibility is traded for a leak surface that cannot exist |
| 6 | Is user-level-only configuration the right first slice, or does the user expect project-level `.mcp.json` (the git-shared, team-visible one) to be the more interesting half? | User-level only, consistent with the app emitting `Scope::User` everywhere today |
| 7 | Does shipping MCP scanning **backend-only**, with no UI until a later cycle, deliver anything the user can see — or should the two cycles land together? | Backend-only is a coherent slice: it is testable, revertible, and the binding contract makes the frontend cycle mechanical |
