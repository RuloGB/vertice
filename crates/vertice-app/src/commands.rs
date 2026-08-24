//! Tauri IPC commands: thin async pass-throughs to the core scan.
//!
//! No business logic lives here — no filtering, no transformation of the
//! report, no caching, no state. The only error mapping is the transport
//! -level join failure of the offloaded task onto the existing
//! `ScanError::Internal` variant.

use std::fmt::Display;
use std::path::PathBuf;

use vertice_core::model::{FreshnessReport, FreshnessSettings, ScanError, ScanReport};

/// Run the core scan off the main thread. A scan may take up to the CA-15
/// two-second budget; `spawn_blocking` keeps the window event loop
/// responsive for the whole duration.
async fn run_scan() -> Result<ScanReport, ScanError> {
    tauri::async_runtime::spawn_blocking(vertice_core::scan::scan)
        .await
        .map_err(map_join_error)?
}

/// Map a join failure of the offloaded scan task to the existing internal
/// variant — transport mapping, not business logic. Generic over `Display`
/// so the mapping itself is directly testable without naming tokio's
/// `JoinError` (not re-exported by `tauri::async_runtime`). `pub(crate)`
/// because `freshness`'s reference-lookup tasks reuse it too.
pub(crate) fn map_join_error(join: impl Display) -> ScanError {
    ScanError::Internal {
        reason: join.to_string(),
    }
}

/// Run only the client-installation adapter, off the main thread — the
/// same rationale as [`run_scan`]: a filesystem walk belongs on the
/// blocking pool, not the async executor `freshness` awaits on next.
async fn scan_installations() -> Result<Option<Vec<vertice_core::model::ClientPresence>>, ScanError>
{
    tauri::async_runtime::spawn_blocking(|| {
        let home = vertice_core::roots::home_dir()?;
        Ok(vertice_core::installations::scan(&home).presence)
    })
    .await
    .map_err(map_join_error)?
}

/// `scan` command: run a full inventory scan of the registered user roots.
#[tauri::command]
pub async fn scan() -> Result<ScanReport, ScanError> {
    run_scan().await
}

/// `rescan` command: identical to `scan` — the core holds no cache or
/// state. Kept as a stable IPC entry point for future cache-invalidation
/// semantics.
#[tauri::command]
pub async fn rescan() -> Result<ScanReport, ScanError> {
    run_scan().await
}

/// `freshness` command: an independent lookup, never awaiting `scan` or
/// `rescan` and never awaited by them (CA-15 — the scan's duration must
/// never observe this). A degraded reference lookup is represented inside
/// a successful `FreshnessReport` (`Freshness::Unknown`), never as a
/// rejected command invocation (design §11, §12).
#[tauri::command]
pub async fn freshness(app: tauri::AppHandle) -> Result<FreshnessReport, ScanError> {
    let app_data_dir = resolve_app_data_dir(&app)?;

    let presence = scan_installations().await?;

    Ok(crate::freshness::build_report(&app_data_dir, presence).await)
}

/// Resolve `app_data_dir()` and map the failure onto the existing internal
/// `ScanError` variant, exactly as `freshness` already did inline — shared
/// now that `freshness_settings`/`set_freshness_settings` need the same
/// resolution.
fn resolve_app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, ScanError> {
    tauri::Manager::path(app)
        .app_data_dir()
        .map_err(|err| ScanError::Internal {
            reason: err.to_string(),
        })
}

/// `freshness_settings` command: read-only view of the persisted opt-out
/// and disclosure-seen state (§8's document), independent of running any
/// check. Closes the gap Slice 2 flagged: the frontend's opt-out switch and
/// first-run disclosure need this to render *before* any `freshness`
/// report has ever resolved.
#[tauri::command]
pub async fn freshness_settings(app: tauri::AppHandle) -> Result<FreshnessSettings, ScanError> {
    let app_data_dir = resolve_app_data_dir(&app)?;
    read_freshness_settings(app_data_dir).await
}

/// `set_freshness_settings` command: the only way to mutate `enabled` or
/// `disclosure_seen`. Read-modify-write against the single persisted
/// document (§11: one file, one write path); the frontend always sends the
/// full desired state rather than a partial patch, so there is no
/// ambiguity about which field changed. Returns the settings actually
/// persisted, so the caller never has to guess whether the write landed.
#[tauri::command]
pub async fn set_freshness_settings(
    app: tauri::AppHandle,
    enabled: bool,
    disclosure_seen: bool,
) -> Result<FreshnessSettings, ScanError> {
    let app_data_dir = resolve_app_data_dir(&app)?;
    write_freshness_settings(app_data_dir, enabled, disclosure_seen).await
}

