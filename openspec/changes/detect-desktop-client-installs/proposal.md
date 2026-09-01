# Proposal: Detect desktop client installations

## Intent

On a Windows machine where Claude Code is installed as the bundled desktop runtime and
OpenCode as a desktop app (neither via npm), the Clients page reports both as **not
detected**. Exploration confirmed two independent defects:

- **H1 — display defect.** `frontend/src/lib/pages/ClientsPage.svelte:50` selects a
  product's record with `Array.find` over its slot list. Records are always emitted in
  probe-table order and every slot always emits a record, so the Claude Code card always
  renders the `claudeCodeNpm` record even when it is `NotDetected` and `claudeCodeBundled`
  is `Detected`. The log proves the bundled slot resolved (repeated "no established
  queryable upstream" lines for it, and no "not detected" line for it). Detection is
  correct; only the card lies.
- **H2 — coverage gap.** No `ClientInstallSlot` variant probes a non-npm OpenCode desktop
  install, so the "OpenCode not detected" log line is truthful. This cannot be fixed in the
  frontend.

The change is worth doing now because the product's central claim — an accurate inventory
of installed AI clients — is false on a real user machine, and the inventory-ui spec is
silent on the aggregation rule that let H1 ship.

## Outcomes

After the fix, on the affected machine:

| Card | Detected badge | Version string | Freshness badge |
| --- | --- | --- | --- |
| Claude Code | Detected (from the bundled record) | bundled install version(s) | evaluated against the bundled record; `Unknown` (no upstream, by design) |
| OpenCode | Detected (from the new desktop record) | desktop version from `app.asar`, or `scan.clientVersionUnavailable` if extraction degrades | `Unknown` (upstream `None`) |
| Codex | unchanged | unchanged | unchanged |

## Scope

### In Scope

1. **H1 selection rule (Option A, chosen).** `presenceFor` MUST return the **first
   `Detected` record among the product's slots, in record order, and fall back to the first
   record of the group when none is `Detected`.**
   Justification: OpenCode becomes a two-slot product in this very change, so the rule must
   hold for any group of N slots — Option A does, with no new copy, no new badge semantics
   and no aggregate status vocabulary. Option B (one card per slot) duplicates the "Claude
   Code" product name and inflates i18n. Option C (aggregate the group) is the better
   end-state UX but needs a multi-installation badge rule and aggregate status copy; it is
   deferred, not rejected.
   Impact: the detected badge, version string and freshness badge are ALL read from the same
   selected record, so they can never disagree. Known limitation, accepted: when two slots
   of one product are both `Detected`, the card shows only the first; the always-visible
   supported-clients table on the scan route still renders one row per record, so no
   information is lost product-wide.
2. **H2 new slot `OpenCodeDesktop`.**
   - `label()` copy: `"OpenCode (desktop app)"`.
   - Hardcoded probe path: `<home>/AppData/Local/Programs/@opencode-aidesktop` (literal
     folder name, leading `@` included) — `home` plus fixed segments only, no `dirs` crate,
     no environment read, per the standing probe-path requirement.
   - Version source: the `app.asar` header. Read the header length from the archive's
     fixed-offset binary prefix; refuse to parse a header exceeding a hardcoded ceiling;
     otherwise parse the header JSON through the existing `jsonc.rs` seam, resolve
     `package.json`'s offset and size, read exactly those bytes and extract `version`.
   - Degradation: EVERY failure mode — oversized header, absent entry, malformed JSON,
     missing or empty `version`, unreadable file — degrades to `Detected` with empty
     `installations`, a state the model already expresses. Detection MUST NOT depend on
     version extraction succeeding, and extraction MUST NOT panic or fail the scan.
   - `upstream_for` arm returns `None`, exactly like `ClaudeCodeBundled`. `app-update.yml`
     declares `anomalyco/opencode` GitHub releases as the app's channel, but it is unverified
     whether the desktop app and the CLI share a release-tag namespace, so a shared repo
     could yield false "outdated" verdicts. Deferred, logged as pending.
   - Frontend group membership: added to the `openCode` group alongside `openCodeNpm`.
3. **Performance budget for the asar read.** The header enumerates every archived file with
   a SHA256 integrity block, against a full scan that currently takes 12–45 ms. This change
   commits to a named ceiling for the header (a maximum header byte size) and a named ceiling
   for total time spent in desktop version extraction per scan; the exact figures are a
   **decision to be fixed in design**. When either ceiling is exceeded, extraction is
   abandoned and the slot degrades to `Detected` with empty `installations` — same path as
   every other failure mode.

### Out of Scope

- **Anthropic's native standalone Claude Code installer** (`install.ps1` / WinGet). A real,
  separate coverage gap; H1 fully explains the reported Claude Code symptom without it. To be
  logged in `internal-docs/pendientes-desarrollo.md` and scoped separately.
- **macOS / Linux path tables.** Windows-only, as today.
- **The scan-route supported-clients table.** It already renders one row per record and is
  correct; unchanged.
- **A real freshness upstream for `OpenCodeDesktop`.** Logged as pending.
- **Option C aggregation** of multiple detected slots into one card.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `client-installation-detector`: new hardcoded probe path for the OpenCode desktop slot; new
  version source (`app.asar` header, via the `jsonc.rs` seam) with its degradation rules and
  performance ceiling; the hardcoded "exactly four records" count in *Every Resolved Probe
  Slot Always Emits A Typed Presence Record* becomes five.
- `inventory-ui`: currently **silent** on `ClientsPage`'s per-product slot-to-card
  aggregation — that silence is exactly why H1 shipped. Add a requirement fixing the
  selection rule when a group's slots disagree in status, for any N.
- `domain-model`: new `ClientInstallSlot::OpenCodeDesktop` variant (sanctioned evolution —
  the enum's doc states growth follows platform/adapter growth).

No delta needed for `component-freshness` (the repeated "no queryable upstream" line is
designed behavior), `application-logging` (the log did its job) or `scan-orchestration` (not
implicated).

## Affected Areas

| Area | Impact | Description |
| --- | --- | --- |
| `crates/vertice-core/src/model/slot.rs` | Modified | New variant + `label()` arm |
| `crates/vertice-core/src/installations.rs` | Modified | `client()`/`version_source()` arms, probe path, asar resolver |
| `crates/vertice-app/src/freshness/upstream.rs` | Modified | New arm returning `None` |
| `frontend/src/lib/bindings/ClientInstallSlot.ts` | Regenerated | `ts_rs`; never hand-edited |
| `frontend/src/lib/pages/ClientsPage.svelte` | Modified | Selection rule + OpenCode group membership |
| `crates/vertice-core/tests/client_installations.rs` + fixtures | Modified | Record-count pin, new slot fixtures |
| `crates/vertice-core/tests/model_contract.rs` | Modified | Pin new label |
| `frontend/src/lib/pages/ClientsPage.test.ts` | Modified | Failing test first: npm NotDetected + bundled Detected |
| `internal-docs/pendientes-desarrollo.md` | Modified | Log the standalone-installer gap and the deferred upstream |

## Risks

| Risk | Likelihood | Mitigation |
| --- | --- | --- |
| CA-16 read-only invariant | Low | Both fixes are additive read-only probes and byte reads; no `File::create`/write path introduced |
| `model/` import allow-list violated by attaching probe logic to the enum | Low | The new variant stays plain data; all path, existence and asar logic lives in `installations.rs`, outside `model/` |
| `ts_rs` binding drift (CI fails on diff) | Med | Regenerate via `cargo test -p vertice-core`, never hand-edit `frontend/src/bindings/` |
| `@opencode-aidesktop` folder name changes | Med | It is an electron-builder artifact derived from the npm scope; a future OpenCode release could rename it, silently killing the probe. Accepted: degradation is `NotDetected`, never an error |
| asar header parse cost blows the scan budget | Med | Hardcoded header-size and time ceilings; exceeding either abandons extraction and degrades to `Detected` with empty `installations` |
| The "exactly four records" count is pinned in the spec and in test files | Med | Move all pins in lockstep within the same change |
| Windows-only path table | Low | Only `windows_install_probes` is touched; macOS/Linux untouched |

## Rollback Plan

Three-layer, revertible as one commit range. Reverting restores the four-variant enum,
regenerates `ClientInstallSlot.ts` back to four variants (core and frontend MUST revert
together, or `npm run check` fails on the binding), restores `presenceFor`'s `find`, and
drops the `upstream_for` arm and the new fixtures. No persisted state, no migration, no user
data touched — the scan is recomputed in memory on every run.

## Dependencies

- None external. Ground truth for the probe path and the asar layout is already confirmed on
  the affected machine (exploration §2b).

## Success Criteria

- [ ] A frontend test with `claudeCodeNpm: NotDetected` + `claudeCodeBundled: Detected` shows
      the Claude Code card as detected, with the bundled version and its freshness badge.
- [ ] The selection rule is proven for a group of N slots, not just two.
- [ ] A fixture home with an `@opencode-aidesktop` install yields a `Detected`
      `OpenCodeDesktop` record with the version extracted from `app.asar`.
- [ ] Fixtures for oversized header, absent `package.json` entry, malformed JSON, missing
      `version` and unreadable file each yield `Detected` with empty `installations`, zero
      panics and a completed scan.
- [ ] `scan_for` emits exactly five records on Windows, in deterministic order; the spec and
      every test pin agree.
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace --locked`, `cargo deny check bans licenses`, and
      `npm run lint && npm run check && npm run test && npm run build` all pass, with no
      binding drift.
- [ ] The standalone-installer gap and the deferred OpenCode upstream are recorded in
      `internal-docs/pendientes-desarrollo.md`.

## Proposal question round

Execution mode is automatic, so no interactive round was run. The following assumptions were
made and are open to correction before specs:

1. Showing only the first `Detected` slot per product (Option A) is acceptable as the
   shipping behavior for a machine carrying BOTH an npm and a desktop install of the same
   client, given the scan-route table still lists every record.
2. `"OpenCode (desktop app)"` is the intended user-facing label copy for the new slot.
3. A desktop OpenCode install whose version cannot be read is better shown as *detected,
   version unavailable* than hidden — consistent with the existing npm-slot behavior.
4. The performance ceilings are a design-phase decision; the proposal only commits to their
   existence and to the degradation path when they are exceeded.
