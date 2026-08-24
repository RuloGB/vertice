# Component Freshness Specification

## Purpose

Define the vocabulary and contract for answering "is this detected component out of date": a three-valued verdict, a total (never-panicking) comparison rule, the reference-source abstraction core depends on, the per-subject upstream-identity mapping rule, the degradation contract for network and cache failures, and the default privacy posture governing the outbound lookup this capability introduces. This is Vertice's first capability with a network-facing side effect; every requirement below is written to keep that side effect optional, disclosed, and fail-safe.

## Requirements

### Requirement: Freshness Is A Closed Three-Valued Verdict

`Freshness` MUST be a closed enum (no `#[non_exhaustive]`) with exactly three variants: `UpToDate`, `Outdated` (carrying the reference version compared against), and `Unknown` (carrying a reason). No two-valued collapse is permitted: an inability to determine freshness MUST NOT be represented as `UpToDate` or as `Outdated`.

#### Scenario: Freshness is exhaustively matchable

- GIVEN a `match` over every `Freshness` variant with no wildcard arm
- WHEN the code is compiled
- THEN compilation succeeds, proving the enum is closed

#### Scenario: An installed version older than the reference is Outdated

- GIVEN an installed version string and a reference version string where the installed version is older
- WHEN the comparison runs
- THEN the result is `Outdated`, carrying the reference version

#### Scenario: An installed version equal to the reference is UpToDate

- GIVEN an installed version string equal to the reference version string
- WHEN the comparison runs
- THEN the result is `UpToDate`

### Requirement: Version Comparison Is Total And Fails Closed To Unknown

The comparison function MUST accept an installed version string and a reference version string and MUST return a `Freshness` for every possible input pair — it MUST NOT panic and MUST NOT return an error type. A version string that fails to parse on either side MUST resolve to `Unknown`, never to a guess, never to `UpToDate`, and never to a silently skipped comparison. This MUST hold regardless of which of the four version-extraction mechanisms produced the string, including a non-semver directory-name-shaped value.

The comparison MUST order an installed prerelease version (e.g. carrying an `-rc.1`-style suffix) against a stable reference using standard semantic-version precedence (a prerelease sorts before its own release), applied without special-casing. This produces exactly two prerelease outcomes, and no others: an installed prerelease that is older than the reference under that ordering MUST yield `Outdated`, carrying the reference version — the user is on a release candidate of a version that has since shipped. An installed prerelease that is newer than the reference under that ordering MUST yield `UpToDate` — the verdict answers "should I update?", and a user already ahead of the latest available reference has no update to make. Neither case introduces a fourth verdict variant; the three-valued vocabulary is exhaustive. Both prerelease outcomes MUST be asserted by dedicated tests, not left to a parsing library's default ordering behavior going unverified.

#### Scenario: An unparseable installed version yields Unknown, never a panic

- GIVEN an installed version string in a non-semver, directory-name shape and a valid reference version string
- WHEN the comparison runs
- THEN the result is `Unknown`, and no panic occurs

#### Scenario: An installed prerelease older than the reference is Outdated

- GIVEN an installed version `0.150.0-rc.1` and a reference version `0.150.0`
- WHEN the comparison runs
- THEN the result is `Outdated`, carrying `"0.150.0"` as the reference version

#### Scenario: An installed prerelease newer than the reference is UpToDate

- GIVEN an installed version `0.151.0-rc.1` and a reference version `0.149.1`
- WHEN the comparison runs
- THEN the result is `UpToDate`, never `Outdated` and never a fourth, "ahead" state

### Requirement: Core Depends On A Reference-Source Abstraction, Never A Concrete Fetcher

`vertice-core` MUST obtain the reference version through a trait it defines and depends on, never through a concrete HTTP client or any other I/O primitive. Core tests MUST exercise this behavior exclusively through a fixed, in-memory stub implementation of that trait. No core test MUST perform any network access.

#### Scenario: A stub source returning "unavailable" yields Unknown for every subject

- GIVEN a reference-source stub configured to report no reference version for any subject
- WHEN freshness is evaluated for a set of subjects through the stub
- THEN every subject's `Freshness` is `Unknown`, and zero diagnostic-channel entries are produced as a side effect

#### Scenario: No core test performs network access

- GIVEN the full `vertice-core` test suite
- WHEN it runs in an environment with no network access
- THEN every freshness-related test still passes

### Requirement: A Subject With No Known Upstream Is Permanently Unknown, Never UpToDate, And Issues No Request

Each freshness subject maps to at most one upstream identity. A subject for which no upstream identity is known or resolvable — including the client-installation subject with no established queryable upstream at all — MUST report `Unknown` and MUST NEVER report `UpToDate`. This MUST hold even if the subject's installed version happens to be syntactically comparable to some other subject's reference — an unverified upstream mapping MUST NOT be assumed or guessed. For such a subject, no outbound network request MUST be made at all: absence of a known upstream is determined before any request would be issued, not discovered by a request that then fails.

#### Scenario: A subject with no known upstream never reports UpToDate

- GIVEN a subject for which the reference source has no known upstream mapping
- WHEN freshness is evaluated for that subject
- THEN the result is `Unknown`, carrying a reason, and is never `UpToDate`

#### Scenario: A subject with no known upstream triggers no outbound request

- GIVEN a subject for which no upstream identity is established
- WHEN freshness is evaluated for that subject
- THEN no outbound network request is made for it, and its verdict is `Unknown` by construction rather than by request failure

