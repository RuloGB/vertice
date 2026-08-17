# Exploration — Domain Model and Type Contract (T2)

**Change**: `domain-model-type-contract`
**Phase**: T2 of `internal-docs/plan-desarrollo-poc.md`
**Artifact store**: openspec
**Status**: complete — ready for `sdd-propose`

---

## 1. Current State (post-T1)

**Workspace** (`Cargo.toml:1-18`): two members, resolver 2. `[workspace.package]` pins `edition = "2021"`, `rust-version = "1.88"`, `license = "MIT OR Apache-2.0"`. `[workspace.dependencies]` declares only `vertice-core`, `serde = { version = "1", features = ["derive"] }`, `thiserror = "2"`. `[workspace.lints.rust] unsafe_code = "deny"`.

**`vertice-core`** (`crates/vertice-core/Cargo.toml:1-14`): depends on `serde` (workspace), `serde_norway = "0.9"`, `thiserror` (workspace). `src/lib.rs:1-25` is a one-module skeleton (`pub mod yaml;`) plus a smoke test; `src/yaml.rs:1-24` is the YAML deserialization seam (`from_str<T: DeserializeOwned>`, `YamlError::Parse` wrapping `serde_norway::Error`). **No domain types exist yet — T2 starts from zero.**

**`vertice-app`** (`crates/vertice-app/Cargo.toml:1-24`): depends on `vertice-core` (workspace path dep) and `tauri = { version = "2", features = [] }`, build-dep `tauri-build = { version = "2" }`. `src/lib.rs:1-11` is a bare `tauri::Builder::default().run(...)` — no commands registered (T10 territory). `Cargo.lock` is committed.

**MSRV** (`rust-toolchain.toml:1-3`): dev/CI channel pinned to `1.97.1`; workspace floor `1.88`, empirically verified against Tauri 2.11.5's transitive deps (`darling`, `icu_*`, `time`, `serde_with`) per `openspec/changes/archive/2026-08-17-bootstrap-workspace-ci/design.md:128-133`. `.github/workflows/ci.yml:157-187` runs a dedicated `msrv` job (`RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --locked --all-targets`); the `quality` job (`ci.yml:48-69`) fails on drift between `Cargo.toml`'s `rust-version` and the workflow `MSRV` env.

**`deny.toml`** (`deny.toml:1-63`): only `cargo deny check bans` is gated in CI (`ci.yml:74-75`), enforcing the core-purity invariant — `tauri`/`tauri-build` are denied except via `wrappers = ["vertice-app"]`. `check advisories` and `check licenses` are explicitly deferred (comment at `deny.toml:3-9`). `[licenses] allow` (`deny.toml:46-57`) covers MIT, Apache-2.0 (+LLVM-exception), BSD-2/3, ISC, Unicode-3.0, Zlib, CC0-1.0, MPL-2.0.

**CI** (`.github/workflows/ci.yml:1-188`): four jobs — `quality` (fmt, MSRV consistency, `cargo deny check bans`; ubuntu-only), `frontend` (lint/check/test/build, uploads `frontend/dist`), `rust` (matrix macOS/Windows/Linux: clippy `-D warnings`, `cargo test --workspace --locked`, `cargo build --release -p vertice-app`; `needs: frontend` because `generate_context!` reads `frontendDist` at compile time), `msrv` (`needs: frontend`). **No job verifies that any generated file is in sync** — T2 must add one.

**Existing specs that constrain T2**:

- `openspec/specs/workspace-architecture/spec.md:25-40` — Core Purity Invariant: `vertice-core` MUST NOT depend, directly or transitively, on `tauri`/`tauri-*`, verified mechanically.
- `openspec/specs/workspace-architecture/spec.md:42-58` — MSRV floor 1.88 must hold; new dependency MSRV must be verified empirically, not read from declared metadata.
- `openspec/specs/ci-quality-gates/spec.md:9-30` — the cross-platform matrix must stay green; a new check must either run on all three OSes or be justified as ubuntu-only (like `quality`).
- `openspec/config.yaml:32-45` — specs use Given/When/Then and RFC 2119 keywords, separate core (Rust) from frontend (TS); design must document `Component, Location, Scope, SearchRoot, ScanReport` and the IPC contract surface.

## 2. What T2 Requires

From `internal-docs/plan-desarrollo-poc.md:61-89`:

- Core types, at minimum: `Component { id, name, kind, description, scope, locations, provenance_hint }`, `ComponentKind { Skill, Agent }`, `Scope { User, Project, Local }` (PoC produces only `User`), `Location { path: Option<PathBuf>, root, origin }`, `SearchRoot { id, path, kind }`, `ClientInstallation { client, version, path }`, `ScanIssue { severity, path, reason }`, `ScanReport { components, installations, roots_scanned, issues, duration }`.
- TypeScript type generation from Rust (`ts-rs` or `tauri-specta`) with CI verification that the generated contract is in sync.
- Typed errors with `thiserror` in the core.
- **Out of scope: any disk I/O.**

Acceptance criteria (`plan-desarrollo-poc.md:82-86`):

1. The model admits a component with no disk path, distinguishable from one with a path.
2. The model admits a component with N locations without duplicating the entity.
3. `scope` exists and is populated, even though `User` is its only PoC value.
4. A Rust type change that breaks the contract fails compilation or CI, not runtime.

## 3. Affected Areas

- `crates/vertice-core/Cargo.toml` — new dependency (`ts-rs`, or `specta` + `tauri-specta`); possibly a hashing crate for `Component.id`.
- `crates/vertice-core/src/lib.rs` — new `pub mod model;`.
- `crates/vertice-core/src/model/*.rs` (new) — the eight domain types plus the `thiserror` error enum(s).
- `crates/vertice-core/tests/` — model contract tests, mirroring the `tests/yaml_behavior.rs` pattern from T1.
- `frontend/src/bindings/` (new) — generated TS bindings that CI diffs.
- `.github/workflows/ci.yml` — new "generated contract in sync" step.
- `deny.toml` — license allow-list check for the new dependency.
- `openspec/specs/` — T2 adds a new capability spec (e.g. `domain-model`); it does not modify `workspace-architecture` or `ci-quality-gates`.

## 4. Design Questions

### Q1 — `ts-rs` vs `tauri-specta`

**Option A: `ts-rs`.** `#[derive(TS)]` per type, export triggered from a `#[test]`.

- Pros: framework-agnostic, usable inside a pure library with zero Tauri awareness; actively maintained (latest `12.0.1`, released ~1 month before this exploration per crates.io); simple one-type-one-file diff; keeps T2 mechanically independent of T10.
- Cons: no confirmed first-class `PathBuf` mapping (see Risk 2) — may need `#[ts(type = "string")]` per field; generates types only, not command signatures, so command name/arg shape stays unchecked until T10.
- Effort: low.

**Option B: `tauri-specta`.** `#[derive(specta::Type)]` on domain types + `tauri_specta::Builder`/`collect_commands!` in `vertice-app`.

- Pros: generates the whole IPC surface (types + typed `invoke` wrappers + typed events), which is what T10's "types crossing IPC are the generated ones" criterion wants end-to-end.
- Cons: the Tauri-2-compatible line is **`2.0.0-rc.25`, a pre-release**, ~3 months old at exploration time. Also couples type generation to command registration: `specta::Type` derives can live in `vertice-core` (specta itself has no Tauri dependency), but `.ts` emission only happens where the builder runs — naturally `vertice-app` — which makes T2's "zero I/O, zero Tauri coupling" story awkward when nothing is registered as a command yet.
- Effort: medium; pre-release status demands version pinning and re-verification at apply time.

**CI verification pattern** (orthogonal to the generator choice):

