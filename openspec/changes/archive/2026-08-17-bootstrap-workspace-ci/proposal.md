# Proposal: Bootstrap Workspace and CI

> Plan trace: **T1** (Phase 0 — Foundations) of `internal-docs/plan-desarrollo-poc.md`.
> Acceptance criteria addressed: **CA-17** (core tests pass on versioned fixtures across the three CI platforms). Establishes the enforcement point for **CA-16** (no writes outside app data dir).

## Intent

No code exists yet. Before any product logic, the repository must compile, test, lint and package on macOS, Windows and Linux, with the core/Tauri boundary enforced by tooling rather than discipline. That boundary is stack decision #5: `vertice-core` stays Tauri-agnostic so the post-PoC CLI is a second binary, not a reimplementation — cheap now, expensive to retrofit. The archived `serde_yaml` also forces a replacement decision that every later adapter depends on.

## Scope

### In Scope
- Cargo workspace: `vertice-core` (pure library) + `vertice-app` (Tauri 2).
- Svelte 5 + Vite + Tailwind frontend, bundled into the binary.
- MSRV pinned in `Cargo.toml` and verified in CI.
- GitHub Actions matrix (macOS/Windows/Linux): `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, frontend lint, app build.
- Recorded YAML crate decision replacing `serde_yaml`, with justification.
- Automated check that `vertice-core` has no `tauri` in its dependency graph.

### Out of Scope
- Any scanning or adapter logic (T3–T7).
- Domain model and generated TypeScript contract (T2).
- Any screen beyond the empty skeleton; i18n wiring (T12).
- SQLite, packaging/signing, E2E harness (T14–T16).

## Capabilities

### New Capabilities
- `workspace-architecture`: crate layout, core purity invariant, MSRV floor.
- `ci-quality-gates`: cross-platform gates that must pass before merge.

### Modified Capabilities
- None (no existing specs).

## Approach

Two-crate workspace with shared `[workspace.package]` metadata and pinned MSRV. Frontend lives under `vertice-app`, built by Vite and embedded via Tauri's bundler. Core purity is enforced mechanically (`cargo tree`/`cargo-deny` assertion in CI), not by review. YAML crate selected by verifying, at decision time, current maintenance activity, block-scalar support (`description: >`) and `serde` integration across `serde_norway`, `serde_yaml_ng`, `serde_yml`, `yaml-rust2`; the decision and rejected options are recorded in the design artifact.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `Cargo.toml` (workspace) | New | Members, MSRV, shared metadata |
| `crates/vertice-core/` | New | Pure library skeleton + first test |
| `crates/vertice-app/` | New | Tauri 2 app + frontend |
| `.github/workflows/ci.yml` | New | Three-platform matrix |
| `openspec/specs/` | New | Two capability specs |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| No viable YAML crate meets all three criteria | Med | Evaluate all four with a block-scalar probe test; fall back to `yaml-rust2` + manual deserialization |
| `serde_yml` maintainership claims publicly disputed | Med | Verify repo activity and provenance before selecting; do not trust crates.io metadata alone |
| Linux `webkit2gtk` version breaks CI build | Med | Pin runner image; document minimum distro version |
| Tauri scaffolding leaks `tauri` into core | Low | Dependency-graph assertion fails CI |
| MSRV drifts from toolchain used locally | Low | `rust-toolchain.toml` + explicit MSRV job |

## Rollback Plan

Change is additive to an empty repo: revert the branch. No three-layer impact to unwind — no consumers exist. If the YAML decision proves wrong later, it is isolated to one dependency and one parsing module (adapters land from T3 onward).

## Dependencies

- None. T1 is the graph root; T2 and T14 depend on it.

## Success Criteria

- [x] `cargo test` and app build pass on all three matrix platforms. Verified live: `https://github.com/RuloGB/vertice/actions/runs/32015782992` — `rust` job green on ubuntu-24.04, windows-2022, macos-14.
- [x] `cargo fmt --check`, `cargo clippy -D warnings` and frontend lint pass in CI. Verified live in the `quality` and `frontend` jobs of the same run.
- [x] `vertice-core` imports nothing from `tauri`, verified by dependency inspection in CI. Enforced by `cargo deny check bans` in the `quality` job; negative-path tested (`tauri` temporarily added to `vertice-core`, confirmed `cargo deny` fails, reverted) during both apply and verify.
- [x] MSRV is declared and a CI job fails if it is violated. `msrv` job green in the same run; empirically confirmed `1.85` fails (real transitive-dep floor) and `1.88` passes.
- [x] YAML crate decision is written with justification and rejected alternatives. See `design.md` — `serde_norway` selected, `serde_yml`/`yaml-rust2` rejected, `serde_yaml_ng` recorded as fallback.
