# Design: T13 Error, Empty, and Non-Actionable States

## Technical Approach

Implement T13 entirely in the Svelte frontend. `App.svelte` retains scan lifecycle and filtering and, only for a ready `ScanReport`, composes a `ScanDiagnostics` presentation component before `InventoryList`. A pure `scanDiagnostics.ts` partitions report data without changing Rust, IPC, bindings, or scan behavior. `InventoryRow` independently derives embedded status from `Location.origin`.

## Architecture Decisions

| Decision | Choice | Rejected | Rationale |
|---|---|---|---|
| Boundary | `ScanDiagnostics.svelte` plus pure classifier/partition helper | Inline logic in `App.svelte` | Separates successful-report interpretation from lifecycle state and gives strict-TDD unit seams. |
| Root partition | Render unavailable roots only from `rootsScanned` where `status === "notFound"`; remove from generic issues only warnings whose `path === null` and whose exact reason is `search root {id} was not found` for an unavailable root ID | Render all issues, or drop every pathless warning | `scan.rs:56-66` emits exactly this warning per unique missing root. The bounded reason set prevents both duplicate root diagnostics and suppression of unrelated pathless warnings. |
| Missing client | `isMissingClientIssue` returns true only when `severity === "warning"`, `path !== null`, and `reason` exactly equals one of `Claude Code (npm) not detected`, `Claude Code (desktop) not detected`, or `OpenCode (npm) not detected` | `endsWith("not detected")`, empty installations, typed core discriminator | `installations.rs:120-175,216-226,307-317` defines a closed Windows probe table and emits those exact strings only for absent probe paths. The predicate is bounded, documented beside the list, and does not classify suffix collisions. |
| Embedded state | `component.locations.some(({ origin }) => origin === "embedded")` | `path === null` | Origin is the explicit model contract; null paths can be non-embedded. |
| Priority | Discreet successful-report notices, never `role="alert"` or an inventory replacement | Hard-failure surface | All `ScanIssue` values are recoverable (`report.rs:30-42`); inventory stays primary. |

## Data Flow

```text
scan()/rescan() -> ScanReport
  -> App.svelte (ready only)
     -> partitionDiagnostics(rootsScanned, issues)
        -> unavailableRoots (notFound roots, once each)
        -> missingClientIssues (exact closed predicate)
        -> genericIssues (all remaining issues except matching root-derived warnings)
     -> ScanDiagnostics (localized chrome; raw paths/reasons)
     -> InventoryList -> InventoryRow (embedded badge from origin)
```

The partition is data-preserving: it neither translates nor constructs payload diagnostics. `ScanDiagnostics` renders nothing when every group is empty.

## File Changes

| File | Action | Description |
|---|---|---|
| `frontend/src/App.svelte` | Modify | Compose diagnostics only during successful ready state, retaining lifecycle/filter behavior. |
| `frontend/src/lib/ScanDiagnostics.svelte` | Create | Render unavailable roots, discreet client notice(s), and generic recoverable issues. |
| `frontend/src/lib/scanDiagnostics.ts` | Create | Export documented exact client predicate and root-aware partition. |
| `frontend/src/lib/InventoryRow.svelte` | Modify | Render localized embedded/non-actionable badge. |
| `frontend/src/lib/i18n/catalogs.ts` | Modify | Add complete English/Spanish diagnostic and embedded chrome. |
| `frontend/src/lib/scanDiagnostics.test.ts` | Create | Unit-test classifier and partition contracts. |
| `frontend/src/App.test.ts` | Modify | Verify mixed-report UI, i18n, and embedded state. |

## Interfaces / Contracts

```ts
type Diagnostics = {
  unavailableRoots: SearchRoot[];
  missingClientIssues: ScanIssue[];
  genericIssues: ScanIssue[];
};
function partitionDiagnostics(roots: SearchRoot[], issues: ScanIssue[]): Diagnostics;
```

The client predicate's exact three-string allow-list must live in one helper with a comment tying it to the closed Windows probes. Adding a client or changing that core grammar requires updating this list and its tests; use a typed discriminator in a separate approved change if that coupling becomes open-ended.

## Testing Strategy

| Layer | What to test | Approach |
|---|---|---|
| Unit | Client classifier | Accept each of the three exact warning/path-present reasons; reject `Other tool not detected` (suffix collision), an exact reason with `Error`, and an exact reason with null path. |
| Unit | Mixed partition | Fixture with two unavailable roots, their two core-style root warnings, one missing client, and one ordinary issue: two root entries, one client notice, one generic issue; assert neither root warning reaches generic and every unavailable root appears exactly once. |
| UI | CA-11/CA-12 | Mixed successful report retains inventory and displays each group with raw reason/path payloads; clean report displays no diagnostics. |
| UI | CA-13/i18n | Embedded-origin component with non-null path is marked; null-path file component is not; catalog chrome switches English/Spanish while payload remains verbatim. |

Strict TDD: add failing unit/UI tests before implementation, then implement and refactor only after green.

## Migration / Rollout

No migration, feature flag, IPC change, binding generation, backend rollout, or filesystem change. Rollback removes only frontend diagnostics, catalog keys, and tests.

## Open Questions

None. The current closed Windows probe grammar makes a bounded presentation predicate reliable; any future generalized client discovery must introduce a typed discriminator rather than broaden string matching.