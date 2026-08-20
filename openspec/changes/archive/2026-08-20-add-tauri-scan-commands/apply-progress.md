# Apply Progress: Add Tauri Scan Commands (T10)

Change: `add-tauri-scan-commands`
Mode: **Strict TDD**
Delivery: single-pr (tasks.md Review Workload Forecast: 400-line budget risk Low, no chained PRs, no decision needed).

## Status

**11/12 tasks complete.** `tasks.md` is updated in place with `[x]` marks (5.3 left unchecked: its static greps are done and clean; the interactive `tauri dev` GUI smoke is deferred — see Remaining tasks). No previous apply-progress existed for this change (first apply session).

## Files changed

| File | Action | What was done |
|---|---|---|
| `crates/vertice-app/src/commands.rs` | Created (92 lines) | Private `async fn run_scan()` wrapping `tauri::async_runtime::spawn_blocking(vertice_core::scan::scan)` with the single `map_err(map_join_error)` line; private `map_join_error(impl Display) -> ScanError::Internal`; `#[tauri::command] pub async fn scan()`/`rescan()` as one-line delegations; `#[cfg(test)]` module with 3 transport tests. No `generate_context!()`, no App, no state, no business logic. |
| `crates/vertice-app/src/lib.rs` | Modified (14 lines, +3/-1) | Added `mod commands;` and `.invoke_handler(tauri::generate_handler![commands::scan, commands::rescan])`. |
| `crates/vertice-app/capabilities/default.json` | Modified (7 lines, description only) | New description states the rationale; `permissions` unchanged: `["core:default"]`. |
| `crates/vertice-app/tauri.conf.json` | Modified (41 lines, CSP only) | CSP now `default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'`. No `unsafe-inline`, no remote content. |
| `frontend/src/lib/scan.ts` | Created (33 lines) | `scan()`/`rescan()` as `invoke<ScanReport>("scan" \| "rescan")` one-liners; `isScanError(error: unknown): error is ScanError` guard validating `kind` (and `detail.reason: string` for `internal`). Types imported only from `../bindings/`. No fs plugin. |
| `frontend/src/lib/scan.test.ts` | Created (82 lines) | Vitest, `vi.mock("@tauri-apps/api/core")` (1 mock): command names, report passthrough by identity (`toBe`), both rejection variants satisfying `isScanError`, 5 negative guard cases + both-variant acceptance. 10 tests. |
| `frontend/src/App.svelte` | Modified (18 lines, +9/-1) | Smoke-only `onMount(() => void scan().catch(...))` — exercises the IPC round-trip once; inventory UI is T11. |

Unchanged, verified by `git status --short -- frontend/src/bindings crates/vertice-core` (empty): **all of `crates/vertice-core` and `frontend/src/bindings`**. Also untouched: `Cargo.toml`, `Cargo.lock`, `deny.toml` (no new dependencies).

## TDD Cycle Evidence

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|---|---|---|---|---|---|---|---|
| 1.1/1.2 command surface RED | `crates/vertice-app/src/commands.rs` (`#[cfg(test)]`) | Unit (crate-internal seam) | ✅ `cargo test -p vertice-app --locked` → 0 tests, exit 0 (pre-change baseline) | ✅ Observed: `error[E0432]: unresolved imports super::map_join_error, super::rescan, super::run_scan, super::scan — no run_scan in commands` | ➖ impl lands in 2.1 | ➖ | ➖ |
| 2.1 `run_scan` + commands | same | Unit | ✅ as above | ✅ (RED from 1.1 covers) | ✅ `cargo test -p vertice-app --locked --lib` → `run_scan_resolves_with_a_consolidated_report`, `scan_and_rescan_both_delegate_to_a_fresh_scan`, `join_failure_maps_to_scan_error_internal` … 3 passed | ✅ 3 cases: Ok report with non-empty roots, scan/rescan identical fresh scans, panicking blocking task → `ScanError::Internal` with non-empty reason | ➖ None needed (file is 4 thin fns) |
| 2.2 handler registration | compile-time via `generate_handler!` | Compile-time | ✅ | ✅ by construction (design: no runtime App test) | ✅ whole-workspace build+test green | ➖ skipped: purely structural, one possible outcome | ➖ |
| 3.1 capabilities description | — | Config | ✅ | ➖ skipped: data-only file, no logic | ✅ gate: permissions still `["core:default"]` | ➖ skipped: purely structural | ➖ |
| 3.2 CSP hardening | — | Config | ✅ | ➖ skipped: data-only file, no logic | ✅ gate: exact design CSP string present | ➖ skipped: purely structural | ➖ |
| 4.1 frontend wrapper RED | `frontend/src/lib/scan.test.ts` | Unit (mocked IPC boundary) | ✅ `npm run test` → 2/2 appTitle tests pass (pre-change) | ✅ Observed: `Error: Cannot find module '/src/lib/scan' imported from .../scan.test.ts` — 1 failed, no tests ran | ➖ impl lands in 4.2 | ➖ | ➖ |
| 4.2 frontend wrapper GREEN | `frontend/src/lib/scan.ts` | Unit | ✅ as above | ✅ (RED from 4.1 covers) | ✅ `npx vitest run src/lib/scan.test.ts` → 10/10 pass | ✅ 10 cases: 2 command names + identity passthrough, 2 typed rejections, 5 negative guard inputs (null/undefined/string/unknown kind/internal missing detail), both-variant acceptance | ➖ None needed |
| 4.3 App.svelte smoke | — (manual smoke is task 5.3) | Smoke | ✅ | ➖ skipped: smoke wiring, no logic to assert (inventory UI is T11) | ✅ svelte-check 0 errors/0 warnings; eslint clean | ➖ skipped: purely structural | ➖ |

