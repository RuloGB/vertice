# Exploration — T10: Tauri commands and permissions

- **Change name**: `add-tauri-scan-commands`
- **Roadmap phase**: T10 (`internal-docs/plan-desarrollo-poc.md:231`)
- **Acceptance criteria**: types crossing the IPC are the ones generated in T2 (not hand-written); the capabilities declaration is reviewable and grants no more than necessary. (No CA-n row closes at T10; T14 later audits the ACL.)
- **Depends on**: T9 (scan orchestrator) — merged (`2026-08-20-add-scan-orchestrator`)
- **Status**: exploration only. No proposal, no spec, no implementation.

## 1. Current state

**Core scan workflow (T9, archived — no gap):**

- `vertice_core::scan::scan() -> Result<ScanReport, ScanError>` (`crates/vertice-core/src/scan.rs:15`). Synchronous and blocking: resolves `home_dir()`, runs four adapters + consolidation sequentially, measures duration internally, returns an in-memory report. **No state, no cache, no persistence** — the `scan-orchestration` spec mandates in-memory results and explicitly forbids IPC/UI in core.

**IPC types are already ready (T2 contract — no gap):**

- `ScanReport` derives `Serialize, Deserialize, TS` (camelCase; `PathBuf`→string, `duration_ms: u32`→number — fully JSON-safe).
- `ScanError` derives `Serialize + TS` (deliberately no `Deserialize`), serde-tagged `{kind, detail}`; the generated binding is `{kind:"noRootsConfigured"} | {kind:"internal", detail:{reason:string}}`.
- Both already live in `frontend/src/bindings/` — **zero new derives needed**.

**`vertice-app` is a bare skeleton:**

- `lib.rs` = `tauri::Builder::default().run(generate_context!())` — no commands, no plugins. Deps: `tauri 2 (features=[])` + `vertice-core` (path).
- `capabilities/default.json` = `core:default` only.
- CSP already present in `tauri.conf.json`: `default-src 'self'; connect-src 'self' ipc: http://ipc.localhost`. No remote content anywhere.

**Frontend:** `@tauri-apps/api` ^2.11.1 already installed, never used yet; `App.svelte` is static; convention is pure functions in `src/lib/*.ts` + Vitest tests; types consumed from `src/bindings/`.

**Specs:** 10 living capabilities, all core/CI-side; T10 introduces one **new** capability, no delta to existing specs expected. `deny.toml` bans `tauri`/`tauri-build` with `vertice-app` as sole wrapper — command code must live in vertice-app only.

## 2. Affected areas

- `crates/vertice-app/src/lib.rs` — register `.invoke_handler` for the two commands
- `crates/vertice-app/src/commands.rs` (new) — thin async handlers + JoinError→ScanError mapping
- `crates/vertice-app/capabilities/default.json` — decision point: stays `core:default` (update description rationale)
- `crates/vertice-app/tauri.conf.json` — verify/tighten CSP
- `frontend/src/lib/scan.ts` (new) + `scan.test.ts` — typed `invoke<ScanReport>` wrapper, Vitest-mocked `@tauri-apps/api/core`
- `frontend/src/App.svelte` — at most a smoke invocation; the inventory UI is T11
- `openspec/specs/` — one new capability (e.g. `desktop-shell`); existing specs untouched
- `crates/vertice-core` — **NO changes** (that is the point)

## 3. Design forks

### Fork A — capabilities / fs scope

| Approach | Description | Pros | Cons | Effort |
|---|---|---|---|---|
| **A1. Stay `core:default` only (recommended)** | All disk access is in vertice-core via `std::fs` against compile-time-fixed roots. Tauri 2 ACL gates only webview→core/plugin IPC, and `tauri-plugin-fs` is not even a dependency, so the webview has **zero filesystem capability**. "Disk access restricted by scope" is satisfied by construction. The PoC has no persistence, so not even app-data-dir access is needed at T10. | Maximal least privilege; reviewable 7-line file; trivial T14 audit | Plan wording mentions scopes + app data dir → must document the interpretation explicitly | Low |
| **A2. Add `tauri-plugin-fs` with scoped permissions** | Grant the webview read scope to known roots + app data dir. | Literal reading of the plan | **Grants** the webview fs access the frontend never uses (everything goes through commands) — enlarges attack surface, violates least privilege and the plan's own "frontend never touches the filesystem" line; new dependency | Medium — **rejected** |

### Fork B — command shape (scan is blocking, up to the CA-15 2s budget)

