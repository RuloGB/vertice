# Proposal: Add Scan Orchestrator

## Intent

Deliver T9's single public core entry point: run the registered-root adapters, preserve every diagnostic, consolidate T8 output, and return a measured in-memory `ScanReport`. This closes CA-12 and CA-15 without introducing IPC, UI, persistence, or SQLite.

## Scope

### In Scope
- Public `vertice-core` scan entry point that resolves the home directory, traverses registered roots through the existing skill, Claude agent, OpenCode agent, and installation adapters, and returns `ScanReport`.
- Aggregate adapter diagnostics plus explicit `ScanIssue` records for unparseable components (with path), undetected clients, and absent roots; an adapter failure MUST not stop remaining work.
- Flatten adapter components, apply existing T8 `consolidate`, include all scanned roots and installations, and measure elapsed duration for `duration_ms`.
- Fixture-based core tests for full-report composition, diagnostic isolation, CA-12, CA-15's under-two-second reference volume, and no writes.

### Out of Scope
- Tauri commands/IPC, frontend/UI, project or local scope, and real-machine tests.
- SQLite, persistence, provenance/history, schema work, or any write operation.
- New adapter parsing rules, root definitions, installation probe tables, or changes to T8 consolidation semantics.

## Capabilities

### New Capabilities
- `scan-orchestration`: Public core scan workflow that composes registered-root adapters into a complete, measured `ScanReport` with non-aborting diagnostics.

### Modified Capabilities
None.

## Approach

Add a thin core orchestration module and public export. It resolves home once, invokes existing adapters independently, derives absent-root issues from their reported statuses, accumulates all results, consolidates components via T8, and measures wall-clock duration outside `model/`. Adapter-specific behavior remains unchanged.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/vertice-core/src/lib.rs` | Modified | Export public scan entry point. |
| `crates/vertice-core/src/scan.rs` | New | Orchestrate adapters, diagnostics, consolidation, and timing. |
| `crates/vertice-core/tests/` | Modified | Versioned-fixture orchestration and performance tests. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Silent diagnostic loss | Medium | Assert every required issue category and isolation path. |
| CA-15 regression | Low | Time the reference fixture in an automated threshold test. |

## Rollback Plan

Revert the public export and orchestration module/tests. Existing adapters, T8 consolidation, model bindings, Tauri layer, and frontend remain unchanged; no persisted state requires migration.

## Dependencies

- T8 duplicate consolidation (`duplicate-consolidation`) is complete.

## Success Criteria

- [ ] A public core call returns consolidated components, roots, installations, accumulated issues, and measured duration.
- [ ] An unreadable component reports its path and does not interrupt scanning (CA-12).
- [ ] The reference-volume scan completes in under two seconds, measured in the report (CA-15).
- [ ] No SQLite dependency, persistence, IPC/UI surface, or write operation is introduced.
