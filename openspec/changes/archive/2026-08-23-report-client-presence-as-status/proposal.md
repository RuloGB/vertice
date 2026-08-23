# Proposal: Report Client Presence As A Typed Status, Not A Warning

Traces to **T13** (`internal-docs/plan-desarrollo-poc.md:287-301`), whose first scope line is "Cliente no detectado: mensaje explícito, distinto de un error y distinto de una lista vacía". T13 depends on T11, which is complete. Primary criterion **CA-11**; must-not-regress **CA-1** and **CA-7**; bounded by **CA-16** (no write outside the app data directory) and **CA-17** (fixture-based tests, three CI legs).

**Scope honesty about T13.** T13's acceptance line is "CA-11, CA-12 and CA-13 verifiable from the interface". This change delivers the **CA-11 slice only**. CA-13 (embedded, non-actionable) is already covered by `inventory-ui`'s "Embedded Component State"; CA-12 (unparseable component surfaced with its path and reason) is untouched here and remains open under T13.

**No out-of-scope PoC feature is introduced**: no MCPs, no `Project`/`Local` scope, no write operation, no new dependency, no new IPC command.

## Intent

A machine with only Claude Code installed — the common case — is currently reported as unhealthy. The absent OpenCode slot emits `ScanIssue { severity: Warning, reason: "OpenCode (npm) not detected" }` (`crates/vertice-core/src/installations.rs:344-349`), `scan.rs:42` folds it into the shared `issues` vector, and `incidentCount` (`frontend/src/lib/scanDiagnostics.ts:58-64`) counts it. The Home banner turns amber and the incident badge lights on Agents and Skills for a perfectly healthy machine. CA-11 asks for "not detected, never an error and never an unexplained empty list"; today it is "not detected, rendered as an incident".

Two structural defects sit under that symptom:

1. **Absence has no typed carrier.** The UI cannot answer "is OpenCode installed?" without reading English prose.
2. **The frontend string-matches core diagnostics.** `MISSING_CLIENT_REASONS` (`scanDiagnostics.ts:11-15`) hardcodes three strings mirroring `InstallSlot::label()` (`installations.rs:122-128`). This coupling has already broken once — PR #30 had to update those constants and codified the coupling as a MUST (`openspec/specs/client-installation-detector/spec.md:206-218`) — and will break again at T16 when the macOS and Linux probe tables arrive.

Third, the user cannot see what Vertice *looked for*. `ScanPage` renders "Detected installations" only when non-empty (`ScanPage.svelte:93-106`); a slot that resolved to nothing is invisible except as prose in the issue list.

## The decision this change reverses

`client-installation-detection` design §2 closed this deliberately: "Option B is REJECTED", `model/` not edited, `frontend/src/bindings/*.ts` byte-identical, `domain-model` explicitly **not** a Modified Capability — a decision the detector spec's Purpose line still records (`spec.md:5`).

That decision was correct when made and is not being called a mistake. §2 rejected B for a stated reason — freezing a `ScanReport` shape before T9's aggregator and T10/T11's consumer existed — and in the same breath **named its own retrofit path**: "*If T10/T11 concludes it needs a structured answer, B is the retrofit, and it is cheap*, because `resolve` already computes the per-slot outcome as a closed value" (design §2, restated as open question §13). T11 is complete. The consumer now exists, the UX gap is observable, and the string coupling has broken once already. **This is a deferred decision falling due on schedule, not an oversight being patched.** The evidence §2 said it lacked is now in hand.

## Approach

`installations.rs` stops emitting the not-detected `ScanIssue` and instead always emits **one typed record per probe slot**, published through `model/` and carried on `ScanReport` as a new field. Each record carries the slot label, the probed path, a closed `status: detected | notDetected`, and the installations resolved for that slot.

**One row per slot, not per installation.** On Windows the field always has exactly three entries: Claude Code CLI (npm), Claude Code (bundled in Claude Desktop), OpenCode (npm). The table renders client, status, and version(s) — three columns.

**CA-7 is guaranteed structurally, not by convention.** A slot resolving to several installations (Claude Desktop can carry multiple coexisting versions; T7 design §6 established this as normal, observed on the reference machine as `2.1.229` and `2.1.234`) lists **all** of them inside its row, each with its own version and path. The slot record's installation collection is a `Vec`, never an `Option` and never a "highest wins" reduction, so merging two installations is unrepresentable — the same argument design §9 already makes for `ScanReport.installations`, which stays exactly as it is. Both channels keep every installation individually visible.

**The probed path travels but is not displayed.** It stays on the typed record so the full scan-report view or a future expandable detail can surface it without another model change and another binding regeneration.