| Approach | Description | Pros | Cons | Effort |
|---|---|---|---|---|
| **B1. Async commands + `spawn_blocking` (recommended)** | `async fn scan() -> Result<ScanReport, ScanError>` wrapping `tauri::async_runtime::spawn_blocking(vertice_core::scan::scan)`. Tauri 2 runs non-async commands on the **main thread** → up to 2s of frozen UI (WebView2 shares the Windows message loop). | Canonical pattern; UI never blocked; explicit | JoinError needs mapping (one line into the existing `ScanError::Internal`) | Low |
| **B2. Sync `#[tauri::command]`** | Shortest code, but runs on the main thread → frozen window. | — | Blocks UI up to 2s | **rejected** |
| **B3. `#[tauri::command(async)]` non-async fn** | Runs on a runtime worker thread; viable but less explicit than `spawn_blocking`, no benefit over B1. | — | Less explicit | Low |

### Fork C — error transport

| Approach | Description | Pros | Cons | Effort |
|---|---|---|---|---|
| **C1. Return `Result<ScanReport, ScanError>` directly (recommended)** | `ScanError: Serialize` already; the invoke rejection payload is the serde-tagged JSON matching the generated TS binding exactly. No DTO, no string mapping → satisfies "types crossing IPC are the generated ones" by construction. | Contract exactness | — | Low |
| **C2. String errors** (`Result<ScanReport, String>`) | Loses the `kind` discriminant, forces a hand-written frontend type. | — | Violates the acceptance criterion | **rejected** |
| **C3. New app-layer error DTO** | Unnecessary duplication; `ScanError` was designed (owned payloads only, Serialize, TS) precisely to cross this boundary. | — | Duplication | **rejected** |

### Fork D — scan vs rescan (OPEN DESIGN QUESTION)

The core has NO cache or state; `scan()` is pure and re-resolves home each call; the T9 spec mandates in-memory/no persistence. In the PoC `rescan()` is therefore **semantically identical** to `scan()` — the difference is frontend intent only (startup auto-scan vs manual reload button, T11).

- **D1 (recommended)**: implement both commands per the plan's explicit scope ("Comandos: `scan()` y `rescan()`. Nada más."), both thin pass-throughs; the delta spec states plainly that no caching exists and that `rescan` keeps the IPC contract stable for future cache-invalidation semantics. Cost ~5 lines.
- **D2**: collapse to one `scan` command the frontend calls on both occasions. Deviates from the plan; saves nothing meaningful.

## 4. Recommendation

Capabilities stay **`core:default` only** — no fs plugin; least privilege by construction, documented as the fulfillment of the plan's "restricted by scope" line, with T14 auditing it. Two **async** commands `scan`/`rescan`, each a thin `spawn_blocking(vertice_core::scan::scan)` wrapper returning `Result<ScanReport, ScanError>` directly; JoinError maps to the existing `ScanError::Internal` variant (transport mapping, not business logic). Frontend gets a typed `lib/scan.ts` invoke wrapper (bindings-imported types only). Verify and tighten CSP in `tauri.conf.json` (add `object-src 'none'; base-uri 'none'`; production needs no `unsafe-inline` — Tailwind v4 emits a stylesheet, and dev-mode inline styles come from the Vite server, which does not carry Tauri's CSP). One new openspec capability spec for the desktop shell/IPC surface; no changes to vertice-core; no new bindings.

## 5. Risks

1. **Wrong command shape (sync)** blocks the main thread up to the 2s CA-15 budget → frozen UI on Windows. Mitigated by the `spawn_blocking` choice.
2. **Plan's "disk access restricted by scope" misread** as "install fs plugin with scopes" would grant the webview unused fs access — the proposal/design must state the no-fs-permission interpretation explicitly for reviewers.
3. **`tauri::generate_context!()` embeds `frontend/dist` at compile time**; `cargo test -p vertice-app` needs `npm run build` first locally (CI already downloads the dist artifact). Keep command tests in a module that does not depend on the built context where possible.
4. **Two identical commands (scan/rescan)** may read as dead API surface; the spec must justify the duplication (plan-mandated, future cache semantics) rather than invent a behavioral difference that does not exist.
5. **Dev-mode CSP**: any violation in `tauri dev` would come from the Vite dev server, not Tauri's CSP — verify once empirically instead of preemptively weakening the policy.

## Ready for proposal

Yes — propose change `add-tauri-scan-commands` (T10): two async commands, capabilities unchanged at `core:default`, direct ScanReport/ScanError transport, CSP hardening, frontend invoke wrapper, new `desktop-shell` (or similar) capability spec. Open question D (scan vs rescan identity) is explicitly deferred to the proposal/spec phase for user confirmation; recommendation is D1 (implement both, per the plan).
