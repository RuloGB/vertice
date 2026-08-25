# Proposal: Durable User Settings and Locale Persistence

## Intent

`internal-docs/pendientes-desarrollo.md` §P1: the app always starts in English and forgets the
language the user picked. `App.svelte:25` hardcodes `createI18n("en")`, so the tested pure
`resolveLocale()` is never used, and the Sidebar selector's choice dies with the session — the
`frontend-i18n` spec already declares the precedence as a MUST, so this is a compliance gap.

Exploring it exposed a deeper defect: `enabled` (freshness opt-out) and `disclosure_seen` are
durable user configuration stored inside `freshness-cache.json`, a disposable TTL'd cache whose
`load` silently treats corrupt/torn files as empty and whose `save` deliberately skips atomic
writes. A torn write silently erases user configuration — the same class of bug as P1.

## Scope

### In Scope

- New durable document `settings.json` in the app data directory holding `locale`, `enabled`,
  `disclosure_seen`, with an IPC read/write command pair modelled on `freshness_settings` /
  `set_freshness_settings`.
- Migrate `enabled` and `disclosure_seen` out of `FreshnessStore`; reduce `freshness-cache.json` to
  the TTL'd `HashMap<String, CacheEntry>` only, keeping its cheap whole-file write.
- Rename `FreshnessStore` (and its ts_rs settings type) to match its narrowed contents.
- Wire `resolveLocale(navigator.languages)` at `App.svelte:25`; precedence: explicit persisted
  choice > system languages > `en`. Persist the Sidebar selection across restarts.

### Out of Scope

- Any migration/compat path for existing `freshness-cache.json` files — beta, no compatibility to
  preserve; migrated settings fall back to documented defaults once.
- `localStorage` and `tauri-plugin-store` (rejected: invisible in support bundles / new dependency
  plus capability widening).
- New locales, per-project scope settings, other UI preferences.

## Capabilities

### New Capabilities
- `user-settings`: the durable settings document, its defaults, its failure semantics, and the IPC
  command pair that reads and writes it.

### Modified Capabilities
- `frontend-i18n`: "Supported Locale Resolution" — replace the ambiguous "manual **session**
  override" with an explicitly durable persisted choice; add restart-persistence scenarios.
- `component-freshness`: `enabled` and `disclosure_seen` requirements now point at the durable
  settings document; the cache document narrows to TTL'd entries.

## Approach

Copy the proven `commands.rs:172-258` pattern (`spawn_blocking` + `resolve_app_data_dir` +
best-effort write) into a dedicated settings module in `vertice-app`. `vertice-core/src/model/`
gains the plain settings type only (stays I/O-free, Tauri-free); `vertice-app` owns all file I/O.
Strict TDD: `cargo test` round-trips (including "app data dir never created") and Vitest precedence
tests are written first.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/vertice-app/src/settings/` | New | Settings store + commands, registered in `lib.rs:56-63` |
| `crates/vertice-app/src/freshness/cache.rs` | Modified | Narrowed to TTL'd entries; store renamed |
| `crates/vertice-core/src/model/freshness.rs` | Modified | `FreshnessSettings` type split/renamed |
| `frontend/src/App.svelte:25` | Modified | Initial locale resolution |
| `frontend/src/lib/i18n/locale.svelte.ts` | Modified | Externally supplied / persisted initial locale |
| `frontend/src/lib/settings.ts` | New | `invoke` wrapper mirroring `lib/freshness.ts` |
| `frontend/src/bindings/*.ts` | Regenerated | Via `cargo test -p vertice-core`; never hand-edited |

## Design-phase decisions (deliberately left open)

1. **First paint**: block briefly on the async settings read (no wrong-language flash) vs. resolve
   synchronously from `navigator.languages` and reconcile (possible one-frame flash for returning
   users).
2. **Write durability**: whether `settings.json` warrants temp-file-plus-rename, and whether a torn
   write is treated as recoverable. The *fallback values* are no longer open — see the settled
   decision below.

## Settled: asymmetric fallback on an unreadable settings document (confirmed 2026-08-25)

A uniform silent fallback to documented defaults — the behavior `freshness-cache.json` uses — is
**not** acceptable for the durable document, because the three fields do not fail equally:

- `locale` falling back to system detection is inert.
- `disclosure_seen` falling back to `false` merely re-shows the privacy disclosure: annoying, safe.
- `enabled` falling back to its documented default of `true` (`cache.rs:45`, `default_enabled`)
  would **resume outbound network requests the user had explicitly turned off**, with no notice.

Decision: an unreadable, missing, or corrupt `settings.json` MUST fall back conservatively —
`enabled` resolves to `false`, not to its normal default. `locale` and `disclosure_seen` keep their
ordinary silent defaults. Rationale: the failure mode must never re-enable outbound traffic on the
user's behalf. Accepted cost: a user who had the check enabled must re-enable it once after a
corruption event.

Note that `enabled`'s default therefore becomes context-dependent: `true` for a document that has
never existed (first run — the existing "The Check Is Enabled By Default" requirement stands), and
`false` for a document that exists but cannot be read. The spec phase must state both cases
explicitly rather than describing a single default.

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Users silently lose `enabled` / `disclosure_seen` on upgrade | High (accepted) | Beta; defaults are the already-specified behavior — disclosure simply reappears once |
| Rename churn across three layers | Med | Bindings regeneration is CI-gated; compiler catches the rest |
| Locale flash on startup | Med | Resolved by design decision 1 |
| Capability creep | Low | CA-16 holds: writes confined to app data dir; `capabilities/default.json` stays `core:default` only |

## Rollback Plan

Revert the change branch. `settings.json` becomes an inert orphan file (never read again);
`freshness-cache.json` is regenerated from defaults on next run since neither document has a
required schema. No data loss beyond the settings already accepted as resettable.

## Dependencies

None. No new crates, no new Tauri plugins, no capability additions.

## Success Criteria

- [ ] A returning user sees the language they chose, after a full app restart.
- [ ] A first-run user with `es-*` system languages sees Spanish without touching the selector.
- [ ] `pt-BR` still falls back to `en`.
- [ ] `enabled` and `disclosure_seen` survive a restart and are absent from `freshness-cache.json`.
- [ ] `capabilities/default.json` still contains only `core:default`; read-only audit finds no write
      outside the app data directory.
- [ ] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `cargo deny check
      bans licenses`, and `npm run lint && check && test && build` all pass with bindings in sync.
