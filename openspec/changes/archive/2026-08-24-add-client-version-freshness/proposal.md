# Proposal: Report Whether A Detected Client Installation Is Out Of Date

> **Spec trace.** Touches `domain-model` (a new verdict type and its generated binding), `workspace-architecture` (a new single-owner seam, and the first outbound network dependency in the workspace), `desktop-shell` (a second IPC command), `inventory-ui` and `frontend-i18n` (a freshness badge and its catalog keys), and `scan-orchestration` (freshness MUST stay outside the scan operation and outside its CA-15 budget). Introduces one **new capability**, `component-freshness`.
>
> **Standing invariants respected.** `vertice-core` stays Tauri-free and HTTP-free; `model/` stays I/O-free and clock-free; the outside world is owned by one module, as `yaml.rs`, `jsonc.rs` and `toml.rs` already are; nothing is written outside the app data directory (**CA-16**); core tests stay fixture-based and never touch the network (**CA-17**).
>
> **This is the first change of the final product**, not a PoC increment. `internal-docs/alcance-poc-vertice.md:37` excluded exactly this feature; that document describes a closed stage and is cited here as provenance — it explains why `version` is an unvalidated `String` and why no HTTP dependency exists — not as a blocker. The architectural invariants in `AGENTS.md` still bind in full and shape *where* the network code may live.

## Intent

Vertice answers "which AI clients are installed, and at what version". It does not answer the question a user actually asks next: **"is that the current version?"**

The concrete trigger, verified during exploration: the user's OpenCode installation is behind the latest release, and Vertice reports its version as bare fact with no indication that a newer one exists. The tool shows a number the user must go and check somewhere else. That is the gap.

Three properties make this the right increment now rather than later:

1. **The detection half is already done and typed.** `add-codex-client-support` and `report-client-presence-as-status` shipped four probe slots, per-slot version sources, and a typed `ClientPresence` record. Every installed version already arrives at the UI as structured data. What is missing is the other side of the comparison.
2. **The comparison is pure and cheap.** "Is `0.148.0` older than `0.152.0`" is a total function of two strings. It belongs in `vertice-core`, needs no I/O, and is trivially fixture-testable.
3. **The mechanism generalizes.** The user has stated that the same "is this out of date?" question will later be asked of **skills and agents**. Modelling the verdict as a keyed report rather than a field on `ClientInstallation` costs almost nothing now and avoids a second breaking model change later.

The genuinely new work — and the reason this proposal is longer than the increment looks — is that answering the question requires a **reference version**, and obtaining one means **Vertice makes the first outbound network call in its history**. That is a privacy-posture change, a dependency-audit change, and an offline-behaviour change, and it deserves an explicit decision rather than arriving as an implementation detail.

## Settled decisions carried into this proposal

These were decided during exploration and by the user on 2026-08-24. They are recorded, not reopened.

### The reference version comes from a live network lookup

The npm registry and the GitHub Releases API, queried at runtime — **not** a version table pinned into the binary and shipped with each Vertice release.

The pinned-manifest alternative was rejected on product grounds, not technical ones: it is cheaper in every respect (zero dependencies, fully offline, trivially testable) but its accuracy decays the moment any client ships a release, and it would make Vertice's own release cadence the ceiling on freshness accuracy. It would routinely say "up to date" to a user who is not — the exact false negative this feature exists to eliminate. A checker that is confidently wrong is worse than no checker.

**Consequence, stated plainly:** Vertice acquires an HTTP client dependency and an outbound call. Offline, rate-limited, timed-out and unparseable responses degrade to `Unknown { reason }` — **never** to an error state, never to a hang, and never to a false "up to date".

### The verdict is three-valued

`UpToDate` / `Outdated { latest }` / `Unknown { reason }`.

Two-valued is rejected. Collapsing "cannot tell" into `UpToDate` is a misleading false negative; collapsing it into `Outdated` is a false positive that sends the user chasing a non-existent update. An honest "unknown, because the registry could not be reached" beats both, and it is the *likeliest* state on a first offline run. `Unknown` is a first-class outcome the UI must render well, not an error path.

