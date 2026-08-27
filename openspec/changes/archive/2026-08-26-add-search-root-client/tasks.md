# Tasks: Model the AI Client That Owns a Search Root

> Scope follows `proposal.md`, `design.md`, and the `domain-model`/`inventory-ui` deltas. Strict TDD is enabled: each implementation task starts with its RED test. All tests use versioned fixtures or in-memory paths only. CA-16 remains binding: add no writes, filesystem mutations, capabilities, commands, or dependencies.

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | ~350–450 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1: Core + bindings → PR 2: Frontend |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|---|---|---|---|
| 1 | Core model, adapters, tests, bindings | PR 1 | Independently green; fixture-only/read-only verification included |
| 2 | Client grouping and detail-page UI | PR 2 | Targets PR 1; frontend tests and i18n included |

## Phase 1: Core + Bindings (Slice 1)

- [x] 1.1 **RED — model contract:** Add failing serde round-trip tests for `SearchRoot.client` `Some(ClaudeCode)` and `None`/JSON `null` in `crates/vertice-core/src/model/location.rs` and `crates/vertice-core/tests/model_contract.rs`; update the 7 existing `Location` literals there and 3 inline `SearchRoot` literals to expose compile failures. Satisfies domain-model: SearchRoot ownership, Location ownership, binding contract; CA-16: no I/O.
- [x] 1.2 **GREEN — model and bindings:** Add append-last `client: Option<ClientKind>` fields and contracts in `crates/vertice-core/src/model/location.rs`; update all 16 `Location` construction sites (9 under `src/`, 7 in `tests/model_contract.rs`) plus the `SearchRoot` literals in `crates/vertice-app/src/commands.rs`. Run `cargo test -p vertice-core` to regenerate only `frontend/src/bindings/SearchRoot.ts` and `Location.ts`; never hand-edit bindings.
- [x] 1.3 **RED → GREEN — root mapping:** In `crates/vertice-core/tests/model_contract.rs` and `crates/vertice-core/src/roots.rs` tests, pin all 11 root IDs to the design mapping (`agents-skills` = `None`, all others = owning `ClientKind`), then update `resolve_single`, `resolve_pair`, `skill_roots`, `agent_roots`, `resolve_opencode`, and MCP roots in `crates/vertice-core/src/roots.rs`. Verify with nonexistent-home paths; CA-16: probes remain read-only.
- [x] 1.4 **RED → GREEN — adapter propagation:** Add fixture tests for Claude skill `Some` and shared `None`, plus referential integrity over the `complete` fixture in `crates/vertice-core/tests/model_contract.rs`; then update `crates/vertice-core/src/skills.rs`, `agents.rs`, `opencode_agents.rs`, `codex_agents.rs`, `mcp_claude.rs`, `mcp_opencode.rs`, `mcp_codex.rs`, and test helper `consolidate.rs`. Change only `skills::walk_one` and `agents::emit_embedded_components` signatures as designed; preserve consolidation and CA-16.

## Phase 2: Frontend (Slice 2)

- [x] 2.1 **RED → GREEN — grouping helper:** Create `frontend/src/lib/clientGroups.test.ts` with arbitrary-order, deduplication, fixed-order, count, empty-input, and shared/null cases; implement `frontend/src/lib/clientGroups.ts` with `ClientGroup`, `groupLocationsByClient`, and hardcoded `CLIENT_LABEL`. Satisfies inventory-ui grouping/proper-noun requirements; no filesystem access.
- [x] 2.2 **RED → GREEN — detail pages and i18n:** Add one wiring test per page in `frontend/src/lib/pages/AgentDetail.test.ts`, `SkillDetail.test.ts`, and `McpDetail.test.ts`; update `AgentDetail.svelte`, `SkillDetail.svelte`, `McpDetail.svelte`, and `frontend/src/lib/i18n/catalogs.ts` so groups render counts, `null` uses `aiClients.shared`, and zero locations retain `aiClientsEmpty`. Verify `npm run lint && npm run check && npm run test && npm run build`; CA-16 unchanged.
