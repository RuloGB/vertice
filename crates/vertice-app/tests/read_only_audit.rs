use std::fs;
use std::path::Path;

#[derive(Debug)]
struct ShellAuditReport {
    commands: Vec<&'static str>,
    permissions: Vec<String>,
    capability_findings: Vec<String>,
    command_findings: Vec<String>,
    static_proof_is_limited: bool,
}

/// One sanctioned write-exception module and the specific enumerated
/// syscalls it is permitted to use — membership alone grants nothing: every
/// entry's path derivation is still independently proved by
/// `assert_write_path_is_derived_from_app_data_dir` (design §10).
struct SanctionedWriter {
    module: &'static str,
    allowed: &'static [&'static str],
}

/// CA-16's complete exception surface. Growing this list is a reviewed
/// event (`assert_eq!(SANCTIONED_WRITERS.len(), 3)` below).
const SANCTIONED_WRITERS: [SanctionedWriter; 3] = [
    SanctionedWriter {
        module: "freshness/cache.rs",
        allowed: &["fs::write", "create_dir"],
    },
    SanctionedWriter {
        module: "logging.rs",
        allowed: &[
            "OpenOptions",
            ".write(",
            ".write_all(",
            "File::create",
            "create_dir",
            "fs::rename",
            "std::fs::rename",
        ],
    },
    SanctionedWriter {
        module: "settings/store.rs",
        allowed: &["fs::write", "create_dir", "fs::rename"],
    },
];

#[test]
fn desktop_shell_exposes_only_scan_commands_and_core_default_capability() {
    let report = audit_desktop_shell_read_only_surface();

    assert_eq!(
        report.commands,
        vec![
            "scan",
            "rescan",
            "freshness",
            "user_settings",
            "set_user_settings",
            "log_file_path"
        ]
    );
    assert_eq!(report.permissions, vec!["core:default".to_string()]);
    assert!(
        report.capability_findings.is_empty(),
        "unexpected capabilities: {:?}",
        report.capability_findings
    );
    assert!(
        report.command_findings.is_empty(),
        "unexpected command surface: {:?}",
        report.command_findings
    );
    assert!(report.static_proof_is_limited);
}

