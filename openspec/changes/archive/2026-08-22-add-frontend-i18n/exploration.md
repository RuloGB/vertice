# Exploration — T12: Frontend internationalization (en / es)

- **Change name**: `add-frontend-i18n`
- **Roadmap phase**: T12 (`internal-docs/plan-desarrollo-poc.md:269–283`)
- **Acceptance criteria (phase-local, not numbered CA-N)**:
  - No user-visible string literal remains in UI components
  - Changing language updates the whole interface with no untranslated fragments
- **Product / design drivers**:
  - Design principle 7: i18n from first commit (English + Spanish) — `openspec/config.yaml:19`, `internal-docs/stack-tecnologico-vertice.md:99`
  - Brief: SkillDock’s non-functional language selector is an explicit anti-pattern — `internal-docs/brief-outline-vertice.md:23`, `internal-docs/brief-outline-vertice.md:48`
- **Depends on**: T11 — archived `openspec/changes/archive/2026-08-21-add-inventory-ui/` (inventory UI live under `openspec/specs/inventory-ui/spec.md`)
- **Parallel with**: T13 (error/empty/non-actionable states) — same T11 dependency; T12 should leave T13 messaging hooks ready but not implement rich diagnostics
- **Status**: exploration only. No proposal, no spec, no implementation.

## Exploration: T12 — Frontend i18n (English and Spanish)

### Current State

**T11 is closed; the inventory UI is real and English-only.**

| Layer | What exists | Evidence |
|-------|-------------|----------|
| Inventory screen | Startup `scan`, reload `rescan`, filters, list, lifecycle states | `frontend/src/App.svelte` |
| Presentational components | Toolbar, list, row, location list | `frontend/src/lib/InventoryToolbar.svelte`, `InventoryList.svelte`, `InventoryRow.svelte`, `LocationList.svelte` |
| Pure helpers + tests | `filterComponents`, `isDuplicate`, `appTitle`, `scan` wrappers | `frontend/src/lib/*.ts` + `*.test.ts` |
| Living UI spec | Inventory behavior; **i18n explicitly out of scope** | `openspec/specs/inventory-ui/spec.md:56` |
| i18n infra | **None** — no catalog, no locale store, no language selector, no i18n dependency | `frontend/package.json` (Svelte/Vite/Tailwind/Vitest/Tauri API only) |
| HTML shell | Hardcoded `lang="en"`, title `Vertice` | `frontend/index.html:2–6` |
| Vitest | `environment: "node"`, pure TS tests only — no DOM harness | `frontend/vitest.config.ts` |

**User-visible English literals today (T12 extraction inventory):**

| Location | Strings |
|----------|---------|
| `App.svelte` | `"No search roots are configured."`, `` `Internal scan failure: ${…}` ``, `"The scan failed unexpectedly."`, `"Scanning for installed components..."`, `"Inventory scan failed."` |
| `InventoryToolbar.svelte` | `"Search by name"`, `"Search components by name"`, `"Filter by kind"`, `"All kinds"`, `"Skills"`, `"Agents"`, `"Reloading..."`, `"Reload"` |
| `InventoryList.svelte` | `"No components to show."` |
| `InventoryRow.svelte` | `"Duplicate"`, title `` `Found at ${n} locations` `` |
| `LocationList.svelte` | `"(no path on disk)"` |
| `appTitle` / header | Product title via `appTitle("Vertice", "0.1.0")` → `"Vertice v0.1.0"` (product name may stay untranslated; structure is presentational) |
| `main.ts` | `"Could not find #app mount element"` — bootstrap throw; **not UI chrome** (decide in design whether in catalog) |
| Dynamic data | `component.name`, `component.description`, `location.path`, `error.detail.reason` — **content from core/disk, not UI chrome** |

**Inherited policy from core (do not reverse in T12):**

- `ScanIssue.reason` is a **developer diagnostic**, not localized copy (`openspec/changes/archive/2026-08-18-skill-frontmatter-reader/design.md:113–130`). T12 MUST NOT put raw `reason` text into translation catalogs or parse it for locale branching.
- Core remains pure: no locale threading through adapters (`model/` import allow-list). Locale is a **frontend-only** concern.
- `inventory-ui` living spec still says i18n is out of scope — T12 should **MODIFIED** that requirement (or add a sibling capability) so the living specs stop contradicting the product.

**CA mapping note (important — do not invent):**

- `alcance-poc-vertice.md` CA-1…CA-17 do **not** include a dedicated i18n criterion.
- T12’s acceptance is the **phase-local** pair in the roadmap (`plan-desarrollo-poc.md:281–283`), plus design principle 7 and the SkillDock anti-pattern in the brief.
- T12 does **not** close CA-11/12/13 (those are T13). It **does** prepare catalogs so T13 copy can land already externalized.

**Plan scope for T12 (verbatim intent):**

1. Frontend i18n infrastructure with catalogs `en` and `es`
2. All visible strings externalized, including empty states and error messages
3. **Working** language selector (must actually switch the UI)
4. Detect system language with manual override

