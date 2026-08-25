//! Wiring for `component-freshness` (design §1, §8, §9): resolve every
//! subject's reference version (cache-hit, live fetch, or stale fallback),
//! then hand the results to `vertice_core::freshness::evaluate` for the
//! pure, synchronous comparison. This module is the only place that
//! decides *whether* to make a request; `fetch.rs` is the only place that
//! *makes* one.

pub mod cache;
pub mod fetch;
pub mod upstream;

use std::collections::HashMap;
use std::path::Path;

use vertice_core::freshness::{MapReferenceVersions, ReferenceLookup};
use vertice_core::model::{ClientPresence, FreshnessReport, FreshnessSubject};

use cache::{CacheEntry, FreshnessStore};
use upstream::UpstreamIdentity;

/// Every `(subject, installed version)` pair carried by `presence`,
/// flattened across every slot's installations (CA-7: a slot may carry
/// many).
fn subjects_from_presence(
    presence: &Option<Vec<ClientPresence>>,
) -> Vec<(FreshnessSubject, String)> {
    let Some(records) = presence else {
        return Vec::new();
    };

    records
        .iter()
        .flat_map(|record| {
            record.installations.iter().map(|installation| {
                (
                    FreshnessSubject::ClientInstallation {
                        slot: record.slot,
                        path: installation.path.clone(),
                    },
                    installation.version.clone(),
                )
            })
        })
        .collect()
}

/// Resolve one identity's reference lookup: cache hit within TTL, else a
/// live fetch, else a stale cache entry within the 7-day ceiling, else
/// `Unavailable` (design §8).
async fn resolve_identity(
    client: &reqwest::Client,
    identity: &UpstreamIdentity,
    store: &FreshnessStore,
    now: u64,
) -> ReferenceLookup {
    let cached = store.cache.get(&identity.cache_key());

    if let Some(entry) = cached {
        if cache::is_fresh(entry, now) {
            return ReferenceLookup::Found(entry.version.clone());
        }
    }

    let fetched = fetch::fetch_reference(client, identity).await;
    match fetched {
        ReferenceLookup::Found(version) => ReferenceLookup::Found(version),
        ReferenceLookup::Unavailable { reason } => match cached {
            Some(entry) if cache::is_within_stale_ceiling(entry, now) => {
                ReferenceLookup::Found(entry.version.clone())
            }
            _ => ReferenceLookup::Unavailable { reason },
        },
        ReferenceLookup::NoUpstream { reason } => ReferenceLookup::NoUpstream { reason },
    }
}