### Core depends on an abstraction; the fetcher lives in `vertice-app`

`vertice-core` compares an installed version against a reference version it **receives as an input**, behind a trait seam. The concrete fetcher — HTTP client, registry endpoints, response parsing, caching — lives in `vertice-app`. Never in `vertice-core`, and never in the WebView.

This is not a preference; it is what the existing invariants dictate. `model/` bans `std::io`. `vertice-core` is meant to stay reusable by a future headless CLI with a small, auditable dependency footprint (eight direct dependencies today), and an HTTP stack is the single largest transitive-dependency addition the workspace has ever considered. And the project already has the pattern: `yaml.rs`, `jsonc.rs` and `toml.rs` each own one external format, and everything else goes through the seam. One module owns the outside world.

The WebView is excluded separately and firmly: `stack-tecnologico-vertice.md:108` fixes that any public-registry lookup is performed by the Rust process and its response treated as untrusted data. `desktop-shell`'s CSP (`default-src 'self'`, no remote content) MUST NOT be relaxed, and the capability grant stays `core:default`.

### The verdict travels as a parallel keyed report

Not a new field on `ClientInstallation`, and not an overload of `ClientPresenceStatus`.

- **`ClientPresenceStatus` is about slot presence** — its own doc comment says so (`presence.rs:32-42`), and `report-client-presence-as-status` fought specifically to give absence its own typed carrier. Folding a best-effort, network-dependent verdict into it would re-conflate two meanings that were just separated.
- **A field on `ClientInstallation`** would couple a struct that is currently pure on-disk fact to a value requiring an external reference, and would break every hand-constructed `ClientInstallation` in the existing tests at compile time.
- **A parallel keyed report** mirrors the pattern already in the codebase (`InstallationScan.presence` alongside `installations`, `installations.rs:36-44`), is additive and empty by default, and is the only shape that generalizes to skills and agents without forcing every domain type to carry an optional field it may never populate.

### Read-only stays

This change is **informational only**. Vertice reports that an update exists; it does not offer to install one. An update action would be Vertice's first mutation of the user's machine, a far larger decision than this change, and is explicitly out of scope. CA-16 is untouched: the only write introduced anywhere is the response cache, and it lives in the app data directory.

## Judgment calls, and this proposal's position

### Default posture: enabled by default, with a visible opt-out and first-run disclosure

`openspec/config.yaml`'s design principle 8 and `internal-docs/stack-tecnologico-vertice.md:115` both say **no telemetry by default**, with any addition being explicit opt-in. The obvious reading is that this check should be opt-in too. **This proposal argues it should not be**, and states the reasoning so a reviewer can overrule it on the record.

The no-telemetry principle governs **outbound user data** — what Vertice would report *about* the user. This lookup is the opposite direction: it is an anonymous read of a public package registry, carrying no inventory, no paths, no component names, no identifier, and no usage signal. Nothing about the user's machine leaves it. Treating "we send your data out" and "we read a public version number" as the same category collapses a distinction that matters, and the cost of collapsing it is a version checker that, for most users, silently never runs. A freshness feature that is off by default does not do its job; it becomes a setting almost nobody discovers, and the user in the trigger case stays on an outdated OpenCode.

This is not a free position, and the counter-argument is real: the request itself reveals an IP address and a rough "someone is running Vertice" signal, and a corporate or air-gapped user may treat *any* unexpected outbound connection as a policy violation regardless of payload. The proposal therefore pairs the default with three obligations that are not optional:

- **First-run disclosure.** The user is told, in the UI, that Vertice checks public registries for the latest versions, what is sent (nothing about them), and where to turn it off. Not buried in a settings pane they must go looking for.
- **A visible, discoverable setting** to disable the check permanently, honoured with no further requests of any kind once off.
- **No identifying request content.** No unique identifier, no machine fingerprint, no inventory data in the request — a plain product user-agent at most.

