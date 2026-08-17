# Design: Domain Model and Type Contract

> Trace: **T2** / enables CA-2, CA-3, CA-4, CA-5, CA-13; extends the CA-17 gate surface.
> Proposal: `openspec/changes/domain-model-type-contract/proposal.md`. Exploration: `explore.md`.
> `rules.design` coverage: core data model (§2), core/Tauri isolation for the CLI pathway (§1), IPC contract surface (§8), per-OS paths (§7), `ScanIssue` taxonomy (§6).

## 1. Technical Approach

Nine plain-data types plus one error enum land in a new `crates/vertice-core/src/model/` module. They carry `Serialize`/`Deserialize`/`TS` derives and no behavior beyond `ComponentId::derive`. `ts-rs` emits one `.ts` file per type into `frontend/src/bindings/`; the files are committed and CI fails on drift. T2 performs **zero disk I/O** — no `std::fs`, no `std::env`, no clock reads.

The Core Purity Invariant (`openspec/specs/workspace-architecture/spec.md:25-40`) is what makes the later CLI possible, and `ts-rs` was chosen precisely because it preserves it: type generation happens *inside the pure library*, with no Tauri awareness anywhere.

```
frontend (Svelte 5) ──IPC──> vertice-app (Tauri) ──calls──> vertice-core::model (pure)
        ▲                                                            │
        └────── frontend/src/bindings/*.ts ◀── ts-rs (cargo test) ───┤
                                                                     │
                            future vertice-cli ──────────────────────┘
```

Both binaries consume the same `ScanReport`. `vertice-app` adds only serialization at the IPC edge; `vertice-cli` would add only formatting. Nothing in `model/` points upward — `cargo deny check bans` still proves it.

## 2. Core Data Model

`crates/vertice-core/src/model/`, one concern per file:

| File | Contents | Public surface |
|---|---|---|
| `mod.rs` | module doc (purity + no-I/O invariant), `pub use` of every type | flat re-export: `vertice_core::model::Component` |
| `identity.rs` | `normalize_name`, `ComponentId::derive` | `ComponentId`; `normalize_name` is `pub(crate)` |
| `component.rs` | `Component`, `ComponentKind`, `Scope` | all `pub` |
| `location.rs` | `Location`, `LocationOrigin`, `SearchRoot`, `SearchRootId`, `SearchRootKind` | all `pub` |
| `installation.rs` | `ClientInstallation`, `ClientKind` | all `pub` |
| `report.rs` | `ScanReport`, `ScanIssue`, `IssueSeverity` | all `pub` |
| `error.rs` | `ScanError` | `pub` |

Submodules stay private; `mod.rs` re-exports. Import allow-list for the whole module: `std::path`, `std::time::Duration`, `serde`, `ts_rs`, `thiserror`, `unicode_normalization`. Explicitly forbidden: `std::fs`, `std::io`, `std::env`, `std::time::SystemTime`/`Instant`. That is the mechanical reading of "pure": `ScanReport.duration_ms` is a value *passed in* by T9, never measured here.

```rust
pub struct Component {
    pub id: ComponentId,
    pub name: String,                    // raw display name, un-normalized
    pub kind: ComponentKind,             // closed: Skill | Agent
    pub description: Option<String>,
    pub scope: Scope,                    // closed: User | Project | Local (PoC emits User)
    pub locations: Vec<Location>,        // N locations, ONE entity
    pub provenance_hint: Option<String>, // opaque display string, never parsed
}
pub struct Location { pub path: Option<PathBuf>, pub root: SearchRootId, pub origin: LocationOrigin }
pub struct ScanReport { pub components: Vec<Component>, pub installations: Vec<ClientInstallation>,
                        pub roots_scanned: Vec<SearchRoot>, pub issues: Vec<ScanIssue>, pub duration_ms: u32 }
```

`Location.root` is a `SearchRootId` reference into `ScanReport.roots_scanned`, not an embedded `SearchRoot` — otherwise the same root is duplicated once per location. Referential integrity is a contract T9 upholds, not a type-level guarantee; T2 states it and tests it on a constructed report.

**Numeric width is a contract decision, not a detail.** `ts-rs` maps `u64`/`i64` to TypeScript `bigint`, but Tauri's JSON IPC delivers a plain `number` — the declared type would lie. Every numeric field is therefore `u32`/`usize` (both map to `number`). `duration_ms: u32` caps a scan at ~49 days; not a real case.

