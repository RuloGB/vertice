//! Tauri IPC commands: thin async pass-throughs to the core scan.
//!
//! No business logic lives here — no filtering, no transformation of the
//! report, no caching, no state. The only error mapping is the transport
//! -level join failure of the offloaded task onto the existing
//! `ScanError::Internal` variant.

use std::fmt::Display;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use vertice_core::model::{
    ClientPresenceStatus, Freshness, FreshnessReport, Prompt, PromptDraft, PromptError,
    PromptUpdate, ScanError, ScanReport, SearchRootStatus, UserSettings,
};

/// Run the core scan off the main thread, logging its start, end, and
/// measured duration (application-logging spec "A scan logs its start,
/// end, and duration"). `label` distinguishes `scan` from `rescan` in the
/// log while both delegate to the same core operation. A failure to log
/// never affects the returned result (D5 class 1) — logging only ever
/// reads the already-computed `ScanReport`, never mutates or gates it.
async fn run_scan(label: &'static str) -> Result<ScanReport, ScanError> {
    run_scan_with(label, |level, message| log::log!(level, "{message}")).await
}

/// The testable core of [`run_scan`]: the start/end lines are emitted
/// through an injected closure so a unit test can capture exactly what
/// would have been logged without installing a process-global logger
/// (mirrors `log_scan_report_with`'s seam — design §14 C1).
async fn run_scan_with(
    label: &'static str,
    mut emit: impl FnMut(log::Level, &str),
) -> Result<ScanReport, ScanError> {
    emit(log::Level::Info, &format!("{label} started"));
    let result = tauri::async_runtime::spawn_blocking(vertice_core::scan::scan)
        .await
        .map_err(map_join_error)?;
    if let Ok(report) = &result {
        emit(
            log::Level::Info,
            &format!("{label} finished in {} ms", report.duration_ms),
        );
        log_scan_report(report);
    }
    result
}

/// Emit one WARN line per `SearchRootStatus::NotFound` root and per
/// `ClientPresenceStatus::NotDetected` client in `report`
/// (application-logging spec "The Four Required Event Classes Are
/// Recorded"). Takes `&ScanReport` — it can only read, never mutate the
/// report already computed and about to be returned to the caller
/// (scan-orchestration spec "Logging a report does not mutate ScanReport
/// or ScanIssue").
fn log_scan_report(report: &ScanReport) {
    log_scan_report_with(report, |level, message| log::log!(level, "{message}"));
}

/// The testable core of [`log_scan_report`]: the emission is factored
/// behind a closure so unit tests can capture what would have been logged
/// without touching the process-global `log` sink (design §14 C1).
fn log_scan_report_with(report: &ScanReport, mut emit: impl FnMut(log::Level, &str)) {
    for root in &report.roots_scanned {
        if root.status == SearchRootStatus::NotFound {
            emit(
                log::Level::Warn,
                &format!(
                    "search root not found: {} ({})",
                    root.id.0,
                    root.path.display()
                ),
            );
        }
    }
    if let Some(records) = &report.client_presence {
        for record in records {
            if record.status == ClientPresenceStatus::NotDetected {
                emit(
                    log::Level::Warn,
                    &format!("client not detected: {}", record.label),
                );
            }
        }
    }
}

/// Emit one WARN line per `Freshness::Unknown { reason }` check in `report`
/// (application-logging spec "A freshness-unknown verdict is logged with
/// its reason"; component-freshness spec "Freshness-Unknown Verdicts Are
/// Also Recorded In The Application Log").
fn log_freshness_report(report: &FreshnessReport) {
    log_freshness_report_with(report, |level, message| log::log!(level, "{message}"));
}

/// The testable core of [`log_freshness_report`], mirroring
/// `log_scan_report_with`'s injectable-sink shape.
fn log_freshness_report_with(report: &FreshnessReport, mut emit: impl FnMut(log::Level, &str)) {
    for check in &report.checks {
        if let Freshness::Unknown { reason } = &check.verdict {
            emit(log::Level::Warn, &format!("freshness unknown: {reason}"));
        }
    }
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
    run_scan("scan").await
}

