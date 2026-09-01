# Exploration — Desktop-app client installations are not surfaced

Status: complete. Next recommended phase: `sdd-propose`.

## 0. Reported problem

Vertice installed on a second Windows machine (home `C:\Users\raul_`) with Claude Code
as a desktop app and OpenCode as a desktop app — neither CLI-installed via npm. The
Clients page reported neither client as detected.

Evidence supplied: the application log.

- `search root not found` fires only for `codex-skills`, `codex-agents`, `codex-mcp`.
  The Claude/agents/OpenCode skill and agent roots produced no warning, so they exist
  and were scanned.
- `client not detected` fires for exactly three slots: `Claude Code CLI (npm)`,
  `OpenCode (npm)`, `Codex CLI (standalone)`.
- There is **no** `client not detected` line for `Claude Code (bundled in Claude Desktop)`.
- `freshness unknown: Claude Code (bundled in Claude Desktop) has no established
  queryable upstream` fires repeatedly.

## 1. Current architecture

Core detection lives in `crates/vertice-core/src/installations.rs`:

- `HostPlatform::current()` is the only compile-target branch; non-Windows yields
  `client_presence: None`.
- `windows_install_probes` builds a flat probe list in fixed slot order:
  `ClaudeCodeNpm`, `ClaudeCodeBundled` (1..N candidates), `OpenCodeNpm`,
  `CodexStandalone`.
- `group_probes_by_slot` preserves that order; `resolve_slot` dispatches on
  `version_source()` to `resolve_npm_slot` (package.json via the `jsonc.rs` seam),
  `resolve_bundled_slot` (MSIX `Claude_*` packages plus the legacy Roaming path,
  version from directory name), or `resolve_codex_slot` (release directory name minus
  a known target triple).
- `scan_for` emits one `ClientPresence` per slot; `flatten_presence` is the only
  producer of `InstallationScan.installations`.

Model: `ClientInstallSlot` (`crates/vertice-core/src/model/slot.rs`) is a closed
four-variant enum exported to TypeScript through `ts_rs`. `ClientPresence`
(`crates/vertice-core/src/model/presence.rs`) carries `slot`, `label`, `probed_paths`,
`status`, `installations`. `Detected` means "a candidate root exists", not "a version
was extracted" — a `Detected` record can legitimately have empty `installations`.

Frontend: `frontend/src/lib/pages/ClientsPage.svelte` groups slots per product
(`claudeCodeNpm` + `claudeCodeBundled` → Claude Code; `openCodeNpm` → OpenCode;
`codexStandalone` → Codex) and selects a single record per group with
`presenceFor = (slots) => (report?.clientPresence ?? []).find((r) => slots.includes(r.slot))`.

A second consumer exists and does **not** have this defect: the always-visible
supported-clients table on the scan route renders one row per `clientPresence` record.

## 2. Root cause — two independent defects

### H1 — Claude Code: frontend selection defect (confirmed)

`Array.prototype.find` returns the first match. Records are always emitted in
probe-table order and every slot always emits a record regardless of status, so
`presenceFor(["claudeCodeNpm", "claudeCodeBundled"])` **always** returns the npm
record. The bundled record is structurally unreachable from that call site. When npm
is `NotDetected` and bundled is `Detected`, the card shows "not detected", shows
`scan.clientVersionUnavailable` instead of the bundled versions, and never evaluates
`badgeFor` against the bundled record.

The log proves the bundled slot *was* detected on that machine:
`subjects_from_presence` (`crates/vertice-app/src/freshness/mod.rs`) builds one
freshness subject per resolved `ClientInstallation`, and `upstream_for` returns `None`
for `ClaudeCodeBundled` by design, producing the repeated "no established queryable
upstream" line. That line cannot fire unless the bundled slot resolved with at least
one installation. Combined with the absence of a bundled "not detected" line, the
Claude Code half of the report is a pure UI defect, not a detection failure.

### H2 — OpenCode: detection coverage gap (confirmed)

`ClientInstallSlot` has exactly one OpenCode variant, `OpenCodeNpm`, probing
`<home>/AppData/Roaming/npm/node_modules/opencode-ai`. No slot probes a desktop or
standalone OpenCode install, and the frontend group lists only `openCodeNpm`, so there
is nothing to mis-select — the log is telling the truth. Fixing this requires a new
slot and resolver in core; no frontend-only fix is possible.

