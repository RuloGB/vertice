# Archive Report: add-client-version-freshness

Date: 2026-08-24
Archiver: sdd-archive (openspec artifact store)

## Summary

`add-client-version-freshness` is archived. It delivers Vertice's **first freshness-checking capability**: a new `component-freshness` capability answers "is this detected client installation out of date" with a closed three-valued verdict (`UpToDate` / `Outdated { latest }` / `Unknown { reason }`), a total (never-panicking) comparison rule with explicit prerelease semantics, a core-owned trait abstraction (`ReferenceVersions`) whose only concrete implementation lives in `vertice-app`, and a per-slot upstream-identity table (npm registry for the two npm slots, GitHub Releases for Codex, permanent `Unknown`/no-request for the bundled Claude Desktop slot with no established upstream). It is also **Vertice's first outbound network call in its history** — `reqwest` with the `native-tls` backend — delivered enabled by default with a first-run disclosure and a visible opt-out, a deliberate, disclosed product decision rather than an implementation detail.

51 of 51 checked task lines in `tasks.md` are `[x]`, with one exception recorded below (task 3.9). `verify-report.md` recorded **3 CRITICAL / 3 WARNING / 1 SUGGESTION**; all three CRITICAL findings were closed by the orchestrator before this archive step, confirmed independently and recorded in `verify-report.md`'s own addendum, with all eight quality gates re-run green afterward. Two user-reported defects, found after the verify pass, were also fixed and are carried into the permanent record below. This archive proceeds under the strict-archive policy: no CRITICAL issue remains open.

## Delivery Shape (as delivered, not as originally forecast)

`tasks.md`'s Review Workload Forecast originally called this **high risk, chained PRs recommended** (three slices: core, app/fetcher, frontend). The user explicitly chose a **single PR with a recorded `size:exception`** on 2026-08-24, accepting the over-budget review load rather than a stacked-PR chain. The three-slice structure was retained as the *implementation and commit* order — each slice reached a fully green state before the next began — but not as a PR boundary. This is recorded here because it is a deviation from the default delivery-strategy guard, made with the user's explicit, informed consent, not a silent skip.

## Spec Merges Performed

Eight delta/new specs were synced into `openspec/specs/`: one new capability, seven modified.

