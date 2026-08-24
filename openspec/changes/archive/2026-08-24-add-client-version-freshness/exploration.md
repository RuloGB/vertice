# Exploration — Client Version Freshness Checker

**Change**: `add-client-version-freshness`
**Phase**: sdd-explore (investigation only — no implementation)
**Date**: 2026-08-24

## Context

Vertice should report not just whether an AI client is *found*, but whether the found
installation is **out of date**. Concrete trigger case: the user's OpenCode installation is
behind the latest release. The frontend should be able to surface an "update available"
warning next to the detected version.

Forward-looking constraint stated by the user: the same mechanism will later be applied to
**skills and agents** ("is this skill/agent outdated?"). The freshness abstraction must
generalize beyond `ClientInstallation` without over-engineering the PoC.

## Current State (evidence)

### 1. `version` is an opaque, per-slot-sourced `String`

`ClientInstallation.version: String` (`crates/vertice-core/src/model/installation.rs:16`)
carries no structure, no parse metadata, no notion of "latest". It is populated from three
distinct, incompatible sources selected by `InstallSlot::version_source()`
(`crates/vertice-core/src/installations.rs:161-167`):

- **npm slots** (Claude Code npm, OpenCode npm): `VersionSource::PackageJson` — the
  `"version"` string field of `package.json`, read through the `jsonc.rs` seam
  (`installations.rs:400-458`, extraction at `installations.rs:464-473`).
  `extract_package_json_version` only rejects empty/non-string values; nothing pins semver shape.
- **Claude Code bundled (Claude Desktop MSIX)**: `VersionSource::DirectoryName` — the version
  is literally a directory name under the candidate root, taken as-is with no validation beyond
  UTF-8 (`resolve_bundled_slot`, `installations.rs:482-570`; conversion at
  `install_from_version_dir`, `installations.rs:717-738`).
- **Codex standalone**: `VersionSource::ReleaseDirectoryName` — derived by stripping a known
  target-triple suffix off a release directory name (`split_release_dir_name`,
  `installations.rs:196-206`). Explicitly prerelease-bearing by design: the test
  `split_release_dir_name_is_prerelease_safe` (`installations.rs:899-907`) pins
  `"0.150.0-rc.1-x86_64-pc-windows-msvc"` -> `Some("0.150.0-rc.1")`.

Three clients, three extraction mechanisms, zero shared validation, `String` all the way to
the generated frontend binding.

### 2. `ClientPresenceStatus` is about slot presence, not version usability

`crates/vertice-core/src/model/presence.rs:32-42` states it explicitly: `Detected` means
"the slot exists", **not** "a usable version was extracted". A `Detected` record with empty
`installations` (present but broken) is a deliberately representable state. Overloading this
enum with freshness would collapse two distinct meanings.

### 3. No network client and no semver crate exist in the workspace

`crates/vertice-core/Cargo.toml` direct dependencies: `jsonc-parser`, `serde`, `serde_norway`,
`thiserror`, `toml`, `ts-rs`, `unicode-normalization`, `walkdir`. No `semver`, no HTTP crate.
`crates/vertice-app/Cargo.toml` depends only on `vertice-core` and `tauri`.

`deny.toml` bans `tauri`/`tauri-build` outside `vertice-app` but does not ban network crates by
name. The stronger, more local constraint is the `src/model/` import allow-list documented in
`AGENTS.md` and repeated in each module header: `std::path`, `std::time::Duration`, `serde`,
`ts_rs`, `thiserror`, `unicode_normalization` — `std::fs`, `std::io`, `std::env`, `SystemTime`,
`Instant` are forbidden.

### 4. CA-16 governs writes, not network

`internal-docs/alcance-poc-vertice.md` and CA-16 constrain filesystem **writes** outside the app
data directory. They do not by themselves forbid outbound network calls. However
`internal-docs/stack-tecnologico-vertice.md:108` fixes the architecture for any future public
registry lookup: it is performed by the Rust process, never the WebView, and responses are
treated as untrusted data. The same document (`:115`) states there is no telemetry by default,
and that any future addition is explicit opt-in.

### 5. The PoC exclusion no longer governs — this is final-product work