**Absence leaves the incident channel.** `MISSING_CLIENT_REASONS`, `isMissingClientIssue`, `partitionDiagnostics`'s missing-client branch and that term of `incidentCount` are deleted. `notFound` search roots receive the same neutral treatment: neutral styling, no danger colour, removed from `incidentCount`. A slot that *failed* (unreadable `package.json`, unlistable `Packages`) still emits its `Error` `ScanIssue` and still counts — broken must never read as absent, which is the CA-11 property `npm-dir-no-package-json` pins.

**Rescan on the Scan route.** Correcting the original framing: `rescan` is already wired unconditionally on Agents and Skills (`ComponentToolbar.svelte:28-35`, `App.svelte:111,122`); Home's retry is gated to `status === "failed"`. `ScanPage.svelte` is the only page with no rescan control and does not accept an `onReload` prop (`ScanPage.svelte:10-22`). Adding one mirrors `ComponentToolbar` and reuses the existing `toolbar.reload` / `toolbar.reloading` keys. Small isolated addition, not a rewiring.

**Slot labels stay untranslated proper nouns** ("Claude Code CLI (npm)"), with only the surrounding chrome localized — see the i18n resolution below.

### Option B, considered and rejected

`IssueSeverity::Info` is cheaper: one enum variant, a one-line binding diff, no new file. **Cheapness is not the counter-argument.** It is rejected because it solves nothing that matters:

- It keeps absence in the issue channel and keeps `scanDiagnostics.ts` string-matching exact English reasons — the coupling that already broke in PR #30 and gets three more strings per platform at T16.
- It delivers no always-visible "every client we look for" table, so the user still cannot see what was searched.
- It directly contradicts `domain-model`'s standing MUST that `ScanIssue.severity` has "exactly two variants" (`openspec/specs/domain-model/spec.md:105-119`), and reopens the severity decision T7 design §4 defended separately and at length (V2: two levels is deliberate, not an oversight). Design §2's temptation table pre-rejected `Info` by name.

Option B reopens a decision defended more firmly than the one Option A reopens, and leaves the two structural defects intact.

### i18n resolution (the gap the exploration found)

`frontend-i18n` currently forbids localizing `ScanIssue.reason` because it is diagnostic passthrough (`openspec/specs/frontend-i18n/spec.md:35`). Today the slot label reaches the UI *inside* that string, so the rule covers it by accident. Once the label is a first-class typed UI field, the rule no longer reaches it — and nothing else does. **Resolution: slot labels are product proper nouns and MUST NOT be localized**, stated explicitly in the `frontend-i18n` catalog boundary rather than left to fall through the gap. "Claude Code CLI (npm)" is the tool's own name; translating it would invent a product name that does not exist. Only the table chrome — title, column headers, the detected / not-detected status words — is catalog-driven.

### The binding contract (explicit obligation)

Adding an exported type and a `ScanReport` field changes `frontend/src/bindings/`. CI regenerates bindings and fails on any diff, running `git add --intent-to-add` first so a brand-new file is also caught. Bindings are regenerated **only** by `cargo test -p vertice-core` and MUST NEVER be hand-edited: the Rust type is the source. This change MUST land the regenerated `ScanReport.ts` plus the new type's `.ts` file in the same commit as the Rust types.

## Scope

### In Scope

- New plain-data type in `crates/vertice-core/src/model/` deriving `Serialize`/`Deserialize`/`TS`, plus a new `ScanReport` field; `model/`'s import allow-list (`model/mod.rs:1-22`) is respected — no I/O, no clock.
- `installations.rs` restructured so every slot always emits a status record; the not-detected `ScanIssue` removed entirely.
- `scan.rs` carries the new field into `ScanReport`.
- Regenerated `frontend/src/bindings/`.
- `scanDiagnostics.ts`: delete `MISSING_CLIENT_REASONS` and `isMissingClientIssue`, drop the missing-client branch and its `incidentCount` term, drop `unavailableRoots` from `incidentCount`.
- `ScanPage.svelte`: always-visible "Supported clients" table (client / status / version(s)), neutral styling for `notFound` roots, and a rescan control mirroring `ComponentToolbar`; `App.svelte` threads `onReload`/`status`.
- New i18n keys for the table chrome, English and Spanish, complete in both.
- Rust and Vitest test **rewrites** (not additions): `tests/client_installations.rs`, `src/scan.rs:129-154`, `frontend/src/lib/scanDiagnostics.test.ts:6-10`.

### Out of Scope

