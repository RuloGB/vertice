# Exploration: P1 — Language detection and persistence (frontend-i18n)

Source: `internal-docs/pendientes-desarrollo.md` §P1. Explored 2026-08-25.

## 1. Executive summary

Both parts of the pending item are real and precisely located as documented; a working, tested
language selector already exists in `Sidebar.svelte` and only needs two additions — wiring
`resolveLocale(navigator.languages)` as the initial value and persisting the explicit choice.
Recommendation: persist via a small **new Rust-owned settings command** (a variant of Option B,
not literally inside `FreshnessStore`) rather than `localStorage`, because the codebase already
has a proven, tested, capability-free pattern for exactly this shape of state
(`freshness_settings` / `set_freshness_settings`) and the project's stated preference is one
inspectable persisted document — but keep it a **separate settings store / `ui-settings.json`
file**, not a field bolted onto `FreshnessStore`, to avoid the semantic-mismatch and rename cost
the pendientes doc itself flags.

## 2. Current state (verified)

All claims in `internal-docs/pendientes-desarrollo.md` §P1 check out against the code, with one
addition the doc did not mention:

- `resolveLocale()` is at `frontend/src/lib/i18n/locale.svelte.ts:17`, pure and tested
  (`frontend/src/lib/i18n/locale.test.ts:24-37`). Confirmed unused in production code —
  `frontend/src/App.svelte:25` is exactly `const i18n = provideI18n(createI18n("en"));`.
- `createI18n()` is at `locale.svelte.ts:40`, keeps `locale` in a `$state` closure with no side
  effects — confirmed, no `localStorage` / IPC anywhere in `frontend/src/lib/i18n/`.
- **Not mentioned in the pendientes doc but directly relevant**: a working language selector
  already exists and is wired to `setLocale` — `frontend/src/lib/Sidebar.svelte:54-64` (a
  `<select>` bound to `i18n.locale`, calling `i18n.setLocale(...)` on change, using catalog keys
  `app.languageLabel` / `languageEnglish` / `languageSpanish` from
  `frontend/src/lib/i18n/catalogs.ts:4-9,163-167,338-341`). The reactive-switch part of the
  feature is done and tested (`locale.test.ts:176-187`). Only initial resolution and cross-restart
  persistence are missing.
- `openspec/specs/frontend-i18n/spec.md:9-21` ("Supported Locale Resolution") **already states**
  the required precedence — manual override, then `navigator.languages` / `navigator.language`,
  then `en` fallback — as a MUST, with scenarios for `es-MX` to `es` and `pt-BR` to `en`. The spec
  is ahead of the implementation: as written today those scenarios are not true end-to-end
  (App.svelte hardcodes `en`), only true of the pure `resolveLocale()` function in isolation.
  Part of this work is closing a compliance gap the spec already declares, not writing net-new
  requirements.
- That requirement's wording, "a manual **session** override", is ambiguous — it reads today as
  in-memory / session-scoped, which is exactly what is implemented. The new work needs the spec to
  say explicitly that an explicit choice is durable across restarts: a wording change, not just code.

## 3. Affected areas

- **Frontend (`frontend/src/`)**
  - `lib/i18n/locale.svelte.ts` — `createI18n` needs an async/persisted-load path or an
    externally-supplied initial locale; `resolveLocale` reused as-is.
  - `App.svelte:25` — call site change to resolve the initial locale.
  - `lib/i18n/locale.test.ts` — new tests for persistence and precedence.
  - If Option B/B': a new `lib/settings.ts` wrapping `invoke(...)`, mirroring `lib/freshness.ts`.
- **Rust app (`crates/vertice-app/src/`)** — only if Option B/B'/C: new commands (analogous to
  `commands.rs:172-197`), registration in the `lib.rs:56-63` `generate_handler!`, and a persistence
  module.
- **Rust core (`crates/vertice-core/src/model/`)** — only if a typed IPC contract is added
  (mirroring `FreshnessSettings` at `crates/vertice-core/src/model/freshness.rs:68-76`). Note
  `model/` must stay I/O-free per its module doc, so file logic stays in `vertice-app`.
- **Specs**: `openspec/specs/frontend-i18n/spec.md`. If literal Option B, also
  `openspec/specs/component-freshness/spec.md`.
- **Bindings**: only if a new Rust type is introduced; regenerated via `cargo test -p vertice-core`.
  Option A touches zero bindings.