/// Build the full report for the already-scanned `presence`. Setting off
/// -> `{ enabled: false, checks: [] }`, no request, no cache read (design
/// §1, §12). Setting on -> resolve every distinct upstream identity
/// actually needed (concurrently, once per identity, never once per
/// installation), then run the pure core comparison. Infallible: every
/// failure degrades to `Unknown` inside a successful report, never a
/// rejected call (component-freshness spec).
pub async fn build_report(
    app_data_dir: &Path,
    presence: Option<Vec<ClientPresence>>,
) -> FreshnessReport {
    let store_path = cache::store_path(app_data_dir);
    let mut store = cache::load(&store_path);

    if !store.enabled {
        return FreshnessReport {
            enabled: false,
            checks: vec![],
        };
    }

    let subjects = subjects_from_presence(&presence);
    let now = cache::now_unix_s();

    // Every distinct upstream identity actually required by a detected
    // subject — never more, so a slot nobody has installed is never
    // queried, and two installations of the same slot never double up.
    let mut identities: Vec<UpstreamIdentity> = Vec::new();
    for (subject, _) in &subjects {
        let FreshnessSubject::ClientInstallation { slot, .. } = subject;
        if let Some(identity) = upstream::upstream_for(*slot) {
            if !identities.contains(&identity) {
                identities.push(identity);
            }
        }
    }

    let mut resolved: HashMap<String, ReferenceLookup> = HashMap::new();
    if !identities.is_empty() {
        // Client construction failing here is a fetch-layer transport
        // problem, not a report failure: every identity degrades to
        // `Unavailable` rather than the whole command erroring out.
        match fetch::build_client() {
            Ok(client) => {
                // Each identity's lookup runs on its own task on Tauri's
                // existing runtime (design §9), so the wall-clock budget
                // for the whole batch is one request's timeout, not the
                // sum of every request's timeout.
                let handles: Vec<(UpstreamIdentity, _)> = identities
                    .iter()
                    .cloned()
                    .map(|identity| {
                        let client = client.clone();
                        let store_snapshot = store.clone();
                        let identity_for_task = identity.clone();
                        let handle = tauri::async_runtime::spawn(async move {
                            resolve_identity(&client, &identity_for_task, &store_snapshot, now)
                                .await
                        });
                        (identity, handle)
                    })
                    .collect();

                for (identity, handle) in handles {
                    let outcome =
                        handle
                            .await
                            .unwrap_or_else(|join_err| ReferenceLookup::Unavailable {
                                reason: format!("reference lookup task failed: {join_err}"),
                            });
                    if let ReferenceLookup::Found(version) = &outcome {
                        store.cache.insert(
                            identity.cache_key(),
                            CacheEntry {
                                version: version.clone(),
                                fetched_at_unix_s: now,
                            },
                        );
                    }
                    resolved.insert(identity.cache_key(), outcome);
                }
            }
            Err(err) => {
                for identity in &identities {
                    resolved.insert(
                        identity.cache_key(),
                        ReferenceLookup::Unavailable {
                            reason: format!("could not build HTTP client: {err}"),
                        },
                    );
                }
            }
        }
    }

    let mut reference_map = MapReferenceVersions::new();
    for (subject, _) in &subjects {
        let FreshnessSubject::ClientInstallation { slot, .. } = subject;
        let lookup = match upstream::upstream_for(*slot) {
            None => ReferenceLookup::NoUpstream {
                reason: format!("{} has no established queryable upstream", slot.label()),
            },
            Some(identity) => resolved
                .get(&identity.cache_key())
                .cloned()
                .unwrap_or_else(|| ReferenceLookup::Unavailable {
                    reason: "reference lookup was not resolved".to_string(),
                }),
        };
        reference_map = reference_map.with(subject.clone(), lookup);
    }

    let checks = vertice_core::freshness::evaluate(&reference_map, &subjects);

    // Persist whatever cache entries were updated, best-effort: a write
    // failure degrades this run's checks not at all (they are already
    // computed), and the next run simply refetches (design §8's
    // corrupt-as-empty handling covers a torn or unwritable file too). The
    // returned checks are unaffected either way — only the silence
    // becomes evidence (design §9).
    if let Err(err) = cache::save(&store_path, &store) {
        log::warn!("could not persist freshness store: {err}");
    }

    FreshnessReport {
        enabled: true,
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use vertice_core::model::{
        ClientInstallSlot, ClientInstallation, ClientKind, ClientPresenceStatus, Freshness,
    };

    static UNIQUE: AtomicU64 = AtomicU64::new(0);

    fn temp_app_data_dir(label: &str) -> std::path::PathBuf {
        let unique = UNIQUE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vertice-freshness-mod-test-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp test dir must be creatable");
        dir
    }

    fn bundled_presence() -> Vec<ClientPresence> {
        vec![ClientPresence {
            slot: ClientInstallSlot::ClaudeCodeBundled,
            label: ClientInstallSlot::ClaudeCodeBundled.label().to_string(),
            probed_paths: vec!["C:\\fixture\\path".into()],
            status: ClientPresenceStatus::Detected,
            installations: vec![ClientInstallation {
                client: ClientKind::ClaudeCode,
                version: "some-msix-directory-name".to_string(),
                path: "C:\\fixture\\path\\claude-code".into(),
            }],
        }]
    }

    /// The load-bearing pin, exercised at the app-orchestration level: a
    /// slot with no known upstream (`ClaudeCodeBundled`) never reports
    /// `UpToDate`, and issues no request to resolve it — this test cannot
    /// touch the network even by accident, because no `UpstreamIdentity`
    /// is ever constructed for it.
    #[test]
    fn no_upstream_subject_degrades_to_unknown_without_any_network_call() {
        let app_data_dir = temp_app_data_dir("no-upstream");

        let report =
            tauri::async_runtime::block_on(build_report(&app_data_dir, Some(bundled_presence())));

        assert!(report.enabled);
        assert_eq!(report.checks.len(), 1);
        match &report.checks[0].verdict {
            Freshness::Unknown { .. } => {}
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn disabled_setting_yields_an_empty_disabled_report_and_reads_no_subjects() {
        let app_data_dir = temp_app_data_dir("disabled");
        let store_path = cache::store_path(&app_data_dir);
        let store = FreshnessStore {
            enabled: false,
            ..FreshnessStore::default()
        };
        cache::save(&store_path, &store).expect("test setup save must succeed");

        let report =
            tauri::async_runtime::block_on(build_report(&app_data_dir, Some(bundled_presence())));

        assert!(!report.enabled);
        assert!(report.checks.is_empty());
    }

    #[test]
    fn no_presence_at_all_yields_an_enabled_report_with_zero_checks() {
        let app_data_dir = temp_app_data_dir("no-presence");

        let report = tauri::async_runtime::block_on(build_report(&app_data_dir, None));

        assert!(report.enabled);
        assert!(report.checks.is_empty());
    }
}