`internal-docs/alcance-poc-vertice.md:37`, under `### Fuera`, excluded exactly this feature
("Comparación con upstream y estado de actualización… la de desactualización, no"), alongside
public registry lookups (`:39`).

**That document describes a completed stage and no longer constrains new work.** The PoC is
closed; this change is the first of the final product. The exclusion is recorded here as
provenance — it explains why `version` is an unvalidated `String` today and why no HTTP
dependency exists — not as a blocker.

What *does* still govern are the structural invariants in `AGENTS.md`, which are architectural
rather than scope-based: `vertice-core` stays Tauri-free, `model/` stays I/O-free, one module owns
each parser seam, and CA-16's read-only rule (no writes outside the app data directory). Those
shape *where* the network code may live, not *whether* the feature is allowed.

### 6. Roadmap references cleaned up (resolved 2026-08-24)

`AGENTS.md`, `openspec/config.yaml`, ten living specs and three Rust doc comments cited
`internal-docs/plan-desarrollo-poc.md`, a document removed when the PoC closed. All live
references were removed in this change's preparation: `AGENTS.md` now states the PoC is complete
and that new work is scoped from `openspec/specs/`; `config.yaml`'s proposal and tasks rules trace
to living capability specs instead of T-phases; spec purposes read "Traces to T*n* of the completed
PoC roadmap" as provenance. The 61 files under `openspec/changes/archive/` keep their original
citations — they are the historical record of closed cycles and were deliberately left untouched.

### 7. Frontend surface today

`frontend/src/lib/pages/ClientsPage.svelte` renders one card per `ClientKind`, joins all
`installations[].version` with `", "` (line 63) and shows a binary detected/not-detected badge
(lines 77-79). There is no freshness slot in the UI. Adding one means a new card element plus
new i18n keys in the existing `clients.*` namespace (`frontend/src/lib/i18n/catalogs.ts`), in
both English and Spanish per design principle 7.

## Two separable decisions

The feature splits into two questions that are easy to conflate:

- **Architecture** — where does the comparison live, and what shape does the verdict take? This one
  has a clear answer that the existing invariants dictate (see "Recommended Direction").
- **Data source** — where does the "latest known version" come from? This is a genuine product fork
  with real tradeoffs, listed below.

Dependency inversion resolves the first independently of the second: `vertice-core` compares
`installed` against a `reference` it receives as an input, so the source can change — or be
swapped per client — without touching core.

## Options and Tradeoffs — where does "latest version" come from?

### Option A — Live network lookup (npm registry / GitHub Releases API)

Rust-side HTTP call, never from the WebView, to `registry.npmjs.org/@anthropic-ai/claude-code` and
`.../opencode` for the npm slots, and the GitHub Releases API for Codex and the bundled Claude
Desktop slot.

- **Accuracy**: the only option that actually answers "is my OpenCode behind the real latest".
- **Where it lives**: `vertice-app`, not `vertice-core`. `model/` bans `std::io`, and core is meant
  to stay reusable by a headless CLI with a small, auditable dependency footprint (8 direct deps
  today). `reqwest` pulls `hyper`, `tokio` and a TLS backend — dozens of transitive crates to vet
  under `cargo deny check licenses`. A lighter client (`ureq`, blocking, rustls) is worth weighing
  against Tauri's already-present async runtime.
- **Costs**: offline behaviour must degrade to `Unknown`, never to a hang or an error state;
  unauthenticated GitHub API is rate-limited to 60 req/h per IP, so a cache in the app data
  directory (the sanctioned write location under CA-16) is effectively mandatory, not an
  optimization; the response schema is untrusted input and must be parsed defensively
  (`stack-tecnologico-vertice.md:108`); an outbound call at startup is a privacy-visible behaviour
  that deserves an explicit setting even though it carries no user data.
- **Testing**: needs a seam. Core tests must never touch the network.

### Option B — Pinned manifest shipped with the app

A static latest-version table compiled into the binary (like `CODEX_TARGET_TRIPLES`,
`installations.rs:191`), or a small JSON manifest fetched once per Vertice release.

- **Accuracy**: degrades the moment a client ships a release, and Vertice's own release cadence
  becomes the ceiling on freshness accuracy. It would routinely tell the user "up to date" when
  they are not — the exact false negative worth avoiding.