/// `rescan` command: identical to `scan` — the core holds no cache or
/// state. Kept as a stable IPC entry point for future cache-invalidation
/// semantics. Labeled distinctly in the log so a rescan is distinguishable
/// from the initial scan (design §11).
#[tauri::command]
pub async fn rescan() -> Result<ScanReport, ScanError> {
    run_scan("rescan").await
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

    let report = crate::freshness::build_report(&app_data_dir, presence).await;
    log_freshness_report(&report);
    Ok(report)
}

/// Resolve `app_data_dir()` and map the failure onto the existing internal
/// `ScanError` variant, exactly as `freshness` already did inline — shared
/// now that `user_settings`/`set_user_settings` need the same resolution.
fn resolve_app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, ScanError> {
    tauri::Manager::path(app)
        .app_data_dir()
        .map_err(|err| ScanError::Internal {
            reason: err.to_string(),
        })
}

/// `user_settings` command: read-only view of the durable settings document
/// (`locale`, `enabled`, `disclosure_seen`), independent of running any
/// check. Creates no file as a side effect of reading.
#[tauri::command]
pub async fn user_settings(app: tauri::AppHandle) -> Result<UserSettings, ScanError> {
    let app_data_dir = resolve_app_data_dir(&app)?;
    read_user_settings(app_data_dir).await
}

/// `set_user_settings` command: a partial patch, not a full-state write
/// (user-settings spec "A Read Command And A Partial-Patch Write Command").
/// Each field is independently optional; an omitted (`None`) field leaves
/// its persisted value unchanged — never reset to a default. This is a
/// deliberate departure from the project's prior always-send-full-state
/// convention: two independent frontend owners write this document (the
/// shell for `locale`, the clients page for `enabled`/`disclosure_seen`),
/// and a stale full-state write from either could silently clobber the
/// other's just-written field. Returns the settings actually persisted, so
/// the caller never has to guess whether the write landed.
#[tauri::command]
pub async fn set_user_settings(
    app: tauri::AppHandle,
    locale: Option<String>,
    enabled: Option<bool>,
    disclosure_seen: Option<bool>,
) -> Result<UserSettings, ScanError> {
    let app_data_dir = resolve_app_data_dir(&app)?;
    write_user_settings(app_data_dir, locale, enabled, disclosure_seen).await
}

pub(crate) type PromptRepositoryState = Arc<Mutex<crate::prompts::store::JsonPromptRepository>>;

pub(crate) fn prompt_repository_state(app_data_dir: PathBuf) -> PromptRepositoryState {
    Arc::new(Mutex::new(
        crate::prompts::store::JsonPromptRepository::new(app_data_dir),
    ))
}

#[tauri::command]
pub async fn list_prompts(
    state: tauri::State<'_, PromptRepositoryState>,
) -> Result<Vec<Prompt>, PromptError> {
    list_prompts_from_state(state.inner().clone()).await
}

#[tauri::command]
pub async fn create_prompt(
    state: tauri::State<'_, PromptRepositoryState>,
    draft: PromptDraft,
) -> Result<Prompt, PromptError> {
    create_prompt_from_state(state.inner().clone(), draft).await
}

#[tauri::command]
pub async fn update_prompt(
    state: tauri::State<'_, PromptRepositoryState>,
    update: PromptUpdate,
) -> Result<Prompt, PromptError> {
    update_prompt_from_state(state.inner().clone(), update).await
}

#[tauri::command]
pub async fn delete_prompt(
    state: tauri::State<'_, PromptRepositoryState>,
    id: String,
) -> Result<(), PromptError> {
    delete_prompt_from_state(state.inner().clone(), id).await
}

async fn list_prompts_from_state(state: PromptRepositoryState) -> Result<Vec<Prompt>, PromptError> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        use crate::prompts::store::PromptRepository;
        let repo = state.lock().unwrap_or_else(|err| err.into_inner());
        repo.list()
    })
    .await
    .map_err(prompt_join_error)
    .and_then(|result| result);
    log_prompt_result("list_prompts", &result);
    result
}

async fn create_prompt_from_state(
    state: PromptRepositoryState,
    draft: PromptDraft,
) -> Result<Prompt, PromptError> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        use crate::prompts::store::PromptRepository;
        let mut repo = state.lock().unwrap_or_else(|err| err.into_inner());
        repo.create(draft)
    })
    .await
    .map_err(prompt_join_error)
    .and_then(|result| result);
    log_prompt_result("create_prompt", &result);
    result
}

