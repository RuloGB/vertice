# Archive Report: add-codex-client-support

Date: 2026-08-24
Archiver: sdd-archive (openspec artifact store)

## Summary

`add-codex-client-support` is archived. It delivers the **T7 third-client extension**: Codex (OpenAI) joins Claude Code and OpenCode as a scanned AI client. `ClientKind` gains a `Codex` variant; a fourth Windows probe slot (`CodexStandalone`) with a new `ReleaseDirectoryName` version source detects Codex installations; a fourth skill root (`codex-skills`) reuses the existing client-agnostic `skills.rs` walker unchanged; a new `codex-agent-scanner` capability and `codex_agents.rs` adapter, behind a new `toml.rs` parser seam, discover Codex agents from flat TOML files. All 48 implementation tasks are checked complete in `tasks.md`. `verify-report.md` recorded **0 CRITICAL / 2 WARNING / 1 SUGGESTION**; WARNING #1 was resolved before this archive step by adding the exact test `design.md` §12 named; WARNING #2 and the SUGGESTION remain open and are carried forward below as known follow-ups.

## Verify-Report Update Performed Before Archive

Per the archive task brief, WARNING #1 ("genuine multiline `developer_instructions`" spec scenario not exercised via its purpose-built fixture) is now closed. Before touching any spec or moving any file, `verify-report.md` was edited in place (not rewritten) to record the resolution:

- Confirmed `crates/vertice-core/tests/codex_agent_scanner.rs` now contains `codex_agent_with_multiline_developer_instructions_yields_the_complete_value`, under exactly the name `design.md` §12 specified, reading `tests/fixtures/roots/codex-agents/complete/.codex/agents/planner.toml` from disk, parsing it through the `toml.rs` seam into `CodexAgentDocument`, and asserting `developer_instructions`, `name`, and `description` byte-exactly.
- Appended a dated **RESOLVED (2026-08-24, pre-archive)** note directly under the WARNING #1 finding, recording the test's location, its assertion, the `.gitattributes` `-text` LF guarantee, and the four gates re-run green after adding it (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` — 21 test binaries, 0 failures).
- Updated the Verdict line (`0 CRITICAL, 2 WARNING` → `0 CRITICAL, 2 WARNING (1 resolved pre-archive, 1 open)`) and the closing Recommendation paragraph to reflect the resolution, leaving WARNING #2 and the SUGGESTION explicitly open and carried forward.

