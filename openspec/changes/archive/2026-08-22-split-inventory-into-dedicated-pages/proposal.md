# Proposal: Split Inventory into Agents, Skills, and Scan Pages

Traces to T11/T13 UI refinement (CA-1, CA-3, CA-11, CA-12, CA-13). Frontend-only.

## Intent

One combined `inventory` view mixes two unrelated libraries and buries scan health. `ScanDiagnostics` renders nothing on a clean scan, so a healthy scan is invisible. `App.svelte` never passes `status`/`failureMessage` to `HomePage`, so a failed scan shows `home.statsPending` forever — Home misrepresents failure as loading. Meanwhile `agents` and `skills` sit in the sidebar as placeholders.

## Scope

### In Scope
- Remove route `inventory` entirely (no redirect) and its `nav.inventory` / `area.inventory` keys.
- Give `agents` and `skills` real content; each lists only its `ComponentKind`.
- New scan-result route showing the full report: roots scanned (found and not found), detected installations, duration, and issues — clean scans included.
- Discreet incident indicator on Agents and Skills that navigates to the scan area when the scan reported issues.
- Home scan-status block (healthy / completed with issues / failed) with issue count, duration, link to the scan area; wire `status` and `failureMessage` into `HomePage`.
- Rework the Home CTA target and `home.ctaBody` copy.

### Out of Scope
- Any Rust, IPC, binding, or capability change. `scan`/`rescan` stay the only commands; splitting by kind is a client-side filter.
- Per-page or per-kind scanning; one startup scan stays shared.
- `mcp` and `prompts` — they remain placeholders.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `inventory-ui`: per-kind pages replace the combined view; kind selector removed; diagnostics move to a dedicated area with full-report rendering; incident indicator and Home scan status added.
- `frontend-i18n`: catalog boundary covers the new page and scan-area chrome; inventory-only keys retired.

## Approach

One shared internal page component parameterized by `kind: ComponentKind`, with thin `AgentsPage`/`SkillsPage` wrappers. `filterComponents`, `InventoryList`, `InventoryRow`, `LocationList` are reused unchanged. `InventoryToolbar` drops its kind `<select>`; search and reload remain. The scan area consumes `rootsScanned`, `installations`, `durationMs`, `issues` and always renders a verdict. Home shows the verdict only; detail lives in the scan area.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `frontend/src/lib/navigation.ts` | Modified | Drop `inventory`, add scan route, update `ROUTES_WITH_CONTENT`. |
| `frontend/src/App.svelte` | Modified | Route table, per-page query state, Home props. |
| `frontend/src/lib/pages/` | New/Removed | Kind page, wrappers, scan page; remove `InventoryPage`. |
| `frontend/src/lib/InventoryToolbar.svelte` | Modified | Remove kind selector. |
| `frontend/src/lib/ScanDiagnostics.svelte` | Modified | Render clean scans too. |
| `frontend/src/lib/pages/HomePage.svelte` | Modified | Scan-status block, CTA rework. |
| `frontend/src/lib/i18n/catalogs.ts` | Modified | Retire/rename keys, add new chrome. |
| `frontend/src/**/*.test.ts` | Modified | Route and placeholder expectations invert. |

## Decisions

1. **Route `scan`**, keys `nav.scan` / `area.scan`, in the `data` nav group in the slot vacated by `inventory`, alongside `subscriptions`. "Scan" is kind-neutral and covers a healthy scan as well as failures; "Diagnostics" would read as failures-only and contradict the full-report requirement.
2. **Two independent `query` values** in the shell, one for Agents and one for Skills. Neither shared nor reset on navigation — a shared query would silently pre-filter the other page.
3. **The incident indicator counts `issues` PLUS roots with `status === "notFound"`.** `frontend/src/lib/scanDiagnostics.ts` derives `unavailableRoots` from `rootsScanned.filter(status === "notFound")`, and `partitionDiagnostics` deliberately drops the matching `search root {id} was not found` warnings from `remainingRecoverableIssues` to avoid double-counting. An issues-only count would therefore raise no badge in exactly the missing-root case that motivates the requirement. The spec MUST carry a scenario where a scan with zero `issues` and one not-found root still raises the indicator on both pages.
4. **Home offers a retry on scan failure**, invoking `rescan`.
5. **Reuse and rename** the existing `inventory.*` / `toolbar.*` / `diagnostics.*` namespaces; do not duplicate keys per page.
6. **Routing stays under `inventory-ui`** for this change. The navigation shell (commit `602c2dd`) has no spec of its own, and a change already touching seven requirements should not also introduce a capability. A dedicated navigation capability is deferred until the route model grows — when `mcp` and `prompts` gain real content.

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Users trust a partial list after a silent root failure | High | Indicator counts not-found roots as well as `issues` (decision 3), spec'd with a zero-issue scenario. |
| Removing the kind selector regresses CA-3 visibility | Low | Duplicate marks and paths stay on the row; covered per page. |
| Broad i18n key churn breaks Spanish parity | Medium | Catalog completeness scenario per locale. |

## Rollback Plan

Revert the frontend commit. No Rust, IPC, binding, capability, or scan behavior is touched, so rollback is a single-layer revert with no data or contract migration.

## Dependencies

- Existing navigation shell (PR #28) and `ScanReport` diagnostic fields from T9.

## Success Criteria

- [ ] Agents and Skills each render only their kind, with search and reload, and no kind selector.
- [ ] A clean scan is visible in the scan area (roots, installations, duration, zero issues).
- [ ] A scan with issues shows an indicator on both list pages that navigates to the scan area.
- [ ] A failed scan makes Home show failure and a retry, never `home.statsPending`.
- [ ] Route `inventory` and its keys no longer exist; `mcp` and `prompts` still render placeholders.
- [ ] Strict-TDD coverage per surface; English and Spanish catalogs stay complete.