- **Costs**: zero new dependencies, fully offline, trivially testable.
- **Verdict**: acceptable only as a fallback layer beneath A, never as the primary source.

### Option C — Local-only comparison between detected installations

Compare a client's own installations against each other.

- Answers a different question ("which of my installs is newest"), already derivable from the
  existing data. Does not deliver the feature. Listed for completeness; not recommended.

### Option D — User-supplied reference

Let the user paste or configure the expected version per client.

- Honest and offline, but it makes the user do the work the tool exists to do. Only defensible as
  an escape hatch for air-gapped setups.

## Architectural Constraints

- The `model/` import allow-list forbids `std::io`, so no fetch can live there. By the crate's own
  layering, a fetch should not live in `vertice-core` at all if it is to stay swappable and
  testable without a live network. Precedent: `yaml.rs` and `jsonc.rs` are the only modules allowed
  to import their parsing crates; everything else goes through a seam function. A future fetch
  module needs the same discipline — one `freshness_source.rs`-shaped seam, mockable in tests.
- ts-rs type contract: any new public model type must derive `TS` with
  `export_to = "../../../frontend/src/bindings/"`, be regenerated by `cargo test -p vertice-core`,
  and be committed alongside the Rust change or CI's bindings-diff check fails.
- CA-16 plus no-telemetry-by-default mean even a local "last known latest" cache must live in the
  app's own data directory.

## Version Comparison Semantics

- None of the three sources is guaranteed semver. npm `package.json` versions are conventionally
  semver but unenforced by Vertice's extraction. The bundled slot version is a raw directory name.
  The Codex version legitimately carries an `-rc.1`-style suffix that a naive `x.y.z` parser
  mishandles.
- No `semver` crate is a dependency anywhere in the workspace today. Adding `semver`
  (MIT / Apache-2.0, dependency-light) passes the current `deny.toml` license allow-list at
  negligible transitive cost. This is the cheap, low-risk part of the feature.
- Comparison must be **three-valued**, not binary: `UpToDate`, `Outdated`, and `Unknown { reason }`
  for when either side fails to parse, or — the PoC's likely reality — no reference source exists.
  Collapsing "cannot tell" into "up to date" is a misleading false negative; collapsing it into
  "outdated" is a false positive. An honest "unknown" beats both.

## Model Shape and Generalization

Candidate, as a new module rather than an overload:

```rust
// crates/vertice-core/src/model/freshness.rs (new)
pub enum Freshness {
    UpToDate,
    Outdated { latest: String },
    Unknown { reason: String },
}
```

Where it hangs — three candidates evaluated:

1. **On `ClientInstallation`** (`installation.rs:14-18`, add `pub freshness: Freshness`) —
   simplest, but couples a best-effort verdict requiring an external reference to a struct that
   today is pure on-disk fact. Every hand-constructed `ClientInstallation` in the existing tests
   (e.g. `installations.rs:753-757`) breaks at compile time the moment the field lands.
2. **On `ClientPresence`** — wrong. That type is deliberately scoped to slot existence
   (`presence.rs:32-35`), and one presence record may carry many installations (CA-7), so a single
   freshness verdict there is ambiguous about which installation it describes.
3. **As a separate keyed report** — e.g. `FreshnessCheck { subject_kind, subject_id, verdict }`
   collected alongside the scan result rather than nested into the domain types. This generalizes
   directly to the announced skills/agents freshness feature without forcing every domain type to
   carry an optional field it may never populate, and mirrors the existing parallel-record pattern
   (`InstallationScan.presence: Option<Vec<ClientPresence>>` alongside `installations`,
   `installations.rs:36-44`).

**Recommendation: option 3.** It is the only shape that requires no breaking change to existing
`ClientInstallation`-constructing tests and is what "generalizes to skills and agents" without
over-engineering — additive, empty by default.

## Recommended Direction

1. Model the verdict now as a new additive `model/freshness.rs`: a pure `Freshness` enum
   (`UpToDate` / `Outdated { latest }` / `Unknown { reason }`), `TS`-derived, following the exact
   pattern of `ClientPresenceStatus`. Zero I/O, zero new deny risk.