### Requirement: Network And Cache Failures Degrade To Unknown, Never A Crash Or An Error State

Any failure while obtaining a reference version — an unreachable network, a rate-limited or erroring response, a request timeout, an unparseable response body, or a corrupt or unreadable cache entry — MUST resolve to `Freshness::Unknown` carrying a descriptive reason. None of these conditions MUST produce a panic, an unhandled error surfaced to the caller, or a fallback to `UpToDate`. A malformed or unexpectedly large response body MUST be treated as untrusted input and MUST NOT be trusted to produce a version string without validation.

#### Scenario: An unreachable network yields Unknown

- GIVEN the reference source cannot reach the network for a given subject
- WHEN freshness is evaluated for that subject
- THEN the result is `Unknown`, carrying a reason, and no panic or unhandled error occurs

#### Scenario: A corrupt cache entry yields Unknown or a live retry, never a crash

- GIVEN a cache entry for a subject that is corrupt or unreadable
- WHEN freshness is evaluated for that subject
- THEN the result is either `Unknown` or the outcome of a fresh live lookup, and evaluation never panics or crashes the caller

### Requirement: Freshness Lookups Never Enter The Scan Diagnostic Channel

A failed or degraded freshness lookup MUST NOT produce a `ScanIssue` and MUST NOT be represented through any incident/diagnostic carrier used by the scan. It is represented exclusively as `Freshness::Unknown`.

#### Scenario: A failed lookup produces zero diagnostic-channel entries

- GIVEN a reference-source failure for one or more subjects
- WHEN freshness is evaluated
- THEN the resulting `Unknown` verdicts are not accompanied by any `ScanIssue` or equivalent diagnostic entry

### Requirement: A Reference Version Extracted From An Upstream Release Field Ordering Is Never Trusted On A Single Field Alone

For an upstream whose release record exposes more than one candidate field that could carry the version (for example, a release's display name and a separate tag identifier), the reference-version extraction MUST evaluate candidate fields in a fixed, ordered sequence, and MUST fall through to the next candidate whenever the current one does not yield a valid, parseable version. A candidate field known to carry a non-version prefix or decoration (such as a release-train prefix on a tag) MUST have that decoration stripped before the value is treated as a version string; a raw, prefix-carrying value MUST NOT be parsed or compared as a version as-is. If no candidate in the sequence yields a valid version, the outcome MUST be the same `Unknown`/no-known-upstream-shaped failure as any other unparseable reference, never a guess and never the raw undecorated string treated as a version.

#### Scenario: A reference field with a version-train prefix is not compared as-is

- GIVEN an upstream release record whose only usable version-bearing candidate carries a leading release-train prefix (e.g. `rust-v0.149.1`)
- WHEN the reference version is extracted
- THEN the prefix is stripped before the value is parsed as a version
- AND the raw, prefix-carrying string is never used as the reference version in a comparison

#### Scenario: The first candidate field is preferred when it parses

- GIVEN an upstream release record whose first candidate field is already a valid, undecorated version string
- WHEN the reference version is extracted
- THEN that first candidate is used, and no fallback candidate is consulted

#### Scenario: Extraction falls through to the next candidate when the first fails to parse

- GIVEN an upstream release record whose first candidate field does not yield a valid version, but whose second candidate does after any required decoration is stripped
- WHEN the reference version is extracted
- THEN the second candidate's value is used
- AND the result is never a value assembled by silently merging or guessing across candidates

### Requirement: The Response Cache Is The Only New Write, Confined To The App Data Directory

The only filesystem write introduced by this capability is a cache of reference-lookup responses. That cache MUST be located inside the application's own data directory (CA-16) and MUST NOT be written anywhere else. Its presence or absence MUST NOT change correctness: deleting the cache MUST NOT cause a crash, only a live lookup or `Unknown`.

#### Scenario: The cache write stays inside the app data directory

- GIVEN a completed live reference lookup that is cached
- WHEN the filesystem is inspected for new writes introduced by this capability
- THEN the only new write is located inside the application data directory

#### Scenario: A missing cache degrades gracefully

- GIVEN the cache file does not exist
- WHEN freshness is evaluated
- THEN the outcome is a live lookup or `Unknown`, never a crash

### Requirement: The Check Is Enabled By Default, Disclosed, And Fully Stoppable

The freshness check MUST be enabled by default. On first exposure to the feature, the user MUST be shown a disclosure stating that public registries are queried and that no information about the user or their machine is sent. A visible, discoverable setting MUST let the user disable the check. Once disabled, the application MUST NOT make any further outbound request for freshness purposes of any kind, for any subject, until the check is re-enabled. Every outbound request MUST carry no unique identifier, no machine fingerprint, no inventory data, no component names, and no filesystem paths.

#### Scenario: Disabling the check stops all further outbound requests

- GIVEN the freshness check is disabled through the setting
- WHEN a subsequent scan or rescan occurs
- THEN no outbound freshness request of any kind is made for any subject

#### Scenario: An outbound request carries no identifying content

- GIVEN a live reference-version request is made
- WHEN its content is inspected
- THEN it contains no unique identifier, no machine fingerprint, no inventory data, no component name, and no filesystem path

#### Scenario: The disclosure is shown before or at the first check

- GIVEN the user has not previously seen the freshness disclosure
- WHEN the feature is first exposed
- THEN the disclosure text, stating that public registries are queried and that nothing about the user is sent, is shown before or alongside the first outbound request