**Explicitly deferred:**

| Concern | Owner |
|---------|-------|
| Rich client-not-detected / unparseable / absent-root / embedded-action UX | T13 |
| Core / IPC / bindings / capabilities | none expected |
| Localizing `ScanIssue.reason` or adapter diagnostics | rejected by T3 design; stay out |
| DOM component test harness | optional; same fork as T11 (default out unless chosen) |
| Persistence of language preference beyond the session | **unspecified by roadmap** — open question for proposal/design |

### Affected Areas

- `frontend/package.json` (+ lockfile) — add chosen i18n library (or document zero-dep approach)
- `frontend/vite.config.ts` — only if the library needs a Vite plugin (e.g. Paraglide)
- `frontend/src/` — new locale modules/catalogs (e.g. `locales/en.json`, `locales/es.json` or library-specific message trees)
- `frontend/src/App.svelte` — locale state ownership, language selector host, wire `t()` / message API into lifecycle copy
- `frontend/src/lib/InventoryToolbar.svelte` — labels, placeholders, aria-labels, options, reload copy
- `frontend/src/lib/InventoryList.svelte` — empty state
- `frontend/src/lib/InventoryRow.svelte` — duplicate badge + title tooltip
- `frontend/src/lib/LocationList.svelte` — null-path placeholder
- `frontend/src/lib/appTitle.ts` (+ test) — only if title pattern becomes locale-aware (version format)
- `frontend/index.html` — `lang` attribute should track active locale (static `en` today)
- `frontend/src/lib/*.test.ts` — pure tests for locale resolve, catalog completeness, message interpolation
- `openspec/specs/inventory-ui/spec.md` — remove/narrow “i18n out of scope”; add language-switch scenarios **or** new `openspec/specs/frontend-i18n/spec.md`
- **Untouched by design:**
  - `crates/vertice-core/**` — no core i18n
  - `crates/vertice-app/**` — no new commands/capabilities expected (system locale can be read from the webview/`navigator` unless design proves otherwise)
  - `frontend/src/bindings/**` — never hand-edit
  - `frontend/src/lib/scan.ts` — keep IPC-only; map errors to message keys in UI layer

### Approaches

1. **Lightweight custom catalogs + Svelte 5 locale store (recommended for PoC)**
   - Typed message dictionaries `en` / `es` (TS or JSON), `t(key, params?)` helper, `$state` locale in `App` (or a tiny `locale.ts` module), `<select>`/segmented language control, initial locale from `navigator.language` (fallback `en`), manual override updates all bound strings reactively.
   - Pros: zero/low dependency surface; matches current “thin lib + presentational Svelte” pattern; pure Vitest can assert catalog key parity and resolution without DOM; full control over SkillDock anti-pattern (selector must call the same reactive path the UI reads).
   - Cons: no ICU/plural pipeline if needs grow; manual key discipline; must design interpolation (`Found at {n} locations`) carefully.
   - Effort: **Medium**

2. **Typesafe-i18n or similar compile-time typed catalogs**
   - Generate typed keys from base locale; runtime loaders for `en`/`es`.
   - Pros: strong key safety; mature patterns for params/plurals.
   - Cons: codegen/tooling in a small PoC; more moving parts in Vite/CI; overkill for ~15–25 strings.
   - Effort: **Medium–High**

3. **Inlang Paraglide (or SvelteKit-oriented i18n) adapted to Vite SPA**
   - Pros: modern Svelte ecosystem story; compile-time messages.
   - Cons: project is a Tauri-embedded Vite SPA, not SvelteKit — adapter friction; Vite plugin + message compile step; larger PR vs string count.
   - Effort: **High** for this PoC size — **not recommended unless** team already standardizes on it

4. **Defer selector; ship catalogs only / browser language only**
   - Pros: smaller diff.
   - Cons: **violates T12 scope** (explicit functional selector + SkillDock lesson). Rejected.

**Boundary rules (all viable approaches):**

- Externalize **UI chrome only** (labels, empty/error/loading, aria, kind filter options, duplicate badge).
- Do **not** translate inventory payload fields (`name`, `description`, paths) or `ScanError.detail.reason` / `ScanIssue.reason`.
- Language switch MUST be reactive across the whole tree (no stale English islands).
- System detection: map `navigator.language` / `navigator.languages` → `en` | `es` with safe fallback (`en` if unsupported).
- Manual override MUST win over system detection for the session (persistence = open question).
- Keep read-only / no new Tauri fs permissions.
- Strict TDD: RED/GREEN pure locale helpers and catalog parity tests first under existing node Vitest.

### Recommendation

Ship **Approach 1**: a minimal frontend-only i18n layer (typed catalogs `en`/`es` + reactive locale + working selector + system detection), extract every current UI chrome string from the T11 components, and leave core/IPC untouched.

**Change name**: `add-frontend-i18n` (parallel naming to `add-inventory-ui`).

