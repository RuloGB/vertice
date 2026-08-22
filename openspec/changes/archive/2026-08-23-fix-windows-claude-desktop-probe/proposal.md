# Proposal: Fix The Windows Claude Desktop Probe Path And Slot Vocabulary

Traces to **T7** (`internal-docs/plan-desarrollo-poc.md:171`). Addresses **CA-7** (both Claude Code installations detected separately, each with its version) and **CA-11** (absent client reported as "not detected"). Bound by CA-16 (read-only) and CA-17 (fixture-based tests, new non-reused tree). No out-of-scope PoC feature is introduced: no MCPs, no project scope, no writes.

## Intent

`windows_install_probes` probes `%USERPROFILE%\AppData\Roaming\Claude\claude-code` for the "desktop" slot. Claude Desktop ships as an **MSIX package**, so its real payload lives under `AppData\Local\Packages\Claude_<publisherhash>\LocalCache\Roaming\Claude\claude-code\<version>\` (confirmed via `CLAUDE_CODE_EXECPATH` on a real Windows 10 machine). MSIX filesystem redirection hides that from an ordinary process, so the classic path does not exist and CA-7 silently degrades into a CA-11 "not detected" warning on a machine where the runtime *is* installed.

The slot vocabulary is also wrong. Three distinct things are conflated: the **Claude Code CLI installed via npm**, the **Claude Code runtime bundled inside Claude Desktop**, and the **Claude Desktop application** itself. "Claude Code (desktop)" names none of them accurately.

## Scope

### In Scope

- Resolve the bundled slot by enumerating `AppData/Local/Packages/Claude_*` and reading `LocalCache/Roaming/Claude/claude-code/<version>/`.
- Keep the classic `AppData/Roaming/Claude/claude-code/` path as a fallback so legacy non-packaged installs still resolve.
- Rename the slot vocabulary so the npm CLI and the desktop-bundled runtime are distinguishable in labels and in `ScanIssue.reason`.
- Regenerate `crates/vertice-core/tests/fixtures/installations/` as a new tree covering packaged, legacy, both, and neither.
- Regenerate `frontend/src/bindings/` if any exported type changes.

### Out of Scope

- Detecting the Claude Desktop **application** as a client — Vertice inventories AI components and the runtimes that execute them; the desktop app as a product adds nothing to that inventory.
- All frontend/UX work. A follow-up `inventory-ui` change will always list every searched slot with found/not-found state, remove absence from the `ScanIssue` channel, and add a rescan button.
- macOS/Linux path tables (T16). `HostPlatform::Unsupported` behaviour is unchanged.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `client-installation-detector`: the fixed three-slot Windows probe table becomes a table where one slot resolves through a bounded package-directory enumeration plus a legacy fallback; slot naming and the `{kind}` label vocabulary change.

## Approach

The probe table stops being a flat `[InstallProbe; 3]` of fully hardcoded paths. The bundled-Claude slot becomes a *resolver*: enumerate the direct children of `home/AppData/Local/Packages` matching the `Claude_*` prefix, and for each, probe `LocalCache/Roaming/Claude/claude-code/`; then probe the legacy `home/AppData/Roaming/Claude/claude-code/`. Every resulting candidate feeds the existing, unchanged `VersionSource::DirectoryName` extraction and `ClientInstallation` assembly, so the "only path resolution is platform-specific" requirement survives.

**Relationship to the hard rule.** The hard rule at `plan-desarrollo-poc.md:179` is that the scanner never infers other tools' paths from operating-system conventions — the `directories` crate applies only to Vertice's own data directory. This change does **not** relax that rule: the enumeration root is still `home` plus hardcoded segments (`AppData/Local/Packages`), with no OS-convention inference and no environment reads.

What does change is the current implementation style. The doc comment on `windows_install_probes` describes the table as fully static paths pushed segment by segment; the bundled-Claude slot now resolves one prefix-filtered, one-level-deep, read-only directory listing under a hardcoded parent. Rationale: the MSIX publisher hash (`pzs8sxrjxfjjc` on the observed machine) is not verified to be universal across machines or installer channels, so hardcoding it would be a guess that fails silently — the exact failure mode this change exists to fix. The relaxation is confined to this one slot, and the spec must say so explicitly so it does not normalise into the others.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/vertice-core/src/installations.rs` | Modified | Probe table, `InstallKind` variants and labels, bundled-slot resolver |
| `crates/vertice-core/tests/fixtures/installations/` | Replaced | New non-reused fixture tree (CA-17) |
| `crates/vertice-core/tests/` installation tests | Modified | New cases: packaged, legacy, both, neither |
| `frontend/src/bindings/` | Regenerated | Only if an exported type changes |
| `frontend/src/lib/scanDiagnostics.ts` | Coupling risk | Hardcodes the old reason strings |
| `openspec/specs/client-installation-detector/spec.md` | Modified | Via delta spec |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `scanDiagnostics.ts` `MISSING_CLIENT_REASONS` matches the exact strings `"Claude Code (desktop) not detected"` etc.; renaming breaks the match **silently** | High | Update the constants in the same change; the follow-up `inventory-ui` change eliminates the string coupling entirely |
| Publisher hash is not universal → a hardcoded segment would fail on other machines | High | Prefix enumeration `Claude_*`, never a literal hash |
| Old fixtures encode the old path table and become invalid | Certain | Regenerate as a new tree per CA-17; never reuse T4/T5/T6 trees |
| A tool spawned from inside Claude Desktop inherits the MSIX redirected view and wrongly reports the legacy path as existing | Medium | Verification must run from an ordinary shell; tests are fixture-based and immune |
| Enumeration matches multiple `Claude_*` packages | Low | Each match is an independent candidate; multiple installs are already two `ClientInstallation` values, not an anomaly |
| Directory enumeration normalises into other slots | Low | Spec confines enumeration to this one slot; the hard rule on OS-convention inference is untouched |

## Rollback Plan

Single-layer, core-only revert. Restore the previous `windows_install_probes` body, the previous `InstallKind` labels, the previous fixture tree, and `scanDiagnostics.ts`. No persisted data, no migration, no IPC surface change; `vertice-app` is untouched. If bindings changed, `cargo test -p vertice-core` regenerates them from the reverted Rust types.

## Dependencies

- None blocking. The `inventory-ui` follow-up depends on **this** change, not the reverse.

## Success Criteria

- [ ] A packaged Claude Desktop install under `AppData/Local/Packages/Claude_*` is detected with its version (CA-7).
- [ ] A legacy `AppData/Roaming/Claude/claude-code/` install is still detected (CA-7).
- [ ] A home with neither yields no installation plus one explicit "not detected" signal, never an error (CA-11).
- [ ] Slot labels distinguish the npm CLI from the desktop-bundled runtime.
- [ ] No hardcoded publisher hash anywhere in the source.
- [ ] All cases covered by fixtures under a new `installations/` tree; no test reads the author's machine (CA-17).
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `npm run lint && npm run check && npm run test && npm run build` all pass, with bindings in sync.
- [ ] No write outside the app data directory (CA-16).