`#[serde(rename_all = "camelCase")]` on structs, and on the unit-only enums so the generated unions read `"skill" | "agent"`, `"user" | "project" | "local"`, `"warning" | "error"`. This depends on `ts-rs`'s default `serde-compat` feature: **`default-features` MUST NOT be disabled**, or the serde renames would silently not reach the `.ts`.

## 3. Decision: `Component.id` derivation

| Option | Tradeoff | Decision |
|---|---|---|
| `Uuid::new_v4()` per parse | N files → N ids; kills acceptance criterion 2 outright | Rejected |
| Hash including content | Splits `issue-creation` into two components (see below) | **Rejected — forbidden by the proposal** |
| Newtype over a stable hash (blake3/sha2) of `(kind, name)` | Correct, but adds a dependency, is undebuggable in logs/DevTools, and buys nothing — no length, secrecy, or uniformity requirement exists | Rejected |
| `DefaultHasher` | `std` documents it as **not stable** across releases or processes | Rejected outright |
| **`ComponentId(String)` = `"{kind}:{normalized_name}"`** | Human-readable, zero new hashing dep, trivially stable, diffable in the generated `.ts` as `string` | **Chosen** |

```rust
pub struct ComponentId(String);   // ts-rs emits: export type ComponentId = string;
// ComponentId::derive(ComponentKind::Skill, "Issue-Creation") == "skill:issue-creation"
```

**Normalization pipeline** (`identity.rs`, applied to `name` only, never to `path`):

1. `trim()` (Unicode whitespace, both ends)
2. **NFC** via `unicode-normalization` (`UnicodeNormalization::nfc`)
3. `str::to_lowercase()` (full Unicode lowercasing, std)

NFC is not optional: macOS surfaces filenames as NFD while Linux and Windows surface NFC, so `revisión` from a synced or macOS-sourced root would otherwise produce two ids for one component. That is exactly the duplication acceptance criterion 2 forbids. Accepted limitation: step 3 is case *conversion*, not case *folding* — `ß` and `ẞ` do not unify. Component names in this ecosystem are kebab-case ASCII; the cost of a full-folding dependency is not justified.

**Stability guarantees.** Deterministic across runs, processes and platforms: no RNG, no clock, no `HashMap` iteration, no hash-algorithm versioning. The only drift vector is the Unicode tables bundled with rustc/`unicode-normalization`, which affects non-ASCII names only, and ids are recomputed every scan (nothing is persisted in the PoC), so drift cannot corrupt stored state.

**Delimiter safety.** `ComponentKind` is a closed enum whose serialized forms (`skill`, `agent`) contain no `:`. A name containing `:` therefore cannot alias another kind's id — the prefix before the first `:` is always exactly a kind.

**Worked collision case** (`alcance-poc-vertice.md:63`). `issue-creation` appears under three roots with *divergent content*:

```
ComponentId "skill:issue-creation"  ──┬── Location { path: Some(rootA/issue-creation/SKILL.md), root: rootA, origin: File }
  ONE Component, three Locations      ├── Location { path: Some(rootB/issue-creation/SKILL.md), root: rootB, origin: File }
                                      └── Location { path: Some(rootC/issue-creation/SKILL.md), root: rootC, origin: File }
```

Content divergence is **not** part of identity: it surfaces downstream (T8/T11) as a duplicate to review, not as two entities. A component with `origin: Embedded` and `path: None` sits in the same `locations` vector and stays distinguishable by `Option`, closing CA-13.

## 4. Decision: `ts-rs` open items

### 4.1 `PathBuf` mapping

`ts-rs` implements `TS` for `Path` and `PathBuf` natively in its `impl_primitives!` block in `ts-rs/src/lib.rs`, mapping both to `string` (same group as `String`, `str`, `char`, the `std::net` address types). `Option<T>` maps to `T | null`, so `Location.path` becomes `path: string | null` with no attribute at all.

**Verified against upstream docs (2026-08-17):** `docs.rs/ts-rs/latest/ts_rs/trait.TS.html` lists both `Path` and `PathBuf` in the `TS` implementors list, mapping to `string`. `Option<T>` has a dedicated impl. The crates.io metadata for `ts-rs` at the same date reports latest version **12.0.1**, license **MIT**, declared `rust-version` **1.78.0**.

What remains unverified is the *emitted shape* of `Option<PathBuf>`: the docs describe `#[ts(optional_fields)]` and `#[ts(optional = nullable)]` attributes for `Option` field rendering, so the exact default (`string | null` vs `string | undefined` vs an optional property) is not settled from documentation alone. This is cheap to settle at apply because the failure mode is **compile-time or diff-visible, not silent**. Apply MUST confirm both halves:

1. `cargo test -p vertice-core` compiles (proves the impl exists);
2. the generated `frontend/src/bindings/Location.ts` literally contains `path: string | null`.

**Pre-approved fallback if either fails:** `#[ts(type = "string")] pub path: Option<PathBuf>` — but note the consequence: `#[ts(type = ...)]` overrides the *whole* field type including the `Option`, so the correct fallback is `#[ts(type = "string | null")]`. The `#[ts(as = "Option<String>")]` form is the alternative and keeps `Option` handling in `ts-rs`'s hands; prefer it if it compiles. Either way the emitted contract is unchanged, so no other decision in this document moves.

### 4.2 License vs `deny.toml`

`ts-rs` is **MIT** (confirmed from crates.io metadata, 2026-08-17), already in the allow-list (`deny.toml:47`). Its dependency closure (`ts-rs-macros` → `syn`/`quote`/`proc-macro2`, plus `thiserror`) is MIT/Apache-2.0, also covered.

**Decision: extend the `quality` job to `cargo deny check bans licenses`.** T1's deferral rationale does not transfer: `check advisories` was deferred because it is *time-dependent* and reddens unrelated PRs (`deny.toml:3-9`); `check licenses` is deterministic for a locked graph, so the precedent argues *for* enabling it, not against.

Bounded fallback, because turning it on evaluates Tauri's entire pre-existing graph for the first time: apply MUST run `cargo deny check licenses` locally **before** editing `ci.yml` and record the actual delta. If the delta is a short list of unambiguously permissive licenses, add them to `[licenses] allow` and enable the gate. If it surfaces a copyleft, unclear, or unlicensed crate, **do not** enable the gate in T2 — narrow-scope it with a `[licenses] exceptions` entry for `ts-rs` only, or defer with a written rationale in the same style as `deny.toml:3-9`. Enabling a gate that reddens CI for a pre-existing, unrelated condition is worse than deferring it.

### 4.3 MSRV

T1 proved that declared `rust-version` metadata is not sufficient — `1.85` was the naive answer and it failed to compile (`archive/2026-08-17-bootstrap-workspace-ci/design.md:128-133`). Same discipline, no shortcuts. After adding `ts-rs` (and `unicode-normalization`), apply MUST run, from the repo root, with `frontend/dist` present:

```
RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --locked --all-targets
```

`--all-targets` is load-bearing: the `ts-rs` export functions are `#[test]`s, so a `ts-rs`-only floor violation is invisible without it. If it fails, the fix is bumping `MSRV` in three places in lockstep (`Cargo.toml` `rust-version`, `ci.yml` `env.MSRV`, the `msrv` job name) — the `quality` job's consistency step fails the build if they drift. `ts-rs` must be a **regular** dependency, not a dev-dependency, because the derive is on the types themselves; combined with `deny.toml`'s `exclude-dev = true`, this means it is in the checked graph for both bans and licenses. Good.

## 5. Decision: generation and CI wiring

| Question | Decision |
|---|---|
| Trigger | `#[ts(export)]`, `ts-rs`'s default `#[test]` model. Regenerating IS running the tests — the mechanism a contributor already runs. A dedicated `--bin export-bindings` was rejected: an extra binary in a *library* crate, invoked by a command nobody runs by habit. |
| Output path | `#[ts(export, export_to = "../../../frontend/src/bindings/")]` per type. Trailing slash = directory; filename derives from the type name. **Corrected at apply:** this design originally specified two `..`, reasoning that `export_to` resolves against `CARGO_MANIFEST_DIR` (`crates/vertice-core/`). That was wrong by one level — empirically, and reproducibly regardless of CWD, three `..` are required to land the files at repo-root `frontend/src/bindings/`. Anyone adding a new domain type must copy the three-`..` form; see `apply-progress.md`. |
| Why not `TS_RS_EXPORT_DIR` | Cargo `[env]` in `.cargo/config.toml` is action-at-a-distance for a 9-type model and its `relative = true` resolution has varied across cargo versions. Explicit and grep-able beats implicit. |
| Local regeneration | `cargo test -p vertice-core` — documented in the module doc of `model/mod.rs` and in the CI step name. |
| CI job | `quality` (ubuntu-only), after `cargo deny`. |

```yaml
- name: Regenerate TS bindings (ts-rs export tests)
  run: cargo test -p vertice-core --locked
- name: Generated contract in sync
  run: git diff --exit-code -- frontend/src/bindings
```

`-p vertice-core` is deliberate: it does not compile `vertice-app`, so `generate_context!` never reads `frontendDist`, so `quality` needs **no** `needs: frontend` and no artifact download. It stays the fast, dependency-free job it is today (`ci.yml:25-75`).