fn audit_desktop_shell_read_only_surface() -> ShellAuditReport {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");
    let commands_source =
        fs::read_to_string(src_dir.join("commands.rs")).expect("commands.rs readable");
    let lib_source = fs::read_to_string(src_dir.join("lib.rs")).expect("lib.rs readable");
    let capability_source =
        fs::read_to_string(manifest_dir.join("capabilities").join("default.json"))
            .expect("default capability readable");

    let commands = exported_tauri_commands(&commands_source);
    let permissions = capability_permissions(&capability_source);
    let mut capability_findings = Vec::new();
    let mut command_findings = Vec::new();

    if permissions != ["core:default"] {
        capability_findings.push(format!(
            "permissions must be exactly core:default, got {permissions:?}"
        ));
    }

    for permission in &permissions {
        for forbidden in [
            "fs:",
            "shell:",
            "dialog:",
            "tauri-plugin-fs",
            "tauri-plugin-shell",
            "tauri-plugin-dialog",
        ] {
            if permission.contains(forbidden) {
                capability_findings.push(format!(
                    "capability permission contains forbidden grant `{forbidden}`"
                ));
            }
        }
    }

    if capability_source.contains("\"scope\"") {
        capability_findings
            .push("capability must not declare filesystem or command scopes".to_string());
    }

    let handler = "tauri::generate_handler![\n            commands::scan,\n            commands::rescan,\n            commands::freshness,\n            commands::user_settings,\n            commands::set_user_settings,\n            commands::log_file_path\n        ]";
    if !lib_source.contains("commands::scan")
        || !lib_source.contains("commands::rescan")
        || !lib_source.contains("commands::freshness")
        || !lib_source.contains("commands::user_settings")
        || !lib_source.contains("commands::set_user_settings")
        || !lib_source.contains("commands::log_file_path")
    {
        command_findings.push(format!(
            "invoke handler must expose exactly scan, rescan, freshness, user_settings, set_user_settings and log_file_path (checked structure: {handler})"
        ));
    }

    // Every file under `src/**` is scanned for the forbidden mutation
    // patterns. A sanctioned module is no longer skipped wholesale: it is
    // looked up in `SANCTIONED_WRITERS` and only its own enumerated
    // `allowed` patterns are permitted — every other forbidden pattern,
    // including the always-denied set (`remove_file`, `remove_dir`,
    // `.set_len(`, `set_permissions`, `hard_link`, `symlink_file`,
    // `symlink_dir`), still fails the audit even inside a sanctioned
    // module. `#[cfg(test)]` module bodies are stripped first: this audit
    // is about the *production* command surface, and test helpers
    // legitimately create scratch directories/files under the OS temp dir,
    // which is not the surface this test proves anything about.
    for (relative_path, source) in all_rs_files_under(&src_dir) {
        let production_source = strip_cfg_test_blocks(&source);
        let sanctioned = SANCTIONED_WRITERS
            .iter()
            .find(|writer| writer.module == relative_path);

        for forbidden in FORBIDDEN_MUTATION_PATTERNS {
            if !is_pattern_permitted(sanctioned, forbidden) && production_source.contains(forbidden)
            {
                command_findings.push(format!(
                    "{relative_path} contains forbidden mutation pattern `{forbidden}`"
                ));
            }
        }
    }

    assert_eq!(
        SANCTIONED_WRITERS.len(),
        3,
        "growing the sanctioned-writer list is a reviewed event"
    );

    // Every sanctioned module's path derivation is proved individually —
    // path derived from `app_data_dir()`, no literal absolute path, no
    // `std::env::` read — rather than accepted merely by presence in the
    // list (design §10, desktop-shell "The Read-Only Audit Recognizes A
    // Second Write Exception").
    for writer in &SANCTIONED_WRITERS {
        let source_full = fs::read_to_string(src_dir.join(writer.module)).unwrap_or_else(|_| {
            panic!("{} must exist once it lands", writer.module);
        });
        let production_source = strip_cfg_test_blocks(&source_full);
        assert_write_path_is_derived_from_app_data_dir(
            writer.module,
            &production_source,
            &mut command_findings,
        );
    }

    ShellAuditReport {
        commands,
        permissions,
        capability_findings,
        command_findings,
        static_proof_is_limited: true,
    }
}

/// Whether `pattern` is one of `sanctioned`'s own enumerated `allowed`
/// patterns. `sanctioned: None` (an unlisted module) permits nothing — the
/// always-denied set (`remove_file`, `remove_dir`, `.set_len(`,
/// `set_permissions`, `hard_link`, `symlink_file`, `symlink_dir`) is never
/// in any module's `allowed` list, so it stays denied everywhere, including
/// inside both sanctioned exceptions (design §10).
fn is_pattern_permitted(sanctioned: Option<&SanctionedWriter>, pattern: &str) -> bool {
    sanctioned.is_some_and(|writer| writer.allowed.contains(&pattern))
}

/// The three path-derivation proof obligations any sanctioned writer must
/// satisfy: references `app_data_dir`, reads no environment variable
/// directly, and contains no literal absolute path (design §10).
fn assert_write_path_is_derived_from_app_data_dir(
    module: &str,
    production_source: &str,
    findings: &mut Vec<String>,
) {
    if !production_source.contains("app_data_dir") {
        findings.push(format!(
            "{module} must derive its path from app_data_dir(), found no reference"
        ));
    }
    if production_source.contains("std::env::") || production_source.contains(" env::") {
        findings.push(format!("{module} must not read the environment directly"));
    }
    for literal_path_marker in ["C:\\\\", "C:/", "\"/home/", "\"/Users/", "\"/etc/"] {
        if production_source.contains(literal_path_marker) {
            findings.push(format!(
                "{module} must not contain a literal absolute path (`{literal_path_marker}`)"
            ));
        }
    }
}

