# Archive Report: Add Tauri Scan Commands

**Change**: add-tauri-scan-commands
**Archived**: 2026-08-20
**Persistence**: OpenSpec (filesystem only — user-mandated; no Engram artifacts for this change)
**Status**: intentional-with-warnings
**Plan trace**: T10 (`internal-docs/plan-desarrollo-poc.md`)

## Completion Gate

- Persisted task artifact (`tasks.md`): **11/12 tasks complete**. The single unchecked entry is task 5.3, annotated in place: its static half (diff greps for read-only/scope violations) was executed and is clean; only the interactive `tauri dev` GUI smoke half is deferred. 5.3 is a quality-gate smoke task, not an implementation task — all implementation tasks (1.1–4.3) and the automatable gate tasks (5.1, 5.2) are checked.
- Apply progress (`apply-progress.md`): present; records the full TDD Cycle Evidence table with observed RED failures, gate results, and both deviations.
- Verification report (`verify-report.md`): verdict **PASS WITH WARNINGS**; **no CRITICAL issues**. Every gate was re-run independently in the verify session (Windows host, cargo 1.97.1, node v22.22.0): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` (215/215, debug and release), `npm run lint` / `check` / `test` (12/12) / `build` — all exit 0; bindings regeneration produced zero diff.
- Archive blocker policy: passed. Only CRITICAL verification issues block archive; none exist.

## Intentional Warning Acceptance

The user explicitly accepted the verification warning for **task 5.3's interactive half**: the real-webview IPC round-trip and the empirical dev-mode CSP check require running `npx --prefix frontend tauri dev` in an interactive user session — it opens a desktop window and runs until manually closed, which no headless agent session can do. The static half of 5.3 (diff greps) is done and clean.

This archive is therefore recorded as `intentional-with-warnings`, mirroring the accepted-warning pattern of the T9 cycle (`2026-08-20-add-scan-orchestrator`). The deferred manual smoke remains a documented known limitation requiring one manual user session; when run, its outcome (IPC round-trip + observed dev-mode CSP) should be recorded against this change's notes.

Related retained warnings from `verify-report.md`: the Non-Blocking scenario's "window event loop remains responsive" clause is empirically confirmable only in that same deferred smoke (offload itself is code- and test-proven); `cargo deny check bans licenses` could not run locally (`cargo-deny` not installed) — purity holds by git evidence and CI remains the enforcement point; verification ran on Windows only, with Linux/macOS legs deferred to CI (same as T9).

## Accepted Apply Deviations

Both deviations were documented by apply and independently assessed as acceptable by verify:

1. **`map_join_error` as a named private function generic over `impl Display`** instead of the design's inline closure over `JoinError`. Tauri 2.11.5's `async_runtime` does not re-export `JoinError` and `vertice-app` has no direct tokio dependency, so the closure parameter type is unnameable in this crate without violating the minimal-dependency posture. The named function keeps `run_scan()` at exactly the design's single `map_err` line and lets the join-failure test use a genuine `JoinError` from a really panicking `spawn_blocking` task. Semantics identical to the design.
2. **Join-failure test asserts variant + non-empty `reason`** instead of a panic-message substring. This matches design.md's own testing strategy verbatim ("assert variant + reason"); tokio's `JoinError: Display` renders a panicked task without the panic payload text, so a substring assertion would be version-fragile.

## Delta Spec Synchronization

| Domain | Action | Details |
|--------|--------|---------|
| `desktop-shell` | Created | New capability; the delta spec was a full spec and was copied directly to `openspec/specs/desktop-shell/spec.md` with 6 requirements: Minimal Scan Command Surface, Non-Blocking Command Execution, Typed IPC Contract, Minimal Capability Grant, Hardened Content Security Policy, Frontend Filesystem Boundary. |

## Out-of-Scope Check

Confirmed nothing out-of-PoC-scope was merged: **no MCPs, no write operations, no project/local scope, no persistence, no watchers/auto-refresh**. Specifically: no `tauri-plugin-fs` or any fs/shell/dialog capability or plugin anywhere (capabilities stay exactly `["core:default"]`); no new dependencies (`Cargo.toml`, `Cargo.lock`, `package.json`, `deny.toml` all clean in git); `crates/vertice-core` and `frontend/src/bindings` untouched; `App.svelte` carries only the single sanctioned startup smoke invocation (inventory UI is T11); read-only grep over all new/changed source files found zero matches for write APIs.

## CA Traceability

No CA-n row closes at T10 by design. The plan-level acceptance criteria for T10 are met: IPC types are the T2-generated `ScanReport`/`ScanError` (zero binding diff after regeneration; no hand-written DTOs), and the capabilities declaration is reviewable and minimal (seven-line file granting exactly `core:default`). **T14 will audit the capabilities declaration against CA-16** (read-only invariant); the declaration and its rationale are now living spec under `desktop-shell` / Minimal Capability Grant.

## Archive Validation

- Main spec created at `openspec/specs/desktop-shell/spec.md`.
- Change folder moved to `openspec/changes/archive/2026-08-20-add-tauri-scan-commands/`.
- Archived artifacts present: proposal, delta spec (`specs/desktop-shell/spec.md`), design, tasks, apply-progress, verify-report, and this archive report (plus `explore.md` from exploration).
- Archived `tasks.md` contains no unchecked implementation tasks; the single unchecked entry is the accepted manual-smoke deferral (5.3), annotated in place.
- Active `openspec/changes/add-tauri-scan-commands/` no longer exists.

## Known Limitations

- Task 5.3's interactive `tauri dev` smoke (real-webview IPC round-trip + empirical dev-mode CSP) is deferred to a manual user session — user-accepted.
- "Window event loop remains responsive" is empirically confirmable only in that manual smoke; the offload mechanism itself is test-proven.
- `cargo deny check bans licenses` remains unexecuted locally until `cargo-deny` is installed; CI is the enforcement point.
- Cross-platform release validation (Linux/macOS legs) remains a CI responsibility.