**Frontend tooling integration** (would otherwise redden the `frontend` job, `ci.yml:92-96`): add `src/bindings/` to the ESLint ignore list — generated code should not be style-linted — but keep it inside the `tsconfig` include set so `npm run check` still type-checks it. Catching type errors in generated code is the point; catching formatting complaints is noise. `frontend/src/bindings/` is generated-only: no hand-written file goes in it, and T2 adds no barrel/`index.ts` (T10 re-exports where it needs them).

## 6. Error paths: `ScanIssue` taxonomy

| Class | Type | Placement | Aborts scan? |
|---|---|---|---|
| Per-item, recoverable: unparseable frontmatter, unreadable file, absent root, **non-UTF-8 path** | `ScanIssue { severity, path: Option<PathBuf>, reason: String }` | inside `Ok(ScanReport).issues` | **Never** |
| Orchestration failure: the scan cannot proceed at all | `ScanError` (`thiserror`) | `Err` of `Result<ScanReport, ScanError>` | Yes |
| Seam-internal parse failure | `YamlError` (`src/yaml.rs:14-18`) | converted to a `ScanIssue.reason` string by T3 | No |

`IssueSeverity` has exactly two variants, `Warning` and `Error`, and **neither aborts the scan** — severity is a display/triage signal for T11, not control flow. An empty scan is `Ok(ScanReport)` with empty collections, never an `Err`. `ScanIssue.path` is `Option<PathBuf>` because an absent root or an embedded component has no meaningful file path.

**Non-UTF-8 paths.** `serde` serialization of `PathBuf` *fails outright* on non-UTF-8 paths (real on Windows WTF-8 surrogates and on Linux arbitrary bytes). The contract: `Location.path` is `Option<PathBuf>` in Rust and its serialization assumes UTF-8-representable paths. From T3 onward an adapter that meets a non-UTF-8 path MUST emit a `ScanIssue` (`path: None`, `reason` carrying the lossy rendering) and MUST NOT construct a `Location` holding it. That keeps the failure at the adapter, not deep in T10's IPC layer where it would kill the whole response. T2 cannot trigger this (zero I/O); T2 fixes the rule.

## 7. Decision: `ScanError` derives `Serialize` + `TS`

**Yes.** Tauri requires the `Err` variant of a command's `Result` to be `Serialize`. Deriving now costs one line; not deriving forces T10 to invent a wrapper or stringify, losing structure.

```rust
#[derive(Debug, thiserror::Error, Serialize, TS)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum ScanError { /* ... */ }
```

Adjacent tagging gives TypeScript a discriminated union the frontend can `switch` on exhaustively, rather than an opaque string. `Deserialize` is **not** derived — the frontend never constructs errors.

**The invariant this creates**, and it must be written down or it will be violated: *every `ScanError` variant payload MUST be owned serializable data (`String`, `PathBuf`, a model enum) — never a foreign error type.* `serde_norway::Error`, `std::io::Error` and friends are not `Serialize`, so a `#[from]` on them would break the derive at the worst moment. Foreign errors are converted with `.to_string()` at the boundary. This is exactly why `YamlError` (`src/yaml.rs`) stays non-serializable and seam-internal: only `model::error::ScanError` crosses IPC.

**T10 consequence:** `#[tauri::command] async fn scan_components() -> Result<ScanReport, ScanError>` compiles with no adapter type, and the frontend imports the generated `ScanError` union directly. Had we deferred, T10 would have had to introduce an IPC-only error DTO and a mapping layer.

## 8. IPC contract surface implied for T10

T2 registers nothing. It fixes what T10's commands may exchange:

| Direction | Payload | Generated TS |
|---|---|---|
| Command result | `ScanReport` | `ScanReport.ts` (transitively pulls all nine types) |
| Command error | `ScanError` | discriminated union, `switch`-exhaustive |
| Command args | none in T2's model | — |
| Events | none | deferred to T10 |

`ts-rs` generates *types*, not command signatures: command **names** and **argument shapes** stay unverified until T10 decides (a `tauri-specta` revisit is explicitly T10-scoped). T2's contribution is that every type crossing the boundary is generated, not hand-written.

## 9. Platform paths (type-contract impact only)

T2 resolves no paths — that is T7/T14. Only the consequences for the *type* contract:

| OS | Root shape a `SearchRoot.path` will hold |
|---|---|
| Windows | `%APPDATA%\...` (and `%USERPROFILE%\.claude\...`) |
| Linux | `$XDG_CONFIG_HOME` / `~/.config/...` |
| macOS | `~/Library/Application Support/...` (plus `~/.claude/...`) |

