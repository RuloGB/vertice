# Exploration — T11: Inventory interface

- **Change name**: `add-inventory-ui`
- **Roadmap phase**: T11 (`internal-docs/plan-desarrollo-poc.md:250–266`)
- **Acceptance criteria**:
  - **CA-1**: app starts and shows the list with no user configuration
  - **CA-3** (visual half): duplicate mark + all three paths visible for the 22 multi-root skills (data half already closed by T8)
- **Depends on**: T10 — archived `2026-08-20-add-tauri-scan-commands` (IPC `scan`/`rescan` ready)
- **Unblocks**: T12 (i18n), T13 (error/empty/non-actionable states)
- **Status**: exploration only. No proposal, no spec, no implementation.

## Exploration: T11 — Inventory interface

### Current State

**T10 dependency is closed and unblocked.** Archived change `openspec/changes/archive/2026-08-20-add-tauri-scan-commands/` delivers:

| Layer | What exists | Path / lines |
|-------|-------------|--------------|
| IPC commands | Async thin `scan` / `rescan` via `spawn_blocking` → `vertice_core::scan::scan` | `crates/vertice-app/src/commands.rs:15–43` |
| Registration | `generate_handler![commands::scan, commands::rescan]` | `crates/vertice-app/src/lib.rs:11` |
| Frontend wrapper | Typed `scan()`, `rescan()`, `isScanError()` — bindings only | `frontend/src/lib/scan.ts:1–33` |
| Wrapper tests | 10 Vitest cases, mocked `@tauri-apps/api/core` | `frontend/src/lib/scan.test.ts` |
| Living spec | `desktop-shell` (6 requirements, 9 scenarios) | `openspec/specs/desktop-shell/spec.md` |
| Capabilities | `core:default` only — no fs/shell/dialog | `crates/vertice-app/capabilities/default.json` |

**Frontend is a smoke shell, not an inventory UI.**

- `App.svelte` (`frontend/src/App.svelte:1–18`): `onMount` fires `scan()` and **discards** the report (`console.warn` on failure only). Renders a centered title — no list, filter, search, or reload.
- Stack ready: Svelte 5 + Vite 8 + Tailwind 4 (`frontend/package.json`, `app.css` = `@import "tailwindcss"` only).
- Conventions: pure functions in `src/lib/*.ts` + Vitest; types from `src/bindings/` (ts-rs generated — never hand-edit).
- **No component test harness**: `vitest.config.ts` uses `environment: "node"` and `include: ["src/**/*.{test,spec}.ts"]` — no jsdom/happy-dom, no `@testing-library/svelte`, no `*.svelte` test include. Existing tests are pure-TS module tests only (`appTitle.test.ts`, `scan.test.ts`).

**Domain model already carries everything the UI needs** (T2 + T8):

```
ScanReport {
  components: Component[]   // consolidated; one row per identity
  installations             // T13 (client not-detected) — out of T11
  rootsScanned              // T13 (absent root) — out of T11
  issues                    // T13 (unparseable) — out of T11
  durationMs
}

Component {
  id: string                // "{kind}:{normalized name}"
  name, kind: "skill"|"agent", description: string|null
  locations: Location[]     // N>1 ⇒ duplicate (CA-3 mark = locations.length > 1)
  scope, provenanceHint
}

Location { path: string|null, root: SearchRootId, origin: "file"|"embedded" }
```

- Duplicate mark is **derived**, not a field: T8 consolidates so 22 skills have `locations.length === 3`, 3 have `=== 1` (`openspec/specs/duplicate-consolidation/spec.md` scenarios). UI MUST NOT re-group by name.
- Paths for CA-3 come from `Location.path` (may be `null` for embedded agents — T13 marks those; T11 should still render the row and list whatever paths exist).
- `ScanError` is rare orchestration failure only (`noRootsConfigured` | `internal`); per-item problems live in `report.issues` and must **not** fail the list render.

**Plan scope for T11 (verbatim intent):**

1. Unified list: name, kind, description, origin path(s)
2. Visual duplicate mark with expand/reveal of **all** paths
3. Filter by kind (skill / agent) + search by name
4. Reload button; **no watchers / no auto-refresh**
5. Auto-scan on startup, zero prior configuration → CA-1

**Explicitly deferred (do not pull into T11):**

| Concern | Owner |
|---------|-------|
| i18n catalogs / language selector | T12 |
| Client not-detected, unparseable row, embedded-without-actions, absent root messaging | T13 |
| CA-16 write audit | T14 |
| Installations panel, issues panel as first-class UI | T13 |
| Core / bindings / capabilities / CSP changes | none expected |