async fn update_prompt_from_state(
    state: PromptRepositoryState,
    update: PromptUpdate,
) -> Result<Prompt, PromptError> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        use crate::prompts::store::PromptRepository;
        let mut repo = state.lock().unwrap_or_else(|err| err.into_inner());
        repo.update(update)
    })
    .await
    .map_err(prompt_join_error)
    .and_then(|result| result);
    log_prompt_result("update_prompt", &result);
    result
}

async fn delete_prompt_from_state(
    state: PromptRepositoryState,
    id: String,
) -> Result<(), PromptError> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        use crate::prompts::store::PromptRepository;
        let mut repo = state.lock().unwrap_or_else(|err| err.into_inner());
        repo.delete(&id)
    })
    .await
    .map_err(prompt_join_error)
    .and_then(|result| result);
    log_prompt_result("delete_prompt", &result);
    result
}

fn prompt_join_error(join: impl Display) -> PromptError {
    PromptError::StoreUnavailable {
        reason: join.to_string(),
    }
}

fn log_prompt_result<T>(operation: &str, result: &Result<T, PromptError>) {
    log_prompt_result_with(operation, result, |level, message| {
        log::log!(level, "{message}")
    });
}

fn log_prompt_result_with<T>(
    operation: &str,
    result: &Result<T, PromptError>,
    mut emit: impl FnMut(log::Level, &str),
) {
    if let Err(PromptError::StoreUnavailable { reason }) = result {
        emit(
            log::Level::Warn,
            &format!("{operation} failed: prompt store unavailable: {reason}"),
        );
    }
}
/// log_file_path command: returns the absolute path of the application
/// log so the frontend can render it as selectable text. Performs no I/O
/// at all — a path join — which is why it is the one command that does
/// not offload to `spawn_blocking`. `async` because the audit's
/// `exported_tauri_commands` matcher keys on `pub async fn <name>(`
/// (desktop-shell spec "The log-path command returns the path without
/// touching the file").
#[tauri::command]
pub async fn log_file_path(app: tauri::AppHandle) -> Result<String, ScanError> {
    let app_data_dir = resolve_app_data_dir(&app)?;
    Ok(crate::logging::log_path(&app_data_dir)
        .to_string_lossy()
        .into_owned())
}

/// The blocking-offloaded read behind `user_settings`, factored out so it is
/// directly testable without an `AppHandle` (mirrors `run_scan`'s shape).
/// Reading never creates a file (user-settings spec "Reading settings does
/// not create the file").
async fn read_user_settings(app_data_dir: PathBuf) -> Result<UserSettings, ScanError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = crate::settings::store::store_path(&app_data_dir);
        crate::settings::store::resolve(crate::settings::store::load(&path))
    })
    .await
    .map_err(map_join_error)
}

/// The blocking-offloaded read-modify-write behind `set_user_settings`,
/// factored out so it is directly testable without an `AppHandle`. Each
/// `None` field leaves the persisted value unchanged — a true partial
/// patch, not a full-state overwrite (user-settings spec Decision 3).
async fn write_user_settings(
    app_data_dir: PathBuf,
    locale: Option<String>,
    enabled: Option<bool>,
    disclosure_seen: Option<bool>,
) -> Result<UserSettings, ScanError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = crate::settings::store::store_path(&app_data_dir);
        let mut settings = crate::settings::store::resolve(crate::settings::store::load(&path));
        if let Some(locale) = locale {
            settings.locale = Some(locale);
        }
        if let Some(enabled) = enabled {
            settings.enabled = enabled;
        }
        if let Some(disclosure_seen) = disclosure_seen {
            settings.disclosure_seen = disclosure_seen;
        }
        // Best-effort, matching `build_report`'s own cache-save tolerance:
        // a write failure here would surface as a silently-reverted change
        // on next read rather than a crash, which is preferable to
        // rejecting an otherwise-successful settings change. The result is
        // unaffected either way — only the silence becomes evidence
        // (design §9).
        if let Err(err) = crate::settings::store::save(&path, &settings) {
            log::warn!("could not persist settings store: {err}");
        }
        settings
    })
    .await
    .map_err(map_join_error)
}