Three consequences, all type-level: (1) `SearchRoot.path` and `Location.path` hold **absolute, opaque** strings across IPC — the frontend never parses them; (2) separator divergence (`\` vs `/`) is a T11 *display* concern and MUST NOT enter identity — `ComponentId` derives from `(kind, name)` only, so ids are byte-identical across the three-OS matrix for the same logical component; (3) T2's tests are fixture-free and construct `PathBuf::from("...")` literals that never touch disk, so they are machine-independent per `rules.verify`.

## 10. File Changes

| File | Action | Description |
|---|---|---|
| `Cargo.toml` | Modify | `[workspace.dependencies]`: `ts-rs = "12"`, `unicode-normalization = "0.1"` |
| `crates/vertice-core/Cargo.toml` | Modify | consume both (regular deps, default features ON) |
| `crates/vertice-core/src/lib.rs` | Modify | `pub mod model;` |
| `crates/vertice-core/src/model/{mod,identity,component,location,installation,report,error}.rs` | Create | the nine types + `ScanError` |
| `crates/vertice-core/tests/model_contract.rs` | Create | acceptance-criteria tests |
| `frontend/src/bindings/*.ts` | Create | generated, committed |
| `frontend/eslint.config.*` | Modify | ignore `src/bindings/` |
| `.github/workflows/ci.yml` | Modify | bindings-in-sync step; `check bans licenses` (subject to §4.2) |
| `deny.toml` | Modify | allow-list delta only if §4.2's local run demands it |
| `openspec/specs/domain-model/` | Create | capability spec (parallel `sdd-spec` agent) |

## 11. Testing Strategy

`strict_tdd: true` — tests first, fixture-free, no disk.

| Layer | What | How |
|---|---|---|
| Unit | `ComponentId::derive` determinism; normalization (trim, case, NFC/NFD equivalence); delimiter safety | `crates/vertice-core/src/model/identity.rs` `#[cfg(test)]` |
| Integration | CA-1: `path: None` vs `Some` distinguishable. CA-2: one `Component`, three `Location`s, equal ids from two derivations. CA-3: `scope` populated. Round-trip `ScanReport` through `serde_json`. Referential integrity of `Location.root`. Empty scan is `Ok`. | `tests/model_contract.rs`, mirroring T1's `tests/yaml_behavior.rs` |
| Contract | CA-4: bindings in sync | `cargo test -p vertice-core` + `git diff --exit-code` in `quality` |
| Static | purity, MSRV, licenses | `cargo deny`, `RUSTUP_TOOLCHAIN=1.88 cargo check --all-targets`, clippy `-D warnings` |

CA-4's **negative path** cannot be an always-on automated test (a test that intentionally desynchronizes the repo would fail every run). It is a one-time manual verification recorded in `verify-report.md`: mutate a field name, run `cargo test -p vertice-core` **without** committing, confirm `git diff --exit-code` exits non-zero, revert.

## 12. Migration / Rollout

No migration. Purely additive; no consumer exists. Rollback = revert the branch (proposal §Rollback Plan). If the identity rule proves wrong after T8, the blast radius is `identity.rs` plus its tests — no type shape changes.

## 13. Open Questions

- [x] §4.1 `PathBuf` has a native `TS` impl mapping to `string` — **confirmed** against `docs.rs/ts-rs/latest/ts_rs/trait.TS.html` (2026-08-17).
- [ ] §4.1 emitted shape of `Option<PathBuf>` (`string | null` vs optional property) — **must be confirmed at apply** by inspecting the generated `Location.ts`. Fallback pre-approved; no redesign either way.
- [ ] §4.2 licenses gate — enable vs narrow-scope depends on the local `cargo deny check licenses` delta against the pre-existing Tauri graph. Decision rule is written; the input is not yet measured. (`ts-rs`'s own MIT license is confirmed and already allowed.)
- [ ] §4.3 MSRV — `ts-rs` declares `rust-version = 1.78.0` (below the 1.88 floor), but the transitive floor of `ts-rs` + `unicode-normalization` must still be checked empirically, not from metadata — T1 proved declared metadata lies.
- [x] Pin `ts-rs = "12"` — latest is **12.0.1** (crates.io, 2026-08-17). Whether it emits its own `index.ts` barrel is still open; if it does, commit it, do not hand-edit.
- [ ] `ClientKind` variant set (which clients the PoC recognizes) — T4/T5 territory; T2 ships the enum shape, the variants may grow before Phase 1 closes.
