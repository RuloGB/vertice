# Design: Add Tauri Scan Commands

## Technical Approach

Expose T9's `vertice_core::scan::scan()` over IPC as two thin async commands (the `desktop-shell` delta spec); the shell adds no business logic. IPC types are the T2-generated `ScanReport`/`ScanError`, unchanged.

**Core data model changes: none.**

**CLI-pathway isolation:** only `vertice-app` imports Tauri (`deny.toml` bans it elsewhere); a future CLI links `vertice-core` as-is. No platform-path work: root resolution (XDG / `%APPDATA%` / `~/Library`) is core-internal since T3–T9.

```text
vertice-core (no tauri) <-path dep- vertice-app::commands (this change)
                        <-path dep- vertice-cli (post-PoC, unchanged)
```

## Architecture Decisions

| Decision | Choice | Alternatives considered | Rationale |
|---|---|---|---|
| Command shape | `async fn` + `tauri::async_runtime::spawn_blocking(scan)` | Sync command; `#[tauri::command(async)]` | Tauri 2 runs sync commands on the main thread; a CA-15 2s scan plus WebView2 sharing the Windows message loop means a frozen window. Cost: one JoinError mapping. |
| Error transport | `Result<ScanReport, ScanError>` directly | `String` errors; app DTO | `ScanError: Serialize` already; the rejection payload equals the generated binding. |
| `scan` vs `rescan` | Both, identical pass-throughs over one private `run_scan()` | Single command | Core has no cache/state (T9). `rescan` keeps the IPC contract stable for future cache semantics; the plan mandates both. No invented difference. |
| Capabilities | `core:default` only | `tauri-plugin-fs` + scopes | ACL gates webview→plugin IPC; no fs-plugin dependency → the webview holds zero fs capability. Disk access is core Rust against compile-time-fixed roots — "restricted by scope" by construction; T14 audits CA-16. |
| Test seam | Private `run_scan()` + `async_runtime::block_on`; no `generate_context!()`/App in tests | Test via built App | `generate_context!()` embeds `frontend/dist` at compile time: crate compile still needs `dist` (accepted — CI downloads the artifact; locally one `npm run build`), but tests build no Tauri context and hit no disk. |

## Data Flow

```text
App.svelte (smoke) -> lib/scan.ts: invoke<ScanReport>("scan" | "rescan")
  -> IPC -> commands::{scan,rescan} -> run_scan() -> spawn_blocking(core::scan)
  <- Ok: ScanReport JSON | Err: ScanError JSON
```

Events: **none** — request/response only.

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-app/src/commands.rs` | Create | Handlers, `run_scan`, JoinError mapping, transport tests |
| `crates/vertice-app/src/lib.rs` | Modify | `mod commands;` + `.invoke_handler(tauri::generate_handler![commands::scan, commands::rescan])` |
| `crates/vertice-app/capabilities/default.json` | Modify | Rationale in description only; permissions unchanged |
| `crates/vertice-app/tauri.conf.json` | Modify | CSP hardening (below) |
| `frontend/src/lib/scan.ts` | Create | Typed invoke wrapper + `isScanError` guard |
| `frontend/src/lib/scan.test.ts` | Create | Vitest, mocked `@tauri-apps/api/core` |
| `frontend/src/App.svelte` | Modify | Smoke invocation; inventory UI is T11 |

## Interfaces / Contracts

`commands.rs` (imports from `vertice_core::model`); the `map_err` line is the whole transport mapping:

```rust
async fn run_scan() -> Result<ScanReport, ScanError> {
    tauri::async_runtime::spawn_blocking(vertice_core::scan::scan)
        .await
        .map_err(|join| ScanError::Internal { reason: join.to_string() })?
}

#[tauri::command]
pub async fn scan() -> Result<ScanReport, ScanError> { run_scan().await }

#[tauri::command]
pub async fn rescan() -> Result<ScanReport, ScanError> { run_scan().await }
```

Exact rejection payload the frontend receives (matches the generated `ScanError` binding):

```json
{ "kind": "internal", "detail": { "reason": "join failure: ..." } }
```

Recoverable per-item problems stay `ScanIssue`s inside `Ok(ScanReport)`; only orchestration/join failures reject.

Frontend wrapper (`frontend/src/lib/scan.ts`):

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { ScanError } from "../bindings/ScanError";
import type { ScanReport } from "../bindings/ScanReport";

export function scan(): Promise<ScanReport>;    // invoke<ScanReport>("scan")
export function rescan(): Promise<ScanReport>;  // invoke<ScanReport>("rescan")
export function isScanError(error: unknown): error is ScanError;
```

The guard narrows untyped rejections to `ScanError`.

Capabilities — exact resulting file:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Minimal capability: core windowing/events only. Disk access lives in vertice-core against compile-time-fixed roots; with no tauri-plugin-fs the webview holds zero filesystem/shell/dialog capability. 'Restricted by scope' by construction; T14 audits CA-16.",
  "windows": ["main"],
  "permissions": ["core:default"]
}
```

CSP before/after:

- Before: `default-src 'self'; connect-src 'self' ipc: http://ipc.localhost`
- After: `default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'`

Dev mode serves the Vite origin (Tauri does not set its headers); this CSP binds bundled production assets. Verify `tauri dev` once empirically; never preemptively weaken.

## Testing Strategy

Strict TDD: red first.

| Layer | What to Test | Approach |
|---|---|---|
| vertice-app | JoinError → `ScanError::Internal` mapping | Panicking blocking task → mapped error; assert variant + reason. Deterministic, no disk/context. |
| vertice-app | Registration | Compile-time via `generate_handler!`; no runtime App test. |
| Frontend | Wrapper + guard | Vitest, `vi.mock("@tauri-apps/api/core")`: command names, report passthrough, rejection satisfies `isScanError`. Follows `src/lib/*.test.ts`. |
| Smoke | Real IPC round-trip | Manual `tauri dev` once; no tauri-driver E2E at T10. |

Clippy, fmt, and the bindings-in-sync gate are unaffected — no core type changes, zero binding diff.

## Migration / Rollout

No migration, no persisted state. Rollback: **core** untouched; **app** — delete `commands.rs`, registration, two CSP directives (back to skeleton); **frontend** — delete `scan.ts` + test, revert App.svelte smoke lines.

## Open Questions

None.
