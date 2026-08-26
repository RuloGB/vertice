# Tasks: Add MCP Server Scanning (backend)

> Trace: closes the proposal's committed open items via `design.md` (approved, not modified here).
> Bound by the four spec files in `specs/`: `mcp-scanner`, `domain-model`, `scan-orchestration`,
> `workspace-architecture`. Authority for every decision below is `design.md`; do not reopen its
> sections. `§n` references point there. `strict_tdd: true` — every implementation task is preceded
> by the RED test that justifies it.
> Scope: backend only (`crates/vertice-core`, `crates/vertice-app`). No frontend work in this change.

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | ~920–1430 (design §13) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | Five slices + one cross-cutting ROOT_ORDER/gates tail, each independently green and independently revertible |
| Delivery strategy | chained |
| Chain strategy | slice-by-dependency |

## Deliberate deviation from `design.md` §13's slice-order suggestion — recorded here because `sdd-tasks` owns this call

`design.md` §13 suggests OpenCode goes first in the per-client slice because it exercises the most
machinery (two-file merge, array-`command` mapping). But `design.md` §5.3 also **pins a specific final
`ROOT_ORDER` array**, `["claude-mcp", "opencode-mcp", "codex-mcp"]`, and the user constraint below (and
`consolidate.rs`'s own pinning-test precedent, `V5`) requires each new root id to be appended to
`ROOT_ORDER` **in the same commit** as the `roots.rs` function that produces it — you cannot list an id
in `ROOT_ORDER` before its resolver function exists, because the pinning test builds its expectation by
*calling* `roots::<fn>(&home).root.id`. Landing OpenCode first would therefore append `opencode-mcp`
before `claude-mcp` exists, producing final order `[..., opencode-mcp, claude-mcp, codex-mcp]` —
contradicting §5.3's stated precedence `claude-mcp < opencode-mcp < codex-mcp` without ever reordering
anything (reordering after the fact is exactly the Codex-precedent bug §5.3 itself warns about).

**Decision: implement client slices in the order Claude Code → OpenCode → Codex**, matching §5.3's
declared final array exactly, and append each root id to `ROOT_ORDER` (with its pinning-test update) in
the same task as that client's `roots.rs` resolver — never deferred, never batched. This is a
sequencing decision within `sdd-tasks`'s explicitly deferred authority (§13: "Deliberately deferred to
`sdd-tasks`... the order of the RED tests inside each slice, and whether slice 4 is one PR or two"); it
does not reopen or contradict any decided content of §5–§8.

## Slice Plan

| Slice | Goal | Design refs | Depends on |
|---|---|---|---|
| 0 | Global RED anchors — the top-level failing tests from §12, written first, closed out progressively | §12 | — |
| 1 | Model + bindings: `ComponentKind::Mcp`, `SearchRootKind::Mcp`, `McpTransport`, `Location.mcp_transport`, 13 struct-literal sites | §2 | — |
| 2 | Shared redaction primitives: `src/mcp.rs`, `src/json_merge.rs`. Includes **E1** (3×3 discrimination matrix) and **E2** (panic-surface hardening) | §3, §4, §6.2, §6.3 | Slice 1 |
| 3 | Claude Code MCP adapter end-to-end (two-file root, first `ROOT_ORDER` entry). Includes **E4** | §5.1–§5.2, §6.3, §6.4, §7 | Slice 2 |
| 4 | OpenCode MCP adapter end-to-end (two-file root, second `ROOT_ORDER` entry). Includes **E3** (OpenCode leg) and **E2**'s invariant-test extension | §5.1–§5.2, §6.3, §6.4, §7 | Slice 3 |
| 5 | Codex MCP adapter end-to-end (single-file root, third `ROOT_ORDER` entry, final array = 11). Includes **E3** (Codex leg) and **E2**'s invariant-test extension | §5.1, §6.2, §6.3, §6.4, §7 | Slice 4 |
| 6 | Consolidation regression pins (§8), cross-client orchestrator fixtures, IPC/path regression (§9) | §8, §9 | Slice 5 |
| 7 | Reference-fixture tripwire + read-only audit (CA-16, V11) | §10.3, §11 | Slice 5 |
| 8 | Gates | — | Slice 7 |

Units 3, 4, 5 have a hard sequential dependency on each other **only through `ROOT_ORDER`** — each
appends after the previous. Their adapter code, fixtures and per-client tests are otherwise independent
and could be developed in parallel branches, but the `ROOT_ORDER`/pinning-test commit for each MUST
land in that Claude → OpenCode → Codex order to avoid a reorder-after-the-fact.

---

## Slice 0: Global RED Anchors (`§12`, written first, closed out across slices 2–6)

Write these as failing tests before any adapter code exists. Most will stay RED across several later
slices and are closed out (GREEN) where noted — this is intentional under strict TDD: the anchor is
written once, at the top, and the slice that finally makes it pass references it back here instead of
re-declaring it.

- [x] 0.1 (RED) `fake_token_in_env_never_reaches_the_serialized_report` — stub in `crates/vertice-core/tests/mcp_redaction.rs`, referencing the not-yet-existing `claude/stdio-secret` fixture. Stub written and confirmed RED (panics with a "pending Slice 3" message, since the fixture and `mcp_claude::scan` do not exist yet). Closed GREEN in Slice 3 (3.9).
- [x] 0.2 (RED) `fake_token_in_env_never_reaches_the_application_log` — stub in `crates/vertice-app/tests/mcp_log_redaction.rs`, using `log_scan_report_with`'s emission-capturing closure (`commands.rs:59-62`). Stub written and confirmed RED. Closed GREEN in Slice 3 (3.10). Scoped honestly per §12 item 2: today's logger emits `root.path` and `record.label`, not `ScanIssue.reason` — the test's claim over `reason` is forward-looking, and the doc comment says so.
- [x] 0.3 (RED->GREEN) `dirty_url_is_reduced_to_scheme_host_and_port` — in `crates/vertice-core/src/mcp.rs`'s test module, expecting exactly `https://mcp.example.test:8443` from the load-bearing fixture URL in §10.2. Closed GREEN in Slice 2 (2.3).
- [x] 0.4 (RED->GREEN) `userinfo_containing_a_path_delimiter_is_rejected_not_truncated` — the direct regression test for the verified leak (§3.1), covering `/`, `?`, `#` variants. Closed GREEN in Slice 2 (2.3).
- [x] 0.5 (RED->GREEN) `unparseable_url_yields_no_transport_and_a_warning_without_echoing_the_url` — closed GREEN in Slice 3 against `mcp_claude::scan`.
- [x] 0.6 (RED->GREEN) `token_bearing_argument_yields_only_a_count` — closed GREEN in Slice 3 against `mcp_claude::scan` and `claude/stdio-secret`.
- [x] 0.7 (RED->GREEN) `entry_without_a_type_field_is_discriminated_structurally` — closed GREEN: Claude half in Slice 3 (`claude/settings-json-only`), Codex half in Slice 5 (`codex/remote-secret`).
- [x] 0.7a (RED->GREEN) `unusable_command_with_a_valid_url_falls_back_to_remote_not_none` — the §6.3/§7.1 usability-discrimination regression test, in `crates/vertice-core/src/mcp.rs`'s test module. Closed GREEN in Slice 2 (2.5, as part of **E1**'s matrix) and exercised per-client in Slices 3–5.
- [x] 0.8 (RED->GREEN) `opencode_array_command_maps_to_command_plus_arg_count` — closed GREEN in Slice 4 against `mcp_opencode::scan` and `opencode/complete`.
- [x] 0.9 (RED->GREEN) `same_server_name_in_three_clients_yields_one_component_with_three_transports` — closed GREEN in Slice 6 (6.4), the first point at which all three adapters exist.
- [x] 0.10 (RED->GREEN) `malformed_config_yields_one_error_with_a_fixed_reason_and_no_parser_text` — closed GREEN in Slice 3 against `mcp_claude::scan` and `claude/malformed`; equivalent per-client coverage in `mcp_opencode_scanner.rs`/`mcp_codex_scanner.rs`.
- [x] 0.11 (RED->GREEN) `home_without_any_mcp_configuration_yields_no_components_and_no_errors` (**CA-11**) — closed GREEN in Slice 6 (6.5), once all three adapters are wired into `scan_for`.

