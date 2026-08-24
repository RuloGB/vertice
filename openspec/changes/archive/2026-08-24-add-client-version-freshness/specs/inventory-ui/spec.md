# Delta for Inventory UI

A freshness badge is added to the view listing detected client installations. It is additive: no existing lifecycle, search, duplicate, or diagnostics requirement changes shape. The one cross-cutting rule this delta pins is that `Outdated` MUST NOT re-enter the incident channel — the exact regression `report-client-presence-as-status` removed.

## ADDED Requirements

### Requirement: Freshness Badge On The Clients View

The view listing detected client installations MUST render a freshness badge beside each installation's version, driven by the freshness report, with exactly four visual states: up to date, outdated, unknown, and pending (shown while the freshness lookup for that entry is in flight and no verdict has arrived yet). Before the freshness report arrives, the badge MUST show the pending state rather than an empty or misleading state. The badge MUST render for `Unknown` as a first-class, non-error state — never hidden and never rendered as if it were a failure of the view itself.

#### Scenario: Pending state before the report arrives

- GIVEN the scan has rendered and the freshness report has not yet resolved
- WHEN the clients view is rendered
- THEN each installation's badge shows the pending state, not an empty cell

#### Scenario: Four distinct states render correctly

- GIVEN a freshness report containing one `UpToDate`, one `Outdated`, and one `Unknown` verdict across three installations
- WHEN the clients view renders after the report resolves
- THEN each installation's badge shows the state matching its verdict, with three visually distinct states plus the pending state used only before resolution

#### Scenario: Unknown renders as a first-class state, not an error

- GIVEN a freshness verdict of `Unknown` for an installation
- WHEN its badge is rendered
- THEN it shows the unknown state distinctly from up-to-date and outdated
- AND the view does not present it as a scan failure or a broken row

### Requirement: An Outdated Verdict Is Never An Incident

A `Freshness::Outdated` verdict MUST NOT be counted toward `incidentCount`, MUST NOT light the incident indicator, and MUST NOT move the Home scan-status block out of its healthy state. An out-of-date client is informational, not a fault.

#### Scenario: An outdated client does not trigger the incident indicator

- GIVEN a report with zero `issues` and a freshness report containing one `Outdated` verdict
- WHEN the Agents or Skills page is rendered
- THEN no incident indicator is shown

#### Scenario: An outdated client does not affect the Home scan-status block

- GIVEN the startup scan succeeded with `issues: []` and a freshness report containing one `Outdated` verdict
- WHEN Home renders
- THEN the scan-status block shows the healthy state
