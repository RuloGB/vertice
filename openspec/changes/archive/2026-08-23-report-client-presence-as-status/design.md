# Design: Report Client Presence As A Typed Status, Not A Warning

> Trace: **T13** (CA-11 slice only). Must-not-regress **CA-1**, **CA-7**. Bounded by **CA-16**, **CA-17**.
> Reverses `archive/2026-08-19-client-installation-detection/design.md` §2 (**T7D**), on the retrofit condition T7D §2/§13 itself recorded. Inherits T7D §5.2 (`cfg!` platform seam), §6 (verbatim version directories), §9 (never merge), and `archive/2026-08-23-fix-windows-claude-desktop-probe/design.md` (**P30D**) decision 1 (slot vocabulary) unchanged.
> `rules.design` coverage: core data model (§2), core/Tauri isolation (§1), IPC contract surface (§6), per-OS paths (§8), `ScanIssue` taxonomy and error paths (§7).
> **Environment note.** No shell was used and no real `%APPDATA%` or user home was probed: this session may see an MSIX-redirected filesystem view that ordinary processes do not, so such a probe is unsound evidence. CA-17 requires fixture-based verification regardless. §0 separates verified from inherited.

## 0. What is verified, and what is inherited on trust

| # | Statement | Basis |
|---|---|---|
| V1 | `ScanReport.installations` is read by **exactly one** production surface, `ScanPage.svelte:93-106`. Other hits are `bindings/ScanReport.ts`, `App.test.ts:55,66`, `scan.test.ts:15`, and the two `scan.installations*` catalog keys | Grepped `installations` across `frontend/src` |
| V2 | `resolve_npm_slot:343` (`!exists(path)`) and `resolve_bundled_slot:503` (`!any_candidate_root_exists`) already compute the per-slot presence verdict as a closed boolean, immediately before pushing the not-detected `Warning`. This is the "cheap retrofit" T7D §2 promised | Read `installations.rs:335-350,437-513` |
| V3 | `InstallSlot::label()` is used in the not-detected reason **and** in four `Error` reasons (`installations.rs:452,469,488,538`). Removing the not-detected issue does **not** remove labels from `ScanIssue` | Read `installations.rs:112-136,425-544` |
| V4 | `model/mod.rs:1-22` allow-list is `std::path`, `std::time::Duration`, `serde`, `ts_rs`, `thiserror`, `unicode_normalization`; `report.rs` already imports only `std::path::PathBuf`, `serde`, `ts_rs` | Read `model/mod.rs`, `model/report.rs` |
| V5 | `scanDiagnostics.ts` carries a **second** string coupling besides `MISSING_CLIENT_REASONS`: `isUnavailableRootWarning` rebuilds `` `search root ${root.id} was not found` `` to match `scan.rs:63` | Read `scanDiagnostics.ts:25-27`, `scan.rs:56-67` |
| V6 | `ScanPage.svelte:10-22` destructures exactly `status, report, failureMessage, diagnostics, incidents` — no `onReload`; `ComponentToolbar.svelte:28-35` is the reload idiom; `App.svelte:126` is the single call site | Read all three |
| V7 | The 14 fixture homes named in the exploration exist and are asserted by name in `tests/client_installations.rs` | Globbed the tree; grepped test fn names |
| **U1** | The three Windows probe paths and the coexisting-versions observation | Inherited from T7D §0 (U1/U2), verified there on 2026-08-19. **Not re-verified here** — see the environment note |
| **U2** | `openspec/specs/*` line references in the proposal (`domain-model:149-163`, `frontend-i18n:35`) | Inherited from the proposal and exploration; not re-read |

**Correction to the proposal.** It states the eight-type binding enumeration "becomes nine". This design needs **two** exported types (`ClientPresence` and its status enum, §2/§4), so the enumeration becomes **ten**. The `domain-model` delta must say ten.

## 1. Technical approach

`resolve_slot` stops pushing a not-detected `ScanIssue` and instead **returns** a `ClientPresence` record for every slot it was given, present or absent. `installations.rs` assembles `Vec<ClientPresence>`; `ScanReport.installations` stops being independently accumulated and becomes a **flattening of those records**, computed in one expression. The frontend reads the typed field and deletes all client-reason string matching.

