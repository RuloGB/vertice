//! `component-freshness`'s core-side behavior: a total, pure, synchronous
//! comparison (`compare`), the reference-source seam `vertice-core` depends
//! on (`ReferenceVersions`, `ReferenceLookup`), its in-memory stub
//! (`MapReferenceVersions` — the test stub AND the app's production
//! adapter), and `evaluate`, which turns a batch of `(subject, installed)`
//! pairs into `FreshnessCheck`s through that seam.
//!
//! This module names no HTTP crate, no clock, and no filesystem primitive.
//! `vertice-core` has no HTTP dependency at all (design §10), so no test
//! here *can* open a socket without adding one, and `deny.toml` fails CI
//! the moment anyone tries. The app fetches every reference version first,
//! then calls [`evaluate`] with the results as plain data.

use crate::model::{Freshness, FreshnessCheck, FreshnessSubject};

/// Outcome of asking a [`ReferenceVersions`] source for one subject's
/// reference. Distinguishes "no known upstream identity exists at all"
/// (never issues a request) from "an upstream identity exists but the
/// value could not currently be obtained" (network/cache failure) — both
/// degrade to [`Freshness::Unknown`] in [`evaluate`], but for different,
/// individually loggable reasons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceLookup {
    Found(String),
    NoUpstream { reason: String },
    Unavailable { reason: String },
}

/// The seam `vertice-core` depends on to obtain a reference version, never
/// a concrete HTTP client or any other I/O primitive (design §10). The app
/// fetches first, then calls [`evaluate`] through this trait — core has no
/// async, no runtime, no executor concept.
pub trait ReferenceVersions {
    fn latest_for(&self, subject: &FreshnessSubject) -> ReferenceLookup;
}

/// Ships in core. The test stub AND the app's production adapter: a plain
/// list of `(subject, lookup)` pairs. A subject with no configured entry
/// reports [`ReferenceLookup::Unavailable`] — "asked, but nothing was
/// obtained" — never [`ReferenceLookup::NoUpstream`], which is reserved for
/// a subject whose upstream identity is knowingly absent (design §6).
#[derive(Debug, Clone, Default)]
pub struct MapReferenceVersions {
    entries: Vec<(FreshnessSubject, ReferenceLookup)>,
}

impl MapReferenceVersions {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Builder-style: register one subject's lookup outcome.
    pub fn with(mut self, subject: FreshnessSubject, lookup: ReferenceLookup) -> Self {
        self.entries.push((subject, lookup));
        self
    }
}

impl ReferenceVersions for MapReferenceVersions {
    fn latest_for(&self, subject: &FreshnessSubject) -> ReferenceLookup {
        self.entries
            .iter()
            .find(|(configured, _)| configured == subject)
            .map(|(_, lookup)| lookup.clone())
            .unwrap_or_else(|| ReferenceLookup::Unavailable {
                reason: "no reference configured for this subject".to_string(),
            })
    }
}

/// Total, pure comparison: every `(installed, reference)` pair yields a
/// `Freshness`, never a panic, never an error type. A version string that
/// fails to parse on either side resolves to `Unknown` (component-freshness
/// spec). Ordering uses standard semver precedence without special-casing,
/// which already gives a prerelease the correct placement relative to its
/// own release (design §7): `Outdated` only when `installed` is strictly
/// older than `reference`; equal-or-newer is always `UpToDate`.
pub fn compare(installed: &str, reference: &str) -> Freshness {
    let installed = semver::Version::parse(installed);
    let reference_version = semver::Version::parse(reference);

    match (installed, reference_version) {
        (Ok(installed), Ok(reference_version)) => {
            if installed < reference_version {
                Freshness::Outdated {
                    latest: reference.to_string(),
                }
            } else {
                Freshness::UpToDate
            }
        }
        _ => Freshness::Unknown {
            reason: "could not parse one or both version strings as semver".to_string(),
        },
    }
}

/// Turn a batch of `(subject, installed version)` pairs into
/// `FreshnessCheck`s by asking `source` for each subject's reference and
/// running [`compare`]. Total, pure, synchronous — never produces a
/// `ScanIssue` or any other diagnostic-channel side effect (spec: "Freshness
/// Lookups Never Enter The Scan Diagnostic Channel").
pub fn evaluate(
    source: &impl ReferenceVersions,
    subjects: &[(FreshnessSubject, String)],
) -> Vec<FreshnessCheck> {
    subjects
        .iter()
        .map(|(subject, installed)| {
            let verdict = match source.latest_for(subject) {
                ReferenceLookup::Found(reference) => compare(installed, &reference),
                ReferenceLookup::NoUpstream { reason } => Freshness::Unknown { reason },
                ReferenceLookup::Unavailable { reason } => Freshness::Unknown { reason },
            };
            FreshnessCheck {
                subject: subject.clone(),
                installed: installed.clone(),
                verdict,
            }
        })
        .collect()
}
