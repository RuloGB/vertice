# Exploration: Report Client Presence As A Typed Status, Not A Warning

> Change: `report-client-presence-as-status`. Follow-up to `openspec/changes/archive/2026-08-23-fix-windows-claude-desktop-probe/`, whose proposal deferred exactly this work: "All frontend/UX work. A follow-up `inventory-ui` change will always list every searched slot with found/not-found state, remove absence from the `ScanIssue` channel, and add a rescan button."
>
> **Environment note.** Every finding below is grounded in repository files and committed fixtures. No conclusion is drawn from probing the real `%APPDATA%` or user home: this session's shell may run inside an MSIX-redirected view that ordinary processes do not see, so such a probe is unsound as evidence. CA-17 requires fixture-based verification regardless.

## 1. Current data flow, end to end

`crates/vertice-core/src/installations.rs:70-95` (`scan_for`) resolves three probe slots (`ClaudeCodeNpm`, `ClaudeCodeBundled`, `OpenCodeNpm`) through `windows_install_probes` and `resolve_slot` (`installations.rs:318-330`). Each slot yields either `ClientInstallation` entries or a `ScanIssue { severity: Warning, reason: "{label} not detected" }` (`installations.rs:344-349`, `installations.rs:508-513`).

`crates/vertice-core/src/scan.rs:24-54` merges that `InstallationScan` into `ScanReport { installations, issues, roots_scanned, components, duration_ms }`. `installations.issues` is folded straight into the shared `issues` vector (`scan.rs:42`), where it becomes indistinguishable from a skill or agent scan issue.

`ScanReport` (`crates/vertice-core/src/model/report.rs:20-28`) crosses the IPC boundary unmodified. `crates/vertice-app/src/commands.rs:15-43` exposes `scan` and `rescan` as thin `spawn_blocking(vertice_core::scan::scan)` wrappers with no business logic; `crates/vertice-app/src/lib.rs:11` registers both.

Frontend: `frontend/src/lib/scan.ts:6-17` calls `invoke<ScanReport>`. `frontend/src/App.svelte:70-82` holds the single `report` state, derives `diagnostics = partitionDiagnostics(report.rootsScanned, report.issues)` (`App.svelte:39-41`) and `incidents = incidentCount(diagnostics)` (`App.svelte:42`), then passes both to `HomePage`, `ComponentKindPage` (Agents/Skills) and `ScanPage`.

`frontend/src/lib/scanDiagnostics.ts:11-15` hardcodes `MISSING_CLIENT_REASONS`, three exact English strings mirroring the Rust `label()` outputs at `installations.rs:122-128`. `isMissingClientIssue` (`scanDiagnostics.ts:17-23`) and `partitionDiagnostics` (`scanDiagnostics.ts:29-49`) split `report.issues` into `unavailableRoots` / `missingClientIssues` / `remainingRecoverableIssues`. `incidentCount` (`scanDiagnostics.ts:58-64`) sums all three lengths.

## 2. Blast radius: every consumer of the pieces that change

| Surface | Consumers |
|---|---|
| `report.issues` | `scan.rs` (assembly), `scanDiagnostics.ts` (`partitionDiagnostics`, `isMissingClientIssue`), `ScanIssueList.svelte`, `ScanPage.svelte`, `scanDiagnostics.test.ts`, `tests/client_installations.rs`, `scan.rs` tests (`missing_roots_and_clients_are_visible_diagnostics` asserts `reason.ends_with("not detected")` count == 3, `scan.rs:146-153`) |
| `report.roots_scanned` / `notFound` | `ScanPage.svelte:71-89` (renders `notFound` with danger colouring), `scanDiagnostics.ts` (`unavailableRoots`), indirectly `HomePage` and `ComponentKindPage` through `incidents` |
| `incidentCount` | `App.svelte:42` -> `HomePage.svelte:83-96` (banner + retry visibility), `ComponentKindPage.svelte:53` (`IncidentIndicator`), `ScanPage.svelte:57-65` (verdict banner colour and text) |
| `partitionDiagnostics` / `isMissingClientIssue` | `App.svelte`, `ScanIssueList.svelte`, and their `.test.ts` files |
| `report.installations` | **Only** `ScanPage.svelte:93-106`, a "Detected installations" list rendered only when non-empty |

The `incidentCount` chain is the mechanism behind the reported defect: a machine without OpenCode produces an `"OpenCode (npm) not detected"` `Warning`, which makes `incidents > 0`, which turns the Home banner amber and lights the incident badge on Agents and Skills.

## 3. `model/` purity: cost of adding a type

`crates/vertice-core/src/model/mod.rs:1-22` declares the import allow-list verbatim (`std::path`, `std::time::Duration`, `serde`, `ts_rs`, `thiserror`, `unicode_normalization`) and forbids `std::fs`, `std::io`, `std::env`, `SystemTime`/`Instant`. A new plain-data type costs nothing structurally — it is the same shape as `ClientInstallation` and `ScanIssue`, which already live there. The real cost is a binding regeneration plus reopening the design decision in §5.