**If the user prefers strict parity with principle 8, the fallback is off-by-default with a one-click enable, and it changes exactly one default constant.** The architecture, the seam, the verdict model, the caching and the UI are identical either way, so this decision is reversible at negligible cost and should not block the change. **CONFIRMED by the user on 2026-08-24: enabled by default, with first-run disclosure and a visible opt-out.** This is now a settled decision, not a proposal-level position. The three obligations above — first-run disclosure, a discoverable off switch honoured with no further requests of any kind, and no identifying request content — are binding requirements of this change, not recommendations, and `sdd-spec` MUST express them as testable requirements rather than as prose.

### Model the `subject_kind` discriminator now, wire only client installations

**Confirmed.** The report key carries a subject-kind discriminator from the first commit, with only the client-installation variant populated by this change.

The reasoning is asymmetry of cost. Adding the discriminator now is one closed enum with one variant, one generated binding, and one extra field in a type that does not exist yet — genuinely cheap, and additive. Adding it *later* means changing a published model type, regenerating bindings, and touching every consumer, in a change that is already about skills and agents and does not need a second concern. The thing that would be expensive — wiring skill and agent freshness, deciding what "latest" even means for a locally-authored skill, and resolving their upstream identities — is explicitly **not** done here.

The one honest risk: a discriminator with a single variant is speculative generality if the skills/agents feature never lands or lands with a different shape. That risk is accepted because the enum is closed and private-facing enough to change, and because the alternative shape (no key at all, a field on `ClientInstallation`) is the one already rejected for independent reasons.

### Upstream identity is per-slot, and two of the four mappings are not yet known

The four `InstallSlot` variants map to **four different upstream namespaces**. This is per-slot work, not one global rule, and this proposal states only what is verifiable in the repository today:

| Slot | Upstream namespace | Status |
|---|---|---|
| `ClaudeCodeNpm` | npm package `@anthropic-ai/claude-code` | **Derived from the repository.** The probe path is `AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code` (`installations.rs:222-232`), and npm installs a package into `node_modules/<package-name>`, so the registry name follows from the install path |
| `OpenCodeNpm` | npm package `opencode-ai` | **Derived from the repository** by the same argument (`installations.rs:245-248`) |
| `CodexStandalone` | A GitHub repository's releases | **NOT derivable from this repository.** Nothing in the codebase names an owner or repository slug — `CODEX_TARGET_TRIPLES` (`installations.rs:191`) is a target-triple table, not an upstream identifier. **Open question for `sdd-design`** |
| `ClaudeCodeBundled` | Unknown | **NOT derivable, and possibly non-existent.** This version is a directory name inside an MSIX package cache (`resolve_bundled_slot`, `installations.rs:482-570`). It is the runtime bundled inside Claude Desktop, distributed through the Microsoft Store, not through a registry Vertice can query. It is not obvious that the npm `@anthropic-ai/claude-code` version is even a valid comparison target for it. **Open question for `sdd-design`** |

**No package name, repository slug, or API URL beyond the two derived above is asserted anywhere in this proposal, and none may be invented during design.** Each must be verified against the real upstream before it is written into code or spec. The two unknown mappings are a design-phase obligation, and `ClaudeCodeBundled` may legitimately resolve to "this slot has no queryable upstream" — in which case its verdict is permanently `Unknown { reason }`, which the three-valued model represents honestly and which is a perfectly acceptable outcome. **A slot with no known upstream MUST NOT be reported as `UpToDate`.**

## Approach

At proposal altitude, six moving parts:

1. **A pure verdict type in `model/`.** A new `Freshness` enum (`UpToDate` / `Outdated { latest }` / `Unknown { reason }`), `TS`-derived, following the exact shape of `ClientPresenceStatus`. Zero I/O, zero clock, inside the existing `model/` import allow-list.
2. **A pure comparison in `vertice-core`**, outside `model/` (mirroring where `installations.rs` sits): a function taking an installed version string and a reference version string and returning a `Freshness`. A `semver` dependency does the parsing; **any** parse failure on either side resolves to `Unknown`, never to a panic, never to a silent skip, and never to a guess. None of the four version sources is semver-guaranteed — the MSIX directory name is the likeliest to fail, and the Codex version legitimately carries an `-rc.1`-style prerelease component that a naive `x.y.z` parser mishandles.
3. **A trait seam for the reference source, owned by one module.** Core depends on the abstraction and never on a concrete fetcher. Core tests inject a fixed stub: no network, no fixture drift, no flake.
4. **The concrete fetcher in `vertice-app`**, implementing that trait: HTTP client, per-slot upstream resolution, defensive parsing of registry responses as untrusted input, a response cache in the app data directory, and total degradation to `Unknown { reason }` on offline, timeout, rate-limit, HTTP error, or unparseable payload. **The cache is not an optimization.** The unauthenticated GitHub API allows roughly 60 requests per hour per IP; without a cache a user who rescans often is throttled into permanent `Unknown`, which is the feature failing in the most confusing possible way.
5. **A second IPC command, asynchronous and separate from the scan.** The scan MUST complete and the UI MUST render without waiting on any network call. Freshness arrives as a later, second result. `desktop-shell` already requires commands to be async and `spawn_blocking`-offloaded; this one follows that contract. **The CA-15 two-second scan budget is measured on the scan alone and MUST NOT absorb network latency.**
6. **Frontend: a badge beside the existing version line** in `ClientsPage.svelte`, driven by the new report, with four visual states — up to date, outdated, unknown, and pending-while-in-flight — and new `clients.*` i18n keys complete in both English and Spanish (design principle 7). **`Outdated` MUST NOT count as an incident.** An out-of-date client is information, not a fault: routing it into `incidentCount` would turn the Home banner amber for a healthy machine, which is precisely the regression `report-client-presence-as-status` just removed.

### The binding contract (explicit obligation)

New public model types change `frontend/src/bindings/`. Bindings are regenerated **only** by `cargo test -p vertice-core` and MUST NEVER be hand-edited. CI regenerates them and fails on any diff, running `git add --intent-to-add` first so a brand-new file is also caught. Every new `.ts` file and every modified one MUST land in the same commit as the Rust types.

## Scope

### In Scope

- New `crates/vertice-core/src/model/freshness.rs`: the `Freshness` verdict, the keyed report type, and its subject-kind discriminator — all plain data, `Serialize`/`Deserialize`/`TS`, within `model/`'s import allow-list.
- A pure version-comparison function in `vertice-core` (outside `model/`), with `semver` as its only new core dependency and `Unknown` as its total fallback. **Corrected by design §16:** app-side, `serde_json` is a second addition alongside the HTTP client (both already present in the dependency graph) for cache and response parsing.
- A trait seam in `vertice-core` for obtaining a reference version, with a test stub. Core never depends on a concrete fetcher.
- The concrete fetcher in `crates/vertice-app`: HTTP client, per-slot upstream resolution for the slots whose upstream is known, defensive response parsing, and a cache in the app data directory.
- A new async IPC command in `vertice-app` returning the freshness report, separate from the scan command.
- The enable/disable setting for the check, and its first-run disclosure.
- Regenerated `frontend/src/bindings/`.
- `ClientsPage.svelte`: a freshness badge with up-to-date / outdated / unknown / pending states, and the call that fetches the report after the scan renders.
- New `clients.*` i18n keys, complete in `en` and `es`.
- Fixture-first failing tests for every behaviour above (`strict_tdd: true`), including the offline, rate-limited and unparseable-response degradation paths — exercised through the seam, never against a live network.
- `Cargo.toml` / `Cargo.lock` movement for the new dependencies, with `cargo deny check bans licenses` re-run and any resulting `deny.toml` movement.

### Out of Scope

