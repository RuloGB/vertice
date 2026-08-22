# Proposal: Audit Read-Only Invariant

## Intent

Close T14 / CA-16 by proving Vertice's PoC scanner is read-only, not merely assuming it. The change preserves product behavior while adding automated and documented evidence that scans do not mutate scanned roots and that the desktop shell exposes no write-capable filesystem surface.

## Scope

### In Scope
- Strengthen the reference-volume scan proof with content hash + `mtime` snapshots before/after a complete scan.
- Audit scanner and shell write surfaces, including Tauri ACL permissions.
- Document manual/system-level verification evidence for CA-16.

### Out of Scope
- Adding persistence, SQLite, history, cache, telemetry, or app-data writes.
- Changing scan output, adapter parsing, UI behavior, or supported roots.
- Introducing filesystem/shell/dialog Tauri capabilities.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `scan-orchestration`: make the read-only scan guarantee explicitly verifiable by metadata/hash evidence.
- `desktop-shell`: require ACL evidence that the webview has no filesystem write capability over scanned roots.

## Approach

Use the exploration's layered proof: extend the existing core fixture test in `scan.rs`, add a precise static audit of write APIs and capability grants, and require verify/archive artifacts to record manual reference-machine evidence. This keeps trust evidence close to the actual scan path without broadening product scope.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/vertice-core/src/scan.rs` | Modified | Stronger fixture no-write proof. |
| `crates/vertice-core/src/*.rs` | Audited | Confirm scanner modules do not open write handles outside app data. |
| `crates/vertice-app/capabilities/default.json` | Audited | Confirm only `core:default` is granted. |
| `crates/vertice-app/src/commands.rs` | Audited | Confirm commands remain thin pass-throughs. |
| `openspec/specs/scan-orchestration/spec.md` | Modified | Read-only evidence requirement. |
| `openspec/specs/desktop-shell/spec.md` | Modified | ACL audit requirement. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Platform-sensitive `mtime` comparisons become flaky. | Med | Snapshot stable metadata and avoid precision assumptions where filesystems differ. |
| Static grep misses indirect write paths. | Med | Define allowed/forbidden APIs and pair static audit with runtime fixture proof. |
| Manual evidence becomes stale. | Low | Treat it as verify/archive evidence, not a replacement for automated tests. |

## Rollback Plan

Revert the proposal/spec/design/tasks and any test/audit changes. Since behavior and persisted data are unchanged, rollback has no data migration or three-layer architecture impact.

## Dependencies

- T9 scan orchestration and T10 Tauri command/capability surface are already available.

## Success Criteria

- [ ] CA-16 is explicitly evidenced by automated hash + `mtime` fixture proof.
- [ ] Static audit confirms no write-capable scanner path or Tauri ACL scope over scanned roots.
- [ ] Verification records manual/system-level read-only evidence.
