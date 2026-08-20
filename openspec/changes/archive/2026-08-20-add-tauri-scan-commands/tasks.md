# Tasks: Add Tauri Scan Commands

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 180–260 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR (shell + frontend land together; either alone is unverifiable end to end) |
| Delivery strategy | single-pr (none received; risk is Low) |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | T10 complete: commands, hardening, frontend wrapper | PR 1 | Base: main; tests included; vertice-core untouched. |

## Phase 1: T10 Command Tests — RED (Minimal Scan Command Surface, Non-Blocking Command Execution, Typed IPC Contract)

- [x] 1.1 RED — Create `crates/vertice-app/src/commands.rs` with a `#[cfg(test)]` module: `async_runtime::block_on(run_scan())` test plus a panicking `spawn_blocking` test asserting JoinError maps to `ScanError::Internal { reason }`. Fails to compile (no `run_scan` yet). No `generate_context!()`, no webview, no disk.
- [x] 1.2 RED — Declare `mod commands;` in `crates/vertice-app/src/lib.rs` so the RED failure surfaces at compile time.

## Phase 2: T10 Command Implementation — GREEN (Minimal Scan Command Surface, Non-Blocking Command Execution, Typed IPC Contract)

- [x] 2.1 GREEN — Implement in `commands.rs` per design: private `async fn run_scan()` wrapping `tauri::async_runtime::spawn_blocking(vertice_core::scan::scan)` with the single `map_err` JoinError→`ScanError::Internal` line; `#[tauri::command] pub async fn scan()`/`rescan()` as one-line delegations. No business logic, no cache, no state.
- [x] 2.2 GREEN — Register `.invoke_handler(tauri::generate_handler![commands::scan, commands::rescan])` in `lib.rs`. Registration correctness is compile-time; no runtime App test.

## Phase 3: T10 Shell Hardening (Minimal Capability Grant, Hardened Content Security Policy)

- [x] 3.1 Update only the `description` in `crates/vertice-app/capabilities/default.json` with the design rationale; permissions stay `core:default` — no fs/shell/dialog plugin or scope. **Read-only invariant (CA-16): webview holds zero filesystem capability; T14 audits this file.**
- [x] 3.2 Harden CSP in `crates/vertice-app/tauri.conf.json`: append `object-src 'none'; base-uri 'none'`. No remote content, no production `unsafe-inline`; do not preemptively weaken for dev mode.

## Phase 4: T10 Frontend Wrapper — RED → GREEN (Frontend Filesystem Boundary, Typed IPC Contract)

- [x] 4.1 RED — Create `frontend/src/lib/scan.test.ts` (Vitest, `vi.mock("@tauri-apps/api/core")`): assert command names `"scan"`/`"rescan"`, report passthrough, and rejection satisfying `isScanError`. Fails (no `scan.ts` yet).
- [x] 4.2 GREEN — Create `frontend/src/lib/scan.ts`: `scan()`/`rescan()` as `invoke<ScanReport>(...)` wrappers plus the `isScanError` guard, importing types only from `../bindings/`. No Tauri fs plugin usage anywhere.
- [x] 4.3 Add at most a smoke invocation of `scan()` in `frontend/src/App.svelte` (inventory UI is T11).

## Phase 5: T10 Quality Gate

- [x] 5.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` (build `frontend/dist` first locally; CI downloads the artifact). Fix only T10-caused failures.
- [x] 5.2 Run `npm run lint && npm run check && npm run test && npm run build` in `frontend/`; confirm zero binding diff (`cargo test -p vertice-core` regenerates clean).
- [ ] 5.3 Manual smoke: `npx --prefix frontend tauri dev` once — real IPC round-trip and empirical dev-mode CSP check. Grep the T10 diff for `File::create`/`OpenOptions::write`, `tauri` imports outside vertice-app, new dependencies, or `tauri-plugin-fs`; reject all as out of scope. **(Static greps done and clean in apply; the interactive `tauri dev` GUI smoke is deferred to sdd-verify / a user session — launching a desktop window from a headless agent is out of bounds.)**