- **(a) Check in the generated file + `git diff --exit-code`.** Fast CI, reviewable diff in PRs, out-of-sync file is a hard CI failure — matches acceptance criterion 4. Requires contributors to regenerate locally, mitigated by making generation part of `cargo test` (ts-rs's default `export_to` model already does this).
- **(b) Generate at test time, never check in.** Cannot go stale by construction, but forces frontend devs to have a Rust toolchain, fails less legibly, and inverts the current job ordering (`rust` currently `needs: frontend`, `ci.yml:77-103`).

**Recommendation: `ts-rs` with pattern (a)**, gated in the `quality` job (ubuntu-only, alongside `cargo deny check bans` — generation is not OS-path-sensitive since T2 does zero disk I/O). Reasoning: `ts-rs` is stable where `tauri-specta`'s Tauri-2 line is not; it keeps T2 decoupled from T10; and the check-in diff gives exactly the CI-failure semantics criterion 4 asks for. If T10 later wants typed command bindings and `tauri-specta` has stabilized, that is a T10-scoped decision, not a T2 blocker.

### Q2 — `Component.id` derivation (hallazgo 3)

**Option A: deterministic id from `(kind, name)`.** Matches T8's stated aggregation key — "aggregation by identity (name + type) into a single `Component` with N locations" (`plan-desarrollo-poc.md:196`). Stable across runs and platforms by construction. Trivially testable.

**Option B: random per-scan id (`Uuid::new_v4()`) with a separate grouping step.** Directly contradicts acceptance criterion 2: N parsed files produce N random ids, so `id` stops being the aggregation key and T2's own acceptance test cannot be written against id equality. Pushes T8's work forward without reducing it.

**Recommendation: Option A** — deterministic id derived from `(kind, normalized name)`. This must be decided in T2, not deferred, because criterion 2 is only checkable if identity is fixed now.

**Critical constraint for design**: identity MUST NOT incorporate content hashing. `alcance-poc-vertice.md:63` documents `issue-creation` as a component with the same name but **divergent content** across roots — content-based identity would split it into two components instead of flagging one duplicate. Open sub-question for `sdd-design`: raw `String` id vs a typed newtype over a stable hash, and the normalization rule (case folding, Unicode normalization form).

### Q3 — `Location.path: Option<PathBuf>` across IPC

- `serde` implements `Serialize`/`Deserialize` for `PathBuf`, but **serialization fails outright on non-UTF-8 paths**. Real on Windows (WTF-8 surrogates) and Linux (arbitrary bytes).
- TypeScript has no `PathBuf`; `Option<PathBuf>` must cross as `string | null`. Lossy but consistent with Tauri's JSON-based IPC, which only round-trips UTF-8-safe strings anyway.
- **Implication**: `ScanIssue` is the escape hatch for "this path could not be represented". A non-UTF-8 path met by a T3+ adapter must produce a `ScanIssue`, not a serialization failure deep in the T10 IPC layer. T2 cannot hit this itself (zero disk I/O), but T2's design MUST state the contract: `Location.path` is `Option<PathBuf>` in Rust, its serialization contract assumes UTF-8-representable paths, and the non-UTF-8 case is handled as a `ScanIssue` from T3 onward.
- Separator differences (`\` vs `/`) are a display concern for T11, not a T2 modeling concern — worth one line in the design doc so it is not rediscovered.

### Q4 — Error taxonomy: `thiserror` vs `ScanIssue`

These are not competing options; they cover different failure classes.

- **`ScanIssue { severity, path, reason }`** — expected, per-item, recoverable failures while walking N roots/files: unparseable frontmatter, unreadable file, absent root. See T3 ("absences and unexpected types produce `ScanIssue`, not panic", `plan-desarrollo-poc.md:98`) and T9 ("nothing is silently omitted... a failing adapter does not abort the rest of the scan", `plan-desarrollo-poc.md:217-218`).
- **`thiserror` enums** — hard, orchestration-level failures where the scan itself cannot proceed, surfaced as `Result<ScanReport, ScanError>` from the top-level scan function (T9).

**Boundary rule**: one item fails → `ScanIssue` appended to `ScanReport.issues`, scan continues. Scan orchestration cannot proceed at all → `Err(ScanError)`. Everything else is data inside a successful `Ok(ScanReport)`.

**Serialization**: `ScanIssue` and its `severity` enum need `Serialize`/`Deserialize`/`TS` derives now, since `ScanReport` is squarely in T2's scope. For the `thiserror` error type, T2 should **decide and document** whether it will also derive `Serialize`/`TS` for structured IPC error propagation (Tauri requires the `Err` variant to be `Serialize`) — retrofitting derives later is cheap, but leaving the question open forces T10 to invent the answer.

### Q5 — Closed enums vs `#[non_exhaustive]`

**Recommendation: closed enums, no `#[non_exhaustive]`**, for `ComponentKind` and `Scope`.

`#[non_exhaustive]` protects **out-of-tree downstream crates**. This workspace has one producer (`vertice-core`) and one consumer (`vertice-app` + frontend), both in-repo — including the post-PoC CLI, which is also in-repo. The protection has near-zero value here, while the cost is real: every `match` needs a wildcard arm, which destroys exhaustiveness checking — and exhaustiveness checking is precisely the mechanism that makes "the model admits exactly these variants" a compiler-verified claim. It also does nothing for TypeScript, where generated unions are closed regardless of the Rust attribute. Revisit only if a genuine external consumer of `vertice-core` appears.

## 5. Risks (ranked)

| # | Risk | Impact / Likelihood | Evidence |
|---|------|--------------------|----------|
| 1 | `tauri-specta`'s Tauri-2 line is pre-release (`2.0.0-rc.25`) | Medium-High / n-a if `ts-rs` chosen | crates.io: latest is a prerelease from ~3 months before exploration. Weakens acceptance criterion 4 ("fails compilation or CI, not runtime") if the library's own API is unstable. Mitigated by recommending `ts-rs`. |
| 2 | `ts-rs`'s native `PathBuf` support is **unconfirmed** | Medium / Low-Medium | Two search attempts returned no direct confirmation either way; only the general `#[ts(type = "..")]` escape hatch is documented. `Location.path` is a mandatory field, so this must be verified against `ts-rs` source before `sdd-design`/`sdd-apply` — same discipline T1 applied to `serde_norway` (`design.md:58, 128-140`). |
| 3 | `serde`'s `PathBuf` serialization fails on non-UTF-8 paths | Medium / Low in T2, real from T3+ | Confirmed: `serde_json` errors on non-UTF-8 path characters. T2 cannot trigger it (no disk I/O) but must fix the contract now so T3+ handles it as a `ScanIssue`. |
| 4 | Leaving `Component.id` derivation unspecified breaks T2's own acceptance criterion | Medium / Medium | `plan-desarrollo-poc.md:84` requires N-locations-without-duplication as a T2-closed criterion, only testable if identity is fixed. `alcance-poc-vertice.md:63`'s `issue-creation` (same name, divergent content) is a concrete trap for content-based identity. |
| 5 | No CI job verifies generated-contract sync | Low-Medium / High without explicit work | `.github/workflows/ci.yml:1-188` has four jobs, none doing a bindings diff. T2 establishes this pattern for the project — no precedent to copy. |
| 6 | `deny.toml` license allow-list may not cover the new dependency, and CI does not check licenses at all | Low / Low-Medium | `deny.toml:46-57` is a fixed allow-list; `deny.toml:3-9` scopes T1's CI gate to `check bans` only. A license violation from the new dependency would not be caught automatically. Either run `cargo deny check licenses` manually at apply time or extend the `quality` job. |
| 7 | New dependency's transitive MSRV floor unverified | Low / Low | `design.md:128-133` shows a crate's transitive floor can exceed its declared `rust-version`; the only reliable method found was pinning `RUSTUP_TOOLCHAIN` and running `cargo check`. `ts-rs` is unlikely to raise the 1.88 floor but must be verified empirically at apply time. |

## 6. Recommended Scope Boundary

**In T2**:

- All eight core types as plain Rust structs/enums with `Serialize`/`Deserialize`/`TS` derives; no methods beyond trivial constructors.
- `Component.id` derivation fixed and documented (deterministic, `(kind, name)`-based, **not** content-based), with rationale recorded in `design.md` — same pattern as T1's YAML crate decision.
- Error taxonomy boundary documented explicitly (`ScanIssue` = recoverable/per-item/inside `Ok`; `thiserror` = orchestration failure), with the `thiserror` enum(s) defined for whatever T2-internal validation exists.
- TS generation tool chosen and wired (`ts-rs` recommended) plus the CI in-sync check (check-in + `git diff --exit-code`, in the `quality` job).
- Fixture-free unit tests proving all four acceptance criteria.
- `deny.toml` license gap either closed or explicitly deferred with written rationale, matching T1's `check bans`-only precedent.

**Deferred to T3+**:

- Any disk I/O, `walkdir` usage, or frontmatter parsing (T3).
- The consolidation/aggregation *algorithm* — T2 fixes only what makes consolidation possible (the id scheme); the grouping function is T8.
- Tauri command registration and IPC wiring beyond type generation (T10).
- Non-UTF-8 path handling *code* — T2 records only the design decision that it becomes a `ScanIssue`.
- SQLite/persistence fields (out of scope for the whole PoC, `alcance-poc-vertice.md:44-49`).
- Platform-specific data-directory resolution (T7/T14).

## 7. Ready for Proposal

**Yes.** Two verification items are flagged for `sdd-design`/`sdd-apply` rather than blocking the proposal: `ts-rs`'s native `PathBuf` support (Risk 2) and the new dependency's license against `deny.toml`'s allow-list (Risk 6).
