# Archive Report: report-client-presence-as-status

Date: 2026-08-23
Archiver: sdd-archive (openspec artifact store)

## Summary

`report-client-presence-as-status` is archived. It delivers the **CA-11 slice of T13**: absence of a client install is now reported through a typed `ClientPresence`/`ClientPresenceStatus` record carried on `ScanReport.client_presence`, replacing the old `Warning` `ScanIssue` ("{label} not detected"). All 28 implementation tasks were checked complete in `tasks.md`, `verify-report.md` recorded 0 CRITICAL / 2 WARNING / 1 SUGGESTION, and both warnings were resolved by the orchestrator before this archive step (proposal Success Criterion 6 wording was corrected; an exhaustive-match test for `ClientPresenceStatus` was added to `crates/vertice-core/tests/model_contract.rs`).

## Spec Merges Performed

Four delta specs were merged into their main specs at `openspec/specs/`:

| Domain | Action | Details |
|--------|--------|---------|
| `domain-model` | ADDED 1, MODIFIED 1 | ADDED "Client Presence Is A Typed Per-Slot Status Record" (6 scenarios). MODIFIED "Rust Types Generate a Matching TypeScript Contract": eight types → ten (`ClientPresence`, `ClientPresenceStatus` added), 3 scenarios added. Purpose line updated from "eight core domain types" to "ten core domain types" for consistency (not explicitly listed in the delta's requirement blocks, since it is prose outside any requirement — fixed at archive time as a documentation-consistency correction, same class of edit as the `client-installation-detector` Purpose-line fix below). |
| `client-installation-detector` | ADDED 2, REMOVED 2 | ADDED "Every Resolved Probe Slot Always Emits A Typed Presence Record" and "An Unsupported Platform Reports No Probe Attempt, Not Absence". REMOVED "An Absent Slot Is Reported As An Explicit 'Not Detected' Signal" and "Frontend Reason-String Matching Tracks The New Label Vocabulary (TypeScript)", both carrying `(Reason: ...)`/`(Migration: ...)` per convention. **Purpose line rewritten**: it previously stated the not-detected representation was closed on the `ScanIssue` carrier, that a typed carrier was explicitly rejected (design §2), and that `domain-model` is not a Modified Capability — all three claims were false after this change and are now corrected, citing `report-client-presence-as-status` and the retrofit condition `client-installation-detection` design §2 itself recorded. This edit was flagged in advance by a note in the `domain-model` delta and by the task instructions, precisely because it is prose outside any ADDED/MODIFIED/REMOVED requirement block and the mechanical merge does not reach it. |
| `inventory-ui` | ADDED 3, MODIFIED 5 | ADDED "Always-Visible Supported Clients Table Replaces The Detected-Installations Panel", "No Probe Table Renders As An Explicit Unsupported State, Never Fabricated Rows", "Scan Route Rescan Control", "The Not-Found-Root Incident Suppression Is Pinned By A Behavioral Contract" (4 ADDED total — see note below). MODIFIED "Non-Blocking Successful Scan Diagnostics", "Incident Indicator on List Pages", "Home Scan-Status Block", "Full Scan Report Route", "Localized Inventory Chrome" (5 MODIFIED, full requirement bodies replaced including carried-over scenarios). |
| `frontend-i18n` | MODIFIED 1 | "Catalog Completeness and Boundary": added the 5 supported-clients-table keys and the 3 retired keys (`diagnostics.missingClient`, `scan.installationsTitle`, `scan.installationsEmpty`) to the non-localization/retirement text, plus `ClientPresence.label` to the non-localization list; 2 new scenarios added. |

Note on the `inventory-ui` count: the delta spec's own `## ADDED Requirements` section contains four requirements (the summary table above lists all four); the archive-report table cell says "ADDED 3" in a first pass — corrected here to **ADDED 4**: "Always-Visible Supported Clients Table Replaces The Detected-Installations Panel", "No Probe Table Renders As An Explicit Unsupported State, Never Fabricated Rows", "Scan Route Rescan Control", and "The Not-Found-Root Incident Suppression Is Pinned By A Behavioral Contract".

## Out-Of-Scope-Check On The Merged Specs

Verified after merge, as required by `rules.archive` and the task brief:

- **`domain-model` "ScanIssue Severity Has Two Non-Aborting Levels"** — untouched by this change. Confirmed still states exactly two variants (`Warning`, `Error`); no `Info` variant was added. The delta's own note said "no delta is written against it," and none was.
- **`domain-model` "Rust Types Generate a Matching TypeScript Contract"** — now enumerates ten types: `Component`, `ComponentKind`, `Scope`, `Location`, `SearchRoot`, `ClientInstallation`, `ClientPresence`, `ClientPresenceStatus`, `ScanIssue`, `ScanReport`. Confirmed by reading the merged requirement body.
- **No out-of-scope PoC feature introduced**: grepped the proposal, design, and delta specs for MCPs, `Project`/`Local` scope, or write operations — none present. `verify-report.md` independently confirms zero matches in the actual diff (item 9, "CA-16" and "Scope honesty" sections).

No contradictions were found in the merged specs.

## Known Limitations Recorded At Archive Time

Per `rules.archive` ("Document known limitations for each phase in archive comments"):

1. **CA-12 remains OPEN.** T13's acceptance line (`internal-docs/plan-desarrollo-poc.md:300`) requires CA-11, CA-12, and CA-13 verifiable from the interface. This change closes the CA-11 slice only. CA-13 (embedded, non-actionable state) was already covered by `inventory-ui`'s pre-existing "Embedded Component State" requirement. **CA-12 (an unparseable component surfaced with its path and reason) is untouched by this change and T13 does NOT close here.** The proposal, design, and tasks artifacts are consistent and honest about this scope boundary throughout.

2. **The design §2 reversal, recorded as a decision falling due, not a mistake.** `client-installation-detection` (archived 2026-08-19) chose to represent client absence as a `ScanIssue` `Warning` and explicitly rejected a typed carrier in its design §2, on the stated grounds that freezing a `ScanReport` shape before T9's aggregator and T10/T11's consumer existed was premature. That same §2 named its own retrofit condition: "If T10/T11 concludes it needs a structured answer, B is the retrofit, and it is cheap, because `resolve` already computes the per-slot outcome as a closed value." T11 completed, the consumer materialized, the string-matching coupling had already broken once (PR #30), and this change exercises exactly that pre-recorded retrofit path. `client-installation-detector`'s Purpose line has been updated in the merged main spec to record this reversal explicitly, citing `report-client-presence-as-status`.

3. **A surviving string coupling, deliberately retained (V5).** `frontend/src/lib/scanDiagnostics.ts`'s `isUnavailableRootWarning` still rebuilds the literal string `` `search root ${root.id} was not found` `` to match `crates/vertice-core/src/scan.rs:63`. It is out of scope for this change but is now **load-bearing** for the narrowed `incidentCount`: if the core string and the frontend reconstruction ever drift, `notFound` roots would silently re-enter the incident count. This is pinned by an exact-match test (returns `incidentCount: 0`) and a deliberate one-word-drift test (returns `incidentCount: 1`, proving exact-string not fuzzy matching) in `scanDiagnostics.test.ts`, and is documented as its own requirement in the merged `inventory-ui` spec ("The Not-Found-Root Incident Suppression Is Pinned By A Behavioral Contract").

4. **macOS and Linux probe tables remain T16.** Until then, those platforms yield `client_presence: None` — the honest "not probed" placeholder, distinct from `Some(vec![])` ("probed, found nothing") and from three fabricated `NotDetected` rows. `HostPlatform::Unsupported`'s existing single `Warning` issue (`path: None`, "not implemented on this platform") is unchanged, verified byte-identical apart from the added `None` tail expression (`verify-report.md` scrutiny item 2).

5. **Verify verdict was PASS WITH WARNINGS (0 CRITICAL, 2 WARNING, 1 SUGGESTION); both warnings were resolved by the orchestrator before this archive step.** WARNING 1 (proposal Success Criterion 6 was textually overbroad — "no frontend code matches on a ScanIssue.reason value" contradicted the deliberately retained `isUnavailableRootWarning`) was resolved by correcting the proposal wording. WARNING 2 (the `domain-model` delta's "ClientPresenceStatus is exhaustively matchable" scenario had no covering test) was resolved by adding an exhaustive-match test to `crates/vertice-core/tests/model_contract.rs`, mirroring the existing `Scope` pattern. The 1 SUGGESTION (an undocumented mechanical touch to `tests/model_contract.rs`, adding `client_presence: None`/`Some` to existing `ScanReport` literals so they still compile) was a non-blocking documentation-accuracy note, not a defect, and does not affect archive readiness.

## Task Completion Gate

`tasks.md` shows all 28 tasks checked `[x]` across 9 phases (Phase 0 fixture-coverage honesty through Phase 8 gates). `verify-report.md` independently re-verified task completeness against source, finding no task claims completion the code contradicts. No stale-checkbox reconciliation was needed.

## CRITICAL Issues Check

`verify-report.md` verdict: **0 CRITICAL** issues. Per policy, CRITICAL issues in `verify-report` always block archive; none were present, so this archive proceeds without exception.

## Tool-Access Limitation — Read This Before Trusting The "Move"

This execution had access to only four tools: `Read`, `Edit`, `Write`, `Glob`. **No shell/Bash tool and no filesystem-move/delete tool were available in this session**, and therefore:

- **No `md5sum` (or any checksum) command could be run.** The task brief supplied MD5 checksums of the nine source artifacts and asked for a before/after comparison; that comparison could not be computed here. The content written to this archive folder was reconstructed by reading each source file in full (via the `Read` tool, which returns line-numbered content) and reproducing it verbatim via `Write`, stripping only the line-number/tab prefix `Read` adds for display. This is **not a guaranteed byte-for-byte filesystem copy** — it is a careful manual transcription. I am reporting this limitation plainly rather than claiming an independently verified checksum match I could not produce.
- **The original `openspec/changes/report-client-presence-as-status/` folder was NOT deleted**, because no delete/move tool was available. This archive step created the archive copy at `openspec/changes/archive/2026-08-23-report-client-presence-as-status/` (all nine original artifacts: `exploration.md`, `proposal.md`, `design.md`, `tasks.md`, `verify-report.md`, `state.yaml`, and the three... actually four... delta spec files under `specs/{domain}/spec.md`), but the source folder still exists at its original active-changes path. **The orchestrator (or a follow-up step with shell/git access) MUST**:
  1. Run `md5sum` (or equivalent) against both the original nine files and the nine archived files to confirm byte-for-byte identity, per the task's explicit checksum contract.
  2. Delete `openspec/changes/report-client-presence-as-status/` once that verification passes, so the change is no longer listed as active (the skill's Step 4 checklist item "Active changes directory no longer has this change" is **not yet satisfied**).

This is reported as a **risk**, not silently glossed over: the merge into main specs (via `Edit`, which performs exact string replacement rather than regeneration, and is therefore low-risk for the fidelity concern the task raised) is complete and mechanically sound, but the archive-folder copy and the removal of the active folder need independent verification and completion by an agent with shell access.

## Success Criteria Verification (from verify-report.md, carried forward)

All twelve proposal Success Criteria were walked through in `verify-report.md` and found Met, with the exception of Criterion 6 whose wording imprecision is WARNING 1 above (resolved before archive). CA-7, CA-11, and CA-16 hold; CA-17 fixture-only testing is confirmed; no scope creep was found.

## Artifact Traceability

| Artifact | Archived Path |
|---|---|
| Exploration | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/exploration.md` |
| Proposal | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/proposal.md` |
| Design | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/design.md` |
| Tasks | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/tasks.md` |
| Verify Report | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/verify-report.md` |
| State | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/state.yaml` |
| Delta: domain-model | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/specs/domain-model/spec.md` |
| Delta: client-installation-detector | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/specs/client-installation-detector/spec.md` |
| Delta: inventory-ui | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/specs/inventory-ui/spec.md` |
| Delta: frontend-i18n | `openspec/changes/archive/2026-08-23-report-client-presence-as-status/specs/frontend-i18n/spec.md` |

## Source Of Truth Updated

The following specs now reflect the new behavior:

- `openspec/specs/domain-model/spec.md`
- `openspec/specs/client-installation-detector/spec.md`
- `openspec/specs/inventory-ui/spec.md`
- `openspec/specs/frontend-i18n/spec.md`

## Result

Status: partial
Executive summary: Spec merges into `openspec/specs/` are complete and verified for content correctness (including the Purpose-line prose edit the mechanical merge would have missed); an archive-folder copy of all nine artifacts was created, but byte-identical checksum verification and deletion of the original active-change folder could not be completed in this session due to the absence of a shell/checksum/delete tool, and are flagged as required follow-up.
Next recommended: none (SDD content work complete) — but the orchestrator must independently run checksum verification and delete `openspec/changes/report-client-presence-as-status/` before treating archival as filesystem-complete.
Risks: (1) Archive-folder content fidelity was not independently checksum-verified — manual transcription via Read+Write, not a guaranteed byte copy. (2) The original active-change folder still exists on disk and must be removed by an agent with delete/shell access.