**Living specs today:** 11 capabilities under `openspec/specs/`. None covers inventory presentation. T11 should add a **new** frontend capability (e.g. `inventory-ui`), not delta core specs. `desktop-shell` stays as-is (consume `scan`/`rescan` only).

**Strict TDD** is on (`openspec/config.yaml` `strict_tdd: true`). Frontend test strategy is a design fork (see Approaches).

### Affected Areas

- `frontend/src/App.svelte` — replace smoke shell with inventory screen: startup `scan()`, hold report state, compose list + controls
- `frontend/src/lib/` — new pure modules (recommended): filter/search predicates, duplicate helper (`locations.length > 1`), optional view-model mappers; keep IPC in existing `scan.ts`
- `frontend/src/lib/*.test.ts` — Vitest coverage for pure logic (and optionally component tests if harness is added)
- `frontend/src/**/*.svelte` (new) — presentational pieces: list, row, duplicate paths disclosure, kind filter, name search, reload control, loading/empty shells (minimal — rich empty/error copy is T12/T13)
- `frontend/src/app.css` — only if global tokens beyond utility classes are needed (prefer Tailwind utilities)
- `frontend/vitest.config.ts` + `package.json` — **only if** component-DOM testing is chosen (jsdom/happy-dom, testing-library, include globs)
- `openspec/specs/inventory-ui/spec.md` (new living capability after archive) — UI behavior, CA-1/CA-3 scenarios
- **Untouched by design:**
  - `crates/vertice-core/**` — no core changes
  - `crates/vertice-app/**` — no new commands, no capability/CSP edits
  - `frontend/src/bindings/**` — never hand-edit
  - `frontend/src/lib/scan.ts` — reuse as-is (`scan` on mount, `rescan` on reload)

### Approaches

1. **Thin App + pure lib filters + presentational Svelte 5 components (recommended)**
   - `App.svelte` owns lifecycle state: `idle | loading | ready | failed`, current `ScanReport | null`, filter kind, search query; calls `scan()` on mount and `rescan()` on reload.
   - Pure functions in `src/lib/` (e.g. `filterComponents(components, { kind, query })`, `isDuplicate(component)`) — unit-tested under existing node Vitest with fixture `Component[]` (no DOM).
   - Small Svelte components for row / path disclosure / toolbar; styling via Tailwind utility classes already in the project.
   - Pros: matches T10 conventions; testable logic without new deps; clear T12 seam (string externalization later); zero core/IPC churn; CA-3 mark is a one-liner on `locations.length`.
   - Cons: component markup itself has weaker automated coverage unless a DOM harness is added later; visual CA-3 still needs a manual/`tauri dev` check on the reference machine.
   - Effort: **Medium**

2. **Single monolithic `App.svelte` with inline filter logic**
   - Everything in one file: state, filter, markup.
   - Pros: fewest files; fast first paint of a demo.
   - Cons: hard to unit-test under strict TDD; fights existing `lib/*.ts` pattern; painful T12 string extraction; review noise in one huge diff.
   - Effort: **Low** short-term, **High** carry cost — **not recommended**

3. **Add component test harness now (jsdom/happy-dom + Testing Library) and TDD against rendered DOM**
   - Extend Vitest env, add deps, write `*.svelte` tests for list/filter/duplicate disclosure.
   - Pros: stronger CA-1/CA-3 regression net in CI without a real WebView; aligns with strict TDD for UI structure.
   - Cons: new toolchain surface mid-PoC; Svelte 5 + Vit 4 testing setup cost; still does not replace real-home visual check for CA-3 paths; larger PR.
   - Effort: **Medium–High** (can be a follow-up slice inside the same change if budget allows, not a blocker for the pure-logic path)

4. **Client-side state store (e.g. custom store / external lib) for report + filters**
   - Pros: scales if many screens appear.
   - Cons: one screen in the PoC; extra abstraction with no second consumer; out of character for current frontend.
   - Effort: **Medium** — **rejected** for PoC

**Boundary rules (all approaches):**

- Startup → `scan()`; Reload → `rescan()` (contract stability from T10; do not call `scan` for both and drop `rescan`).
- Duplicate ⇔ `component.locations.length > 1`; show **all** `location.path` values (and tolerate `null` without inventing actions — T13).
- Filter/search are **view-only** over `report.components`; never re-invoke core for filtering.
- Loading state required (scan may approach CA-15 2s budget; commands are non-blocking but the UI must not look frozen/broken).
- Hard command failure (`ScanError`): show a minimal failure surface so CA-1 is not a blank window; polished copy/i18n is T12, diagnostics panels are T13.
- Empty successful report (`components: []`): show an empty list region (not a crash); richer empty messaging is T13.
- No watchers, no interval refresh, no fs access from the webview.