2. Model the pure comparison in `vertice-core` (not in `model/`, mirroring where `installations.rs`
   sits) as `compare_versions(installed: &str, reference: &str) -> Freshness`, taking the reference
   as a plain input. Add `semver` for parsing, with `Unknown` as the fallback whenever either side
   fails to parse.
3. **Put the reference-version source behind a trait seam owned by one module.** Core depends on
   the abstraction (`trait ReferenceVersions { fn latest_for(&self, subject) -> Option<String> }`),
   never on a concrete fetcher. This is the same discipline as `yaml.rs` and `jsonc.rs`: one module
   owns the outside world, everything else goes through it. Core tests inject a fixed stub; no
   network, no fixture drift.
4. **Implement Option A (live lookup) as the first concrete source, in `vertice-app`**, with:
   offline and rate-limited responses degrading to `Unknown { reason }` rather than to an error;
   a response cache in the app data directory (the only sanctioned write location, CA-16); and
   defensive parsing of registry responses as untrusted input
   (`stack-tecnologico-vertice.md:108`). Keep Option B available as a compiled-in fallback if the
   first release should still say something useful with no network.
5. **Make the check explicit and non-blocking.** The scan must complete and render without waiting
   on the network; freshness arrives as a second, later result. An outbound call at startup is
   user-visible behaviour and deserves a setting, per the no-outbound-by-default posture
   (`stack-tecnologico-vertice.md:115`) — even though no user data leaves the machine.
6. Frontend: a badge beside the existing version line in `ClientsPage.svelte`, driven by the new
   report, with three i18n states (`clients.freshnessUpToDate`, `clients.freshnessOutdated`,
   `clients.freshnessUnknown`) in both `en` and `es`, matching the existing `clients.*` namespace,
   plus a pending state while the lookup is in flight.

## Risks

| # | Risk | Notes |
| - | ---- | ----- |
| 1 | **First outbound call in the product's history** | Vertice has never made one. It changes the app's privacy posture, its offline behaviour, and its dependency audit surface. Worth a deliberate decision and a setting, not a silent addition. |
| 2 | **Version strings**: three extraction mechanisms, none semver-enforced | `semver::Version::parse` must resolve failures to `Unknown`, never panic or silently skip. The MSIX directory-name version is the likeliest to fail parsing. |
| 3 | **Reference identity mismatch** | The npm package name, the GitHub repo, and the MSIX bundle version are three different namespaces. Mapping a `ClientKind` + install slot to the right upstream identifier is per-slot work, not one global rule. |
| 4 | **Rate limits and caching** | Unauthenticated GitHub API allows ~60 req/h per IP. Without a cache in the app data directory, a user who reloads often gets throttled into permanent `Unknown`. |
| 5 | **Type contract**: new public model types require the regenerated binding in the same commit | Otherwise CI's bindings-diff check fails. |
| 6 | **Scan latency**: freshness must not block the scan | CA-15's <2s scan budget must survive; the lookup is asynchronous and its absence must never delay first render. |

## Open Questions (prioritized, for the user)

1. ~~**Data source.**~~ **DECIDED (2026-08-24): Option A — live lookup against the npm registry and
   the GitHub Releases API.** Vertice takes on its first outbound call and an HTTP dependency in
   `vertice-app`. Option B (pinned manifest) is not adopted, not even as a fallback layer, unless
   design finds a concrete reason. Offline and rate-limited responses degrade to
   `Unknown { reason }`.
2. **Default behaviour.** If the lookup is live: on by default with an opt-out, or off by default
   with an opt-in? This is a product posture decision, not a technical one.
3. **Generalization now or later.** Should the design already model the keyed report with a
   `subject_kind` covering skills and agents — even if only `ClientInstallation` is wired in this
   change — or stay client-only and generalize when the skills/agents freshness feature is
   scheduled? Modelling it now costs little; wiring it now costs a lot.
4. **Update affordance.** Does "outdated" stay purely informational, or does the product eventually
   offer to update? Vertice is read-only today; an update action would be its first mutation of the
   user's machine and a much larger decision than this change.

## SDD Result Contract

- **status**: done
- **next_recommended**: `sdd-propose` — the architecture is settled; open question 1 (data source)
  should be answered first, since it decides whether the proposal includes an HTTP dependency.
- **skill_resolution**: none (no registry skill matches a read-only Rust architecture exploration)