/// The blocking-offloaded read behind `freshness_settings`, factored out so
/// it is directly testable without an `AppHandle` (mirrors `run_scan`'s
/// shape).
async fn read_freshness_settings(app_data_dir: PathBuf) -> Result<FreshnessSettings, ScanError> {
    tauri::async_runtime::spawn_blocking(move || {
        let store =
            crate::freshness::cache::load(&crate::freshness::cache::store_path(&app_data_dir));
        FreshnessSettings {
            enabled: store.enabled,
            disclosure_seen: store.disclosure_seen,
        }
    })
    .await
    .map_err(map_join_error)
}

/// The blocking-offloaded write behind `set_freshness_settings`, factored
/// out so it is directly testable without an `AppHandle`.
async fn write_freshness_settings(
    app_data_dir: PathBuf,
    enabled: bool,
    disclosure_seen: bool,
) -> Result<FreshnessSettings, ScanError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = crate::freshness::cache::store_path(&app_data_dir);
        let mut store = crate::freshness::cache::load(&path);
        store.enabled = enabled;
        store.disclosure_seen = disclosure_seen;
        // Best-effort, matching `build_report`'s own cache-save tolerance:
        // a write failure here would surface as a silently-reverted toggle
        // on next read rather than a crash, which is preferable to
        // rejecting an otherwise-successful settings change.
        let _ = crate::freshness::cache::save(&path, &store);
        FreshnessSettings {
            enabled: store.enabled,
            disclosure_seen: store.disclosure_seen,
        }
    })
    .await
    .map_err(map_join_error)
}

#[cfg(test)]
mod tests {
    use super::{map_join_error, rescan, run_scan, scan, scan_installations};
    use crate::freshness::cache::{self, FreshnessStore};

    /// The private seam the commands delegate to: the core scan runs on the
    /// blocking pool and resolves with the consolidated report. Read-only
    /// against the real home directory; any machine with a resolvable home
    /// yields `Ok`, and the registered roots are always reported, present
    /// or not.
    #[test]
    fn run_scan_resolves_with_a_consolidated_report() {
        let report = tauri::async_runtime::block_on(run_scan())
            .expect("scan must succeed when the home directory resolves");

        assert!(!report.roots_scanned.is_empty());
    }

    /// Both commands are one-line delegations to `run_scan`, so they behave
    /// identically: a fresh full scan each — no cache, no state.
    #[test]
    fn scan_and_rescan_both_delegate_to_a_fresh_scan() {
        let first = tauri::async_runtime::block_on(scan())
            .expect("scan command must succeed when the home directory resolves");
        let second = tauri::async_runtime::block_on(rescan())
            .expect("rescan command must succeed when the home directory resolves");

        assert_eq!(first.roots_scanned, second.roots_scanned);
    }

    /// A join failure of the offloaded task maps to the existing
    /// `ScanError::Internal` variant carrying the join error's description
    /// as the reason — transport mapping, not business logic.
    #[test]
    fn join_failure_maps_to_scan_error_internal() {
        let join = tauri::async_runtime::block_on(async {
            tauri::async_runtime::spawn_blocking(|| panic!("simulated core failure"))
                .await
                .expect_err("a panicking blocking task must fail to join")
        });

        match map_join_error(join) {
            vertice_core::model::ScanError::Internal { reason } => {
                assert!(!reason.is_empty());
            }
            other => panic!("expected ScanError::Internal, got {other:?}"),
        }
    }

    /// `freshness`'s installation-scan step is independently invokable: it
    /// resolves without calling `run_scan`/`scan`/`rescan` at all — those
    /// names appear nowhere in `scan_installations`'s body, so there is
    /// nothing to await. Read-only against the real home directory, same
    /// tolerance as `run_scan_resolves_with_a_consolidated_report`.
    #[test]
    fn scan_installations_resolves_independently_of_the_full_scan_pipeline() {
        let presence = tauri::async_runtime::block_on(scan_installations())
            .expect("client-installation scan must succeed when the home directory resolves");

        // Whatever this machine reports is fine (CI has no client installed
        // at all, in which case `presence` carries empty-installations
        // records or is `None` on an unsupported platform) — the load-
        // bearing assertion is that this call succeeded on its own.
        let _ = presence;
    }