```
                                   vertice-core                     (no tauri, no new crate)
 frontend ──IPC──> vertice-app ──> ├── model/  + ClientPresence, ClientPresenceStatus   (plain data, §2)
 future vertice-cli ───────────>   ├── installations                                    (§3)
                                   └── scan  ── assembles ScanReport                    (§5)

 installations::scan_for(home, platform)
   Windows      -> per slot: candidates -> resolve_slot -> ClientPresence { status, installations, .. }
                   Some(vec![3 records])
   Unsupported  -> None + the existing single Warning, byte-identical to today   (§4)
        │
        └─ InstallationScan { presence: Option<Vec<ClientPresence>>, issues }
                 installations = flatten_presence(&presence)      <-- the only producer (§3)
```

The CLI pathway is untouched: `scan_for(home, platform)` still takes an explicit home, reads no environment, and `vertice-app` gains nothing.

## 2. Core data model changes

`Component`, `Location`, `Scope`, `SearchRoot`, `ClientInstallation`, `ClientKind`, `ScanIssue`, `IssueSeverity` are **unchanged**. `ScanReport` gains one field; `model/` gains one file.

```rust
// crates/vertice-core/src/model/presence.rs — allow-list respected (V4):
// std::path::PathBuf, serde, ts_rs only. No fs, no io, no env, no clock.

/// One probe slot's verdict. Exactly one record per slot the platform's
/// table defines (three on Windows), emitted whether or not anything
/// resolved. A slot is a *place we look*, not an installation: one record
/// MAY carry many installations (CA-7).
pub struct ClientPresence {
    /// The slot's settled proper-noun label, e.g. "OpenCode (npm)".
    /// Core-owned, unique within a report, never localized (§6).
    pub label: String,
    /// Every path probed for this slot, in deterministic order; the legacy
    /// bundled path is always last. Non-empty by construction. Carried, not
    /// displayed (§6).
    pub probed_paths: Vec<PathBuf>,
    pub status: ClientPresenceStatus,
    /// Never `Option`, never reduced to "highest wins" (CA-7, T7D §9).
    pub installations: Vec<ClientInstallation>,
}

pub enum ClientPresenceStatus { Detected, NotDetected }
```

`ScanReport` gains `pub client_presence: Option<Vec<ClientPresence>>` (`clientPresence` in TS).

**`Detected` means "the slot exists on disk", not "we have a version."** That is precisely V2's boolean. `Detected` with an empty `installations` and an `Error` issue is the *present-but-broken* case and is deliberately representable — §7.

## 3. Decision: the central one — two channels for one fact

| Option | Consequence | Decision |
|---|---|---|
| **A — records embed `ClientInstallation`; `ScanReport.installations` becomes a derived flattening** | One producer. `installations` keeps its meaning, its `domain-model` requirement and its only consumer's contract. Divergence requires editing one expression | **Chosen** |
| B — records hold indices into `ScanReport.installations` | A `usize` crosses `ts_rs` as `number`; a stale index renders a wrong version **silently**. Referential integrity is unenforceable in the binding and unrepresentable-by-construction is this codebase's whole method (T7D §5.1, §9) | **Rejected** |
| C — remove `ScanReport.installations`, rewrite `ScanPage:93-106` | Genuinely single-channel, and V1 proves only one consumer exists. But it is a **field removal** from a type the `domain-model` spec enumerates, the proposal scopes it out, and every future "what is installed" consumer must re-implement the flatten | **Rejected** |

**The invariant, designed rather than asserted.** `InstallationScan.installations` is not accumulated by `resolve_slot` at all. It is produced by one private function:

```rust
fn flatten_presence(presence: &Option<Vec<ClientPresence>>) -> Vec<ClientInstallation>
// None => vec![]; Some => concat of each record's installations, in record order
```

Ordering is preserved exactly as today (slot order = probe-table order, candidates sorted byte-wise, legacy last — T7D §7), so `ScanReport.installations` is byte-identical to the current output for every fixture. The equality test the proposal requires is therefore a **tripwire against a future second producer**, not the guarantee itself; the guarantee is that there is nowhere else to push.

## 4. Decision: `HostPlatform::Unsupported`

| Option | Consequence | Decision |
|---|---|---|
| Three `NotDetected` records | Tells a macOS user their clients are absent when Vertice never looked. The exact lie CA-11 exists to forbid (T7D §5.2) | **Rejected** |
| `Vec<ClientPresence>` with `vec![]` | `[]` is ambiguous between "not probed" and "probed, zero slots", and the UI would have to *infer* semantics from emptiness — the same class of mistake as string-matching | **Rejected** |
| Data-carrying enum `Probed{..} \| Unsupported` | Explicit, but serde's externally-tagged encoding is a first for `model/`, and at T16 the union collapses to one variant — a binding break exactly where §5.2 promised "purely additive" | **Rejected** |
| **`Option<Vec<ClientPresence>>`, `None` on `Unsupported`** | `null` ≠ `[]` is explicit and needs no new type; matches the existing `Option<PathBuf>` idiom in `ScanIssue`/`Location`. At T16 `None` simply stops occurring — **the shape never breaks**, the UI branch becomes dead and is deleted with its test | **Chosen** |