Public sources indicate OpenCode ships a Windows desktop application distinct from the
`opencode-ai` npm CLI, with candidate locations under `%LOCALAPPDATA%\opencode\` and
`%LOCALAPPDATA%\Programs\`. **These paths are unverified against a primary source** and
must be confirmed against the real machine before any probe is written. It is also
unclear whether the desktop build exposes a version-bearing manifest or directory name,
which decides whether a new `VersionSource` variant is needed. The OpenCode GitHub org
appears under `anomalyco/opencode` rather than `sst/opencode`; verify before wiring any
`GitHubReleases` freshness upstream.

### H3 — Claude Code native standalone installer (new finding, out of scope)

Anthropic also ships a native Windows installer (`irm https://claude.ai/install.ps1 | iex`,
also WinGet) that is neither npm nor the MSIX Claude Desktop bundle. Its bootstrap
script stages a helper binary under `%USERPROFILE%\.claude\downloads` and delegates the
real install, so the final install root is not visible publicly. No slot probes it.

H1 fully explains the reported Claude Code symptom, so H3 is **not** required to close
this bug. Recommendation: record it in `internal-docs/pendientes-desarrollo.md` and
scope it separately rather than inflating this change.

## 2b. Ground truth confirmed on the affected machine (home `C:\Users\raul_`)

- `AppData\Local\Packages\Claude_pzs8sxrjxfjjc` exists → the `ClaudeCodeBundled` slot
  resolved `Detected`. H1 is confirmed as the sole cause of the Claude Code symptom; no
  new Claude Code probe is needed for this change.
- OpenCode desktop install root: `<home>/AppData/Local/Programs/@opencode-aidesktop`
  (literal folder name, leading `@` included — an electron-builder artifact derived from
  the npm scope, so it could change in a future OpenCode release).
- It is an Electron app. Contents: `OpenCode.exe`, `Uninstall OpenCode.exe`, Chromium
  runtime files, `locales/`, and `resources/` containing `app.asar` (143 MB),
  `app.asar.unpacked/`, `elevate.exe` and `app-update.yml`.
- The only loose `package.json` files belong to native third-party dependencies under
  `app.asar.unpacked/node_modules/`, never to OpenCode itself.
- `resources/app-update.yml` (116 bytes) carries `provider: github`,
  `owner: anomalyco`, `repo: opencode`, `channel: latest`,
  `updaterCacheDirName: '@opencode-aidesktop-updater'`. **No version field.** It does
  confirm the org rename this document flagged as unverified: the repository is
  `anomalyco/opencode`, not `sst/opencode`.
- `%APPDATA%\OpenCode` / `%APPDATA%\opencode` do not exist.

### Version source decision

The first 1200 bytes of `app.asar` confirm the archive layout: a 16-byte binary pickle
prefix followed by a plain JSON header (`{"files":{"node_modules":{...`). OpenCode's own
`package.json`, carrying `version`, lives inside that archive, so the version IS
reachable with `std::fs` alone — no new crate, no registry read, no PE parsing, and
therefore no violation of the `client-installation-detector` rule against OS-convention
or environment reads.

The cost is real and must be bounded: the header enumerates every archived file with a
SHA256 integrity block, and JSON cannot be seeked by key without parsing, so the whole
header must be parsed to locate `package.json`'s offset. A full scan currently takes
12-45 ms.

Chosen approach: read the header length from the archive's fixed-offset prefix, refuse
to parse it if it exceeds a hardcoded ceiling, otherwise parse it through the existing
`jsonc.rs` seam, resolve `package.json`'s offset and size, read exactly those bytes and
extract `version`. Any failure at any step — oversized header, absent entry, malformed
JSON, missing or empty `version` — degrades to `Detected` with empty `installations`,
which is already a legal state in the model (`resolve_npm_slot` produces it for a
present-but-broken directory). Detection never depends on version extraction succeeding.

Freshness upstream for this slot stays `None` for now. `app-update.yml` declares the
app's own update channel as `anomalyco/opencode` GitHub releases, but it is unverified
whether the desktop app and the CLI share a release tag namespace; comparing the desktop
version against a shared repository's latest release could produce false "outdated"
verdicts. Wiring a real upstream is deferred and logged as pending.

## 3. Governing specs

- `client-installation-detector` — governs H2. "Windows Probe Paths Are Hardcoded,
  Never OS-Convention-Derived" constrains any new probe (no `dirs` crate, no env read).
  "Every Resolved Probe Slot Always Emits A Typed Presence Record" needs its "exactly
  four records" count updated when a slot is added.
- `domain-model` — `ClientInstallSlot`'s doc states growth follows platform/adapter
  growth; adding a slot is sanctioned evolution, not a violation.
- `inventory-ui` — the supported-clients table requirement is not violated (it renders
  per record). The spec is **silent** on `ClientsPage`'s per-product slot-to-card
  aggregation, which is why H1 shipped unnoticed. A new requirement must state the
  selection rule when a group's slots disagree in status.