**Spec shape**: prefer a new capability `frontend-i18n` (or `i18n`) plus a small **MODIFIED** on `inventory-ui` to drop “i18n out of scope” and require message-key usage for chrome. Do not delta core domain specs.

**Acceptance mapping for later proposal:**

| Source | T12 obligation |
|--------|----------------|
| Roadmap T12 | Catalogs en/es; no UI literals; full UI updates on language change |
| Design principle 7 | EN+ES from the start — no retrofit later |
| Brief SkillDock critique | Selector must **functionally** switch language for non-default audiences |
| T3 reason policy | `reason` stays verbatim technical detail, outside catalogs |
| CA-1…17 | No dedicated CA; do not claim CA-N closure solely via T12 |

**Testing (strict TDD, pragmatic):**

1. Pure module tests: resolve system language → supported locale; fallback; `t(key)` for both catalogs; key-set parity `en` ≡ `es`; interpolation.
2. Keep Vitest on `node` (same as T11 default).
3. Manual `tauri dev` checklist: start under EN system, start under ES (or override), flip selector, confirm toolbar/list/empty/error/loading/duplicate/null-path all switch with no English leftovers.
4. Optional lint/test guard: fail if new bare string literals appear in `*.svelte` outside allowlisted dynamic data — design may choose a simple test inventory over ESLint complexity for PoC.

**Out of T12 PR surface:** T13 diagnostic panels, core locale APIs, binding regeneration, capability JSON, localizing machine diagnostics, DOM harness (unless explicitly pulled in).

**Open questions for proposal/design (not blockers for explore):**

1. Persist manual locale (e.g. `localStorage` inside app webview) vs session-only? Roadmap silent — recommend session-only for PoC unless product wants sticky preference (still no disk outside app data; `localStorage` is webview-local).
2. Product name `"Vertice"` translated or brand-invariant? Recommend brand-invariant.
3. `component.kind` raw enum (`skill`/`agent`) vs localized label in the row chip? Toolbar already has localized option labels; row currently shows raw `component.kind` — recommend localized display label mapped from kind.
4. Coordinate with T13: land i18n first so T13 adds keys instead of new English literals (preferred sequencing even though phases are parallelizable).

**Workload signal for later `sdd-tasks`:** likely **Low–Medium** 400-line risk (catalogs + thin helper + string extraction across ~5 Svelte files + tests + spec). One PR should fit unless a heavy i18n framework (Approaches 2–3) is chosen.

### Risks

1. **Selector that does not actually switch UI** — SkillDock failure mode. Mitigate: single reactive locale source; every chrome string reads through it; manual test matrix EN↔ES.
2. **Partial extraction** — English islands (aria-labels, `title=`, placeholders, `index.html lang`). Mitigate: explicit string inventory in tasks; checklist includes a11y attributes and `lang`.
3. **Translating the wrong layer** — localizing `reason`/paths/names. Mitigate: spec MUST distinguish chrome vs payload; tests forbid catalog entries for diagnostic passthrough.
4. **T13 race** — if T13 lands English-only rich copy first, double extraction cost. Mitigate: prefer T12 before or immediately with T13; share message-key convention in design.
5. **Framework overkill** — heavy i18n toolchain for ~20 strings. Mitigate: Approach 1 default.
6. **Catalog drift** — `es` missing keys → runtime blanks. Mitigate: parity test (same key set both locales) as CI gate.
7. **System locale edge cases** — `es-MX` / `en-GB` / `pt-BR`. Mitigate: primary subtag map (`es*`→`es`, `en*`→`en`, else `en`).
8. **Strict TDD vs Svelte markup** — no DOM harness means chrome wiring is manual. Mitigate: pure tests for i18n core + manual smoke (same honest pattern as T11).
9. **Claiming a CA-N that does not exist** — there is no CA-18 for i18n. Mitigate: proposal traces to T12 roadmap criteria + principle 7, not a fabricated CA number.

### Ready for Proposal

**Yes.** T11 is archived and the English inventory UI gives a concrete, bounded string surface. T12 is a pure frontend concern with clear non-goals (core, T13 diagnostics, reason localization).

Orchestrator should tell the user:

1. Proposed change name: **`add-frontend-i18n`**.
2. Exploration artifact written at `openspec/changes/add-frontend-i18n/exploration.md`.
3. Artifact store mode used: **openspec** (project preference + `openspec/` layout; only `exploration.md` written).
4. Recommended approach: lightweight typed catalogs + reactive locale + working selector + system detection; no core/IPC changes.
5. Next phase: **`sdd-propose`** for `add-frontend-i18n` (then spec → design → tasks).
6. Confirm before propose (optional, not blockers):
   - Session-only locale vs sticky `localStorage` — default **session-only** or sticky webview storage if product wants remember-me.
   - Approach 1 vs typed library — default **Approach 1**.
   - Whether T12 should land **before** T13 — default **yes** to avoid re-extracting T13 copy.