The `Unsupported` arm is otherwise **byte-identical to today**: zero installations, exactly one `Warning` with `path: None` and the same reason. `tests/client_installations.rs:442` stays green with one added assertion (`client_presence.is_none()`); it is the tripwire.

## 5. Decision: reversing T7D §2, in its own table format

| Option | Consequence | Decision (2026-08-23) |
|---|---|---|
| A — absence stays on `ScanIssue` (T7D's choice) | Zero model edit. The UI cannot answer "is OpenCode installed?" without parsing English, and every platform added at T16 adds three more parsed strings | **Superseded** |
| B — typed status published through `model/` (T7D's rejected option) | Two exported types, a `ScanReport` field, one binding regeneration. Absence gains a carrier; the string coupling is deleted, not extended | **Chosen** |

**T7D §2 was correct for its evidence and is not called a mistake.** It rejected B to avoid freezing a `ScanReport` shape before T9's aggregator and T10/T11's consumer existed, and in the same paragraph recorded the retrofit condition and priced it: *"If T10/T11 concludes it needs a structured answer, B is the retrofit, and it is cheap, because `resolve` already computes the per-slot outcome as a closed value"* (§2, restated §13). T11 is complete, the aggregator exists, the UX gap is observable, and V2 confirms the closed value is still sitting there. The condition is met; the price is what was quoted. `IssueSeverity::Info` remains rejected on T7D §4's V2 reasoning — `IssueSeverity` and `domain-model`'s "exactly two variants" MUST stay byte-identical, and that is a review check.

**Precedent bound, so this does not become a licence.** A `model/` edit is justified when the consumer exists *and* the current carrier forces a consumer to parse prose. Neither half alone suffices.

## 6. IPC contract surface and the frontend seam

**No new command, no event, no capability.** `crates/vertice-app/` and `capabilities/default.json` are untouched; `scan`/`rescan` remain thin `spawn_blocking` pass-throughs (`commands.rs:15-43`). The contract change is entirely in the **payload shape** of the existing commands: `ScanReport` gains `clientPresence`.

**Bindings.** Regenerated only by `cargo test -p vertice-core`, **never hand-edited**, landing in the same commit as the Rust types. CI runs `git add --intent-to-add` first, so the new files are caught.

| Binding file | Action |
|---|---|
| `frontend/src/bindings/ClientPresence.ts` | **New** |
| `frontend/src/bindings/ClientPresenceStatus.ts` | **New** |
| `frontend/src/bindings/ScanReport.ts` | Modified — one field |
| every other `bindings/*.ts` | **Unchanged** — a diff there means something leaked into `model/` |

**`Diagnostics` and the incident definition.**

```ts
export type Diagnostics = {
  unavailableRoots: SearchRoot[];   // kept for display; neutral, never an incident
  recoverableIssues: ScanIssue[];   // renamed: "remaining" named two partitions that no longer exist
};
export function incidentCount(d: Diagnostics): number { return d.recoverableIssues.length; }
```

`MISSING_CLIENT_REASONS`, `isMissingClientIssue` and the missing-client branch are **deleted**. An incident is now exactly "a `ScanIssue` that is not the echo of a `notFound` root" — so a broken install still lights the badge and an uninstalled client never does. `isUnavailableRootWarning` **survives** and with it V5's second string coupling; it is out of scope here (it belongs to `scan.rs`'s root vocabulary, not the client vocabulary) but it must be pinned by a test and recorded as a known coupling, because if it ever fails to match, `notFound` roots silently become incidents again.

**`ScanPage`.** The "Detected installations" panel is **replaced** by the always-visible "Supported clients" table (client / status / version(s)) — keeping both would render the same fact twice on one route. Each version cell carries the installation path as a `title` tooltip, the idiom already used for root paths at `ScanPage.svelte:75`, so CA-7's per-installation path survives without a path column. `notFound` roots lose `text-danger` for neutral muted styling. A rescan button joins the header, mirroring `ComponentToolbar.svelte:28-35`, reusing `toolbar.reload`/`toolbar.reloading`, disabled while `status === "loading"`; `App.svelte:126` threads `onReload={() => void runScan("reload")}`.

**i18n.** Slot labels are product proper nouns and are rendered verbatim in both locales — "Claude Code CLI (npm)" is the tool's own name. Only chrome is catalog-driven. New keys (complete in `en` and `es`): `scan.clientsTitle`, `scan.clientDetected`, `scan.clientNotDetected`, `scan.clientVersionUnavailable`, `scan.clientsUnsupportedPlatform`. Removed keys (their only consumers are deleted): `diagnostics.missingClient`, `scan.installationsTitle`, `scan.installationsEmpty` — with their assertions in `locale.test.ts:95-96`.

## 7. Error paths (`ScanIssue` taxonomy)

**No new severity, no new field, no reason-string change.** Exactly one row is deleted from T7D §8 / P30D's table:

| Condition | Before | After |
|---|---|---|
| No candidate path exists for a slot | `Warning` `"{label} not detected"`, `path: Some(..)` | **no issue**; `status: NotDetected`, paths in `probed_paths` |
| every other row (npm `package.json` unreadable / unparseable / not an object / no `"version"` string; bundled candidate with zero version dirs; non-UTF-8 version name; unreadable `Packages`; `DirEntry` error) | `Error` | **unchanged, byte-identical reasons** |
| platform `Unsupported` | one `Warning`, `path: None` | **unchanged** |

**`InstallSlot::label()` MUST NOT be deleted** (V3): it still builds four `Error` reasons and now also fills `ClientPresence.label`. Labels therefore still travel inside `ScanIssue.reason` — harmless, because no frontend code parses `Error` reasons, and `frontend-i18n`'s passthrough rule still covers that channel.

**Detected + broken, and what the UI shows.** Status answers *is it there*; installations answer *what did we learn*. A row may be `Detected` with zero installations: the status cell reads "Detected", the version cell reads `scan.clientVersionUnavailable`, and the `Error` sits in `ScanIssueList` and counts as an incident. Broken can never read as absent, and absent can never read as broken.

| Fixture home | Proves |
|---|---|
| `nothing` | 3 records, all `NotDetected`, 0 installations, **0 issues** — the CA-11 pin |
| `packaged-and-legacy` | bundled record holds multiple never-merged installs in **one** row — the CA-7 pin |
| `two-packages` | a third `Claude_*` package contributing nothing does not distort the record |
| `npm-dir-no-package-json` | `Detected` + 0 installations + 1 `Error` — broken ≠ absent |
| `packaged-empty` | same shape for the bundled slot |
| `no-version-key`, `version-not-a-string`, `package-json-empty`, `package-json-unreadable` | four distinct failure paths, all `Detected` + 0 installations + 1 `Error` |
| `packages-unreadable` | enumeration `Error` **and** the legacy candidate still resolving in the same record |
| `non-claude-packages` | bundled `NotDetected` with 0 issues although `Packages` exists |
| `opencode-npm`, `legacy`, `packaged` | mixed detected/not-detected rows in one report |
| `isolation` | one broken slot never changes another slot's status |

**No fixture is added, changed or deleted.** CA-17 holds: every assertion runs `scan_for(home, HostPlatform::Windows)` on all three CI legs.

## 8. Platform-specific paths

Unchanged from T7D §11 / P30D. Windows only: `<home>\AppData\Roaming\npm\node_modules\@anthropic-ai\claude-code`, the `Claude_*` MSIX enumeration under `<home>\AppData\Local\Packages` plus the legacy `<home>\AppData\Roaming\Claude\claude-code`, and `<home>\AppData\Roaming\npm\node_modules\opencode-ai`. macOS (`~/Library/Application Support`) and Linux (XDG) tables remain **T16**; until then those platforms yield `client_presence: None` (§4). No OS convention resolver, no `dirs`, no env read — `plan-desarrollo-poc.md:179`.

## 9. File changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/model/presence.rs` | **Create** | §2 — two exported plain-data types |
| `crates/vertice-core/src/model/mod.rs` | Modify | `mod presence;` + one `pub use` |
| `crates/vertice-core/src/model/report.rs` | Modify | one `ScanReport` field |
| `crates/vertice-core/src/installations.rs` | Modify | `resolve_slot` returns a record; delete the not-detected push; add `flatten_presence`; module doc supersedes its "`model/` is unmodified" claim |
| `crates/vertice-core/src/scan.rs` | Modify | carry `client_presence`; rewrite the test at `129-154` |
| `crates/vertice-core/tests/client_installations.rs` | Rewrite | assert records instead of reason strings |
| `crates/vertice-core/tests/model_contract.rs` | Modify | `client_presence: None` in three `ScanReport` literals; one exhaustive-match test for `ClientPresenceStatus`, mirroring the `Scope` pattern |
| `crates/vertice-core/tests/fixtures/**` | **Unchanged** | |
| `frontend/src/bindings/` | Regenerated | §6 — 2 new, 1 modified, never hand-edited |
| `frontend/src/lib/scanDiagnostics.ts` + `.test.ts` | Modify | §6 |
| `frontend/src/lib/ScanIssueList.svelte` (+ test) | Modify | drop the missing-client section |
| `frontend/src/lib/pages/ScanPage.svelte` | Modify | table, neutral roots, rescan |
| `frontend/src/App.svelte` (+ `App.test.ts`) | Modify | thread `onReload`; fixtures gain `clientPresence` |
| `frontend/src/lib/i18n/catalogs.ts` (+ `locale.test.ts`) | Modify | §6 |
| `crates/vertice-app/**`, `capabilities/default.json` | **Unchanged** | §6 |
| `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Unchanged** | no new dependency; no `tauri` in core |

**CA-16 structurally**: the disk surface stays `symlink_metadata`, `read_to_string`, `read_dir`. This change removes code and adds none; no `File::create`, `OpenOptions`, `fs::write`, `create_dir*` or `remove_*` anywhere, in source or tests.

## 10. Testing strategy (`strict_tdd: true` — RED first)

The two load-bearing failing tests, written **before** any implementation, in this order:

1. `nothing_yields_three_not_detected_records_and_zero_issues` (`tests/client_installations.rs`) — the CA-11 pin. It must fail to *assert*, not to compile, so the field and types land first as an empty/never-populated shape.
2. `bundled_slot_record_carries_every_coexisting_installation` over `packaged-and-legacy` — the CA-7 pin. A record collapsing to one installation, or a status derived from `installations.is_empty()`, fails here.

| Layer | What | How |
|---|---|---|
| Unit (`installations.rs`) | `flatten_presence` concatenates in record order; `None` → empty | in-module, no I/O |
| Integration | the §7 fixture table, one test per row | `scan_for(home, Windows)`, all three CI legs |
| Integration — invariant | flattened record installations `==` `ScanReport.installations`, element-for-element, on `packaged-and-legacy` and `isolation` | §3's tripwire |
| Integration — platform | `scan_for(home, Unsupported)` → `client_presence == None`, 0 installations, exactly 1 `Warning` with `path: None` | §4 tripwire, existing test extended |
| Integration — determinism / read-only | two runs byte-identical; fixture tree unchanged (CA-16) | existing tests carried over |
| Contract | no `ClientInstallation` with an empty version; `IssueSeverity` still has two variants | existing + review check |
| Frontend (Vitest) | `incidentCount` is 0 for a report whose only diagnostics are a `NotDetected` record and a `notFound` root; **non-zero** for a broken-`package.json` `Error`; no source file references a client reason string | `scanDiagnostics.test.ts` rewrite |
| Frontend | the table renders three rows including `NotDetected` ones; two versions in one row; `clientPresence: null` renders the unsupported-platform copy | `App.test.ts` |
| Frontend | rescan button invokes `rescan` and disables while loading | `App.test.ts`, mirroring the Agents/Skills assertions |
| i18n | `en` and `es` complete for the new keys; removed keys absent | `locale.test.ts` |

Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, `npm run lint && npm run check && npm run test && npm run build`, bindings in sync.

## 11. Migration / rollout

**No migration.** Nothing is persisted; `ScanReport` is rebuilt on every scan, so an old and a new report never coexist. Single PR with a pre-accepted `size:exception` — the core type, the binding regeneration and the consumer cannot be split without leaving `main` with a typed field nothing reads. Rollback is the proposal's three ordered layers; a partial rollback fails at TypeScript compile time on the missing field, never silently at runtime.

## 12. Open questions

- [x] Two channels for one fact — embed, with `installations` a derived flattening from a single producer. §3
- [x] `HostPlatform::Unsupported` — `Option<..>` with `None`; the existing `Warning` is unchanged. §4
- [x] `Detected` + `Error` — representable and intended; status means "present", not "usable". §7
- [x] Slot identity in the UI — the core-owned `label` string; `InstallSlot` stays private (P30D decision 1), so no third exported type. §2
- [x] The "Detected installations" panel — replaced, with the install path preserved as a tooltip. §6
- [ ] `isUnavailableRootWarning`'s `"search root {id} was not found"` coupling (V5) survives this change — **out of scope, recorded**, and pinned by a test.
- [ ] **CA-12** (unparseable component surfaced with path and reason) is untouched and **remains open under T13**. This change does not close T13.
- [ ] macOS and Linux probe tables — **T16**; `client_presence: None` is the honest placeholder until then.
