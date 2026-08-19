# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Vertice is a desktop app (PoC stage) that inventories AI components — skills and agents — installed across AI clients (Claude Code, OpenCode, Copilot, Codex…) on the user's machine. It is **read-only**: it never writes outside the application data directory.

Stack: Rust core library + Tauri 2 shell + Svelte 5 / Vite / Tailwind 4 frontend.

## Commands

Rust (workspace root):

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace --locked
```

```bash
cargo test -p vertice-core --locked
```

Single Rust test:

```bash
cargo test -p vertice-core case_variants_collapse_to_one_identity -- --exact
```

Frontend (from `frontend/`):

```bash
npm run lint && npm run check && npm run test && npm run build
```

Single frontend test:

```bash
npx vitest run src/lib/appTitle.test.ts
```

Run the desktop app in dev (Tauri drives `npm run dev` on port 1420 itself):

```bash
npx --prefix frontend tauri dev
```

Dependency policy gate (core purity + license allow-list):

```bash
cargo deny check bans licenses
```

Note: `cargo` may not be on PATH in this environment; if the Rust commands fail to resolve, say so rather than reporting the gates as passing.

## Architecture

Three layers, with hard structural invariants enforced mechanically — do not weaken them:

- **`crates/vertice-core`** — pure domain library. Must **never** depend on `tauri` or any `tauri-*` crate, directly or transitively. `deny.toml` bans `tauri`/`tauri-build` with `vertice-app` as the only allowed direct parent, so an accidental import fails CI immediately. The reason is a future CLI binary that reuses the same logic.
- **`crates/vertice-app`** — the only crate that imports `tauri`. Owns the runtime, IPC commands, capabilities, and bundles `frontend/dist`.
- **`frontend/`** — Svelte 5 SPA. Consumes core types via generated TypeScript bindings.

Two seams inside the core deserve care:

- **`src/model/`** is plain data with zero I/O. Its module doc declares an import allow-list (`std::path`, `std::time::Duration`, `serde`, `ts_rs`, `thiserror`, `unicode_normalization`) and forbids `std::fs`, `std::io`, `std::env`, `SystemTime`/`Instant`. Values like `ScanReport::duration_ms` are passed in by the caller, never measured here. Scanning and path resolution belong to later phases.
- **`src/yaml.rs`** is the *only* module allowed to import the YAML crate (`serde_norway`). Everything else goes through `yaml::from_str`. `tests/yaml_behavior.rs` pins the behaviors that seam guarantees, so the crate can be swapped by touching one file.

**Type contract (Rust → TypeScript).** Every public model type derives `ts_rs::TS` with `#[ts(export, export_to = "../../../frontend/src/bindings/")]`. Running `cargo test -p vertice-core` regenerates `frontend/src/bindings/*.ts`. CI regenerates them and fails on any diff (using `git add --intent-to-add` first, so a *new* uncommitted binding is also caught). Never hand-edit files in `frontend/src/bindings/` — change the Rust type and regenerate.

**Component identity** (`src/model/identity.rs`) is a human-readable string `"{kind}:{normalized name}"`, never a hash, derived from `(kind, name)` alone — never from `Location` or file content. Normalization is trim → NFC → lowercase (NFC matters because macOS surfaces NFD).

## Versions and CI

- MSRV floor is declared in three places that must agree: `Cargo.toml` `rust-version`, the `MSRV` env in `.github/workflows/ci.yml`, and `rust-toolchain.toml` `channel` (which pins a *newer* exact toolchain — MSRV is a floor, not a pin). A CI step fails the build if they drift, so update them together.
- CI jobs: `quality` (fmt, MSRV consistency, `cargo deny`, bindings-in-sync), `frontend` (lint/check/test/build, uploads `dist`), `rust` (clippy/test/release build — Linux only on pull requests; Linux + Windows + macOS on push to `main` and on `workflow_dispatch`, which is where CA-17 is enforced), `msrv` (`cargo check` at the floor). The Rust jobs download the frontend artifact rather than rebuilding it. `paths-ignore` (`internal-docs/**`, `openspec/**`, `CLAUDE.md`) means a documentation-only change produces no run at all, and pull requests not targeting `main` are not validated.
- `cargo deny check advisories` is deliberately **not** a PR gate — it is time-dependent and would red-flag unrelated PRs. `licenses` is deterministic and is gated.

## Working conventions

- **Spec-driven development.** `openspec/config.yaml` defines the workflow; `openspec/specs/` holds the merged living specs and `openspec/changes/archive/` the completed cycles. Read the relevant spec before changing behavior it covers, and keep RFC 2119 wording plus Given/When/Then scenarios when editing specs. Strict TDD is enabled for this project.
- **The roadmap is `internal-docs/plan-desarrollo-poc.md`** (Spanish), phases T1–T16 with a dependency graph, and `internal-docs/alcance-poc-vertice.md` holds acceptance criteria CA-1…CA-17. Changes should trace to a task and its CA numbers. T1 (workspace/CI) and T2 (domain model + type contract) are done; T3 onward is the adapter work.
- **Read-only invariant (CA-16)** is a first-commit invariant, not a final-phase check: no `File::create`, `OpenOptions::write`, or equivalent outside the app data directory. Tauri capabilities in `crates/vertice-app/capabilities/default.json` stay minimal (`core:default` only — no fs, shell, or dialog permissions).
- **Core tests run against versioned fixtures**, never against the machine's real installation. Real-tool oracles (`opencode debug …`, `claude agents`) are manual verification.
- **Frontmatter parsing must not use regex** — multiline block scalars (`description: >`) break it; go through the YAML seam.
- **Scope** (`User` / `Project` / `Local`) exists in the model even though the PoC only produces `User`. `Location::path` is `Option` because some components are embedded with no file on disk.
- Internal planning docs are Spanish; all code, comments, specs, and commit messages are English. Conventional commits, no AI attribution.
