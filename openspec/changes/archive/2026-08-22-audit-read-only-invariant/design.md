# Design: Audit Read-Only Invariant

## Technical Approach

Preserve product behavior and prove CA-16 with layered evidence: a full reference-fixture tree immutability assertion in `vertice-core`, maintainable static audits for core/app mutation surfaces, Tauri ACL checks for the webview boundary, and verify/archive evidence that states the limits of static proof. Core remains pure; IPC, UI, generated bindings, persistence, and scan output stay unchanged.

## Architecture Decisions

| Decision | Choice | Alternatives considered | Rationale |
|---|---|---|---|
| Runtime proof scope | Replace file-byte snapshots with full-tree snapshots for files and directories. Track relative path, entry kind, length for files, stable content hash for files, platform permission evidence (Unix mode, Windows file attributes, or readonly fallback), required modified timestamps, and symlink target only when a fixture entry is actually a symlink. | File-only snapshots; OS-specific filesystem monitors. | CA-16 forbids all scanned-root mutation, so create/delete/rename directories, chmod, truncation, and modified-time mutations must be observable without adding platform tooling; link mutation APIs are covered statically unless a portable runtime symlink fixture is added. |
| Snapshot hashing | Use test-local deterministic byte hashing while still comparing metadata. | `DefaultHasher`; `sha2` dependency. | The assertion only needs equality inside one test run; production dependencies and public APIs stay untouched. |
| Mutation inventory | Add sorted test-local deny lists covering direct and indirect filesystem mutation APIs. | Ad-hoc grep for `File::create`/`OpenOptions::write`; manual-only review. | The policy must be broad enough to catch future regressions, not just today’s obvious calls. |
| Static proof limits | Treat source/capability audits as regression guards, not mathematical proof of all transitive behavior. | Claim static text search proves complete absence of writes. | Rust traits, macros, dependencies, and platform APIs can hide mutation; runtime fixture proof and manual verification remain required layers. |
| Boundary placement | Keep all new helpers/tests in test code; production scanner/app code is changed only if needed to expose no new behavior. | Add scanner API, telemetry, app-data writes, or frontend tests. | Evidence must not violate the read-only invariant it is proving. |

## Data Flow

```text
reference fixture root
  -> walk files + dirs without following links; record symlink targets only if symlink entries exist
  -> snapshot(entry kind, relative path, metadata, file hash/link target)
  -> scan_for(home, Windows)
  -> snapshot again
  -> assert identical snapshot + CA-15 speed + in-memory report

core/app/capability sources
  -> static audit deny lists
  -> cargo test evidence
  -> verify/archive records automated scope + supplemental manual proof
```

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/scan.rs` | Modify tests | Replace `fixture_tree_bytes` with a full-tree snapshot helper that includes files, directories, stable file hash, file length, platform permission evidence, and required modified timestamps. Existing symlink entries are recorded if present, but the current reference fixture has none. |
| `crates/vertice-core/tests/read_only_audit.rs` | Create | Audit `crates/vertice-core/src/**/*.rs` for forbidden mutation inventory: `File::create`, `create_new`, `OpenOptions`, `write*`, `append`, `truncate`, `set_len`, `set_permissions`, `remove_*`, `rename`, `copy`, `create_dir*`, hard/sym-link creation, `std::io::Write`, `BufWriter`, and platform extension write/truncate modes. |
| `crates/vertice-app/tests/read_only_audit.rs` or app unit tests | Create/Modify | Audit `commands.rs`, `lib.rs`, and `capabilities/default.json`; assert IPC remains scan/rescan pass-through and capability permissions remain exactly `core:default` with no fs/shell/dialog or mutation scope strings. |
| `crates/vertice-app/capabilities/default.json` | Keep/audit | No content change expected; it is evidence input. |
| `openspec/changes/audit-read-only-invariant/verify-report.md` | Later | Record test commands, audited surfaces, static-audit limits, and supplemental manual/system-level evidence. |

## Interfaces / Contracts

No public Rust API, Tauri command, TypeScript binding, UI, persistence, or product behavior changes. New structures are test-only.

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| RED | File-only proof and narrow deny-list are insufficient | Add failing assertions for full-tree metadata coverage and forbidden mutation categories. |
| GREEN | CA-16 layered evidence | Implement test helpers and audits only; keep deny-list sorted and scoped. |
| Verification | Realistic confidence | Run Rust/app tests plus static searches; record that static audits do not prove dependency or macro transitive absence alone. |

## Migration / Rollout

No migration required. This is an evidence/audit change only.

## Open Questions

None.