- **Any update action.** Vertice does not download, install, or launch an updater. Read-only stands (CA-16).
- **Freshness for skills and agents.** The discriminator is modelled; nothing is wired. What "latest" means for a locally-authored skill is an unanswered product question.
- **Any change to how installed versions are detected.** `installations.rs`'s probe table, version sources, `ClientPresence` records and their spec are unchanged. Freshness reads the versions detection already produces.
- **Any change to `ClientPresenceStatus`, `ClientInstallation`, `ScanIssue`, or `IssueSeverity`.** No new severity, no new field on existing installation types.
- **Freshness as a scan-blocking step** or as a term in `ScanReport`'s duration. The scan does not wait on the network.
- **Any WebView network access.** The CSP stays `default-src 'self'`; the capability grant stays `core:default`.
- **A pinned-manifest fallback layer.** Not adopted, not even beneath the live lookup, unless `sdd-design` finds a concrete reason.
- **macOS and Linux probe tables.** Unchanged; an unsupported platform reports no presence records and therefore no freshness subjects.
- **Notification, background polling, or scheduled checks.** The check runs when the user is looking at the app, not on a timer.
- **Any telemetry.** Nothing about the user or their inventory is transmitted, now or as a side effect of this change.

## Capabilities

### New Capabilities

- **`component-freshness`** — the verdict vocabulary (`UpToDate` / `Outdated` / `Unknown`), the comparison rule and its total-fallback-to-`Unknown` guarantee, the reference-source abstraction contract, the per-subject upstream-identity mapping rule, the degradation contract for offline / rate-limited / unparseable responses, the caching obligation and its CA-16 location constraint, and the rule that a subject with no known upstream is `Unknown` and never `UpToDate`.

### Modified Capabilities

- **`domain-model`** — "Rust Types Generate a Matching TypeScript Contract" enumerates exactly ten core types (`spec.md:209-211`); the new verdict, report and discriminator types extend that enumeration, with their generated bindings.
- **`workspace-architecture`** — the single-owner seam inventory grows again (`yaml.rs`, `jsonc.rs`, `toml.rs`, and now the reference-source seam), and this is the **first seam whose owner lives in `vertice-app` rather than `vertice-core`**, which the capability must state explicitly rather than leave implied. The Tauri-free and HTTP-free containment MUST for `vertice-core` is restated with the new dependency in view.
- **`desktop-shell`** — "Minimal Scan Command Surface" grows from two commands to three. The new command is async, `spawn_blocking`-offloaded, and returns typed generated types with no hand-written DTO. The capability grant and the CSP are unchanged and their unchanged state is a review check.
- **`inventory-ui`** — a freshness badge on the clients view, with its pending and unknown states; and an explicit rule that `Outdated` is **not** an incident and MUST NOT affect `incidentCount` or the Home banner.
- **`frontend-i18n`** — "Catalog Completeness and Boundary" extends to the new keys in both catalogs. Version strings and upstream package names are passthrough data and MUST NOT be localized, consistent with the existing proper-noun rule for slot labels.
- **`scan-orchestration`** — "Measured Reference-Volume Performance" needs an explicit statement that freshness is outside the measured scan operation and cannot consume the CA-15 budget; and "Visible and Isolated Diagnostics" needs to say that a failed freshness lookup produces a `Freshness::Unknown`, **not** a `ScanIssue`.

### Explicitly NOT Modified

`skill-scanner`, `agent-scanner`, `opencode-agent-scanner`, `codex-agent-scanner`, `frontmatter-reader`, `duplicate-consolidation`, `ci-quality-gates`.

**CORRECTED by `sdd-design` (2026-08-24, design §2 and §16): `client-installation-detector` is Modified and has left this list.** The subject key must be derivable from public data, and design resolved it by promoting the private `InstallSlot` (`installations.rs:133-139`) to a public model type `ClientInstallSlot` and adding a `slot` field to `ClientPresence`. Keying on `ClientPresence.label` was rejected: it is display copy that has already been reworded once, and matching on it would revive exactly the string-matching that `report-client-presence-as-status` deleted.