| Domain | Action | Details |
|--------|--------|---------|
| `component-freshness` | **New capability** | Copied verbatim — the delta was already written in living-spec shape (`## Purpose` + `## Requirements`), not delta bookkeeping format, so no ADDED/MODIFIED conversion was needed. All 9 requirements and their scenarios carried over unchanged. |
| `domain-model` | MODIFIED 1 requirement | "Rust Types Generate a Matching TypeScript Contract": body updated to enumerate all sixteen types (ten pre-existing + `ClientKind` + the six new: `Freshness`, `FreshnessSubject`, `FreshnessCheck`, `FreshnessReport`, `ClientInstallSlot`, `FreshnessSettings`). **Merge judgment, stated so it is auditable**: the delta's own scenario list omitted three pre-existing scenarios not superseded by this change (`ClientKind's binding reflects three variants`, `The new presence types export their own bindings`, `ScanReport's new field is optional at the binding boundary`) — carried over from the two prior archived cycles. A literal wholesale replacement would have silently deleted them. Per the archive skill's rule (preserve requirements/scenarios not mentioned in the delta when a replacement would be destructive), all three were preserved and the delta's five new scenarios were additively inserted alongside them (one of which, `ClientPresence's binding gains the slot field`, was edited to also declare the `slot` field per the delta's own "ClientPresence gains slot" scenario, merged into the existing "new presence types" scenario's field list). Purpose line updated to record the six-type growth and name the change. |
| `workspace-architecture` | ADDED 2 requirements | "vertice-core Stays HTTP-Free" and "The Reference-Version Seam Is Owned By vertice-app" — both carried verbatim from the delta, appended after the existing "A Third Parser Seam, toml.rs" requirement. Purpose line extended to record the fourth seam and the HTTP-free containment restatement. |
| `desktop-shell` | MODIFIED 4 requirements | "Minimal Scan Command Surface", "Non-Blocking Command Execution", "Typed IPC Contract", "Minimal Capability Grant" — each wholesale-replaced; the delta already contained every pre-existing scenario for all four (verified line-by-line before replacing, since a destructive loss here would be exactly the failure mode this archive was warned about) plus its own new freshness-related scenarios. **This delta already documented the shipped five-command surface** (`scan`, `rescan`, `freshness`, `freshness_settings`, `set_freshness_settings`), not the stale "exactly three" the proposal/design originally stated — the orchestrator's pre-archive CRITICAL-1 correction (see `verify-report.md`'s addendum) had already been applied to this delta file before archive began. "Hardened Content Security Policy" and "Frontend Filesystem Boundary" are untouched by the delta and were left as-is. Purpose line extended with the command-count-growth note. |
| `client-installation-detector` | ADDED 1 requirement | "Published Presence Records Carry A Stable, Non-Display Slot Discriminator" — carried verbatim, inserted after "Every Resolved Probe Slot Always Emits A Typed Presence Record" (the closest related existing requirement) and before "Each Codex Release Directory Is Its Own Installation". Purpose line extended to record this capability as a further Modified Capability of the same lineage as the two prior reversals it already documents (`report-client-presence-as-status`, `add-codex-client-support`), stating explicitly that detection behavior stays byte-identical and only `ClientPresence`'s published shape changed. |
| `inventory-ui` | ADDED 2 requirements | "Freshness Badge On The Clients View" and "An Outdated Verdict Is Never An Incident" — both carried verbatim, inserted immediately before the existing "No Probe Table Renders As An Explicit Unsupported State" requirement, in the same section of the spec that already documents the supported-clients table. Purpose line extended with a one-sentence provenance note. |
| `frontend-i18n` | MODIFIED 1 requirement | "Catalog Completeness and Boundary": wholesale-replaced; the delta's scenario list already contained every pre-existing scenario (verified before replacing) plus the four freshness-chrome scenario additions/renames (`Spanish catalog stays complete` renamed to `..., including freshness chrome`; `A freshness verdict's reference version stays verbatim in both locales` added). No preservation gap. |
| `scan-orchestration` | MODIFIED 2 requirements | "Measured Reference-Volume Performance": wholesale-replaced, delta complete. "Visible and Isolated Diagnostics": **merge judgment** — the delta's scenario list omitted the pre-existing "A malformed Codex agent file does not abort the scan" scenario (added by `add-codex-client-support`, not superseded by this change). Preserved it and additively inserted the delta's new "A failed freshness lookup never becomes a ScanIssue" scenario alongside it, for the same destructive-loss reason as the `domain-model` merge above. Purpose line extended with a provenance sentence. |

### Merge-judgment calls, stated so they are auditable

Two of the eight merges required judgment beyond a literal wholesale replacement, both for the same reason: a delta's `MODIFIED Requirements` scenario list did not re-list every scenario a **prior** archived change had already added to that same requirement, because the delta author (writing before those prior cycles' content was necessarily back in view) enumerated only the scenarios this change's own behavior required. A literal replace-in-place would have silently deleted scenarios unrelated to this change's actual scope:

1. **`domain-model` / "Rust Types Generate a Matching TypeScript Contract"** — three scenarios from the `add-codex-client-support` and `report-client-presence-as-status` cycles were preserved (see table above).
2. **`scan-orchestration` / "Visible and Isolated Diagnostics"** — one scenario from `add-codex-client-support` was preserved (see table above).

Both preservations follow the exact precedent recorded in the `add-codex-client-support` archive report (2026-08-23), which established this same judgment call for the same class of situation.

## Verify-Report Resolution Performed Before Archive (already applied, confirmed here)

`verify-report.md`'s original body recorded **3 CRITICAL** findings and explicitly stated the strict-archive policy: "NEVER archive a change that has CRITICAL issues in its verification report." All three were closed by the orchestrator in an addendum appended to `verify-report.md` itself (dated 2026-08-24, present in the archived copy), confirmed independently before acting on each:

1. **Command surface contradicted the spec.** `specs/desktop-shell/spec.md` said "exactly three commands" while the shipped shell exposes five (`scan`, `rescan`, `freshness`, `freshness_settings`, `set_freshness_settings`). Root cause named precisely: the gap was *narrated* in `apply-progress.md`'s Slice 3 completion note but never *corrected* in `design.md` §11 or the spec text. Fixed: `specs/desktop-shell/spec.md` now documents the five-command surface (already reflected in the merge above — no further edit was needed at archive time), and `design.md` §11 was corrected from "one new command" to three, with the explicit correction note this archive's copy of `design.md` preserves.
2. **`domain-model`'s type enumeration undercounted.** The spec listed five new types, omitting `FreshnessSettings` (which exists, derives `TS`, and ships a binding). Fixed to six new types plus the modified `ClientPresence` — reflected in the merged spec above.
3. **Two privacy/incident-isolation scenarios had no covering test.** "An outbound request carries no identifying content" was *unassertable as written* (`reqwest::Client` exposes no way to read back its own configured headers) until `fetch::user_agent()` was extracted specifically to make it testable; two tests were then added. "`Outdated` never triggers the incident indicator or the Home block" was structurally true but untested; a dedicated `App.test.ts` case was added, honestly recorded as passing on first run (a regression guard, not a red-to-green TDD cycle, since the guarantee was already structurally true — there is no code path from a verdict to `incidentCount`).

All eight quality gates were re-run after these three fixes and confirmed green in `verify-report.md`'s addendum: `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, `npm run lint`, `npm run check` (216 files, 0 errors), `npm run test` (109 tests), `npm run build`.

**One WARNING was explicitly acknowledged and left open by the orchestrator, not silently dropped**: the `read_only_audit.rs` `cache.rs` literal-path-marker exemption checks a fixed five-substring list (`C:\`, `C:/`, `/home/`, `/Users/`, `/etc/`) and would not catch every conceivable literal-path construction (e.g. a bare `D:\` or an unquoted relative literal that never touches `app_data_dir()`). The test's own `static_proof_is_limited: true` field already says so in code; broadening the marker list was judged to add confidence without changing the fact that it remains a heuristic, not a proof. Carried forward as a known limitation below.

## Two User-Reported Defects, Found After Verify, Fixed Before Archive

Both are recorded here because they materially affect the feature's real-world reliability and are not visible from `tasks.md`'s checkbox state alone.

### Defect 1 — freshness stuck pending after toggling the check off and back on

**Symptom**: unchecking "Check for newer versions" and re-checking it left every client card stuck on the pending copy until the user navigated away and back.

**Root cause**: the lookup ran exactly once, in the component's initialization block; `toggleEnabled`'s disable path cleared state but nothing re-triggered the lookup on re-enable. Leaving/returning to the page destroyed and remounted the component, masking the bug by accident.

**Fix**: the lookup was extracted into a `loadFreshness()` function called both from init and from `toggleEnabled` when the setting is switched back on.

**A second, adjacent defect was fixed alongside it, not itself reported**: every lookup now carries a monotonically incrementing token, and a response whose token is stale at resolution time is discarded. Without this, disabling the check *while a request was in flight* would let a late response paint a verdict after the user had already opted out — the same class of bug, in the direction that matters most for the privacy posture this feature is built around. Two RED-first regression tests pin both: `re-runs the check when the setting is switched back on, instead of staying pending` (the user's exact report) and `ignores an in-flight response that lands after the check is switched off` (the token guard).

### Defect 2 — Codex could never be validated

**Symptom**: the Codex card always reported "could not be validated", even though `openai/codex` publishes GitHub releases normally.

**Root cause — the implementation's, not the upstream's.** `MAX_RESPONSE_BYTES` was a single shared 256 KiB ceiling applied to every upstream's response before parsing. Measured against the real endpoints: npm `opencode-ai` (2,042 B) and npm `@anthropic-ai/claude-code` (3,301 B) both fit comfortably, but GitHub's `openai/codex` `releases/latest` response is **272,440 bytes** — it embeds the release's entire 160-entry asset array, even though the only fields this code reads (`name`, `tag_name`) sit before byte ~1,715. The shared ceiling silently rejected the response as oversize, mapped to `Unavailable`, and surfaced as a permanent `Unknown` — a confidently wrong-looking failure with a completely mundane cause.

**Fix**: the ceiling became per-upstream-kind — npm 64 KiB (still ~20x its real payload), GitHub 4 MiB (~15x its real payload). A dedicated regression test builds a >256 KiB fixture with a 160-entry asset array and asserts the version still resolves, and asserts its own fixture exceeds the *old* ceiling so it cannot silently stop being a regression test.

**The systemic finding, which matters more than the bug itself**: an existing test (`freshness_live_upstream_endpoints_still_match_the_documented_shape`) already exercised the real Codex endpoint and would have caught this — but it is correctly `#[ignore]`d under CA-17, since core/app tests must never touch the network in CI, so it never ran automatically. This class of defect — upstream payload shape, size, or schema drift — has **no automated detector at all**, by design, and the mitigation is procedural rather than mechanical: `cargo test -p vertice-app --lib -- --ignored freshness_live` must be run as a manual release step before every release and after any change to the upstream table or parsers. This is carried into "Known Limitations" below as an operational obligation, not merely a historical note.

**A related, un-fixed weakness worth naming**: the `reason` string carried in the report (e.g. "GitHub response exceeds the size ceiling") was never surfaced in the UI, so this diagnosable failure looked to the user like an inexplicable one. Out of scope for this defect fix; carried forward below.

Both defects were fixed with RED-first regression tests and all quality gates re-run green afterward (`cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace --locked` — 42 tests in `vertice-app` after the second fix, `cargo deny check bans licenses`, `npm run check` 216 files/0 errors, `npm run test` 109 tests, and the live opt-in test passing against the real registries).

## Out-Of-Scope-Check On The Merged Specs

- **No out-of-scope PoC feature introduced.** Grepped the eight delta specs and the merged output for MCP servers, `Project`/`Local` scope, or write operations — none present, consistent with `verify-report.md`'s independent confirmation (core purity, `deny.toml` containment, and the single sanctioned cache write, all independently re-verified in that report).
- **`skill-scanner`, `agent-scanner`, `opencode-agent-scanner`, `codex-agent-scanner`, `frontmatter-reader`, `duplicate-consolidation`, `ci-quality-gates`** — confirmed untouched, matching the proposal's "Explicitly NOT Modified" list (as corrected by design §2/§16, which added `client-installation-detector` to the Modified list and is reflected in this archive's merges above).
- **No update affordance, no telemetry, no scan-blocking freshness step** — confirmed absent from the merged specs and from the delivered code, matching the proposal's Out of Scope section.