## 4. The `ts_rs` binding contract

Every public model type carries `#[ts(export, export_to = "../../../frontend/src/bindings/")]` (`installation.rs:13`, `report.rs:19,35,48`). `cargo test -p vertice-core` regenerates the `.ts` files as a side effect of the derived export tests. CI regenerates and diffs, running `git add --intent-to-add` first so a brand-new binding file is also caught. Bindings MUST NEVER be hand-edited; the Rust type is the source.

## 5. Existing spec surface: what moves

**`openspec/specs/inventory-ui/spec.md`** — all four requirements named in the brief exist and are affected:

- "Non-Blocking Successful Scan Diagnostics" (§92-105) — already requires that a missing-client notice be discreet.
- "Incident Indicator on List Pages" (§186-205) — fires when `issues` is non-empty OR any root is `notFound`.
- "Home Scan-Status Block" (§207-228) — three states: healthy / completed-with-issues / failed.
- "Full Scan Report Route" (§171-184).

A fifth requirement is affected and was not named in the brief: **"Localized Inventory Chrome"** (§73-81) — a new "Supported clients" table needs new catalog keys under it.

**`openspec/specs/domain-model/spec.md`**

- "ScanIssue Severity Has Two Non-Aborting Levels" (§105-119) states "exactly two variants: `Warning` and `Error`". Option A leaves this untouched; **Option B contradicts it directly**.
- "Rust Types Generate a Matching TypeScript Contract" (§149-163) enumerates exactly eight core types. Option A adds a ninth and needs a MODIFIED delta.

**`openspec/specs/client-installation-detector/spec.md`**

- "An Absent Slot Is Reported As An Explicit 'Not Detected' Signal" (§64-93) — REMOVED under Option A.
- "Frontend Reason-String Matching Tracks The New Label Vocabulary (TypeScript)" (§206-218) — REMOVED under Option A. This requirement *is* the string coupling the change objects to, currently codified as a MUST.
- The capability's Purpose line records that design §2 closed the not-detected representation on the `ScanIssue` carrier and explicitly rejected a typed carrier, and that `domain-model` is not a Modified Capability. Option A reverses that sentence and makes `domain-model` a Modified Capability for the first time in this capability's history.

## 6. i18n

`frontend/src/lib/i18n/catalogs.ts` holds `diagnostics.missingClient` ("Supported client unavailable" / "Cliente compatible no disponible"), the only existing key tied to client absence; it labels a section inside `ScanIssueList.svelte`, not a table. `scan.installationsTitle` and `scan.installationsEmpty` already exist for the current "Detected installations" list.

An always-visible "Supported clients" table needs new keys — a title, and per-status labels for detected / not detected. **A decision is required** on slot labels: today the label reaches the UI inside `ScanIssue.reason`, and `openspec/specs/frontend-i18n/spec.md:35` explicitly forbids localizing `ScanIssue.reason` because it is diagnostic passthrough. Once the label becomes a first-class typed UI field it stops being passthrough, so it needs either a translation strategy or an explicit decision that slot labels remain untranslated proper nouns.

A rescan button already has catalog coverage: `toolbar.reload` / `toolbar.reloading` exist, as does `home.scanRetry`. A Scan-route button can reuse `toolbar.reload` rather than add keys.

## 7. Fixtures and tests

The fixture tree is `crates/vertice-core/tests/fixtures/client-installations/`, with 14 fixture homes (`nothing`, `legacy`, `packaged`, `packaged-and-legacy`, `two-packages`, `packaged-empty`, `non-claude-packages`, `packages-unreadable`, `opencode-npm`, `isolation`, `no-version-key`, `version-not-a-string`, `package-json-empty`, `package-json-unreadable`, `npm-dir-no-package-json`). This is the tree PR #30 introduced; the older `fixtures/installations/` tree referenced by the T7 design no longer exists on disk.

Tests pinning the current contract, all of which need **rewrites rather than additions** under Option A:

- `crates/vertice-core/tests/client_installations.rs` — asserts exact `Warning` severities and `reason` strings per fixture.
- `crates/vertice-core/src/scan.rs:129-154` — asserts `reason.ends_with("not detected")` count == 3.
- `frontend/src/lib/scanDiagnostics.test.ts:6-10` — hardcodes the same three reason strings and tests `isMissingClientIssue`, `incidentCount`, `partitionDiagnostics` against them.

## 8. Two candidate approaches

### Option A — typed per-slot presence field on `ScanReport`

One entry per probe slot carrying client, slot label, probed path, and `status: detected | notDetected`, with version(s) when detected.