### Test Summary

- **Total tests written**: 13 (3 Rust + 10 Vitest)
- **Total tests passing**: 13/13 new; full workspace suite green (89 core lib + 3 app lib + 115 core integration, 0 failures); frontend 12/12 (2 pre-existing + 10 new)
- **Layers used**: Unit (13). Integration/E2E deliberately not used — design mandates no runtime App test at T10; the real IPC round-trip is the manual smoke (5.3)
- **Approval tests** (refactoring): None — no refactoring tasks
- **Pure functions created**: 3 (`map_join_error`, `scan`/`rescan` wrappers as thin pure delegations, `isScanError`)

## Gate results (actually run, this environment — cargo 1.97.1 on PATH, Windows host)

| Gate | Command | Result |
|---|---|---|
| Rust fmt | `cargo fmt --all --check` | **PASS** |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** — exit 0, 0 warnings |
| Rust tests | `cargo test --workspace --locked` | **PASS** — exit 0; all suites ok (3 app + 89 core lib + 22/18/7/14/9/8/24/13/7/1 integration + doc-tests) |
| Frontend lint | `npm run lint` | **PASS** — eslint clean |
| Frontend check | `npm run check` | **PASS** — svelte-check 0 errors, 0 warnings |
| Frontend test | `npm run test` | **PASS** — 12/12 (2 files) |
| Frontend build | `npm run build` | **PASS** — `dist` built (also built pre-RED as baseline, since `generate_context!` embeds it) |
| Bindings diff | `git status --short -- frontend/src/bindings crates/vertice-core` | **PASS** — empty (zero diff; `cargo test --workspace` regenerated bindings clean) |
| Read-only grep | `Select-String` over all 5 changed/new source files for `File::create\|OpenOptions::write\|fs::write\|create_dir\|remove_\|tauri-plugin-fs\|@tauri-apps/plugin` | **PASS** — no matches |
| Capabilities scope | grep `permissions` in `capabilities/default.json` | **PASS** — still exactly `["core:default"]` |
| Change scope | `git status --short` | **PASS** — only the 7 files above + this change's openspec folder; no core/bindings/dependency-file diffs |

Not run: `cargo deny check bans licenses` — not required by this change's gates and dependency files are untouched (verified by git status). Linux/macOS CI legs and the real webview IPC round-trip are not exercisable from this session (see Remaining tasks).

## Deviations from design

1. **`map_join_error` is a named private function generic over `impl Display`, not an inline closure over `JoinError`.** Design §Interfaces shows `.map_err(|join| ScanError::Internal { reason: join.to_string() })`. Tauri 2.11.5's `async_runtime` re-exports `tokio::task::JoinHandle as TokioJoinHandle` but **not** `JoinError` (verified against the vendored crate source), and vertice-app has no direct tokio dependency, so the closure's parameter type is unnameable in this crate. Extracting the mapping into a named `fn map_join_error(join: impl Display) -> ScanError` keeps `run_scan()` at exactly the design's single `map_err` line while making the transport mapping directly testable with a genuine `JoinError` from a real panicking `spawn_blocking` task — which is what tasks.md 1.1's "panicking spawn_blocking test asserting JoinError maps to ScanError::Internal" requires. Semantics are identical to the design's closure.
2. **The assertion on the join-failure test checks a non-empty `reason`, not a panic-message substring.** tokio's `JoinError: Display` renders a panicked task as `task … panicked` without the panic payload text, so `reason.contains("simulated core failure")` would not hold; the test asserts the `Internal` variant plus a non-empty `reason`, matching the design's "assert variant + reason".

No other deviations. `vertice-core` untouched; no DTOs; no cache/state; no fs/shell/dialog capability; no new dependencies; no events.

## Issues found

None. Environment note: an earlier pipeline-style invocation of `cargo test --workspace --locked` reported `$LASTEXITCODE: -1` through a `Select-String` chain — a PowerShell 5.1 pipeline artifact; re-run with output redirected to a file reported exit 0 with every suite `ok`.

## Workload / PR Boundary

- Mode: single-pr (forecast Low risk, 180–260 estimated lines; no decision required)
- Current work unit: Unit 1 — T10 complete (commands, hardening, frontend wrapper)
- Boundary: this batch covered Phases 1–5 of tasks.md (all 12 tasks attempted; 11 landed, 5.3's manual smoke deferred)
- Estimated review budget impact: ~140 added source/test lines + 2 one-line config edits — well inside the 400-line budget

## Remaining tasks

- [ ] 5.3 (manual half only): run `npx --prefix frontend tauri dev` once in an interactive session — confirm the real IPC round-trip (App.svelte smoke invocation resolves/rejects visibly) and empirically check dev-mode CSP. Not done from this headless agent session because it opens a desktop window on the user's machine and runs until manually closed. Static half of 5.3 (diff greps) is done and clean. Ready input for sdd-verify.