## Known Limitations Recorded At Archive Time

1. **`native-tls`'s Linux CI leg remains unverified.** Verified locally on Windows only throughout every slice; the Linux leg needs system OpenSSL headers for `native-tls`/`openssl-sys` and has not been confirmed in this environment. Must be confirmed on the first CI run before this dependency choice is treated as fully settled across all three platform legs.
2. **The freshness badge has never been observed in the running Tauri desktop app.** Its four states are covered by six passing component tests and a Vite-dev-server browser session where the IPC calls fail and degrade cleanly (confirming the degradation path, not the live badge) — but the badge itself needs a real `freshness` IPC response, which only the packaged/dev Tauri runtime can produce. `apply-progress.md` records this limitation plainly and recommends confirming visually with `npx --prefix frontend tauri dev` before release.
3. **Upstream payload shape/size/schema drift has no automated detector.** By CA-17's own design, the one test that would catch it (`freshness_live_upstream_endpoints_still_match_the_documented_shape`) is `#[ignore]`d and never runs in CI. `cargo test -p vertice-app --lib -- --ignored freshness_live` is now a documented **release-step obligation**, not optional housekeeping — run it before every release and after any change to the upstream table or the response parsers.
4. **The `read_only_audit.rs` `cache.rs` literal-path exemption is grep-proven against five substring markers, not machine-proven.** Explicitly acknowledged and left as a known heuristic rather than broadened (see "Verify-Report Resolution" above); the test's own `static_proof_is_limited: true` field states this in code.
5. **A freshness verdict's `reason` string is carried in the report but not surfaced in the UI.** A diagnosable failure (e.g. an oversize-response rejection) currently looks to the user like an inexplicable "Unknown". Not fixed in this change; a badge tooltip or diagnostics row carrying `reason` is a natural follow-up.
6. **Pre-existing, unworsened**: the `inventory-ui` spec/code drift where `clientPresence` renders on two routes (`ScanPage.svelte`'s supported-clients table and the separate `ClientsPage.svelte` this change's badge was added to) while the living spec documents rendering it on one. `verify-report.md` confirms this drift is untouched and unworsened by this change (`ScanPage.svelte:110` still keys on `record.label`, exactly as before) and it remains tracked separately, out of scope for this archive.
7. **`fetch_reference`'s size ceiling is enforced after the full response body is downloaded into memory** (`response.bytes().await`), not via a streamed byte-limit. Not a spec violation — the ceiling is checked before parsing, matching spec text literally — but flagged in `verify-report.md` as a hardening opportunity if this ever becomes a real-world attack surface (SUGGESTION, non-blocking).