Detection *behaviour* remains untouched — probes, paths, version sources, ordering and issues are byte-identical — but the capability's **published shape** changes, and `domain-model` gains five new types plus one modified type rather than the "new verdict and report types" this proposal originally anticipated.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/vertice-core/src/model/freshness.rs` | **New** | Verdict, keyed report, subject-kind discriminator; plain data only |
| `crates/vertice-core/src/` (comparison module) | **New** | Pure version comparison; `semver` is imported here and nowhere else |
| `crates/vertice-core/src/` (reference-source trait) | **New** | The abstraction core depends on; no concrete implementation |
| `crates/vertice-core/src/installations.rs` | **Unchanged** | Detection is not touched — the point of the parallel report |
| `crates/vertice-core/src/model/presence.rs`, `installation.rs` | **Unchanged** | No new field, no overloaded status |
| `crates/vertice-app/` (fetcher module) | **New** | HTTP client, upstream resolution, defensive parsing, cache |
| `crates/vertice-app/` (command) | Modified | Third IPC command; capabilities file unchanged |
| `crates/vertice-app/capabilities/default.json` | **Expected unchanged** | Still `core:default`; a direct Rust HTTP client needs no Tauri permission |
| `frontend/src/bindings/` | Regenerated | New `.ts` files; never hand-edited |
| `frontend/src/lib/pages/ClientsPage.svelte` | Modified | Freshness badge and its four states |
| `frontend/src/lib/i18n/catalogs.ts` | Modified | New `clients.*` keys, `en` + `es` |
| `frontend/src/lib/scanDiagnostics.ts` | **Expected unchanged** | `Outdated` is not an incident |
| `Cargo.toml`, `Cargo.lock` | Modified | `semver` in core; an HTTP client in `vertice-app` |
| `deny.toml` | **Possibly modified** | The HTTP stack's transitive licences must be verified; unverified today |
| Core tests / fixtures | New | Comparison cases and stubbed-seam degradation cases; **no network** |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| **The first outbound call in the product's history** changes Vertice's privacy posture, offline behaviour and audit surface | **High — certain, by design** | Made an explicit, disclosed, user-controllable decision rather than an implementation detail: first-run disclosure, a visible off switch, no identifying request content, and a documented default position that a reviewer can overrule by changing one constant |
| **The HTTP stack's transitive dependencies fail `cargo deny check licenses`**, or push the workspace MSRV above its floor | **High — entirely unverified** | Must be closed in `sdd-design` before any dependency is pinned. The MSRV floor is declared in three places that must agree and a CI step fails on drift; if the chosen client's floor is higher, **the client choice changes, not the floor**. A blocking, dependency-light client is worth weighing against reusing Tauri's already-present async runtime |
| **Two of the four upstream identities are unknown**, and a design-phase guess would produce confidently wrong verdicts | **High** | Named as open questions above; no slug or URL is asserted anywhere in this proposal. A slot with no verified upstream reports `Unknown { reason }` and MUST NEVER report `UpToDate` |
| **Version strings are not semver** across four extraction mechanisms, and a parse failure silently reads as up-to-date | Med | The comparison is total: any parse failure on either side resolves to `Unknown`. Fixture cases for the MSIX directory-name shape and the `0.150.0-rc.1` prerelease shape must exist and fail before the comparison is written |
| **Rate-limiting turns freshness into permanent `Unknown`** for an active user | Med | The cache is mandatory, not an optimization, and lives in the app data directory (CA-16). A stale-but-recent cached reference is a better answer than a throttled one |
| **A freshness lookup blocks the scan or first render**, breaking CA-15 | Med | Structurally separate command and separate result; the scan does not await it. A regression here is a spec violation, pinned by `scan-orchestration`'s delta |
| **`Outdated` leaks into the incident channel**, turning the Home banner amber for a healthy machine | Med | Explicit spec rule plus a test; this is the exact regression `report-client-presence-as-status` removed and it must not return through a side door |
| **A malicious or malformed registry response** is trusted | Med | Responses are untrusted input by standing policy (`stack-tecnologico-vertice.md:108`). Parsing is defensive with a bounded response size; a failure yields `Unknown`, never a panic and never an injected string rendered as a version |
| **Binding drift**: a new `.ts` file is forgotten and CI's `--intent-to-add` gate fails late | Med | Regenerate via `cargo test -p vertice-core` in the same commit; never hand-edit |
| **A test reaches the real network**, making CI flaky and machine-dependent | Med | Core tests inject the stub seam; the trait exists precisely so this is structurally impossible in core. Any live-network test in `vertice-app` must be explicitly opt-in and excluded from CI (CA-17) |
| **The `subject_kind` discriminator is speculative generality** if skills/agents freshness never lands or lands differently | Low | The enum is closed and cheap to change; the alternative shapes were rejected for independent reasons |
| **`vertice-core` accidentally acquires the HTTP dependency** through a convenience refactor | Low | `deny.toml` bans `tauri` outside `vertice-app` mechanically; **design must decide whether to add the chosen HTTP crate to that ban list with `vertice-app` as the sole allowed parent**, so the containment is enforced rather than reviewed |
| **A cache write lands outside the app data directory**, breaking CA-16 | Low | The only write introduced by this change; its path is a design-level requirement and subject to the existing read-only audit |