const FORBIDDEN_MUTATION_PATTERNS: [&str; 16] = [
    "File::create",
    "OpenOptions",
    ".write(",
    ".write_all(",
    "fs::write",
    "std::fs::write",
    ".set_len(",
    "remove_file",
    "remove_dir",
    "std::fs::rename",
    "fs::rename",
    "create_dir",
    "hard_link",
    "symlink_file",
    "symlink_dir",
    "set_permissions",
];

/// Every `.rs` file under `dir`, recursively, as `(path relative to `dir`
/// with forward slashes, file contents)`. No external walker dependency —
/// a small hand-rolled recursion is enough for this audit's own source
/// tree.
fn all_rs_files_under(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect_rs_files(dir, dir, &mut out);
    out
}

/// Remove every `#[cfg(test)] mod ... { ... }` block from `source` by
/// brace-depth counting, leaving only production code. Deliberately
/// conservative: it only strips a block that is immediately introduced by
/// the literal `#[cfg(test)]` attribute on its own line, which is this
/// workspace's exclusive convention (grepped: every test module in this
/// crate is written exactly that way).
fn strip_cfg_test_blocks(source: &str) -> String {
    let marker = "#[cfg(test)]";
    let mut result = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(marker_index) = rest.find(marker) {
        result.push_str(&rest[..marker_index]);
        let after_marker = &rest[marker_index + marker.len()..];
        let Some(brace_open) = after_marker.find('{') else {
            // No block follows (shouldn't happen in practice) — keep
            // scanning past the marker itself so we make progress.
            result.push_str(marker);
            rest = after_marker;
            continue;
        };

        let body = &after_marker[brace_open..];
        let mut depth = 0usize;
        let mut end_index = None;
        let mut in_string = false;
        let mut escaped = false;
        for (index, ch) in body.char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_index = Some(index + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        match end_index {
            Some(end) => {
                rest = &body[end..];
            }
            None => {
                // Unbalanced braces (shouldn't happen for valid Rust) —
                // stop stripping to avoid discarding the rest of the file.
                result.push_str(after_marker);
                rest = "";
                break;
            }
        }
    }

    result.push_str(rest);
    result
}

fn collect_rs_files(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(root, &path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let contents = fs::read_to_string(&path).unwrap_or_default();
            out.push((relative, contents));
        }
    }
}

fn exported_tauri_commands(source: &str) -> Vec<&'static str> {
    let mut commands = Vec::new();
    let mut next_public_async_fn_is_command = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == "#[tauri::command]" {
            next_public_async_fn_is_command = true;
            continue;
        }

        if next_public_async_fn_is_command {
            if trimmed.starts_with("pub async fn scan(") {
                commands.push("scan");
            } else if trimmed.starts_with("pub async fn rescan(") {
                commands.push("rescan");
            } else if trimmed.starts_with("pub async fn freshness(") {
                commands.push("freshness");
            } else if trimmed.starts_with("pub async fn user_settings(") {
                commands.push("user_settings");
            } else if trimmed.starts_with("pub async fn set_user_settings(") {
                commands.push("set_user_settings");
            } else if trimmed.starts_with("pub async fn log_file_path(") {
                commands.push("log_file_path");
            }
            next_public_async_fn_is_command = false;
        }
    }

    commands
}

fn capability_permissions(source: &str) -> Vec<String> {
    let Some(permissions_key_index) = source.find("\"permissions\"") else {
        return Vec::new();
    };
    let permissions_section = &source[permissions_key_index..];
    let Some(start_index) = permissions_section.find('[') else {
        return Vec::new();
    };
    let Some(end_index) = permissions_section[start_index..].find(']') else {
        return Vec::new();
    };

    permissions_section[start_index + 1..start_index + end_index]
        .split(',')
        .filter_map(|raw| {
            let permission = raw.trim().trim_matches('"');
            (!permission.is_empty()).then(|| permission.to_string())
        })
        .collect()
}