## Task Completion Gate

`tasks.md` shows 51 of 51 checked task lines `[x]`, with **one exception, task 3.9, marked `[~]` (partial)**, carried into this archive with its original stale text intact: "cargo deny check bans licenses could not be executed here." This is documented as **stale, not incomplete**: `apply-progress.md`'s own later "Orchestrator gate verification" section proves `cargo deny check bans licenses` was subsequently run in the same apply run and passed (`bans ok, licenses ok` — the earlier "not found" result was an environment `PATH` issue, not a real gate failure), and `verify-report.md` independently re-ran the same command in its own session with the same passing result. `tasks.md` itself was never edited to flip the checkbox, which `verify-report.md` correctly flagged as WARNING #3 (a documentation-fidelity gap, not a functional one).

**Archive-time decision on this gap**: per the Task Completion Gate rule in `skills/sdd-archive/SKILL.md`, an unchecked/partial task blocks archive unless it is a stale checkbox that `apply-progress.md`/`verify-report.md` together prove is complete. That proof exists here, independently, in two separate documents (the apply run's own later correction, and the verify agent's independent re-run) — this is exceptional mechanical reconciliation with documented proof, not a routine override, and it is recorded here per the skill's requirement to state the exact reason. No edit was made to `tasks.md`'s checkbox itself during this archive step, consistent with the instruction to preserve every archived artifact's content faithfully rather than "clean it up"; the `[~]` mark and its stale prose are carried into the archive exactly as they existed at verify time, with this report serving as the authoritative record of why the underlying gate is genuinely green.

## CRITICAL Issues Check

`verify-report.md`'s original verdict recorded **3 CRITICAL** issues; per policy this would ordinarily block archive outright. All three were independently confirmed and closed by the orchestrator before this archive step began, with the closure recorded in `verify-report.md`'s own addendum (not a separate document, so the resolution travels with the finding it resolves) and all eight quality gates re-run green afterward. This archive proceeds with **0 CRITICAL issues open**, consistent with the strict "NEVER archive with open CRITICAL issues" policy — none are open.

## Tool-Access Limitation — Read This Before Trusting The "Move"

This execution had access to only four tools: `Read`, `Edit`, `Write`, `Glob`. **No shell/Bash tool and no filesystem-move/delete tool were available in this session**, exactly as the two prior archive cycles (`add-codex-client-support`, `report-client-presence-as-status`, both 2026-08-23) recorded under the same constraint. Consequently:

- **No checksum command (`md5sum`/equivalent) could be run.** Every one of the 14 archived files (`exploration.md`, `proposal.md`, `design.md`, `tasks.md`, `apply-progress.md`, `verify-report.md`, and the 8 spec files under `specs/{domain}/spec.md`) was reconstructed by reading the source file in full via the `Read` tool (which returns line-numbered content) and reproducing it verbatim via `Write`, stripping only the line-number/tab prefix `Read` adds for display. Every file was read in a single `Read` call each — none required `offset`/`limit` truncation — so no partial-read reconstruction risk exists. This is **careful manual transcription, not an independently verified byte-for-byte filesystem copy**, reported plainly per the task's explicit instruction rather than claimed as a checksum match that was never produced.
- **The original `openspec/changes/add-client-version-freshness/` folder was NOT deleted**, because no delete/move tool was available. This archive step created the full copy at `openspec/changes/archive/2026-08-24-add-client-version-freshness/` (all 14 pre-existing artifacts, transcribed as above; no `state.yaml` existed in the source to carry over, and none was authored, since the openspec convention marks it optional), but the source folder still exists at its original active-changes path.
- **The orchestrator (or a follow-up step with shell/git access) MUST:**
  1. Run a checksum comparison (or `git diff --no-index`) against the 14 pre-existing source files and their archived counterparts to confirm byte-for-byte identity beyond this manual-transcription attestation.
  2. Delete `openspec/changes/add-client-version-freshness/` once that verification passes, so the change is no longer listed as active — the skill's Step 4 checklist item "Active changes directory no longer has this change" is **not yet satisfied**.

This is reported as a **risk**, not silently glossed over: the merge into the eight main specs (via `Edit`, which performs exact string replacement rather than regeneration, and is therefore low-risk for the fidelity concern the task raised) is complete and mechanically sound — every merge was performed against content freshly read from the living spec in the same tool session, with two explicit, auditable preservation judgment calls recorded above. The archive-folder copy is a careful verbatim transcription of content read in full, unmodified, in the same tool session. But it is not a filesystem-level `mv`, and the removal of the active folder needs completion by an agent with shell/delete access.

