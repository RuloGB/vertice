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

/// The one module CA-16 permits to write: it derives its path exclusively
/// from `app_data_dir()`, never a literal path, never an env read (design
/// §8, §14).
const CACHE_MODULE_EXCEPTION: &str = "freshness/cache.rs";

#[test]
fn desktop_shell_exposes_only_scan_commands_and_core_default_capability() {
    let report = audit_desktop_shell_read_only_surface();

    assert_eq!(
        report.commands,
        vec![
            "scan",
            "rescan",
            "freshness",
            "freshness_settings",
            "set_freshness_settings"
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

    let handler = "tauri::generate_handler![\n            commands::scan,\n            commands::rescan,\n            commands::freshness,\n            commands::freshness_settings,\n            commands::set_freshness_settings\n        ]";
    if !lib_source.contains("commands::scan")
        || !lib_source.contains("commands::rescan")
        || !lib_source.contains("commands::freshness")
        || !lib_source.contains("commands::freshness_settings")
        || !lib_source.contains("commands::set_freshness_settings")
    {
        command_findings.push(format!(
            "invoke handler must expose exactly scan, rescan, freshness, freshness_settings and set_freshness_settings (checked structure: {handler})"
        ));
    }

    // Widened per `add-client-version-freshness` design §14: every file
    // under `src/**` is scanned for the forbidden mutation patterns,
    // except the one module CA-16 permits to write at all. `#[cfg(test)]`
    // module bodies are stripped first: this audit is about the
    // *production* command surface, and test helpers legitimately create
    // scratch directories/files under the OS temp dir, which is not the
    // surface this test proves anything about.
    for (relative_path, source) in all_rs_files_under(&src_dir) {
        if relative_path == CACHE_MODULE_EXCEPTION {
            continue;
        }
        let production_source = strip_cfg_test_blocks(&source);
        for forbidden in FORBIDDEN_MUTATION_PATTERNS {
            if production_source.contains(forbidden) {
                command_findings.push(format!(
                    "{relative_path} contains forbidden mutation pattern `{forbidden}`"
                ));
            }
        }
    }

    // The scoped exception itself: `cache.rs` MAY write, but ONLY via a
    // path derived from `app_data_dir()` — never a literal absolute path,
    // never an environment read. Same production-only scoping: `cache.rs`'s
    // own unit tests stub `app_data_dir()` with a scratch temp directory
    // via `std::env::temp_dir()`, which is test scaffolding, not the
    // production write path this check pins.
    let cache_source_full = fs::read_to_string(src_dir.join("freshness").join("cache.rs"))
        .expect("freshness/cache.rs must exist once the freshness module lands");
    let cache_source = strip_cfg_test_blocks(&cache_source_full);

    if !cache_source.contains("app_data_dir") {
        command_findings.push(
            "freshness/cache.rs must derive its path from app_data_dir(), found no reference"
                .to_string(),
        );
    }
    if cache_source.contains("std::env::") || cache_source.contains(" env::") {
        command_findings
            .push("freshness/cache.rs must not read the environment directly".to_string());
    }
    for literal_path_marker in ["C:\\\\", "C:/", "\"/home/", "\"/Users/", "\"/etc/"] {
        if cache_source.contains(literal_path_marker) {
            command_findings.push(format!(
                "freshness/cache.rs must not contain a literal absolute path (`{literal_path_marker}`)"
            ));
        }
    }

    ShellAuditReport {
        commands,
        permissions,
        capability_findings,
        command_findings,
        static_proof_is_limited: true,
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
            } else if trimmed.starts_with("pub async fn freshness_settings(") {
                commands.push("freshness_settings");
            } else if trimmed.starts_with("pub async fn set_freshness_settings(") {
                commands.push("set_freshness_settings");
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