---

## Slice 1: Core Model — `ComponentKind::Mcp`, `SearchRootKind::Mcp`, `McpTransport`, `Location.mcp_transport` (`domain-model` delta)

- [x] 1.1 (RED) Update `crates/vertice-core/tests/model_contract.rs`: the `ComponentKind` exhaustive-match test gains an `Mcp` arm; the (new or existing) `SearchRootKind` exhaustive-match test gains an `Mcp` arm; all 7 `Location` struct literals gain `mcp_transport: None`. This does not compile yet — that is the RED state for a closed-enum addition in this codebase's convention. Confirmed RED (11 compile errors) before implementing the model changes. Added two new exhaustive-match tests (`component_kind_is_exhaustively_matchable_without_a_wildcard_arm`, `search_root_kind_is_exhaustively_matchable_without_a_wildcard_arm`) since neither existed yet.
- [x] 1.2 (GREEN) Create `crates/vertice-core/src/model/mcp.rs` (§2): `McpTransport` enum, exactly `Stdio { command: String, arg_count: usize, env_keys: Vec<String> }` and `Remote { url: String, header_keys: Vec<String> }`, deriving `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS`, `#[ts(export, export_to = "../../../frontend/src/bindings/")]`, `#[serde(rename_all = "camelCase")]`. No `#[non_exhaustive]`. Import allow-list only: `serde`, `ts_rs`.
- [x] 1.3 (GREEN) Modify `crates/vertice-core/src/model/component.rs`: add `ComponentKind::Mcp`; widen the `Component` doc comment from "a skill or an agent" to name three kinds.
- [x] 1.4 (GREEN) Modify `crates/vertice-core/src/model/location.rs`: add `SearchRootKind::Mcp`; add `Location.mcp_transport: Option<McpTransport>`; widen `Location`'s doc comment to state it now answers "where" and, for one kind, "how is it reached" (§2).
- [x] 1.5 (GREEN) Modify `crates/vertice-core/src/model/mod.rs`: re-export `McpTransport`.
- [x] 1.6 (GREEN) Modify `crates/vertice-core/src/model/identity.rs`: add the one exhaustive match arm `ComponentKind::Mcp => "mcp"` (`identity_prefix`, V7 — the only exhaustive kind match in `src/`).
- [x] 1.7 (GREEN) Fix all 13 `Location` struct-literal construction sites broken by the new field, adding `mcp_transport: None` to each (V8): `src/agents.rs` ×2, `src/codex_agents.rs` ×1, `src/consolidate.rs` ×1 (the test helper `fn location(...)`), `src/opencode_agents.rs` ×1, `src/skills.rs` ×1, and the 7 in `tests/model_contract.rs` already touched by 1.1. Did **not** touch `model/location.rs:14`'s type definition or the `consolidate.rs` helper's return-type brace — V8's correction honoured.
- [x] 1.8 Confirmed 1.1's RED tests now pass; confirmed the crate compiles.
- [x] 1.9 Ran `cargo test -p vertice-core` to regenerate bindings. Four files changed/new as expected: `ComponentKind.ts`, `SearchRootKind.ts`, `Location.ts`, new `McpTransport.ts`. `Component.ts` also changed (doc-comment-only diff, propagated from 1.3's widened `Component` doc comment — expected collateral, not a leak). `UserSettings.ts` shows as modified in `git status` but has an empty `git diff` (pre-existing LF/CRLF line-ending normalization noise, unrelated to this change). Bindings are regenerated in this same apply pass, matching the "same commit" convention.
- [x] 1.10 Confirmed no other `bindings/*.ts` file changed in content.
- [x] 1.11 Noted for the record (no code change): `ComponentKind` becoming three variants is a breaking change for the frontend's exhaustive handling. `frontend/src/` outside `bindings/` was not touched.

---

## Slice 2: Shared Redaction Primitives — `src/mcp.rs` + `src/json_merge.rs` (`mcp-scanner`, `workspace-architecture` deltas)

No I/O in this slice. No adapter yet produces an MCP component.

### 2.1 `json_merge.rs` — the move (§4.3)

- [x] 2.1.1 (RED) Confirmed `opencode_agents.rs`'s existing merge tests currently pass (baseline, not a new test) before moving them.
- [x] 2.1.2 (GREEN) Created `crates/vertice-core/src/json_merge.rs`. Moved `merge_all` and `merge_two` verbatim from `opencode_agents.rs`, `pub(crate)`. Moved all **ten** named unit tests verbatim: `base_only_key_survives`, `overlay_only_key_survives`, `shared_key_partial_override_merges_per_field_not_per_object`, `array_vs_anything_overlay_replaces_wholesale`, `scalar_vs_object_overlay_replaces`, `object_vs_scalar_overlay_replaces`, `overlay_null_replaces_and_does_not_delete`, `fold_over_zero_inputs_yields_nothing`, `fold_over_one_input_yields_identity`, `keys_differing_only_by_case_are_not_normalized_before_merging`. Confirmed none left behind.
- [x] 2.1.3 (GREEN) Modified `opencode_agents.rs` to call `crate::json_merge::{merge_all, merge_two}` instead of its own local definitions. Zero behaviour change.
- [x] 2.1.4 Added `mod json_merge;` to `crates/vertice-core/src/lib.rs`.
- [x] 2.1.5 Confirmed the existing `opencode-agents` fixture suite (24 tests) stays green — proof the move changed nothing.

### 2.2 `sanitize_url` (§3)

- [x] 2.2.1 (RED) Closed out anchors 0.3 and 0.4 in `crates/vertice-core/src/mcp.rs`'s test module: the full §3.2 table as individual unit tests, including every rejection row (empty/whitespace/control chars, no `://`, invalid scheme, tail containing `@` for `/`/`?`/`#` variants, malformed IPv6 bracket, malformed port) and every acceptance row (port preserved, path/query/fragment/userinfo dropped, IPv6 host, percent-encoded `%40` passed through unmodified per §3.2's documented residual). Confirmed RED (function did not exist) before implementing.
- [x] 2.2.2 (GREEN) Implemented `pub(crate) fn sanitize_url(raw: &str) -> Option<String>` in `crates/vertice-core/src/mcp.rs` per §3.1's seven numbered steps. Pure, I/O-free, regex-free, dependency-free — uses only `str::split_once`/`rsplit_once`/`split_at`/`strip_prefix`/`find`/`chars`, never bracket indexing (satisfies 2.6.3's `clippy::indexing_slicing` deny ahead of schedule).
- [x] 2.2.3 Confirmed 2.2.1's tests now pass, including 0.3/0.4 (21 `sanitize_url`-related unit tests, all green).