## Rollback Impact (three layers)

Additive at every layer. Revert in dependency order.

1. **Core (`vertice-core`)** — delete `model/freshness.rs`, the comparison module, the reference-source trait, their tests and the `lib.rs` `pub mod` lines; remove `semver` from `Cargo.toml`. `installations.rs`, `presence.rs`, `installation.rs`, `scan.rs` and every existing fixture have **nothing to revert** — they were never edited. This is the direct payoff of the parallel-report shape: no existing test or type is disturbed in either direction.
2. **Bindings** — `cargo test -p vertice-core` regenerates `frontend/src/bindings/` from the reverted types, removing the new `.ts` files. **Never hand-edited, in either direction.** The `--intent-to-add` gate confirms the revert is complete.
3. **App (`vertice-app`)** — remove the fetcher module, the third IPC command and its registration, and the setting. `capabilities/default.json` and the CSP were never edited, so neither needs a revert; their untouched state is the review check.
4. **Frontend** — remove the badge, its fetch call and the new catalog keys. `scanDiagnostics.ts` is untouched because `Outdated` never entered the incident channel.
5. **Supply chain** — remove the HTTP client and `semver`, regenerate `Cargo.lock`, revert any `deny.toml` movement, and re-run `cargo deny check bans licenses`. **This is the only layer whose revert is not free**, and the only one that must be verified rather than assumed.

**Migration: none.** Nothing is persisted except the response cache, which is a pure cache: deleting it loses no user data and the app functions identically without it, degrading to a live lookup or to `Unknown`. **A partial rollback** (core reverted, frontend not) fails at TypeScript compile time on the missing binding, not silently at runtime. **A partial rollback of the fetcher alone** leaves the UI showing `Unknown` for every client — degraded, honest, and not a crash, which is itself evidence the degradation path is designed correctly.

## Open Questions for `sdd-design`

**Must be closed before any code — do not guess:**

1. **The Codex upstream identity.** Which GitHub owner and repository publishes the standalone Codex releases Vertice detects, and does its Releases API expose a version comparable to the release-directory-name version? Not derivable from this repository.
2. **The bundled Claude Desktop upstream identity, or its absence.** Is there any queryable upstream for a version taken from an MSIX package cache directory, and is the npm `@anthropic-ai/claude-code` version even a valid comparison target for it? If not — a legitimate outcome — this slot reports `Unknown { reason }` permanently, and the spec must say so.
3. **Which HTTP client**, weighed on transitive dependency count, licence set under `cargo deny`, MSRV floor, and whether reusing Tauri's already-present async runtime beats a small blocking client.
4. **Whether the chosen HTTP crate joins `deny.toml`'s ban list** with `vertice-app` as sole allowed parent, making core's HTTP-free property mechanically enforced rather than reviewed.
5. **The subject key.** What public value identifies the installation a verdict belongs to, given that `InstallSlot` is private and not a model type? If a public discriminator is required, `client-installation-detector` and `domain-model` gain deltas and the "explicitly NOT modified" list above must be corrected.
6. **Cache policy** — location within the app data directory, TTL, format, and behaviour when the cache is corrupt or unreadable (which must be `Unknown` or a live retry, never a crash).
7. **Prerelease comparison semantics.** Is an installed `0.150.0-rc.1` "outdated" relative to a released `0.150.0`? Is a user on a prerelease newer than the latest stable `UpToDate`, or a fourth state the model does not have? This is a product question with a real wrong answer.
8. **Timeout and retry budget**, and what the UI shows while a lookup is in flight versus when it has given up.