## Artifact Traceability

| Artifact | Archived Path |
|---|---|
| Exploration | `openspec/changes/archive/2026-08-24-add-client-version-freshness/exploration.md` |
| Proposal | `openspec/changes/archive/2026-08-24-add-client-version-freshness/proposal.md` |
| Design | `openspec/changes/archive/2026-08-24-add-client-version-freshness/design.md` |
| Tasks | `openspec/changes/archive/2026-08-24-add-client-version-freshness/tasks.md` |
| Apply Progress | `openspec/changes/archive/2026-08-24-add-client-version-freshness/apply-progress.md` |
| Verify Report (with orchestrator addendum) | `openspec/changes/archive/2026-08-24-add-client-version-freshness/verify-report.md` |
| Delta/new: component-freshness | `openspec/changes/archive/2026-08-24-add-client-version-freshness/specs/component-freshness/spec.md` |
| Delta: domain-model | `openspec/changes/archive/2026-08-24-add-client-version-freshness/specs/domain-model/spec.md` |
| Delta: workspace-architecture | `openspec/changes/archive/2026-08-24-add-client-version-freshness/specs/workspace-architecture/spec.md` |
| Delta: desktop-shell | `openspec/changes/archive/2026-08-24-add-client-version-freshness/specs/desktop-shell/spec.md` |
| Delta: client-installation-detector | `openspec/changes/archive/2026-08-24-add-client-version-freshness/specs/client-installation-detector/spec.md` |
| Delta: inventory-ui | `openspec/changes/archive/2026-08-24-add-client-version-freshness/specs/inventory-ui/spec.md` |
| Delta: frontend-i18n | `openspec/changes/archive/2026-08-24-add-client-version-freshness/specs/frontend-i18n/spec.md` |
| Delta: scan-orchestration | `openspec/changes/archive/2026-08-24-add-client-version-freshness/specs/scan-orchestration/spec.md` |

## Source Of Truth Updated

The following specs now reflect the new behavior:

- `openspec/specs/component-freshness/spec.md` (new)
- `openspec/specs/domain-model/spec.md`
- `openspec/specs/workspace-architecture/spec.md`
- `openspec/specs/desktop-shell/spec.md`
- `openspec/specs/client-installation-detector/spec.md`
- `openspec/specs/inventory-ui/spec.md`
- `openspec/specs/frontend-i18n/spec.md`
- `openspec/specs/scan-orchestration/spec.md`

## Result

Status: partial
Executive summary: The eight spec merges into `openspec/specs/` are complete and content-verified (including two preserved-scenario judgment calls recorded above), all three verify-report CRITICAL findings were confirmed closed with gates re-run green, both user-reported post-verify defects are recorded with their fixes, and a full 14-artifact archive-folder copy was created at `openspec/changes/archive/2026-08-24-add-client-version-freshness/` — but byte-identical checksum verification and deletion of the original active-change folder could not be completed in this session due to the absence of a shell/checksum/delete tool, and are flagged as required follow-up, consistent with the same limitation recorded in both prior archive cycles.
Next recommended: none (SDD content work — spec merge, CRITICAL-finding verification, and change closure — is complete) — but the orchestrator must independently run a checksum/diff verification and delete `openspec/changes/add-client-version-freshness/` before treating archival as filesystem-complete.
Risks: (1) Archive-folder content fidelity was not independently checksum-verified — careful manual transcription via full-file Read + Write, not a guaranteed byte copy. (2) The original active-change folder still exists on disk and must be removed by an agent with delete/shell access. (3) `native-tls`'s Linux CI leg remains unverified locally (Windows only) — a real open risk for the first CI run on this feature. (4) The freshness badge has never been observed in the running Tauri desktop app — recommend a manual `npx --prefix frontend tauri dev` confirmation before release. (5) Upstream payload drift has no automated CI detector by design (CA-17); `cargo test -p vertice-app --lib -- --ignored freshness_live` is now a documented release-step obligation, not a nice-to-have. (6) `tasks.md`'s task 3.9 checkbox remains `[~]` with stale prose, intentionally left unedited per the archive fidelity instruction — this report is the authoritative record that the underlying gate is genuinely green.
