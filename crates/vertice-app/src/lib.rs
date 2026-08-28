//! `vertice-app`: the Tauri 2 desktop shell. Owns the Tauri runtime, IPC
//! commands, and the bundled Svelte 5 + Vite + Tailwind frontend. Depends on
//! `vertice-core` (path dependency) for all domain logic — this crate must
//! stay the only place the `tauri` crate is imported in the workspace.

mod commands;
mod freshness;
mod logging;
mod prompts;
mod settings;

use std::path::{Path, PathBuf};
use tauri::Manager;

/// The startup sequence's decision logic, factored out of `.setup` so it is
/// testable without a Tauri `App` or a real global logger installation
/// (mirrors `commands::log_scan_report_with`'s injectable-closure seam —
/// design §14 C1 — deliberately reused here instead of installing a second
/// process-global logger, which `log::set_boxed_logger` only ever allows to
/// succeed once per process and would make `cargo test`'s parallel runs
/// flaky). Mirrors `.setup`'s three branches exactly: a successful sink
/// initialisation logs exactly one INFO startup line (application-logging
/// spec "Startup is logged once"); a failed initialisation, or an
/// unresolvable `app_data_dir`, reports exactly one stderr line and never
/// logs anything (application-logging spec "Sink Initialisation Failure Is
/// Reported Once, On Stderr"). `.setup` calls this with the real
/// `log::info!`/`eprintln!` sinks, so runtime behaviour is unchanged.
fn startup_sequence(
    app_data_dir: Result<PathBuf, impl std::fmt::Display>,
    init: impl FnOnce(&Path) -> Result<(), logging::InitError>,
    version: &str,
    mut log_info: impl FnMut(&str),
    mut report_stderr: impl FnMut(&str),
) {
    match app_data_dir {
        Ok(app_data_dir) => match init(&app_data_dir) {
            Ok(()) => log_info(&format!("vertice {version} starting")),
            Err(err) => report_stderr(&format!("vertice: {err}")),
        },
        Err(err) => report_stderr(&format!(
            "vertice: could not resolve the application data directory: {err}"
        )),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = tauri::Manager::path(app).app_data_dir();
            if let Ok(path) = app_data_dir.as_ref() {
                app.manage(commands::prompt_repository_state(path.clone()));
                app.manage(commands::subscription_repository_state(path.clone()));
            }
            startup_sequence(
                app_data_dir,
                logging::init,
                env!("CARGO_PKG_VERSION"),
                |message| log::info!("{message}"),
                |message| eprintln!("{message}"),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan,
            commands::rescan,
            commands::freshness,
            commands::user_settings,
            commands::set_user_settings,
            commands::log_file_path,
            commands::list_prompts,
            commands::create_prompt,
            commands::update_prompt,
            commands::delete_prompt,
            commands::list_subscriptions,
            commands::create_subscription,
            commands::update_subscription,
            commands::delete_subscription
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Vertice application");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path whose parent is a regular file, not a directory: any attempt
    /// to `create_dir_all` onto it fails deterministically — the same shape
    /// `logging::tests::init_against_an_uncreatable_directory_returns_err_and_does_not_panic`
    /// uses, reused here so `logging::init` itself (not a stand-in) produces
    /// a real `InitError` without ever installing a global logger.
    fn unwritable_child_path(label: &str) -> PathBuf {
        let parent = std::env::temp_dir().join(format!(
            "vertice-lib-startup-test-{label}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&parent).expect("test setup parent dir must be creatable");
        let blocked = parent.join("blocked-by-a-file");
        std::fs::write(&blocked, b"not a directory").expect("test setup file must be writable");
        blocked.join("nested")
    }

    /// application-logging spec "Startup is logged once": a successful sink
    /// initialisation emits exactly one INFO line, carrying the version, and
    /// nothing on stderr.
    #[test]
    fn startup_sequence_logs_exactly_one_info_line_on_successful_init() {
        let mut info_lines: Vec<String> = Vec::new();
        let mut stderr_lines: Vec<String> = Vec::new();

        startup_sequence(
            Ok::<_, std::io::Error>(PathBuf::from("does-not-matter")),
            |_app_data_dir| Ok(()),
            "9.9.9",
            |message| info_lines.push(message.to_string()),
            |message| stderr_lines.push(message.to_string()),
        );

        assert_eq!(info_lines, vec!["vertice 9.9.9 starting".to_string()]);
        assert!(stderr_lines.is_empty());
    }

    /// application-logging spec "Sink Initialisation Failure Is Reported
    /// Once, On Stderr": a real failing `logging::init` call reports exactly
    /// one stderr line and logs nothing.
    #[test]
    fn startup_sequence_reports_stderr_once_and_never_logs_on_init_failure() {
        let bad_app_data_dir = unwritable_child_path("init-failure");
        let mut info_lines: Vec<String> = Vec::new();
        let mut stderr_lines: Vec<String> = Vec::new();

        startup_sequence(
            Ok::<_, std::io::Error>(bad_app_data_dir),
            logging::init,
            "9.9.9",
            |message| info_lines.push(message.to_string()),
            |message| stderr_lines.push(message.to_string()),
        );

        assert!(info_lines.is_empty());
        assert_eq!(stderr_lines.len(), 1);
        assert!(stderr_lines[0].starts_with("vertice: "));
    }

    /// The unresolvable-`app_data_dir` branch also reports exactly one
    /// stderr line and never logs — the third branch `.setup` can take.
    #[test]
    fn startup_sequence_reports_stderr_once_when_app_data_dir_is_unresolvable() {
        let mut info_lines: Vec<String> = Vec::new();
        let mut stderr_lines: Vec<String> = Vec::new();

        startup_sequence(
            Err("app data directory not found"),
            |_app_data_dir| Ok(()),
            "9.9.9",
            |message| info_lines.push(message.to_string()),
            |message| stderr_lines.push(message.to_string()),
        );

        assert!(info_lines.is_empty());
        assert_eq!(stderr_lines.len(), 1);
        assert!(stderr_lines[0].starts_with("vertice: "));
    }
}

mod subscriptions;