/// `logging.rs`'s exception is narrow: it does not extend to
/// `remove_file`, nor to a literal Windows path marker — both stay denied
/// even inside a sanctioned module (design §14 E2, second half).
#[test]
fn logging_module_exception_does_not_extend_to_remove_file_or_a_literal_windows_path() {
    let logging_writer = SANCTIONED_WRITERS
        .iter()
        .find(|writer| writer.module == "logging.rs")
        .expect("logging.rs must be a sanctioned writer");

    assert!(!is_pattern_permitted(Some(logging_writer), "remove_file"));
    assert!(!is_pattern_permitted(Some(logging_writer), "remove_dir"));
    assert!(!is_pattern_permitted(Some(logging_writer), ".set_len("));
    assert!(!is_pattern_permitted(
        Some(logging_writer),
        "set_permissions"
    ));
    assert!(!is_pattern_permitted(Some(logging_writer), "hard_link"));
    assert!(!is_pattern_permitted(Some(logging_writer), "symlink_file"));
    assert!(!is_pattern_permitted(Some(logging_writer), "symlink_dir"));
}

/// `cache.rs`'s allow-list is two entries only: any forbidden pattern
/// outside `["fs::write", "create_dir"]` — including patterns `logging.rs`
/// is allowed — must still fail if `cache.rs` ever used it (design §14 E3).
#[test]
fn cache_module_allow_list_does_not_extend_beyond_its_own_two_entries() {
    let cache_writer = SANCTIONED_WRITERS
        .iter()
        .find(|writer| writer.module == "freshness/cache.rs")
        .expect("freshness/cache.rs must be a sanctioned writer");

    assert!(is_pattern_permitted(Some(cache_writer), "fs::write"));
    assert!(is_pattern_permitted(Some(cache_writer), "create_dir"));
    assert!(!is_pattern_permitted(Some(cache_writer), "OpenOptions"));
    assert!(!is_pattern_permitted(Some(cache_writer), ".write_all("));
    assert!(!is_pattern_permitted(Some(cache_writer), "File::create"));
    assert!(!is_pattern_permitted(Some(cache_writer), "fs::rename"));
    assert!(!is_pattern_permitted(Some(cache_writer), "remove_file"));
}

/// `settings/store.rs`'s allow-list is exactly three entries: any forbidden
/// pattern outside `["fs::write", "create_dir", "fs::rename"]` must still
/// fail if the module ever used it (user-settings spec "The settings
/// module's allow-list does not extend beyond its three operations").
#[test]
fn settings_store_allow_list_does_not_extend_beyond_its_own_three_entries() {
    let settings_writer = SANCTIONED_WRITERS
        .iter()
        .find(|writer| writer.module == "settings/store.rs")
        .expect("settings/store.rs must be a sanctioned writer");

    assert!(is_pattern_permitted(Some(settings_writer), "fs::write"));
    assert!(is_pattern_permitted(Some(settings_writer), "create_dir"));
    assert!(is_pattern_permitted(Some(settings_writer), "fs::rename"));
    assert!(!is_pattern_permitted(Some(settings_writer), "remove_file"));
    assert!(!is_pattern_permitted(Some(settings_writer), "remove_dir"));
    assert!(!is_pattern_permitted(Some(settings_writer), "OpenOptions"));
    assert!(!is_pattern_permitted(Some(settings_writer), "File::create"));
    assert!(!is_pattern_permitted(Some(settings_writer), ".write_all("));
    assert!(!is_pattern_permitted(Some(settings_writer), ".set_len("));
    assert!(!is_pattern_permitted(
        Some(settings_writer),
        "set_permissions"
    ));
    assert!(!is_pattern_permitted(Some(settings_writer), "hard_link"));
    assert!(!is_pattern_permitted(Some(settings_writer), "symlink_file"));
    assert!(!is_pattern_permitted(Some(settings_writer), "symlink_dir"));
}

/// A module named nowhere in `SANCTIONED_WRITERS` is permitted nothing at
/// all — the blanket `continue` this design replaces is gone.
#[test]
fn an_unsanctioned_module_is_permitted_no_forbidden_pattern() {
    assert!(!is_pattern_permitted(None, "fs::write"));
    assert!(!is_pattern_permitted(None, "create_dir"));
}