- `component-freshness` — not violated. The repeated "no queryable upstream" line is
  designed behavior for the bundled slot.
- `application-logging` — not violated; the log did its job and is what made this
  diagnosis possible without machine access.
- `scan-orchestration` — not implicated.

## 4. Options for H1

| Option | Description | Tradeoffs |
| --- | --- | --- |
| A. Prefer a detected record, else the first | `presenceFor` picks the first `Detected` record among the group's slots, falling back to the first record | Minimal, closes the defect completely; when two slots are both installed only one is shown |
| B. One card per slot | Drop product grouping; mirror the scan route's per-record table | Structurally simplest and least bug-prone; two "Claude Code" cards is confusing product-wise, larger i18n surface |
| C. Aggregate the group | Union all detected slots' installations into one card | Best UX; needs a new badge rule for multiple installations and new aggregate status copy; largest test and design surface |

Suggested split for the proposal: ship A as the fix for the reported bug; revisit C once
OpenCode (H2) and possibly Claude Code (H3) also carry multiple slots, because a
two-slot product will otherwise force this decision again.

## 5. Cost of adding a slot (H2)

1. `crates/vertice-core/src/model/slot.rs` — new variant plus `label()` arm. `ts_rs`
   regenerates `frontend/src/bindings/ClientInstallSlot.ts` on
   `cargo test -p vertice-core`; never hand-edit bindings, and CI fails on drift.
2. `crates/vertice-core/src/installations.rs` — `client()` and `version_source()` arms
   (both exhaustive, so the compiler forces the update), a new probe-path builder in
   `windows_install_probes`, and possibly a new `VersionSource` plus resolver if the
   desktop build's version is not reachable via the three existing sources.
3. `crates/vertice-app/src/freshness/upstream.rs` — a new `upstream_for` arm, `None`
   unless a checkable registry identity is confirmed.
4. `frontend/src/lib/pages/ClientsPage.svelte` (and `clientGroups.ts` where relevant) —
   add the slot to the OpenCode group.
5. `crates/vertice-core/tests/client_installations.rs` and its fixture tree.
6. `crates/vertice-core/tests/model_contract.rs` — pins `label()` per variant.

## 6. Test strategy (Strict TDD)

- Core: `crates/vertice-core/tests/client_installations.rs`, fixture-driven via
  `fixture_home(case)` against `crates/vertice-core/tests/fixtures/client-installations/`.
  Needs: the "nothing" fixture's hardcoded four-record expectation bumped, a happy-path
  fixture for the new slot, an absent-slot fixture, and broken-candidate fixtures
  mirroring the existing per-slot isolation cases.
- `crates/vertice-core/tests/model_contract.rs` — pin the new label string.
- Frontend: `frontend/src/lib/pages/ClientsPage.test.ts` currently constructs **only**
  `claudeCodeNpm` records — it never builds a `claudeCodeBundled` record at all, which
  is exactly why H1 shipped. Add a failing test for "npm NotDetected + bundled Detected"
  before the fix, plus new fixtures for the OpenCode desktop slot.
- Run `npm run check` as well as `npm run test`; vitest does not typecheck bindings.

## 7. Open questions requiring evidence from the second machine

1. Does the scan route's supported-clients table on that machine show a `Detected` row
   for "Claude Code (bundled in Claude Desktop)"? A yes closes H1 with no ambiguity.
2. Listing of `C:\Users\<user>\AppData\Local\Packages\` filtered to `Claude_*`, and of
   `C:\Users\<user>\AppData\Roaming\Claude\claude-code\`.
3. If the install is actually the native standalone (H3): `Get-Command claude`, plus
   listings of `%USERPROFILE%\.local\bin` and `%USERPROFILE%\.claude\`.
4. For OpenCode desktop: listings of `%LOCALAPPDATA%\opencode\`, `%LOCALAPPDATA%\Programs\`
   (any `opencode`-prefixed entry) and `%APPDATA%\opencode\`, including whether any
   version-bearing manifest or directory name exists.

Items 2 and 4 are **blocking for H2**: a probe built on an unconfirmed path would ship
silently dead.

## 8. Risks

- CA-16 read-only invariant: no risk. Both fixes are additive read-only probes matching
  the existing resolvers' shape.
- `model/` import allow-list: the new variant stays plain data; all path/existence logic
  belongs in `installations.rs`. Risk only if probing logic is attached to the enum.
- Windows-only path table: both fixes touch `windows_install_probes` only; macOS/Linux
  remain out of scope.
- Unverified web-sourced paths (see §7) — the main correctness risk for H2.
- Scope creep: keep H3 out of this change.
- Fixture-count pins: the "exactly four records" expectation is hardcoded in the spec
  and in at least two test files and must move in lockstep.