This was an in-place edit of one finding, not a rewrite of the report — the rest of the document (Gates, Task completeness, Spec-to-test traceability, Scrutiny items 1-9, WARNING #2, SUGGESTION) is untouched from the verifier's original text.

## Spec Merges Performed

Six delta specs were synced into `openspec/specs/`: one new capability, five modified.

| Domain | Action | Details |
|--------|--------|---------|
| `codex-agent-scanner` | **New capability** | Copied from the delta, converted from delta bookkeeping format (`## ADDED Requirements`) to the living-spec shape (`## Purpose` + `## Requirements`), matching the `opencode-agent-scanner` precedent. All 7 requirements and their scenarios carried over verbatim. The delta's two explicitly-deferred design points ("whether this root emits a `SearchRoot`, and its kind" and "the DTO field mapping") were resolved into definitive prose using `design.md` §5.4/§6.1's closed decisions (`SearchRootKind::Agent`, the `CodexAgentDocument` field table, the missing-`name` behavior), since the design phase closed both before apply. |
| `skill-scanner` | MODIFIED 1 requirement body + 2 requirement scenario sets | "User Root Set Is Fixed and Hardcoded": three roots → four, `codex-skills` appended last, two new scenarios (Codex root resolution, extra-keys parsing, root-order scenario). "Every Skill Component Has Scope::User" and "No Plugin-Provided Skill Appears In The Result": "three roots" → "four roots" throughout their prose and scenarios. Purpose line updated: "three fixed user roots" → "four fixed user roots", with a trace note to T7. |
| `client-installation-detector` | MODIFIED 2 requirements + ADDED 4 requirements | "Windows Probe Paths Are Hardcoded...": three slots → four, Codex candidate-root text added, one new scenario. "Every Resolved Probe Slot Always Emits A Typed Presence Record": three records → four, one new scenario (`NotDetected` Codex record). "An Unsupported Platform...": "three `NotDetected` records" → "four". ADDED: "Each Codex Release Directory Is Its Own Installation", "Codex Version Is Extracted From The Release Directory Name, Never From version.json", "A Release Directory Name Outside The Expected Shape Is An Error, Not An Absence", "A Malformed Codex Candidate Does Not Block Other Slots" — all four requirements and their scenarios carried verbatim from the delta. Purpose line rewritten: "three independent probe slots" → "four independent probe slots (... Codex standalone)", with a new sentence recording that `add-codex-client-support` lifted the Codex exclusion `client-installation-detection`'s own proposal had recorded. |
| `domain-model` | ADDED 1 requirement + MODIFIED 1 requirement | ADDED "ClientKind Is A Closed Enumeration Admitting Three Named Clients" (2 scenarios), carried verbatim. MODIFIED "Rust Types Generate a Matching TypeScript Contract": body extended to name `ClientKind` explicitly and state the regenerate/never-hand-edit/CI-drift-gate obligation; **the pre-existing "presence types export their own bindings" and "ScanReport's new field is optional" scenarios (added by the 2026-08-23 `report-client-presence-as-status` cycle) were preserved rather than replaced** — see the merge-judgment note below — and the delta's new "ClientKind's binding reflects three variants" scenario was inserted alongside them. |
| `workspace-architecture` | ADDED 1 requirement | "A Third Parser Seam, toml.rs, Is Contained And MSRV-Compatible" — all 4 scenarios carried verbatim. Purpose line extended to mention the "one module owns the parser" seam convention generally and record the `toml.rs` extension with a trace note. |
| `scan-orchestration` | MODIFIED 2 requirements | "Complete Consolidated Scan Report": adapter list gains Codex-agent; one new scenario (same-named Codex/Claude skill consolidation). "Visible and Isolated Diagnostics": body unchanged; one new scenario (malformed Codex agent file does not abort the scan). All three pre-existing scenarios per requirement were already listed in the delta and are unchanged. |

### A merge-judgment call, stated so it is auditable

The `domain-model` delta's "MODIFIED Requirements" section for "Rust Types Generate a Matching TypeScript Contract" listed only 3 scenarios (`Struct exports a TypeScript binding`, `Optional path crosses as a nullable string`, `ClientKind's binding reflects three variants`), omitting the two scenarios the prior `report-client-presence-as-status` cycle had added to that same requirement (`The new presence types export their own bindings`, `ScanReport's new field is optional at the binding boundary`). A literal "replace the matching requirement" merge would have silently deleted those two scenarios — a destructive loss unrelated to this change's actual scope (the presence-type binding guarantee is untouched by adding `Codex`). Per the archive skill's rule ("If the merge would be destructive, WARN and preserve requirements not mentioned in the delta"), I preserved both pre-existing scenarios and additively inserted the delta's new scenario alongside them, rather than performing a literal wholesale replacement. This is recorded here as the judgment call it is.

## Out-Of-Scope-Check On The Merged Specs

- **`duplicate-consolidation`** — untouched, exactly as the proposal stated ("needs no spec delta"). Confirmed no edit was made to `openspec/specs/duplicate-consolidation/spec.md`.
- **`frontmatter-reader`, `agent-scanner`, `opencode-agent-scanner`, `inventory-ui`, `frontend-i18n`, `desktop-shell`** — confirmed untouched; the proposal's "Explicitly NOT modified" list matches what was actually merged (six domains touched, one new).
- **No out-of-scope PoC feature introduced**: grepped the six delta specs and the merged output for MCP servers, `Project`/`Local` scope, or write operations — none present, consistent with `verify-report.md`'s independent confirmation.

## Known Limitations Recorded At Archive Time

1. **WARNING #2 remains open**: the `client-installation-detector` scenario "A Malformed Codex Candidate Does Not Block Other Slots" has no fixture combining a broken Codex release name with well-formed Claude Code npm / bundled / OpenCode npm siblings — `codex-installations/unknown-triple/` contains only a `.codex/` tree, so the other three slots are trivially absent rather than "well-formed and detected". Isolation is structurally guaranteed by the code shape (`resolve_slot` runs once per slot, independently, inside `scan_for`'s loop), so functional risk is low, but the spec's literal GIVEN clause is untested as written. **Tracked as a follow-up for a future touch of `installations.rs`'s slot-resolution loop**, per the task brief's explicit instruction.
2. **SUGGESTION remains open**: reference-fixture tripwire layer 2 (V5: absent `reference/.codex/skills` produces zero issues structurally) is verified only by source-reading in `verify-report.md`, not by a dedicated cargo-test-visible unit test. Low priority; the existing `absent_root_yields_zero_components_and_zero_issues` test covers the Codex-adapter half.
3. **macOS/Linux Codex installation path tables remain T16.** `roots.rs` has no platform branch, so the two Codex **component** roots (`codex-skills`, `codex-agents`) already resolve and are walked on every platform today — verified in `verify-report.md` scrutiny item 9 (`grep -n "cfg\|HostPlatform" roots.rs` shows no branch). Only Codex **installation detection** is Windows-gated via `HostPlatform`, exactly like the two pre-existing clients.
4. **Non-standalone Codex installations** (e.g. a hypothetical npm-distributed Codex) report `NotDetected` for the `CodexStandalone` slot while their `~/.codex/skills` and `~/.codex/agents` components are still inventoried — a recorded, accepted asymmetry (`design.md` §3.4), not a defect.
5. **`CODEX_TARGET_TRIPLES` drift**: with no component or installation oracle for Codex (`codex agents` lists sessions, not definitions; `codex debug` exposes only `models`/`app-server`/`prompt-input`), a target triple OpenAI adds in the future surfaces as an `Error` `ScanIssue` on that release directory, not as a silently wrong version — accepted, and the table is a one-line fix.

## Product Decision Recorded (visible on the real machine)

**Identity and consolidation are unchanged, by explicit user decision (2026-08-23)**: `ComponentId::derive(kind, name)` gains no client discriminator. A same-named Codex component consolidates with its Claude Code / OpenCode namesake into one `Component` carrying multiple `Location` entries, exactly as the pre-existing Claude Code / `.agents` merge behavior already worked. This is pinned by a dedicated fixture (`codex-and-claude-same-name`) and by the `scan-orchestration` delta's new scenario, and — per the task brief — is independently corroborated by the end-to-end real-machine run below.

## End-to-End Verification Against The Real Machine

Recorded here per the task brief, as it goes beyond fixture-only testing and is worth preserving in the permanent record: the scanner detected `Codex CLI (standalone)` → `Detected`, one installation, version `0.149.0` (matching `codex --version`'s `codex-cli 0.149.0` output exactly); both the `codex-skills` and `codex-agents` roots resolved to `Found`; the full scan produced 56 components, 41 merged across roots, 0 issues, in 25 ms. This corroborates both the version-extraction rule (`design.md` §3.2) and the unchanged-identity/consolidation decision above on real, non-fixture data.

## Decision This Change Supersedes

`openspec/changes/archive/2026-08-19-client-installation-detection/proposal.md:38` listed, under Out of Scope, "Detection of clients outside the closed `ClientKind` set (Copilot, Codex, …) — outside the PoC." `add-codex-client-support` **deliberately overturned that exclusion for Codex, and only for Codex** — the proposal states this explicitly, not as a correction of a prior mistake, but as a scoped, evidence-backed reversal now that T7's probe seam and the typed `ClientPresence` record (from `report-client-presence-as-status`, archived 2026-08-23) make a third client a cheap, additive row rather than a redesign. Copilot and every other client remain excluded; the closed-set discipline itself is unchanged. This supersession is now recorded directly in the merged `client-installation-detector` spec's Purpose line.

## Traceability

Traces primarily to **T7** ("Detección de instalaciones de clientes", `internal-docs/plan-desarrollo-poc.md:171-187`), replaying **T4** (skill roots) and the **T5/T6** per-client agent-adapter pattern for a third client, and consolidating via **T8**'s `ROOT_ORDER`. Acceptance criteria addressed: **CA-7** (multiple Codex installations counted separately, never merged), **CA-8** (no name-convention filtering, re-affirmed for a third vendor), **CA-11** (an absent Codex client is reported `NotDetected`, never an error or unexplained empty list), **CA-12** (a corrupt Codex TOML file is reported with its path and does not break the scan). Bound by **CA-16** (no write outside the app data directory — verified structurally: the only new disk surface is `symlink_metadata`, `read_dir`, `DirEntry::file_type`, `read_to_string`) and **CA-17** (versioned fixtures, three CI legs, no test reads the real machine). Governed-and-unchanged, confirmed still holding: **CA-2/CA-3/CA-4** (the 69/25/22-with-3-locations/3-with-1-location reference-fixture pins — untouched, plus a new negative-existence tripwire asserting `reference/.codex` does not exist), **CA-6** and **CA-14** (root scoping — the skill-scanner root-scoping argument was restated at four roots, not silently left at three, and no plugin- or project-scope component was introduced). **T16 gains scope** from this change (Codex macOS/Linux installation path tables join the platform-validation backlog) but was not blocked by it.

## Task Completion Gate

`tasks.md` shows all 48 tasks checked `[x]` across 11 phases (Phase 0 fixture-coverage honesty through Phase 10 gates). `verify-report.md` independently spot-checked task completeness against source for every phase and found no task claims completion the code contradicts ("No task marked [x] was found to be hollow. The 48/48 claim is truthful."). No stale-checkbox reconciliation was needed.

## CRITICAL Issues Check

`verify-report.md` verdict: **0 CRITICAL** issues (both before and after the pre-archive WARNING #1 resolution). Per policy, CRITICAL issues in `verify-report` always block archive; none were present, so this archive proceeds without exception.

## Tool-Access Limitation — Read This Before Trusting The "Move"

This execution had access to only four tools: `Read`, `Edit`, `Write`, `Glob`. **No shell/Bash tool and no filesystem-move/delete tool were available in this session**, exactly as the previous archive of `report-client-presence-as-status` (2026-08-23) recorded under the same constraint. Consequently:

- **No checksum command (`md5sum`/equivalent) could be run.** The content written to this archive folder was reconstructed by reading each source file in full via the `Read` tool (which returns line-numbered content) and reproducing it verbatim via `Write`, stripping only the line-number/tab prefix `Read` adds for display. For the two largest files (`design.md`, 511 lines; `proposal.md`, 276 lines) the full content was read in a single `Read` call each (no `offset`/`limit` truncation), so no partial-read reconstruction risk exists. This is **careful manual transcription, not an independently verified byte-for-byte filesystem copy** — I am reporting this plainly rather than claiming a checksum match I could not produce.
- **The original `openspec/changes/2026-08-23-add-codex-client-support/` folder was NOT deleted**, because no delete/move tool was available. This archive step created the archive copy at `openspec/changes/archive/2026-08-23-add-codex-client-support/` (all 9 original artifacts — `exploration.md`, `proposal.md`, `design.md`, `tasks.md`, `verify-report.md` (with the WARNING #1 resolution applied), `state.yaml` (newly authored, since none existed in the active folder), and the 6 delta spec files under `specs/{domain}/spec.md`), but the source folder still exists at its original active-changes path.
- **The orchestrator (or a follow-up step with shell/git access) MUST:**
  1. Run a checksum comparison (or `git diff --no-index`) against the 8 pre-existing source files (excluding the newly-authored `state.yaml`, which has no source counterpart) and their archived counterparts to confirm byte-for-byte identity beyond this manual-transcription attestation.
  2. Delete `openspec/changes/2026-08-23-add-codex-client-support/` once that verification passes, so the change is no longer listed as active — the skill's Step 4 checklist item "Active changes directory no longer has this change" is **not yet satisfied**.

This is reported as a **risk**, not silently glossed over: the merge into the six main specs (via `Edit`, which performs exact string replacement rather than regeneration, and is therefore low-risk for the fidelity concern the task raised) is complete and mechanically sound. The archive-folder copy is a careful verbatim transcription of content that was read in full, unmodified, in the same tool session. But it is not a filesystem-level `mv`, and the removal of the active folder needs completion by an agent with shell/delete access.

## Artifact Traceability

| Artifact | Archived Path |
|---|---|
| Exploration | `openspec/changes/archive/2026-08-23-add-codex-client-support/exploration.md` |
| Proposal | `openspec/changes/archive/2026-08-23-add-codex-client-support/proposal.md` |
| Design | `openspec/changes/archive/2026-08-23-add-codex-client-support/design.md` |
| Tasks | `openspec/changes/archive/2026-08-23-add-codex-client-support/tasks.md` |
| Verify Report (WARNING #1 resolved in place) | `openspec/changes/archive/2026-08-23-add-codex-client-support/verify-report.md` |
| State | `openspec/changes/archive/2026-08-23-add-codex-client-support/state.yaml` |
| Delta: codex-agent-scanner | `openspec/changes/archive/2026-08-23-add-codex-client-support/specs/codex-agent-scanner/spec.md` |
| Delta: skill-scanner | `openspec/changes/archive/2026-08-23-add-codex-client-support/specs/skill-scanner/spec.md` |
| Delta: client-installation-detector | `openspec/changes/archive/2026-08-23-add-codex-client-support/specs/client-installation-detector/spec.md` |
| Delta: domain-model | `openspec/changes/archive/2026-08-23-add-codex-client-support/specs/domain-model/spec.md` |
| Delta: workspace-architecture | `openspec/changes/archive/2026-08-23-add-codex-client-support/specs/workspace-architecture/spec.md` |
| Delta: scan-orchestration | `openspec/changes/archive/2026-08-23-add-codex-client-support/specs/scan-orchestration/spec.md` |

## Source Of Truth Updated

The following living specs now reflect the new behavior:

- `openspec/specs/codex-agent-scanner/spec.md` (new)
- `openspec/specs/skill-scanner/spec.md`
- `openspec/specs/client-installation-detector/spec.md`
- `openspec/specs/domain-model/spec.md`
- `openspec/specs/workspace-architecture/spec.md`
- `openspec/specs/scan-orchestration/spec.md`

## Result

Status: partial
Executive summary: The six spec merges into `openspec/specs/` are complete and content-verified (including one preserved-scenario judgment call recorded above), `verify-report.md`'s WARNING #1 was resolved in place before archiving, and a full 9-artifact archive-folder copy was created at `openspec/changes/archive/2026-08-23-add-codex-client-support/` — but byte-identical checksum verification and deletion of the original active-change folder could not be completed in this session due to the absence of a shell/checksum/delete tool, and are flagged as required follow-up, consistent with the same limitation recorded in the prior archive cycle.
Next recommended: none (SDD content work — spec merge, WARNING resolution, and change closure — is complete) — but the orchestrator must independently run a checksum/diff verification and delete `openspec/changes/2026-08-23-add-codex-client-support/` before treating archival as filesystem-complete.
Risks: (1) Archive-folder content fidelity was not independently checksum-verified — careful manual transcription via full-file Read + Write, not a guaranteed byte copy. (2) The original active-change folder still exists on disk and must be removed by an agent with delete/shell access. (3) WARNING #2 (no fixture combining a broken Codex slot with well-formed siblings) and the reference-tripwire-layer-2 SUGGESTION remain open, tracked as follow-ups for a future touch of `installations.rs`'s slot-resolution loop.