- **Blast radius**: a new type in `model/`; a new field on `ScanReport`; `installations.rs` restructured so `resolve_slot` always emits a slot-status record; the `notDetected` `ScanIssue` removed entirely (two requirements removed from `client-installation-detector`); `scanDiagnostics.ts` loses `MISSING_CLIENT_REASONS` and `isMissingClientIssue`, `partitionDiagnostics` loses its missing-client branch, `incidentCount` loses that term; `ScanPage.svelte` gains an always-visible table; Rust and Vitest tests rewritten.
- **Bindings**: one new `.ts` file plus a regenerated `ScanReport.ts`. `domain-model` becomes a Modified Capability.
- **Design §2 impact**: reverses a closed decision. Mitigating fact: §2's own table already rated the retrofit "cheap" because `resolve` already computes the per-slot outcome as a closed value, and named T10/T11 as the point where a typed carrier might be needed. T11 is done and the UX gap is now observable, so this is a deferred decision being revisited on schedule rather than an oversight being patched.
- **Leaves unsolved**: nothing structural.
- **Ageing under T16**: scales cleanly. New platform slots add typed entries; no per-platform string coupling to invent.
- **Effort**: medium.

### Option B — add `IssueSeverity::Info`

- **Blast radius**: one enum variant in `model/report.rs`; `installations.rs` switches `Warning` -> `Info`; `partitionDiagnostics`/`incidentCount` need a decision on whether `Info` counts as an incident.
- **Bindings**: a one-line `IssueSeverity.ts` diff, no new file.
- **Design impact**: does not reopen §2's carrier decision, but **does** contradict `domain-model`'s "exactly two variants" MUST, and reopens the severity decision the T7 design defended separately and at length (V2: two levels is deliberate, not an oversight). The design pre-rejected this exact option by name.
- **Leaves unsolved**: causes 1 and 3 entirely — no always-visible "every client we look for" table, and `scanDiagnostics.ts` still string-matches exact English reasons, the coupling that already broke once in PR #30. Addresses cause 4 only partially.
- **Ageing under T16**: no better than today; three more hardcoded strings per platform.
- **Effort**: low, but it needs an explicit argument for why the recorded V2 reasoning no longer holds — cheapness alone is not that argument.

### Reading

Option A fixes confirmed causes 1, 3 and 4, and follows the retrofit path the T7 design explicitly left open once T11 existed. Option B is symptom restyling and re-triggers a decision the design defended more firmly than the one A reopens. Accounting for the frontend test rewrites B still requires, A is not dramatically more expensive.

## 9. Open questions

**Genuine product decisions** (require the user):

1. Should "Supported clients" rows carry the probed path, or version only? The path helps support and debugging but is technical noise in a general table.
2. One row per **slot** (three rows: npm CLI, bundled-in-Desktop, OpenCode) or one row per **resolved installation** (which can exceed three when several Claude Desktop versions coexist)? This determines the typed field's shape.
3. Where does the rescan control live on the Scan route — beside the verdict banner, or in the page header mirroring `ComponentToolbar`?
4. Do `notFound` search roots get exactly the same neutral treatment as not-detected clients, or does "a configured root that vanished" stay visually distinct from "a client the user never installed"?
5. Do slot labels get localized, given `frontend-i18n` currently forbids localizing `ScanIssue.reason` as passthrough (see §6)?

**Answerable from code, no interruption needed**: current data flow and blast radius (§1-2); `model/` purity cost (§3); which spec requirements need deltas (§5); rescan wiring (see §11).

## 10. Roadmap trace

- **Phase T13** — "Estados de error, vacíos y componentes no accionables" (`internal-docs/plan-desarrollo-poc.md:287-301`). Its scope line reads "Cliente no detectado: mensaje explícito, distinto de un error y distinto de una lista vacía", which is precisely this gap. T13 depends on T11, which is done.
- **CA-11** — absent client reported as "not detected" (`plan-desarrollo-poc.md:372`, tied to T7 and T13). Primary criterion.
- **CA-16** (no writes outside the app data directory) and **CA-17** (fixture-based tests on three platforms in CI) bound the implementation.
- **CA-1** (starts and shows the list with no configuration) and **CA-7** (both Claude Code installations detected separately, each with its version) must not regress; N coexisting Claude Code installations still need to render distinctly in any new table.

## 11. Correction to the brief

The claim that "`rescan` is only reachable from Home, and only in the failed state" is **half wrong**. `rescan` (`frontend/src/lib/scan.ts:15-17`; IPC command `crates/vertice-app/src/commands.rs:41-43`) is already wired **unconditionally** to the Reload button (`ComponentToolbar.svelte:28-35`, keys `toolbar.reload` / `toolbar.reloading`) on both the Agents and Skills pages, via `ComponentKindPage.svelte:56` and `App.svelte:111,122`. Home's retry button (`HomePage.svelte:98-105`) is indeed gated to `status === "failed"`.

The actual gap is narrower: **`ScanPage.svelte` is the only page with no rescan control at all** — it does not even accept an `onReload` prop, unlike `ComponentKindPage`. Adding one mirrors the existing `ComponentToolbar` pattern and threads `onReload`/`status` from `App.svelte`. This is a small isolated addition, not a rewiring of `rescan` reachability.