- macOS and Linux probe tables — **T16**. `HostPlatform::Unsupported` behaviour is unchanged and still emits its single "not implemented on this platform" `Warning`; it MUST NOT be rewritten into three `notDetected` rows, which would tell a macOS user their clients are absent when Vertice did not look (design §5.2).
- CA-12 (unparseable component surfaced in the UI) — remains open under T13.
- Displaying the probed path in the table; the data travels, the column does not.
- Any change to `ScanReport.installations`, `ClientInstallation`, `IssueSeverity`, or the `Error`-severity taxonomy.
- New fixture homes. The PR #30 tree (`tests/fixtures/client-installations/`, 14 homes) already covers every case; assertions change, fixtures do not.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `inventory-ui`: five requirements. "Non-Blocking Successful Scan Diagnostics" and "Full Scan Report Route" (the route must always render every probe slot with its status); "Incident Indicator on List Pages" and "Home Scan-Status Block" (a not-detected client and a `notFound` root no longer make a report unhealthy); "Localized Inventory Chrome" (new table chrome keys).
- `domain-model`: "Rust Types Generate a Matching TypeScript Contract" enumerates exactly eight core types (`spec.md:149-163`) and becomes ten (`ClientPresence` plus its status enum are two exported types, not one) — a MODIFIED delta. A new requirement governs the typed presence record. **First time this capability is modified by client-detection work**, reversing design §2.
- `client-installation-detector`: two requirements REMOVED — "An Absent Slot Is Reported As An Explicit 'Not Detected' Signal" (`spec.md:64-93`) and "Frontend Reason-String Matching Tracks The New Label Vocabulary (TypeScript)" (`spec.md:206-218`, which *is* the string coupling). The Purpose line's claim that `domain-model` is not a Modified Capability is superseded.
- `frontend-i18n`: **yes, Modified.** "Catalog Completeness and Boundary" (`spec.md:35`) enumerates the required catalog coverage — the new table chrome extends it — and its non-localization rule names `ScanIssue.reason` specifically. Slot labels leave that string and must be named explicitly as non-localized, or the rule silently stops covering them. Leaving this capability unmodified would let the decision fall through a spec gap.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/vertice-core/src/model/` | New + Modified | New status type; new `ScanReport` field |
| `crates/vertice-core/src/installations.rs` | Modified | Per-slot status record; not-detected `ScanIssue` removed |
| `crates/vertice-core/src/scan.rs` | Modified | Carry the new field; test at `129-154` rewritten |
| `crates/vertice-core/tests/client_installations.rs` | Rewritten | Assert status records instead of reason strings |
| `crates/vertice-core/tests/fixtures/client-installations/` | **Unchanged** | PR #30's tree already covers every case |
| `frontend/src/bindings/` | Regenerated | New `.ts` file + `ScanReport.ts`; never hand-edited |
| `frontend/src/lib/scanDiagnostics.ts` + `.test.ts` | Modified | String coupling deleted; `incidentCount` narrowed |
| `frontend/src/lib/pages/ScanPage.svelte` | Modified | Supported-clients table; rescan control; neutral `notFound` |
| `frontend/src/App.svelte` | Modified | Thread `onReload`/`status` into `ScanPage` |
| `frontend/src/lib/i18n/catalogs.ts` | Modified | New table chrome keys, `en` + `es` |
| `crates/vertice-app/` | **Unchanged** | No new command, no capability change |
| `Cargo.toml`, `Cargo.lock`, `deny.toml` | **Unchanged** | No new dependency |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Binding drift: the new `.ts` file is forgotten and CI's `--intent-to-add` gate fails late | Medium | Regenerate via `cargo test -p vertice-core` in the same commit as the Rust type; never hand-edit |
| `incidentCount` narrowed too far — a genuine `Error` slot stops counting, so a broken install reads as healthy | Medium | Only the missing-client and `notFound`-root terms are removed; `Error` issues stay. Pin with a test asserting a broken-`package.json` fixture still produces an incident |
| A slot with N installations collapses to one row *and* one version, silently regressing CA-7 | Medium | The record's installation collection is a `Vec`, not an `Option`; a two-version bundled fixture asserts two versions rendered in one row and fails before implementation (`strict_tdd: true`) |
| Two parallel channels for the same fact (`ScanReport.installations` and the new field) drift apart | Medium | The slot records are the sole producer of `installations`; a determinism test asserts the flattened slot installations equal `ScanReport.installations` exactly |
| `HostPlatform::Unsupported` is rewritten into three `notDetected` rows, lying to macOS/Linux users | Medium | Explicitly out of scope; the existing `Unsupported` test (design §5.2) stays green unmodified and is the tripwire |
| Reversing design §2 sets a precedent that any UI want justifies a `model/` edit | Low | The reversal is licensed by §2's own recorded retrofit condition, and the delta must cite it; the bar stays "the consumer exists and the evidence is in hand" |
| `domain-model`'s "exactly two variants" MUST is touched by accident | Low | Option A does not add `IssueSeverity::Info`; that requirement stays byte-identical and its untouched state is a review check |
| Untranslated slot labels read as an i18n bug to a Spanish user | Low | Surrounding chrome is fully localized; the `frontend-i18n` delta records labels as proper nouns so the choice is auditable, not accidental |
| Removing a MUST from `client-installation-detector` looks like weakening the detector | Low | Both removals carry `(Reason: ...)`; the detector's behaviour strengthens — absence gains a typed carrier and loses a parsed string |

## Rollback Plan

Three-layer revert, in dependency order. Every layer is revertible independently except the binding pair, which moves atomically with the core types.

1. **Core (`vertice-core`)** — restore the not-detected `ScanIssue` emission in `installations.rs`, remove the new `model/` type and the `ScanReport` field, restore the `scan.rs` assembly and its test. No fixture change to undo (the tree was never touched), no dependency, no lockfile movement, no `deny.toml` entry.
2. **Bindings** — `cargo test -p vertice-core` regenerates `frontend/src/bindings/` from the reverted Rust types, removing the new `.ts` file and restoring `ScanReport.ts`. **Never hand-edited, in either direction.** The `--intent-to-add` gate confirms the revert is complete.
3. **Frontend** — restore `MISSING_CLIENT_REASONS`, `isMissingClientIssue`, the `partitionDiagnostics` branch and the `incidentCount` terms; remove the supported-clients table, the `ScanPage` rescan control and the `App.svelte` prop threading; drop the new catalog keys.

**`vertice-app` is untouched**, so the IPC surface and `capabilities/default.json` need no revert. **Migration: none** — the PoC persists nothing; `ScanReport` is rebuilt on every scan, so an old and a new report never coexist. A partial rollback (core reverted, frontend not) fails at TypeScript compile time on the missing binding field, not silently at runtime.

## Dependencies

- **PR #30** (`fix-windows-claude-desktop-probe`, archived 2026-08-23) — merged. It settled the slot vocabulary and the fixture tree this change asserts against, and explicitly deferred this work.
- **T11** — complete. It is the consumer whose existence design §2 named as the retrofit condition.
- No blocking external dependency. T16 depends on this change's shape, not the reverse.

## Delivery

Single PR. `size:exception` pre-accepted by the user; chained PRs are **not** proposed — the core type, the binding regeneration and the frontend consumer cannot be split without leaving `main` with a typed field nothing reads or a table with no data.

## Success Criteria

- [ ] A machine with Claude Code but no OpenCode reports **zero incidents**: green Home banner, no badge on Agents or Skills (CA-11).
- [ ] The `scan` route always shows exactly three supported-client rows on Windows, each with `detected` or `notDetected`, whether or not anything was found.
- [ ] A bundled slot with two coexisting Claude Desktop versions renders **both** versions inside its single row, neither merged nor reduced (CA-7).
- [ ] A `notFound` search root renders neutrally and is excluded from `incidentCount`.
- [ ] A slot with an unreadable or malformed `package.json` still produces an `Error` issue **and** still counts as an incident — broken never reads as absent.
- [ ] `MISSING_CLIENT_REASONS` and `isMissingClientIssue` no longer exist anywhere in `frontend/`; no frontend code matches a `ScanIssue.reason` value to decide whether a client is installed. The one surviving reason match, `isUnavailableRootWarning`, is out of scope and deliberately retained: it suppresses the duplicate search-root warning and is load-bearing for the narrowed incident count, so it is pinned by an exact-match test and a drift test rather than removed.
- [ ] `ScanPage` offers a rescan control that invokes `rescan` and shows a loading state, mirroring `ComponentToolbar`.
- [ ] `HostPlatform::Unsupported` still yields zero installations and exactly one `Warning` with `path: None`, unchanged.
- [ ] Catalogs are complete in `en` and `es` for all new chrome; slot labels are identical in both.
- [ ] Every assertion runs against `tests/fixtures/client-installations/`; no test reads the author's machine (CA-17).
- [ ] No write outside the app data directory; no `File::create`, `OpenOptions::write` or equivalent introduced (CA-16).
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, and `npm run lint && npm run check && npm run test && npm run build` all pass, with `frontend/src/bindings/` in sync and regenerated, never hand-edited.