- **Tests**: strict TDD on both layers — Vitest for resolution and persistence logic, `cargo test`
  for any new command/store round-trip (see `commands.rs:520-631`, including "survives a
  never-created app data directory").

## 4. Requirements implied by the topic

1. Initial locale precedence, MUST: explicit persisted user choice > `navigator.languages` /
   `navigator.language` (via `resolveLocale`) > `en` fallback. Already declared in the spec; the
   gap is wiring.
2. An explicit choice via the Sidebar selector MUST persist across app restarts, not just the
   current session.
3. Persistence MUST NOT regress first paint — the UI renders synchronously today, so the design
   must decide between resolving synchronously from `navigator.languages` and reconciling once the
   persisted value loads, or briefly blocking.
4. The chosen path must respect CA-16 (read-only outside the app data directory) and must not widen
   the Tauri capabilities surface unnecessarily.

## 5. Approach comparison

| | **A — `localStorage`** | **B (literal) — field in `FreshnessStore`** | **B' (recommended) — new sibling settings store + command pair** | **C — Tauri Store plugin** |
|---|---|---|---|---|
| Files touched | ~2 + tests | ~6 + tests, plus a rename of `FreshnessStore` | ~7 + tests | new dependency `tauri-plugin-store` |
| Spec deltas | `frontend-i18n` only | `frontend-i18n` **and** `component-freshness` — spec pollution across an unrelated capability | `frontend-i18n` + a small new or extended capability spec | `frontend-i18n` + capabilities/permissions |
| Bindings impact | none | regenerates `FreshnessSettings.ts`, whose name no longer matches its contents | new small, correctly named type | none unless a typed wrapper is added |
| Inspectable from log / settings file | no | yes, but conflated with freshness data | yes, in its own clearly named file | yes, plugin-managed |
| CA-16 / capabilities risk | none | none functionally, but couples an unrelated capability's write path to a UI concern | none — reuses `spawn_blocking` + `resolve_app_data_dir` + best-effort write from `commands.rs:214-258` | new permission needed; `capabilities/default.json:6` deliberately keeps `core:default` only |
| Effort | very low | medium, plus rename churn | medium, copy-pastable from a proven pattern | medium-high, plus `cargo deny` allow-list |
| Risk | setting invisible in logs and support bundles | naming and semantic drift; the `cache.rs` module doc explicitly says "the persisted freshness document" | low — isolated new concern | new supply-chain dependency for one field |

No generic app-settings store exists today (searched `AppSettings`, `SettingsStore`,
`UserSettings` — no hits), so B' is genuinely new, not a rename of something half-existing.

## 6. Firm recommendation

**Option B' — a new, small, dedicated settings store, not `localStorage` and not a
`FreshnessStore` field.**

The pendientes doc frames this as a binary A-vs-literal-B choice, but literal B has a cost it
already names: the rename of `FreshnessStore` plus pollution of `component-freshness`, whose
Purpose section (`openspec/specs/component-freshness/spec.md:5`) scopes it to whether a detected
component is out of date. Language is not a freshness concern.

At the same time `localStorage` throws away a cheap benefit: the project already has a fully
proven, tested, minimal-capability pattern for exactly this shape of state
(`freshness_settings` / `set_freshness_settings` in `commands.rs:172-258`, `FreshnessStore::save` /
`load` in `cache.rs:69-87`) that costs nothing extra in Tauri capabilities and nothing extra in
dependencies. Copying that pattern into a correctly named module
(`crates/vertice-app/src/settings/store.rs`, file `ui-settings.json`) produces something
inspectable and consistent without conflating two unrelated capabilities on disk, and gives future
UI-only settings a home that is not named after freshness.

Tauri's Store plugin is rejected: a new dependency and a capabilities widening for a single string
value the project can already persist with tools it owns end to end.

## 7. Decisions taken and remaining questions

### Resolved by the user (2026-08-25)

The split is by **durability semantics**, not by domain. The user's goal — all user configuration
in one inspectable place — is accepted, and reusing `freshness-cache.json` for it is rejected on a
technical ground, not on naming:

- `freshness-cache.json` is a disposable TTL'd cache. `load` treats a missing, corrupt, or
  unreadable file as an empty document in silence (`cache.rs:68`), and `save` writes the whole file
  with no temp-file-plus-rename precisely because "a torn write is indistinguishable from a corrupt
  cache" (`cache.rs:77`). Correct for a cache; fatal for durable configuration, because a torn
  write would silently erase the user's language choice — reintroducing the very P1 symptom this
  change fixes.

Resulting split:

- **`settings.json`** — durable user configuration: `locale`, `enabled` (freshness check),
  `disclosure_seen`. Must survive; needs durable write semantics.
- **`freshness-cache.json`** — reduced to the TTL'd `HashMap<String, CacheEntry>` only. Stays
  disposable and keeps its cheap whole-file write.

Consequences:

- `enabled` and `disclosure_seen` **migrate out** of `FreshnessStore` in this same cycle. The
  earlier "stay locale-only" scope option is closed: the two settings are currently misplaced in a
  disposable document and moving them is part of the fix, not scope creep.
- Renaming `FreshnessStore` is no longer a cost to avoid; it becomes part of the design. The
  project is in beta with no compatibility to preserve, so no migration path for existing
  `freshness-cache.json` files is required — the settings keys simply fall back to their defaults
  once, which is the already-specified behavior.
- The naming question is settled: `settings.json`, not `ui-settings.json`, since the document holds
  general user configuration rather than UI-only state.

### Still open — for the design phase

1. **First-paint behavior**: block rendering briefly on the async settings read (no flash of the
   wrong language), or resolve synchronously from `navigator.languages` and reconcile when the
   persisted value arrives (possible one-frame flash for a returning user)? Product/UX call.
2. **Write durability for `settings.json`**: whether the durable document warrants
   temp-file-plus-rename, given that the cache deliberately does not. Related to whether a torn
   write of `settings.json` should fall back to defaults or be treated as recoverable.

## 8. Suggested change id and spec capabilities touched

- Change id: `add-locale-persistence` (matches the existing `add-*` convention in
  `openspec/changes/archive/`).
- Spec capabilities:
  - `frontend-i18n` (existing) — amend "Supported Locale Resolution" to state durability across
    restarts, and add persistence scenarios.
  - `component-freshness` (existing) — its `enabled` and `disclosure_seen` requirements now point
    at the durable settings document instead of the cache document; the cache document's spec is
    narrowed to the TTL'd entries only.
  - A new `user-settings` capability owning the durable settings document and its IPC command pair.
    Final placement (new capability vs. an extension of `desktop-shell`) to be fixed in the proposal.

## 9. Next recommended SDD phase

`sdd-propose`, targeting change id `add-locale-persistence`, on the basis of the resolved split in
section 7.