**Deferred, with target:**

- **Skills and agents freshness** — modelled by the discriminator, wired in a later change.
- **macOS and Linux upstream identities** — arrive with those platforms' probe tables.
- **Any update affordance** — a separate, larger decision that would end Vertice's read-only guarantee.

## Strict TDD

`openspec/config.yaml` sets `strict_tdd: true`. These MUST exist and fail before implementation:

- An installed version older than the reference yields `Outdated { latest }` carrying the reference version.
- An installed version equal to the reference yields `UpToDate`.
- An unparseable installed version (an MSIX-style directory name) yields `Unknown`, never `UpToDate` and never a panic.
- A prerelease-shaped installed version (`0.150.0-rc.1`) is compared per the semantics decision above, asserted explicitly rather than left to the parser's default.
- A reference-source stub returning "unavailable" yields `Unknown { reason }` for every subject and **zero** `ScanIssue` values.
- A subject with no known upstream yields `Unknown`, never `UpToDate`.
- No core test performs any network access (CA-17).
- No `File::create`, `OpenOptions::write` or equivalent is introduced outside the app data directory (CA-16).

## Delivery

**Changed-line risk: high; chained PRs recommended.** Three natural slices, each independently green and independently revertible, with final slicing left to `sdd-tasks`:

1. **Core, pure and offline**: the `Freshness` model, the comparison function, the reference-source trait, the stub, the bindings. Ships with zero network code and zero UI; fully testable.
2. **App, the concrete fetcher**: HTTP client, upstream resolution, cache, the IPC command, the setting. This is the slice carrying the dependency decision and the `cargo deny` risk, and it is the one worth reviewing hardest.
3. **Frontend**: the badge, its four states, the i18n keys, the disclosure.

Slice 1 alone leaves `main` with a typed capability nothing consumes — acceptable and self-consistent, since a stub source reports `Unknown` honestly.

## Proposal Question Round

The interactive question round could not be run from this phase. These are the product questions whose answers would change this proposal, each with the assumption currently written into it. Answer, correct, or skip — a second round is available.

| # | Question | Assumption currently written in |
|---|---|---|
| 1 | Is an anonymous read of a public registry genuinely a different category from telemetry for this product, such that it can be **on by default**? Or does "no telemetry by default" mean "no outbound connection by default" in spirit, making opt-in the honest reading? | On by default, with first-run disclosure and a visible off switch; the reasoning and the one-constant fallback are stated above |
| 2 | If Vertice cannot reach the network, is a visible "unknown" badge on every client the right experience, or does that read as the app being broken? Would showing **nothing** be better than showing "unknown" in the offline case? | `Unknown` is always rendered as a first-class state, never hidden — an honest gap beats a silent one |
| 3 | If the bundled Claude Desktop slot turns out to have **no queryable upstream at all**, is a permanent "unknown" acceptable, or should that slot be excluded from the freshness view entirely so it does not look broken? | Permanent `Unknown` with a reason; the slot stays visible |
| 4 | A user on a **prerelease newer than the latest stable** — is that "up to date", or does the product want a distinct state for it? | Unresolved; listed as a design open question, because both answers are defensible and one is wrong for this product |
| 5 | Does "outdated" stay purely informational for the foreseeable future, or is an update affordance a near-term want that should shape the model now rather than later? | Purely informational; an update action would end the read-only guarantee and is a separate, larger decision |
| 6 | Is the first outbound network call worth taking on **now**, or would a first release that ships freshness only for the two slots whose upstream is verifiable (`@anthropic-ai/claude-code`, `opencode-ai`) be a better first slice? | All four slots are in scope, with two reporting `Unknown` until their upstream is verified in design |