### Recommendation

Ship **Approach 1**: inventory UI as a thin Svelte 5 composition over the existing `scan`/`rescan` wrappers, with **pure, Vitest-tested** filter/search/duplicate helpers in `frontend/src/lib/`, and presentational components styled with Tailwind.

**Change name**: `add-inventory-ui` (parallel to `add-tauri-scan-commands` / `add-scan-orchestrator`).

**Spec shape**: new capability `inventory-ui` (frontend). Do not modify `desktop-shell`, `duplicate-consolidation`, or domain-model specs except by reference.

**CA mapping in the future proposal/spec:**

| Criterion | T11 obligation |
|-----------|----------------|
| CA-1 | On launch, invoke `scan` with no settings UI; render consolidated component list when the report arrives |
| CA-3 | For any component with `locations.length > 1`, show a clear duplicate affordance and list every path (reference machine: 22 skills × 3 paths) |
| CA-2/4/8 data | Already guaranteed by core T8; UI must display consolidated rows, not raw pre-merge entries |

**Testing (strict TDD, pragmatic):**

1. RED/GREEN pure modules first: fixtures with mixed kinds, multi-location duplicates, null paths, empty description.
2. Keep Vitest on `node` unless the change explicitly adopts Approach 3.
3. Manual verification note (same pattern as T10 task 5.3): `tauri dev` on the reference machine for CA-1 + visual CA-3 — document as known limitation if deferred, do not pretend unit tests close visual CA-3 alone.

**Out of T11 PR surface:** i18n framework, issues/installations/roots panels, embedded action suppression UX, core/Rust, bindings regeneration, capability JSON, CSP.

**Workload signal for later `sdd-tasks`:** likely **Medium** 400-line risk (several Svelte files + lib tests + new spec). Prefer one PR if under budget; split only if component-harness (Approach 3) is bundled in.

### Risks

1. **Scope creep into T12/T13** — polishing every empty/error/client-absent string and i18n in T11 bloats the change and blurs phase gates. Mitigate: T11 ships structural UI + minimal English placeholders acceptable until T12 externalizes strings; issues/installations stay off-canvas.
2. **Mis-defining “duplicate”** — deriving from name equality or re-grouping client-side would fight T8 and break CA-2/CA-3. Mitigate: spec MUST state `locations.length > 1` as the only mark rule; tests pin it.
3. **CA-3 only half-automatable** — unit tests prove disclosure logic; three real paths need a real (or rich fixture) report in a WebView. Mitigate: pure tests + explicit manual `tauri dev` checklist on reference install; optional later DOM harness.
4. **Blocking UX on slow scan** — commands are async, but UI with no loading state looks broken during up to ~2s. Mitigate: explicit loading state from invoke start to settle.
5. **Treating `ScanError` like empty inventory** — orchestration failure ≠ zero components. Mitigate: distinct `failed` state; successful empty report still shows empty list chrome.
6. **Null `Location.path` in the paths list** — embedded agents (if present in report) have `path: null`. Mitigate: render safely without offering install/update/uninstall (full treatment is T13); never assume `path` is always a string.
7. **Hand-edited bindings or new IPC DTOs** — would violate the T2 contract. Mitigate: import only from `frontend/src/bindings/`; no new commands.
8. **Literal strings everywhere** — T12 will require extraction. Mitigate: keep user-visible strings in a small constants module or clustered in components so T12 is mechanical, but do **not** build i18n infra in T11.
9. **T10 manual smoke still open** — archive notes task 5.3 interactive `tauri dev` half deferred. Mitigate: T11 manual CA-1 pass effectively re-validates IPC; no blocker, but first real UI exercise of the shell.

### Ready for Proposal

**Yes.** T10 is archived and the IPC/type contract is sufficient; T11 is a pure frontend composition problem with a clear CA-1/CA-3 target and hard non-goals (T12/T13/core).

Orchestrator should tell the user:

1. Proposed change name: **`add-inventory-ui`**.
2. Exploration artifact written at `openspec/changes/add-inventory-ui/exploration.md`.
3. Recommended approach: thin Svelte inventory over existing `scan`/`rescan`, pure testable filter/duplicate helpers, no core/IPC/capability changes.
4. Next phase: **`sdd-propose`** for `add-inventory-ui` (then spec → design → tasks).
5. Confirm before propose (optional product nits, not blockers):
   - Kind filter control shape (segmented control vs select) — cosmetic.
   - Whether Approach 3 (component DOM harness) is in or out of the first PR — default **out**.
   - Accept minimal English UI copy until T12 — default **yes** per phase split.
)