### 2.3 `KeyNames`, `ArgCount`, `Lenient` (§6.2)

- [x] 2.3.1 (RED) Unit tests proving: `KeyNames` deserializes any map keeping only key names, values consumed via `IgnoredAny`, no constructor accepting a value; `ArgCount` deserializes any sequence keeping only its length; `Lenient<T>` degrades a wrong-typed field to `WrongType` instead of failing the whole document. Tested via `serde_json::from_str` (already a dev-dependency; JSON is another self-describing format exercising the same generic `Deserialize` implementation Codex's TOML path will use).
- [x] 2.3.2 (GREEN) Implemented `KeyNames(Vec<String>)`, `ArgCount(usize)`, `Lenient<T> { Value(T), WrongType }` in `crates/vertice-core/src/mcp.rs` via hand-written `serde::de::Visitor` implementations. `Lenient<T>`'s scalar visit methods (`bool`/`i64`/`u64`/`f64`/`str`) re-deserialize into `T` via `serde::de::value`'s scalar deserializers (a standard, dependency-free technique); `visit_seq`/`visit_map` drain via `IgnoredAny` and degrade to `WrongType` unconditionally, since `Lenient<T>` is only ever used for scalar fields.
- [x] 2.3.3 Confirmed 2.3.1's tests pass.

### 2.4 `McpScan` (§4.2)

- [x] 2.4.1 (GREEN) Added `pub(crate) struct McpScan { pub roots: Vec<SearchRoot>, pub components: Vec<Component>, pub issues: Vec<ScanIssue> }` to `crates/vertice-core/src/mcp.rs`, shared by all three per-client adapters (one root, N components, N issues per client). Not yet constructed anywhere — no adapter exists until Slice 3; flagged via a scoped `#![allow(dead_code)]` at the top of `mcp.rs` with a comment naming the future consumer, removed once the first adapter lands.

### 2.5 E1 — the transport discrimination rule is not total; enumerate the 3×3 matrix once

`design.md` §13.5 (E1, confirmed by both adversarial-review judges): §6.3's prose and §7.1's table each
state the discrimination rule piecemeal, and patching individual rows has produced an uncovered cell
twice in a row. The fix is one enumerated matrix, not another row.

- [x] 2.5.1 (RED) Write `crates/vertice-core/src/mcp.rs` test module: one test per cell of the full
      `command` {absent, present-usable, present-unusable} × `url` {absent, present-valid,
      present-unsanitizable} matrix — **9 cells**, named systematically
      (`matrix_command_absent_url_absent`, `matrix_command_absent_url_valid`,
      `matrix_command_absent_url_unsanitizable`, `matrix_command_usable_url_absent`,
      `matrix_command_usable_url_valid`, `matrix_command_usable_url_unsanitizable`,
      `matrix_command_unusable_url_absent`, `matrix_command_unusable_url_valid`,
      `matrix_command_unusable_url_unsanitizable`). Each asserts one `(transport, issue count, reason)`
      triple. Cross-reference against §7.1's existing rows and 0.7a — most cells already have a
      documented expected outcome there; the two cells that do not (found by round-3 review, "usable
      `command` + present-but-invalid `url`") get their outcome decided here, not silently inferred:
      **decision — usable `command` wins outright, `Stdio`, plus the existing "URL refused" `Warning`
      reason is NOT also emitted** (the command is fine and was used; the invalid URL is surplus
      configuration, not a fault worth a second issue) — record this explicitly in the test's doc
      comment as the resolution chosen here, since it is genuinely new content beyond what design.md
      states.
- [x] 2.5.2 (GREEN) Implemented `pub(crate) fn discriminate_transport(command: CommandInput, url: UrlInput) -> TransportOutcome` in `crates/vertice-core/src/mcp.rs`. `CommandInput`/`UrlInput` carry each client's already-extracted, normalized command-usability and url-validity inputs; `TransportOutcome { transport: Option<McpTransport>, issue: Option<TransportIssue> }` carries at most one transport and at most one issue *category* — the exact `ScanIssue.reason` text (with server key and client label interpolated, per §7.2's allow-list) is left to the per-client adapter that will call this function starting Slice 3, so this module stays free of any `ScanIssue` construction and any interpolation to police. Confirmed 2.5.1 passes (all 9 matrix cells). Confirmed 0.7a passes.
- [x] 2.5.3 (Design amendment — **PERFORMED 2026-08-25**, before implementation began. `design.md` §6.3 now carries the full 3 x 3 matrix as the single normative statement, with the `command` usable + `url` unsanitizable cell resolved as `Stdio` with zero Warnings, matching 2.5.1's decision. §7.1's table was aligned to it: the two `url` rows are now conditioned on the command's state, and a new row records the silent cell. Nothing is left to reconcile at PR time.) Historical note: `design.md` §6.3's prose paragraph ("The rule, identical in all three adapters...") and §7.1's table both restate this matrix piecemeal and now disagree with the two cells resolved in 2.5.1. This phase's write scope is `tasks.md` only — `design.md` may not be edited here. **Action required before or alongside this slice's PR**: either get explicit sign-off to amend `design.md` §6.3/§7.1 as part of the implementing commit (replacing the prose with a reference to the matrix and its resolved cells), or file the amendment as a fast-follow proposal. Do not let this slice merge with `design.md` and the shipped code silently disagreeing.

### 2.6 E2 — `.unwrap()`/`.expect()`/`panic!` clause is neither enforceable nor complete as written

`design.md` §13.5 (E2, confirmed by both judges): the grep-based invariant is over-broad (this crate's
convention is inline `#[cfg(test)]` modules; `opencode_agents.rs` alone has 11 `.expect(...)` calls in
its own test module) and under-broad (misses `command[0]` indexing and unguarded arithmetic).

- [x] 2.6.1 (GREEN, eliminates the gap rather than naming it) — closed in Slice 4: `mcp_opencode.rs::extract_command_input` uses `items.first()`/`items.len().saturating_sub(1)`, never `command[0]` indexing. Confirmed by `clippy::indexing_slicing` (deny scope, 2.6.3) passing clean on the file.
- [x] 2.6.2 (GREEN) Confirmed every `arg_count`/count computation in `mcp.rs` uses `saturating_add` (`ArgCount`'s visitor) — no unguarded arithmetic. Re-confirmed for `mcp_claude.rs`/`mcp_opencode.rs`/`mcp_codex.rs`: all three re-use `mcp.rs`'s primitives or the hand-rolled, always-draining `mcp_codex.rs` visitors (`LenientArgCount`, `LenientKeyNames`) built the same way.
- [x] 2.6.3 (GREEN) Added `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` as an inner attribute at the top of `crates/vertice-core/src/mcp.rs`. Added `#[allow(clippy::unwrap_used, clippy::expect_used)]` on that file's own `#[cfg(test)] mod tests` block only. `sanitize_url` and `sanitize_host_port` deliberately avoid `[]` slicing/indexing syntax entirely (using `split_at`/`split_once`/`strip_prefix`/`get` instead), so the deny lint has nothing to allow-list in non-test code.
- [x] 2.6.4 (GREEN, repeated in Slices 3–5) — `mcp_claude.rs`, `mcp_opencode.rs` and `mcp_codex.rs` each carry the same `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` inner attribute plus a scoped `#[allow(clippy::unwrap_used, clippy::expect_used)]` on their own test module, mirroring `mcp.rs`. `cargo clippy --workspace --all-targets -- -D warnings` is clean across all four modules (confirmed below).

**Defect found and fixed while implementing Slice 5, recorded here since it belongs to E2's territory (a panic-adjacent hazard, not the literal clause):** an earlier version of `crate::mcp::Lenient<T>` generalized `visit_seq`/`visit_map` to retry a real `T::deserialize` over the mismatched shape before falling back to `WrongType`. For a `T` that itself expects a seq/map (e.g. `ArgCount`, or a per-entry struct), a failed retry does not fully drain the underlying `MapAccess`/`SeqAccess`, corrupting the surrounding TOML/JSON document's parse and producing a spurious "trailing characters" error — caught by two new unit tests in `mcp.rs` (`lenient_field_over_a_map_degrades_when_inner_shape_mismatches`, and the pre-existing `lenient_field_wrong_shape_array_degrades_too` failing after the change). **Resolution:** `Lenient<T>`'s `visit_seq`/`visit_map` were reverted to their original Slice-2 shape (unconditional full drain via `IgnoredAny`, `WrongType`) — `Lenient<T>` stays scoped to genuinely scalar `T`, exactly as its Slice-2 doc comment already said. `mcp_codex.rs` instead defines three dedicated, hand-rolled, always-fully-draining `Deserialize` impls (`McpServersField`, `CodexEntrySlot`, `LenientArgCount`, `LenientKeyNames`) for its seq-/map-shaped lenient fields, each documented with the drain requirement. Pinned by `wrong_typed_args_table_does_not_corrupt_the_rest_of_the_document` in `mcp_codex.rs`.
- [x] 2.6.5 Confirmed `cargo clippy --workspace --all-targets -- -D warnings` passes clean against `mcp.rs` today, enforcing the deny scope. Full confirmation across all four MCP modules is Slice 8's gate, once `mcp_claude.rs`/`mcp_opencode.rs`/`mcp_codex.rs` exist.

---

## Slice 3: Claude Code MCP Adapter — the first `ROOT_ORDER` entry (`mcp-scanner` delta)

Two-file root (M1/M2): `~/.claude.json` (base) + `~/.claude/settings.json` (overlay). Depends on the assumption **A10** (settings.json wins at the leaf) shipping unconfirmed — cosmetic blast radius only per §0.4.

- [x] 3.1 (RED) Add fixture homes under `crates/vertice-core/tests/fixtures/mcp/claude/`: `complete`, `two-files-partial-override`, `settings-json-only`, `stdio-secret`, `remote-secret`, `remote-dirty-url`, `remote-userinfo-ambiguous-url`, `remote-unparseable-url`, `malformed`, `malformed-secret-adjacent`, `root-key-wrong-type`, `entry-wrong-type`, `entry-unusable-command-valid-url`, `empty-root-key`, `absent`, `blank-key`. Every fake secret contains the literal `FAKE` (§10.2's fixed vocabulary); every directory carries at least one file; no symlink, ever.
- [x] 3.2 (RED) Write `crates/vertice-core/tests/mcp_claude_scanner.rs` covering every §10.4 row for the `claude/*` fixtures: stdio-secret (env key survives, value nowhere), remote-secret (header key survives, value nowhere), remote-dirty-url (port preserved, everything else stripped), remote-userinfo-ambiguous-url (`None`, not a truncated authority — closes 0.4 for Claude), remote-unparseable-url (`None` + `Warning`, no echoed URL — closes 0.5), malformed (one fixed-reason `Error`, no parser text — closes 0.10 for Claude), malformed-secret-adjacent (the `FAKE` guard empirically proves §7.2's no-interpolation rule on the JSONC path), root-key-wrong-type (zero components, one `Warning`), entry-wrong-type (`mcp_transport: None`, one `Warning`), entry-unusable-command-valid-url (`Remote`, one `Warning`, never `None` — closes 0.7a for Claude), settings-json-only (no `type` key, still `Stdio` — closes 0.7's Claude half), two-files-partial-override (overlay overriding one field does not erase the base's `command`; both locations carry the merged effective transport per §5.2), absent (zero components, zero issues, existing `NotFound` root warning), blank-key (`name: ""`, `id: "mcp:"`, emitted, no issue).
- [x] 3.3 (RED) **E4 — trace `claude/empty-root-key`, do not leave it orphaned.** Add `empty_mcp_servers_object_yields_zero_components_and_no_issue` to `mcp_claude_scanner.rs`, tracing it to §7.1's "MCP root key present but empty" row (`mcpServers: {}` — a well-formed, explicitly empty inventory is an answer, not a fault; zero components, zero issues). **RESOLVED 2026-08-25, before implementation began**: `design.md` §10.4 now carries a row for `claude/empty-root-key` tracing it to the "MCP root key present but empty" behaviour (zero components, zero issues). The table is no longer stale and nothing needs flagging at PR time.
- [x] 3.4 (RED) Add `entry_without_a_type_field_is_discriminated_structurally_claude` closing the Claude half of anchor 0.7.
- [x] 3.5 (GREEN) Create `crates/vertice-core/src/roots.rs` additions (§5.1, §5.3): generalize `resolve_single` to take `suffix: &[&str]` (four existing call sites updated mechanically, no behaviour change); add `resolve_pair(home, id, kind, base, overlay)` helper naming the existing status fold; add `claude_mcp_root(home: &Path) -> ResolvedRoot` built from `resolve_pair(home, "claude-mcp", SearchRootKind::Mcp, [".claude.json"], [".claude", "settings.json"])`.
- [x] 3.6 (GREEN) Create `crates/vertice-core/src/mcp_claude.rs` (§6.1, §6.3, §6.4): three-phase shape (read → redact → assemble), root key `mcpServers`, `type` field never read, structural discrimination via 2.5.2's `discriminate_transport`, `command`/`args` mapping per §6.3's normative table (Claude row), field mapping per §6.4 (`description`/`provenance_hint` always `None`, `scope: User`, one `Location` per declaring file sharing the merged effective transport). Uses `json_merge` for the two-file merge (§5.2). Apply 2.6.4's deny-lint pattern to this file from creation.
- [x] 3.7 Add `pub mod mcp_claude;` and `mod mcp;` (if not already added in Slice 2) to `crates/vertice-core/src/lib.rs`.
- [x] 3.8 Confirm 3.2–3.4's RED tests now pass. Confirm 0.5, 0.6, 0.7 (Claude half), 0.7a (Claude), 0.10 (Claude) are closed.
- [x] 3.9 (GREEN) Close out anchor 0.1: `fake_token_in_env_never_reaches_the_serialized_report`, now runnable against `claude/stdio-secret`.
- [x] 3.10 (GREEN) Close out anchor 0.2: `fake_token_in_env_never_reaches_the_application_log` in `crates/vertice-app/tests/`, now runnable end to end for the Claude leg. Keep the doc-comment hedge about `reason` being forward-looking (0.2's note).
- [x] 3.11 (GREEN — `ROOT_ORDER`, same commit as 3.5) Modify `crates/vertice-core/src/consolidate.rs`: `ROOT_ORDER` grows from 8 to **9** entries, appending `"claude-mcp"` after `"codex-agents"`. Update `root_order_matches_the_roots_module_in_order`'s pinning test in the **same commit**: add `expected.push(crate::roots::claude_mcp_root(&home).root.id.0.clone());` after the existing pushes. Never land the `roots.rs` change and this pinning-test update in separate commits.
- [x] 3.12 (GREEN) Wire `crate::mcp_claude::scan(home)` into `crates/vertice-core/src/scan.rs`'s orchestrator: `extend` `roots_scanned`, `components`, `issues` — the fourth adapter class, matching the pattern of the existing three `extend`s.
- [x] 3.13 (RED then GREEN) Extend `scan-orchestrator/complete` fixture: add secret-free `.claude.json` (and `.claude/settings.json` for M1 coverage) carrying one plainly-named stdio server. Update `scan.rs` orchestrator test counts: `roots_scanned.len()` 8 → 9, `components.len()` 12 → 13. `report.issues.is_empty()` stays broken until Slice 5 (two more clients still missing) — track this explicitly, do not assert it green here.
- [x] 3.14 (RED then GREEN) Extend `scan-orchestrator/missing-root-client`: no fixture change; assertions move `roots_scanned.len()` 8 → 9, path-less `Warning`s 8 → 9.
- [x] 3.15 (GREEN) Extend `scan-orchestrator/reference-volume` with a deliberately oversized, secret-free `.claude.json` per §9.3's budget measurement. Confirm its three existing assertions (V10) stay green, including tree-snapshot equality (**CA-16**) and `duration_ms < 2000`.
- [x] 3.16 Update `tests/model_contract.rs`: confirm the `ComponentKind`/`SearchRootKind` exhaustive-match tests still compile with no change needed here (already closed in Slice 1).

---

## Slice 4: OpenCode MCP Adapter — the second `ROOT_ORDER` entry (`mcp-scanner` delta)

Two-file root (V4/C1/M8): `opencode.json` (base) + `opencode.jsonc` (overlay), same merge order the
agent root already ships and pins. `command` is an array with no separate `args` key (M9/C3) — the
one client whose shape genuinely differs from the other two.

- [x] 4.1 (RED) Add fixture homes under `crates/vertice-core/tests/fixtures/mcp/opencode/`: `complete`, `two-files-partial-override`, `empty-command-array`, `stdio-secret`, `remote-secret`, `malformed`, `malformed-secret-adjacent`, `root-key-wrong-type`, `absent`, plus **new**, per **E3**: `entry-unusable-command-valid-url`.
- [x] 4.2 (RED) Write `crates/vertice-core/tests/mcp_opencode_scanner.rs` covering every §10.4 row for `opencode/*`: stdio-secret, remote-secret, malformed (fixed reason, no parser text), malformed-secret-adjacent (`FAKE` guard on the JSONC path, second instance), root-key-wrong-type, absent, empty-command-array (`command: []` ⇒ `mcp_transport: None` + `Warning` — the "no arguments" vs "no command" boundary), two-files-partial-override (same deep-merge proof as Claude's, second instance, using the shipped-and-pinned `opencode.json`/`opencode.jsonc` order).
- [x] 4.3 (RED) **E3 — OpenCode leg of the "identical in all three adapters" claim.** Add `unusable_command_with_a_valid_url_falls_back_to_remote_not_none_opencode` against `opencode/entry-unusable-command-valid-url` (a wrong-typed/empty `command` array element 0, or a non-string `command[0]`, plus a valid `url`), asserting `Remote` + one `Warning`, never `None` — closes E3's OpenCode gap: today only `claude/entry-unusable-command-valid-url` and its RED test exist, and OpenCode's array-command shape is structurally different enough that an implementer could satisfy every Claude fixture and still get this wrong here with no failing test before this task.
- [x] 4.4 (RED) Add `opencode_array_command_maps_to_command_plus_arg_count` closing anchor 0.8: `["npx", "-y", "pkg"]` ⇒ `command: "npx"`, `arg_count: 2`.
- [x] 4.5 (GREEN) Modify `crates/vertice-core/src/roots.rs`: add `opencode_mcp_root(home: &Path) -> ResolvedRoot` built from `resolve_pair(home, "opencode-mcp", SearchRootKind::Mcp, [".config", "opencode", "opencode.json"], [".config", "opencode", "opencode.jsonc"])`.
- [x] 4.6 (GREEN) Create `crates/vertice-core/src/mcp_opencode.rs` (§6.1, §6.3, §6.4): root key `mcp`; `command: [str...]` with no separate `args` key — `command.first()` (per 2.6.1, never `command[0]`) → `Stdio.command`, `command.len().saturating_sub(1)` → `arg_count`, elements past index 0 counted never inspected; `type` never read; env map key `environment` (**A4**, unconfirmed — an absent key yields no keys and no issue, ship as designed); headers map key `headers` (**A5′**, unconfirmed, same treatment). Apply 2.6.4's deny-lint pattern from creation.
- [x] 4.7 Add `pub mod mcp_opencode;` to `crates/vertice-core/src/lib.rs`.
- [x] 4.8 Confirm 4.2–4.4's RED tests now pass. Confirm E3's OpenCode leg is closed.
- [x] 4.9 (GREEN — `ROOT_ORDER`, same commit as 4.5) `ROOT_ORDER` grows from 9 to **10**, appending `"opencode-mcp"` after `"claude-mcp"`. Update the pinning test in the same commit: `expected.push(crate::roots::opencode_mcp_root(&home).root.id.0.clone());`.
- [x] 4.10 (GREEN) Wire `crate::mcp_opencode::scan(home)` into `scan.rs`'s orchestrator (third `extend` triple for MCP, fifth adapter class overall).
- [x] 4.11 (RED then GREEN) Extend `scan-orchestrator/complete`: add secret-free `mcp` block to the existing `.config/opencode/opencode.json`. Counts move: `roots_scanned.len()` 9 → 10, `components.len()` 13 → 14. `issues.is_empty()` still not restorable — one client remains.
- [x] 4.12 (RED then GREEN) Extend `scan-orchestrator/missing-root-client`: `roots_scanned.len()` 9 → 10, path-less `Warning`s 9 → 10.

---

## Slice 5: Codex MCP Adapter — the third `ROOT_ORDER` entry, final array = 11 (`mcp-scanner` delta)

Single-file root (M5/M6): `~/.codex/config.toml`. Only consumer of `Lenient`/`KeyNames`/`ArgCount`
built in Slice 2. Codex's remote transport (`{ url }`, no `command`, no `type` anywhere) is the row
the design flagged as least certain and M6 closed affirmatively.

- [x] 5.1 (RED) Add fixture homes under `crates/vertice-core/tests/fixtures/mcp/codex/`: `complete`, `empty-args`, `args-non-string-element`, `stdio-secret`, `remote-secret`, `malformed`, `malformed-secret-adjacent`, `root-key-wrong-type`, `entry-field-wrong-type`, `absent`, plus **new**, per **E3**: `entry-unusable-command-valid-url`.
- [x] 5.2 (RED) Write `crates/vertice-core/tests/mcp_codex_scanner.rs` covering every §10.4 row for `codex/*`: stdio-secret, remote-secret (`{ url }` only, no `command`, no `type` — closes anchor 0.7's Codex half), malformed (fixed reason, no `toml`-crate `Display` text embedded — a direct regression for §7.2's TOML-side hazard), malformed-secret-adjacent (`FAKE` guard proving no-interpolation on the TOML path empirically), root-key-wrong-type (root table `mcp_servers`, confirmed `mcpServers` absent), entry-field-wrong-type (one wrong-typed scalar field degrades via `Lenient`, does **not** escalate to a file-level `Error`), empty-args (`args: []` ⇒ `arg_count: 0`, valid `Stdio`, **not** degraded — M5's real observed shape), args-non-string-element (`args: ["--flag", 42]` ⇒ `arg_count: 2`, valid `Stdio`, **no** `Warning` — counted never inspected), absent.
- [x] 5.3 (RED) **E3 — Codex leg of the "identical in all three adapters" claim.** Add `unusable_command_with_a_valid_url_falls_back_to_remote_not_none_codex` against `codex/entry-unusable-command-valid-url`, closing E3 fully (Claude Slice 3, OpenCode Slice 4, Codex here — all three legs now fixture-verified, not asserted by prose alone).
- [x] 5.4 (RED) Add `entry_without_a_type_field_is_discriminated_structurally_codex` closing anchor 0.7's Codex half — a `{ url }`-only entry with no `type` anywhere yields `Remote`.
- [x] 5.5 (GREEN) Modify `crates/vertice-core/src/roots.rs`: add `codex_mcp_root(home: &Path) -> ResolvedRoot` built from `resolve_single(home, "codex-mcp", SearchRootKind::Mcp, [".codex", "config.toml"])` (now taking `&[&str]` per 3.5).
- [x] 5.6 (GREEN) Create `crates/vertice-core/src/mcp_codex.rs` (§6.1, §6.2, §6.3, §6.4): root key `mcp_servers` (snake_case); `command: str` + `args: [str...]` (args absent ⇒ 0, empty array observed and real per M5); no `type` field exists — structural discrimination is the only option; env map key `env`, a nested table; headers map key `http_headers` (**A8′**, unconfirmed, absent-key-is-not-an-error makes it safe to ship). Uses `Lenient<T>`, `KeyNames`, `ArgCount` from `src/mcp.rs` — the TOML path's redaction is enforced by the deserializer, never allocating a value at all. Apply 2.6.4's deny-lint pattern from creation.
- [x] 5.7 Add `pub mod mcp_codex;` to `crates/vertice-core/src/lib.rs`.
- [x] 5.8 Confirm 5.2–5.4's RED tests now pass. Confirm E3 is fully closed (all three legs). Confirm anchor 0.7 is fully closed (both legs).
- [x] 5.9 (GREEN — `ROOT_ORDER`, same commit as 5.5) `ROOT_ORDER` grows from 10 to **11** (final), appending `"codex-mcp"` after `"opencode-mcp"`. Update the pinning test in the same commit: `expected.push(crate::roots::codex_mcp_root(&home).root.id.0.clone());`. Confirm the final array matches §5.3's literal `[..., "claude-mcp", "opencode-mcp", "codex-mcp"]` exactly.
- [x] 5.10 (GREEN) Wire `crate::mcp_codex::scan(home)` into `scan.rs`'s orchestrator (final `extend` triple, sixth and last adapter class).
- [x] 5.11 (RED then GREEN) Extend `scan-orchestrator/complete`: add secret-free `.codex/config.toml` with one plainly-named stdio server. Counts move to their **final** values: `roots_scanned.len()` 10 → **11**, `components.len()` 14 → **15**, `report.issues.is_empty()` **restored** — all three MCP clients now present and secret-free.
- [x] 5.12 (RED then GREEN) Extend `scan-orchestrator/missing-root-client`: `roots_scanned.len()` 10 → **11**, path-less `Warning`s 10 → **11**, `client_presence` unaffected (no client-presence change from MCP roots).
- [x] 5.13 Confirm `scan-orchestrator/corrupt-skill`, `corrupt-codex-agent`, `codex-claude-same-skill` fixtures need no change — none pins a root count.

---

## Slice 6: Consolidation Regression Pins, Cross-Client Fixtures, IPC/Path Regression (§8, §9)

- [x] 6.1 Confirm (no code change, §8's conclusion): `location_key`'s `(root_rank, root_id, path)` needs no extension. `ROOT_ORDER` and its pinning test are the only `consolidate.rs` changes across Slices 3–5 — verified by re-reading `consolidate.rs` after Slice 5: only `ROOT_ORDER` and its pinning test differ; `location_key`, `merge_into`, `consolidate` untouched.
- [x] 6.2 (RED->GREEN) Added `shared/several-servers-one-file` fixture (`.claude.json` with three servers, `gamma`/`alpha`/`beta`, declared out of alphabetical order) and `several_servers_in_one_file_consolidate_in_deterministic_order` in `tests/mcp_redaction.rs`: asserts consolidated order `alpha, beta, gamma`, and that an independent second scan+consolidate run yields the identical order.
- [x] 6.3 (RED->GREEN) Added `shared/same-name-three-clients` fixture home (server `github`, different `command` per client: `claude-github-cli`/`opencode-github-cli`/`codex-github-cli`) and `shared/disabled-flagged` (using the verified `enabled: bool` shape on a Codex and an OpenCode entry, M7/M9 — not an invented flag).
- [x] 6.4 (GREEN) Closed out anchor 0.9: `same_server_name_in_three_clients_yields_one_component_with_three_transports` in `tests/mcp_redaction.rs` — one `Component`, three `Location`s, three transports, ordered `claude-mcp → opencode-mcp → codex-mcp` (now meaningful, since `ROOT_ORDER`'s final array is in place). Confirmed the conservation property (sum of location counts equals input length) stays true across the three MCP adapters, matching `total_location_count_is_conserved`'s shape.
- [x] 6.5 (GREEN) Added `scan-orchestrator/mcp-same-name-three-clients`, the new orchestrator home for `specs/scan-orchestration/spec.md:43-47`, and `mcp_same_name_three_clients_consolidates_into_one_component_three_transports` in `src/scan.rs`'s own test module (the only place with access to the private `scan_for`). Closed out anchor 0.11: `home_without_any_mcp_configuration_yields_no_components_and_no_errors` (**CA-11**) in `tests/mcp_redaction.rs`, using `shared/no-mcp-anywhere` — asserts all three adapters yield zero components/zero issues and every resolved root is `NotFound`; the orchestrator-level path-less `Warning` per root is exercised separately by the existing `missing-root-client` test.
- [x] 6.6 (GREEN) Confirmed via `disabled_flagged_entries_are_still_emitted_with_no_provenance_hint` in `tests/mcp_redaction.rs`: `shared/disabled-flagged`'s Codex and OpenCode entries are both emitted, each with `provenance_hint: None`, despite `enabled: false`.
- [x] 6.7 Verified (no code change, §9.1): `capabilities/default.json` is byte-identical (`git diff --stat` empty). No new `#[command]`/`invoke_handler` line anywhere in `crates/vertice-app/`; `commands.rs`'s only diff across this whole change is a test-only addition inside its `#[cfg(test)] mod tests` block (anchor 0.2, Slice 3) — `scan`/`rescan` remain thin pass-throughs, no new IPC command, no new event, no capability change.
- [x] 6.8 Verified (no code change, §9.2): no `#[cfg(target_os/windows/unix/macos)]` branch anywhere in `claude_mcp_root`, `opencode_mcp_root`, `codex_mcp_root`, `resolve_pair`, or the generalized `resolve_single` — the only `#[cfg(unix)]` in `roots.rs` is pre-existing and unrelated. Dotfiles under `home` on all three platforms, exactly like `.claude`, `.config/opencode`, `.codex` already are.

---

## Slice 7: Reference-Fixture Tripwire and Read-Only Audit (CA-2/CA-3/CA-4, CA-16, V11)

- [x] 7.1 Confirmed `crates/vertice-core/tests/fixtures/roots/reference/` is byte-identical to its pre-change state (`git status --porcelain` empty).
- [x] 7.2 Confirmed `reference_fixture_tree_yields_69_entries`, its 25-id corroborator, and the CA-3/CA-4 (22-with-3-locations / 3-with-1-location) assertions in `skill_scanner.rs` remain textually unmodified (`git diff --stat` empty for that file); ran the full suite (16 tests, all green).
- [x] 7.3 Verified (no code change, V11): no MCP root resolves inside `tests/fixtures/roots/reference/` — it contains only `.config/opencode/skills/**` (no `opencode.json`/`opencode.jsonc`), no `.claude.json`, no `.codex/config.toml` anywhere in the tree.
- [x] 7.4 Grepped `crates/vertice-core/src/` and `crates/vertice-core/tests/` for `File::create`, `OpenOptions::write`, `fs::write`, `create_dir*`, `remove_*`, `symlink*` — every match is `symlink_metadata`/`read_link` (read-only) or the pre-existing `read_only_audit.rs` audit's own pattern list; confirmed no new mutation match. Also ran `read_only_audit.rs`'s mechanical audit test directly: green, `findings.is_empty()`.
- [x] 7.5 Confirmed no fixture under `tests/fixtures/mcp/**` contains a symlink or a junction anywhere (`find ... -type l` empty).
- [x] 7.6 Ran the two `FAKE`-guard assertions from §10.2 across the **full** fixture tree: `fake_guard_holds_across_the_full_secret_bearing_fixture_tree` (`tests/mcp_redaction.rs`) folds every secret-bearing case from all three clients into one serialized `components`+`issues` pair; `mcp_secrets_never_reach_the_scan_report_log_across_the_full_fixture_tree` (`crates/vertice-app/src/commands.rs`) does the same over `log_scan_report_with`'s emitted text. Both green.
- [x] 7.7 Confirmed `Cargo.toml`, `Cargo.lock`, `deny.toml` are byte-identical to their pre-change state (`git status --porcelain` empty for all three).

---

## Slice 8: Gates

- [x] 8.1 Ran `cargo fmt --all --check` (clean after one `cargo fmt --all` auto-fix pass on this slice's new files); `cargo clippy --workspace --all-targets -- -D warnings` (clean); `cargo test --workspace --locked` (all green — see the apply report's full output). `cargo deny` is **not resolvable on PATH** in this execution environment (`error: no such command: 'deny'`) — reported plainly rather than claimed as passing, per this task's own instruction.
- [x] 8.2 Re-ran `cargo test -p vertice-core` and diffed `frontend/src/bindings/`: `ComponentKind.ts`, `SearchRootKind.ts`, `Location.ts`, `McpTransport.ts` (new) differ as expected; `Component.ts` carries only the doc-comment-only collateral from 1.9; `UserSettings.ts` shows as modified in `git status` but has an empty `git diff` (pre-existing CRLF/LF normalization noise, confirmed unrelated). No further drift accumulated across Slices 2–7.
- [x] 8.3 Confirmed `capabilities/default.json`, `deny.toml` are byte-identical to their pre-change state (`git status --porcelain` empty for both). `crates/vertice-app/`'s only diff across the whole change is test-only additions inside `#[cfg(test)] mod tests` in `commands.rs` (anchors 0.2 and this slice's 7.6 whole-tree log guard) — no source behaviour change, no new IPC surface (re-confirms 6.7/7.7).
- [x] 8.4 Confirmed `Cargo.toml`'s `rust-version = "1.88"`, the CI `MSRV: "1.88"` env, and `rust-toolchain.toml`'s `channel = "1.97.1"` (a newer pin, consistent with MSRV being a floor) still agree. No new dependency was added by this change.
- [x] 8.5 Ran `tests/mcp_no_error_interpolation_invariant.rs` explicitly: 5 tests, all green — `no_scan_issue_reason_interpolates_beyond_the_fixed_allow_list` scans all four MCP modules' `format!`/`write!` calls (test modules and the two URL-building helpers excluded) for any interpolated identifier outside the allow-list; `no_mcp_module_calls_unwrap_expect_or_panic_outside_its_own_tests` re-confirms §7.2's panic-surface hardening textually; `no_mcp_module_imports_the_jsonc_or_toml_crate_directly` confirms the seam-containment check; two meta-tests confirm the test-module-stripping and function-stripping helpers are not silently vacuous. One calibration fix was needed and applied: `mcp.rs`'s own module doc comment named `.unwrap()`/`.expect()` in literal method-call syntax in prose, which the textual scan correctly (if inconveniently) matched — reworded to prose without the literal syntax, exactly the documented convention `tests/toml_seam_invariant.rs` already establishes for its own alias-name check.
- [x] 8.6 From `frontend/`: `npm run lint` (clean), `npm run test` (14 files / 124 tests, all green — Vitest does not typecheck the bindings, confirmed), `npm run build` (succeeds, 459ms). **`npm run check` FAILS with 11 type errors** in `ComponentKindPage.svelte`, `App.test.ts`, `filterComponents.test.ts`, and `inventory.test.ts` — every one is `ComponentKind`/`Location` now requiring an `"mcp"` arm / `mcpTransport` field the existing frontend source does not yet handle. This is **not** a regression introduced by Slices 6–8: it is the exact, named consequence of Slice 1's binding regeneration, documented in `design.md` §9.1 as a genuine breaking change ("an unhandled `"mcp"` is the expected symptom, and the frontend cycle must be told before it plans") and explicitly out of scope per this file's own header ("Scope: backend only... No frontend work in this change"). **RESOLVED 2026-08-26 by explicit user decision — scope amendment, documented not silent.** The 11 errors were fixed with a deliberately minimal type unblock, because CI's `frontend` job runs `npm run check`: leaving it red would have made this branch unmergeable, and a backend change that leaves `main` red is not done. Two edits only: `ComponentKindPage.svelte`'s `KIND_ROUTE` gains `mcp: "mcp"` (a one-line change — the `mcp` route already existed in `navigation.ts:13` and in the i18n catalogs), and the `Location` fixtures in `App.test.ts`, `filterComponents.test.ts` and `inventory.test.ts` gain `mcpTransport: null`. **This is a knowing, approved amendment to the "backend only" scope stated in `proposal.md` and `design.md` §9.1.** It touches one frontend source file. It is NOT frontend feature work: no MCP page, no transport rendering, no UI for secret key names. That remains a separate cycle, recorded as P4 in `internal-docs/pendientes-desarrollo.md`. Verified after the fix: `npm run lint`, `npm run check` (225 files, 0 errors), `npm run test` (124), `npm run build` — all green.
