# Proposal: Domain Model and Type Contract

> Plan trace: **T2** (Phase 0 — Foundations) of `internal-docs/plan-desarrollo-poc.md`.
> Acceptance criteria: enables **CA-2**, **CA-3**, **CA-4** (aggregation by identity, T8), **CA-13** (embedded components with no path, T5) and **CA-5** (merged agents, T5/T6). Extends the **CA-17** gate surface with a generated-contract check. No CA closes here; T2 makes them modelable.

## Intent

T3–T8 all produce or consume these types; none can start without them. The plan is explicit: modeling "one file = one component" here invalidates T5, T6 and T8 simultaneously. Two identity traps must be closed now, not later: a component with no disk path must stay distinguishable from one with a path, and one component with N locations must not become N entities. Nothing exists yet — `vertice-core` is a YAML seam plus a smoke test.

## Scope

### In Scope
- Eight core types: `Component`, `ComponentKind`, `Scope`, `Location`, `SearchRoot`, `ClientInstallation`, `ScanIssue`, `ScanReport`. Plain data, `Serialize`/`Deserialize`/`TS` derives, no behavior beyond trivial constructors.
- Deterministic `Component.id` derived from `(kind, normalized name)`, with the normalization rule written down.
- Error taxonomy boundary: `ScanIssue` (recoverable, per item, inside `Ok(ScanReport)`) vs a `thiserror` enum (orchestration failure, surfaced as `Err`).
- `ts-rs` wiring plus the CI in-sync gate.
- Unit tests, fixture-free, proving T2's four acceptance criteria.

### Out of Scope
- Any disk I/O, `walkdir`, frontmatter parsing (T3).
- The aggregation algorithm itself (T8) — T2 fixes only the identity that makes it possible.
- Tauri command registration and IPC wiring (T10).
- Non-UTF-8 path handling *code* — T2 records only the contract.
- SQLite/persistence, platform data-directory resolution (T7/T14).

## Capabilities

### New Capabilities
- `domain-model`: the eight types, identity derivation, scope population, error taxonomy boundary, and the Rust→TypeScript contract.

### Modified Capabilities
- `ci-quality-gates`: adds a generated-contract-in-sync requirement.

## Approach

Types land in a new `crates/vertice-core/src/model/` module.

**Generation: `ts-rs`, not `tauri-specta`.** The Tauri-2 line of `tauri-specta` is still pre-release (`2.0.0-rc.25`); an unstable generator weakens exactly the acceptance criterion it would serve ("fails compilation or CI, not runtime"). `ts-rs` is stable, framework-agnostic, usable inside a pure library, and keeps T2 mechanically independent of T10. Typed command bindings remain a T10-scoped decision.

**CI verification: check in the generated `.ts` + `git diff --exit-code`**, in the ubuntu-only `quality` job alongside `cargo deny check bans`. Generation is not OS-path-sensitive (T2 does zero disk I/O), the diff is reviewable in the PR, and an out-of-sync file is a hard CI failure.

**Identity: deterministic, never content-based.** `alcance-poc-vertice.md:63` records `issue-creation` appearing under the same name with divergent content across three roots. Content-based identity would split that single duplicate into two components and invalidate T8.

**`Location.path: Option<PathBuf>`** crosses IPC as `string | null`. The serialization contract assumes UTF-8-representable paths; the non-UTF-8 case becomes a `ScanIssue` from T3 onward.

**`ComponentKind` and `Scope` are closed enums**, no `#[non_exhaustive]`: exhaustiveness checking is the mechanism that makes "the model admits exactly these variants" compiler-verified, and there are no out-of-tree consumers.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/Cargo.toml` | Modified | `ts-rs` dependency |
| `crates/vertice-core/src/lib.rs` | Modified | `pub mod model;` |
| `crates/vertice-core/src/model/` | New | Eight types + error enum |
| `crates/vertice-core/tests/` | New | Contract tests |
| `frontend/src/bindings/` | New | Generated, checked-in TS |
| `.github/workflows/ci.yml` | Modified | In-sync gate in `quality` |
| `openspec/specs/domain-model/` | New | Capability spec |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| `ts-rs` native `PathBuf` mapping unconfirmed | Med | Verify against `ts-rs` source in design; fall back to `#[ts(type = "string")]` per field |
| `ts-rs` license outside `deny.toml` allow-list; CI runs only `check bans` | Low-Med | Run `cargo deny check licenses` manually at apply; close or defer the gap in writing, matching T1's precedent |
| `ts-rs` raises the transitive MSRV floor above 1.88 | Low | Empirical `RUSTUP_TOOLCHAIN=1.88 cargo check`, same discipline as T1 |
| Normalization rule for `name` left vague, breaking T8 grouping | Med | Design must fix case folding and Unicode form explicitly, with tests |
| Contributors forget to regenerate bindings | Med | Make generation part of `cargo test`; CI diff fails loudly |

## Open Question

Whether the `thiserror` enum also derives `Serialize`/`TS` for structured IPC errors. Tauri requires the `Err` variant to be `Serialize`. Retrofitting is cheap, but leaving it unanswered forces T10 to invent the answer. To be resolved in design, not silently.

## Rollback Plan

Additive across all three layers; no consumer exists yet.

- **Core**: revert `src/model/` and the `ts-rs` dependency. The T1 YAML seam is untouched.
- **App (`vertice-app`)**: zero impact — no commands registered, no types imported.
- **Frontend**: delete `src/bindings/`. Nothing imports it yet.
- **CI**: remove the in-sync step from `quality`; the other four jobs are unaffected.

Reverting the branch restores the exact post-T1 state. If the identity rule proves wrong after T8, the blast radius is one derivation function plus its tests — the type shape itself does not change.

## Dependencies

- T1 (workspace, CI, `serde_norway`) — complete and archived.
- Blocks T3–T8, T10.

## Success Criteria

- [ ] A `Component` with `Location.path = None` is representable and distinguishable from one with `Some(path)`.
- [ ] A `Component` with N `locations` exists as a single entity; two parses of the same `(kind, name)` yield equal ids.
- [ ] `scope` is present and populated on every `Component`; `Scope::User` is the only value the PoC emits.
- [ ] Changing a Rust type without regenerating bindings fails CI (`git diff --exit-code`), verified by negative-path test.
- [ ] The error taxonomy boundary and the non-UTF-8 path contract are written in `design.md`.
- [ ] `cargo deny check bans` and the full three-platform matrix stay green; MSRV 1.88 holds.