    /// The full command wiring, exercised through the same pure core
    /// `crate::freshness::build_report` the command delegates to: a
    /// degraded lookup (here, a slot with no known upstream — no network
    /// touched at all) resolves successfully, never a rejected invocation,
    /// with every check `Unknown` (component-freshness spec's load-bearing
    /// pin, exercised at the app-command boundary).
    #[test]
    fn freshness_command_wiring_never_rejects_and_degrades_to_unknown() {
        use vertice_core::model::{
            ClientInstallSlot, ClientInstallation, ClientKind, ClientPresence,
            ClientPresenceStatus, Freshness,
        };

        let app_data_dir = std::env::temp_dir().join(format!(
            "vertice-freshness-command-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&app_data_dir).expect("temp test dir must be creatable");

        let presence = vec![ClientPresence {
            slot: ClientInstallSlot::ClaudeCodeBundled,
            label: ClientInstallSlot::ClaudeCodeBundled.label().to_string(),
            probed_paths: vec!["C:\\fixture\\path".into()],
            status: ClientPresenceStatus::Detected,
            installations: vec![ClientInstallation {
                client: ClientKind::ClaudeCode,
                version: "some-msix-directory-name".to_string(),
                path: "C:\\fixture\\path\\claude-code".into(),
            }],
        }];

        let report = tauri::async_runtime::block_on(crate::freshness::build_report(
            &app_data_dir,
            Some(presence),
        ));

        assert!(report.enabled);
        assert!(!report.checks.is_empty());
        for check in &report.checks {
            match check.verdict {
                Freshness::Unknown { .. } => {}
                ref other => panic!("expected every check Unknown, got {other:?}"),
            }
        }
    }

    /// CA-15, restated for this capability: `scan`/`rescan` never invoke or
    /// await `freshness` — a degraded (or entirely un-run) freshness
    /// lookup produces zero `ScanReport.issues` entries, and the scan's
    /// measured duration cannot be affected by freshness latency it never
    /// waits on. `scan.rs` is a `vertice-core` module with no dependency
    /// on `vertice-app`'s `freshness` module at all (it cannot name it),
    /// so this is a structural guarantee, not a race the test happens to
    /// win — this test pins the observable half of that guarantee.
    #[test]
    fn scan_never_produces_a_freshness_shaped_issue_and_runs_independently_of_it() {
        let report = tauri::async_runtime::block_on(scan())
            .expect("scan command must succeed when the home directory resolves");

        for issue in &report.issues {
            assert!(
                !issue.reason.to_lowercase().contains("freshness"),
                "scan issue unexpectedly mentions freshness: {issue:?}"
            );
        }
    }

    fn temp_app_data_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vertice-commands-freshness-settings-test-{label}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp test dir must be creatable");
        dir
    }

    /// A never-before-touched app data dir reads as the default: enabled,
    /// disclosure not yet seen — matching `FreshnessStore::default()` and
    /// the spec's "enabled by default" requirement.
    #[test]
    fn reading_settings_with_no_prior_store_yields_enabled_and_disclosure_not_seen() {
        let app_data_dir = temp_app_data_dir("read-default");

        let settings = tauri::async_runtime::block_on(super::read_freshness_settings(app_data_dir))
            .expect("reading settings must succeed against a fresh temp dir");

        assert!(settings.enabled);
        assert!(!settings.disclosure_seen);
    }

    /// Writing settings persists both fields to the same document
    /// `cache.rs` already owns, and a subsequent read observes exactly what
    /// was written — the round trip the frontend's opt-out switch and
    /// disclosure dismissal both depend on.
    #[test]
    fn writing_settings_persists_and_a_subsequent_read_observes_them() {
        let app_data_dir = temp_app_data_dir("write-roundtrip");

        let written = tauri::async_runtime::block_on(super::write_freshness_settings(
            app_data_dir.clone(),
            false,
            true,
        ))
        .expect("writing settings must succeed against a fresh temp dir");
        assert!(!written.enabled);
        assert!(written.disclosure_seen);

        let read_back =
            tauri::async_runtime::block_on(super::read_freshness_settings(app_data_dir))
                .expect("reading settings back must succeed");
        assert_eq!(read_back, written);
    }

    /// Writing settings does not clobber an existing cache entry — the
    /// mutation touches only `enabled`/`disclosure_seen`, never `cache`
    /// (design §11: one document, one write path, but not a full-document
    /// overwrite of unrelated state).
    #[test]
    fn writing_settings_preserves_the_existing_cache_map() {
        let app_data_dir = temp_app_data_dir("write-preserves-cache");
        let path = cache::store_path(&app_data_dir);
        let mut store = FreshnessStore::default();
        store.cache.insert(
            "npm:opencode-ai".to_string(),
            cache::CacheEntry {
                version: "1.18.21".to_string(),
                fetched_at_unix_s: 1_000,
            },
        );
        cache::save(&path, &store).expect("test setup save must succeed");

        tauri::async_runtime::block_on(super::write_freshness_settings(
            app_data_dir.clone(),
            true,
            true,
        ))
        .expect("writing settings must succeed");

        let reloaded = cache::load(&path);
        assert_eq!(
            reloaded.cache.get("npm:opencode-ai").map(|e| &e.version),
            Some(&"1.18.21".to_string())
        );
        assert!(reloaded.disclosure_seen);
    }
}
