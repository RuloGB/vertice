//! Tauri IPC commands: thin async pass-throughs to the core scan.
//!
//! No business logic lives here — no filtering, no transformation of the
//! report, no caching, no state. The only error mapping is the transport
//! -level join failure of the offloaded task onto the existing
//! `ScanError::Internal` variant.

use std::fmt::Display;

use vertice_core::model::{ScanError, ScanReport};

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
/// `JoinError` (not re-exported by `tauri::async_runtime`).
fn map_join_error(join: impl Display) -> ScanError {
    ScanError::Internal {
        reason: join.to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::{map_join_error, rescan, run_scan, scan};

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
}