#[cfg(test)]
mod tests {
    use super::{
        log_freshness_report_with, log_scan_report_with, map_join_error, rescan, run_scan,
        run_scan_with, scan, scan_installations,
    };
    use vertice_core::model::{
        ClientPresence, ClientPresenceStatus, Freshness, FreshnessCheck, FreshnessReport,
        FreshnessSubject, ScanReport, SearchRoot, SearchRootId, SearchRootKind, SearchRootStatus,
    };

    /// The private seam the commands delegate to: the core scan runs on the
    /// blocking pool and resolves with the consolidated report. Read-only
    /// against the real home directory; any machine with a resolvable home
    /// yields `Ok`, and the registered roots are always reported, present
    /// or not.
    #[test]
    fn run_scan_resolves_with_a_consolidated_report() {
        let report = tauri::async_runtime::block_on(run_scan("scan"))
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

    fn scan_report_fixture(
        roots: Vec<SearchRoot>,
        presence: Option<Vec<ClientPresence>>,
    ) -> ScanReport {
        ScanReport {
            components: vec![],
            installations: vec![],
            roots_scanned: roots,
            issues: vec![],
            client_presence: presence,
            duration_ms: 42,
        }
    }

    /// scan-orchestration spec, "Logging a report does not mutate
    /// ScanReport or ScanIssue": observing a completed report for logging
    /// purposes leaves it byte-identical to what a scan already produced
    /// (design §14 C2, D5 class 1).
    #[test]
    fn scan_result_is_byte_identical_whether_or_not_a_working_sink_observed_it() {
        let report = tauri::async_runtime::block_on(run_scan("scan"))
            .expect("scan must succeed when the home directory resolves");
        let before = report.clone();

        // No global logger is installed in this test process, so this
        // observation runs against a "sink" that silently drops every
        // line — exactly the failed-initialisation case D5 class 1
        // requires to be indistinguishable from a working sink as far as
        // the report is concerned.
        log_scan_report_with(&report, |_level, _message| {});

        assert_eq!(report, before);
    }

    /// application-logging spec "A scan logs its start, end, and duration":
    /// exactly one INFO "started" line and one INFO "finished in N ms" line
    /// are emitted per scan, carrying the real measured duration.
    #[test]
    fn run_scan_emits_one_info_start_line_and_one_info_finish_line_carrying_the_duration() {
        let mut emitted: Vec<(log::Level, String)> = Vec::new();

        let report = tauri::async_runtime::block_on(run_scan_with("scan", |level, message| {
            emitted.push((level, message.to_string()));
        }))
        .expect("scan must succeed when the home directory resolves");

        assert_eq!(emitted.len(), 2);
        assert!(emitted.iter().all(|(level, _)| *level == log::Level::Info));
        assert_eq!(emitted[0].1, "scan started");
        assert_eq!(
            emitted[1].1,
            format!("scan finished in {} ms", report.duration_ms)
        );
    }

    /// Anchor 0.2 (`add-mcp-scanning` `tasks.md`, `design.md` §12), closed
    /// GREEN here in Slice 3 (task 3.10): `log_scan_report_with` is
    /// private to this module, so the real assertion lives here rather
    /// than in the `tests/mcp_log_redaction.rs` integration stub, which
    /// records that decision (design §12 item 2's own hedge). A `claude/
    /// stdio-secret`-shaped `ScanReport` — built from
    /// `vertice_core::mcp_claude::scan` against the real fixture — never
    /// emits a `FAKE`-vocabulary secret through the log-capturing closure.
    /// Scoped honestly: today's logger reads only `root.path` and
    /// `record.label`, never `report.issues`, so this exercises the
    /// `root.path` half for real and the `ScanIssue.reason` half only as
    /// forward-looking defense in depth (design §7.2/§10.2's hedge).
    #[test]
    fn mcp_secrets_never_reach_the_scan_report_log() {
        let mut fixture_home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fixture_home.pop();
        fixture_home.push("vertice-core");
        fixture_home.push("tests");
        fixture_home.push("fixtures");
        fixture_home.push("mcp");
        fixture_home.push("claude");
        fixture_home.push("stdio-secret");

        let mcp_scan = vertice_core::mcp_claude::scan(&fixture_home);
        assert!(
            !mcp_scan.components.is_empty(),
            "the stdio-secret fixture must yield at least one component"
        );

        let report = scan_report_fixture(mcp_scan.roots.clone(), None);
        let report = ScanReport {
            components: mcp_scan.components,
            issues: mcp_scan.issues,
            ..report
        };

        let mut emitted = String::new();
        log_scan_report_with(&report, |_level, message| {
            emitted.push_str(message);
        });

        assert!(!emitted.contains("FAKE"));
    }

    /// `add-mcp-scanning` `tasks.md` 7.6: the final, whole-tree run of the
    /// log half of the `FAKE` guard (design §10.2) — every secret-bearing
    /// fixture, across all three clients, folded into one `ScanReport` and
    /// logged together, not just the Claude-only Slice-3 subset
    /// `mcp_secrets_never_reach_the_scan_report_log` already covers.
    #[test]
    fn mcp_secrets_never_reach_the_scan_report_log_across_the_full_fixture_tree() {
        let secret_bearing: [(&str, &str); 8] = [
            ("claude", "stdio-secret"),
            ("claude", "remote-secret"),
            ("claude", "remote-dirty-url"),
            ("claude", "remote-userinfo-ambiguous-url"),
            ("claude", "malformed-secret-adjacent"),
            ("opencode", "stdio-secret"),
            ("opencode", "remote-secret"),
            ("opencode", "malformed-secret-adjacent"),
        ];

        let mut components = Vec::new();
        let mut issues = Vec::new();
        let mut roots = Vec::new();

        for (client, case) in secret_bearing {
            let mut fixture_home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            fixture_home.pop();
            fixture_home.push("vertice-core");
            fixture_home.push("tests");
            fixture_home.push("fixtures");
            fixture_home.push("mcp");
            fixture_home.push(client);
            fixture_home.push(case);

            let scan = match client {
                "claude" => vertice_core::mcp_claude::scan(&fixture_home),
                "opencode" => vertice_core::mcp_opencode::scan(&fixture_home),
                _ => unreachable!("only claude/opencode carry secret-bearing cases in this table"),
            };
            components.extend(scan.components);
            issues.extend(scan.issues);
            roots.extend(scan.roots);
        }

        for case in ["stdio-secret", "remote-secret", "malformed-secret-adjacent"] {
            let mut fixture_home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            fixture_home.pop();
            fixture_home.push("vertice-core");
            fixture_home.push("tests");
            fixture_home.push("fixtures");
            fixture_home.push("mcp");
            fixture_home.push("codex");
            fixture_home.push(case);

            let scan = vertice_core::mcp_codex::scan(&fixture_home);
            components.extend(scan.components);
            issues.extend(scan.issues);
            roots.extend(scan.roots);
        }

        assert!(!components.is_empty());

        let report = scan_report_fixture(roots, None);
        let report = ScanReport {
            components,
            issues,
            ..report
        };

        let mut emitted = String::new();
        log_scan_report_with(&report, |_level, message| {
            emitted.push_str(message);
        });

        assert!(!emitted.contains("FAKE"));
    }

    /// application-logging spec "A missing root and an undetected client
    /// are both logged": one WARN line per `NotFound` root and one per
    /// `NotDetected` client, each carrying the concrete value (design §14
    /// C1).
    #[test]
    fn scan_report_with_not_found_root_and_not_detected_client_emits_one_warn_line_each() {
        let report = scan_report_fixture(
            vec![SearchRoot {
                id: SearchRootId("claude-skills".to_string()),
                path: "C:\\missing\\claude-skills".into(),
                kind: SearchRootKind::Skill,
                status: SearchRootStatus::NotFound,
            }],
            Some(vec![ClientPresence {
                slot: vertice_core::model::ClientInstallSlot::CodexStandalone,
                label: "Codex".to_string(),
                probed_paths: vec!["C:\\missing\\codex".into()],
                status: ClientPresenceStatus::NotDetected,
                installations: vec![],
            }]),
        );

        let mut emitted: Vec<(log::Level, String)> = Vec::new();
        log_scan_report_with(&report, |level, message| {
            emitted.push((level, message.to_string()));
        });

        assert_eq!(emitted.len(), 2);
        assert!(emitted.iter().all(|(level, _)| *level == log::Level::Warn));
        assert!(emitted
            .iter()
            .any(|(_, message)| message.contains("claude-skills")));
        assert!(emitted.iter().any(|(_, message)| message.contains("Codex")));
    }

    /// component-freshness spec "Freshness-Unknown Verdicts Are Also
    /// Recorded In The Application Log": the `reason` value is carried
    /// verbatim into the WARN line (design §14 C1).
    #[test]
    fn freshness_report_with_an_unknown_verdict_emits_a_warn_line_carrying_the_reason_verbatim() {
        let reason = "upstream lookup timed out after 2000ms".to_string();
        let report = FreshnessReport {
            enabled: true,
            checks: vec![FreshnessCheck {
                subject: FreshnessSubject::ClientInstallation {
                    slot: vertice_core::model::ClientInstallSlot::CodexStandalone,
                    path: "C:\\fixture\\codex".into(),
                },
                installed: "1.0.0".to_string(),
                verdict: Freshness::Unknown {
                    reason: reason.clone(),
                },
            }],
        };

        let mut emitted: Vec<(log::Level, String)> = Vec::new();
        log_freshness_report_with(&report, |level, message| {
            emitted.push((level, message.to_string()));
        });

        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].0, log::Level::Warn);
        assert!(emitted[0].1.contains(&reason));
    }

    fn prompt_draft(title: &str, body: &str) -> vertice_core::model::PromptDraft {
        vertice_core::model::PromptDraft {
            title: title.to_string(),
            body: body.to_string(),
            tags: vec!["ipc".to_string()],
            best_for_context: Some("command tests".to_string()),
        }
    }

    #[test]
    fn prompt_command_helpers_return_typed_crud_results_and_errors() {
        let state = super::prompt_repository_state(temp_app_data_dir("prompt-commands"));

        let created = tauri::async_runtime::block_on(super::create_prompt_from_state(
            state.clone(),
            prompt_draft("Draft", "Body"),
        ))
        .expect("create prompt command helper");
        assert_eq!(created.title, "Draft");

        let listed = tauri::async_runtime::block_on(super::list_prompts_from_state(state.clone()))
            .expect("list prompt command helper");
        assert_eq!(listed, vec![created.clone()]);

        let updated = tauri::async_runtime::block_on(super::update_prompt_from_state(
            state.clone(),
            vertice_core::model::PromptUpdate {
                id: created.id.clone(),
                title: "Updated".to_string(),
                body: "New body".to_string(),
                tags: vec![],
                best_for_context: None,
            },
        ))
        .expect("update prompt command helper");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title, "Updated");

        tauri::async_runtime::block_on(super::delete_prompt_from_state(
            state.clone(),
            updated.id.clone(),
        ))
        .expect("delete prompt command helper");
        assert!(
            tauri::async_runtime::block_on(super::list_prompts_from_state(state))
                .expect("list after delete")
                .is_empty()
        );
    }

    #[test]
    fn prompt_command_helpers_map_validation_and_not_found_as_typed_prompt_errors() {
        let state = super::prompt_repository_state(temp_app_data_dir("prompt-errors"));

        let invalid = tauri::async_runtime::block_on(super::create_prompt_from_state(
            state.clone(),
            prompt_draft(" ", "Body"),
        ))
        .expect_err("invalid prompt rejected");
        assert_eq!(
            invalid,
            vertice_core::model::PromptError::InvalidInput {
                field: "title".to_string()
            }
        );

        let missing = tauri::async_runtime::block_on(super::delete_prompt_from_state(
            state,
            "missing".to_string(),
        ))
        .expect_err("missing prompt rejected");
        assert_eq!(
            missing,
            vertice_core::model::PromptError::NotFound {
                id: "missing".to_string()
            }
        );
    }

    #[test]
    fn prompt_store_unavailable_results_emit_a_warning_at_the_command_boundary() {
        let result: Result<(), vertice_core::model::PromptError> =
            Err(vertice_core::model::PromptError::StoreUnavailable {
                reason: "unsupported prompt store schema version 99".to_string(),
            });
        let mut emitted: Vec<(log::Level, String)> = Vec::new();

        super::log_prompt_result_with("update_prompt", &result, |level, message| {
            emitted.push((level, message.to_string()));
        });

        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].0, log::Level::Warn);
        assert!(emitted[0].1.contains("update_prompt failed"));
        assert!(emitted[0]
            .1
            .contains("unsupported prompt store schema version 99"));
    }

    #[test]
    fn prompt_validation_and_not_found_results_do_not_emit_diagnostic_warnings() {
        let validation: Result<(), vertice_core::model::PromptError> =
            Err(vertice_core::model::PromptError::InvalidInput {
                field: "title".to_string(),
            });
        let not_found: Result<(), vertice_core::model::PromptError> =
            Err(vertice_core::model::PromptError::NotFound {
                id: "missing".to_string(),
            });
        let mut emitted: Vec<(log::Level, String)> = Vec::new();

        super::log_prompt_result_with("create_prompt", &validation, |level, message| {
            emitted.push((level, message.to_string()));
        });
        super::log_prompt_result_with("delete_prompt", &not_found, |level, message| {
            emitted.push((level, message.to_string()));
        });

        assert!(emitted.is_empty());
    }
    fn temp_app_data_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vertice-commands-user-settings-test-{label}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp test dir must be creatable");
        dir
    }

    /// `add-locale-persistence`, Decision 3: `set_user_settings` is a
    /// partial patch. A locale-only patch must not clobber an `enabled`
    /// value another writer (the clients page) already persisted — a
    /// stale full-state write from the shell would silently re-enable
    /// outbound network requests the user had just turned off.
    #[test]
    fn a_locale_patch_does_not_clobber_enabled() {
        let app_data_dir = temp_app_data_dir("locale-patch-no-clobber");

        tauri::async_runtime::block_on(super::write_user_settings(
            app_data_dir.clone(),
            None,
            Some(false),
            None,
        ))
        .expect("persisting enabled must succeed");

        let patched = tauri::async_runtime::block_on(super::write_user_settings(
            app_data_dir,
            Some("es".to_string()),
            None,
            None,
        ))
        .expect("patching locale must succeed");

        assert_eq!(patched.locale, Some("es".to_string()));
        assert!(!patched.enabled);
    }

    /// Regression: the settings-write path must create its own app data
    /// directory. Mirrors the former `freshness_settings` equivalent this
    /// change ports over (design §9, §14 A2: "the toggle survives
    /// restart").
    #[test]
    fn writing_settings_survives_a_never_created_app_data_directory() {
        let app_data_dir = std::env::temp_dir().join(format!(
            "vertice-commands-user-settings-not-created-{}",
            std::process::id()
        ));
        assert!(!app_data_dir.exists());

        let written = tauri::async_runtime::block_on(super::write_user_settings(
            app_data_dir.clone(),
            None,
            Some(false),
            Some(true),
        ))
        .expect("writing settings must succeed even when the app data dir never existed");
        assert!(!written.enabled);
        assert!(written.disclosure_seen);

        let read_back = tauri::async_runtime::block_on(super::read_user_settings(app_data_dir))
            .expect("reading settings back must succeed");
        assert_eq!(read_back, written);
    }

    /// A never-before-touched app data dir reads as the documented
    /// `Missing`-outcome defaults: enabled, disclosure not yet seen, no
    /// explicit locale.
    #[test]
    fn reading_settings_with_no_prior_store_yields_enabled_and_disclosure_not_seen() {
        let app_data_dir = temp_app_data_dir("read-default");

        let settings = tauri::async_runtime::block_on(super::read_user_settings(app_data_dir))
            .expect("reading settings must succeed against a fresh temp dir");

        assert!(settings.enabled);
        assert!(!settings.disclosure_seen);
        assert_eq!(settings.locale, None);
    }

    /// Writing settings persists all three fields, and a subsequent read
    /// observes exactly what was written.
    #[test]
    fn writing_settings_persists_and_a_subsequent_read_observes_them() {
        let app_data_dir = temp_app_data_dir("write-roundtrip");

        let written = tauri::async_runtime::block_on(super::write_user_settings(
            app_data_dir.clone(),
            Some("es".to_string()),
            Some(false),
            Some(true),
        ))
        .expect("writing settings must succeed against a fresh temp dir");
        assert!(!written.enabled);
        assert!(written.disclosure_seen);
        assert_eq!(written.locale, Some("es".to_string()));

        let read_back = tauri::async_runtime::block_on(super::read_user_settings(app_data_dir))
            .expect("reading settings back must succeed");
        assert_eq!(read_back, written);
    }
}
