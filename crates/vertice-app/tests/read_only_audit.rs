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

#[test]
fn desktop_shell_exposes_only_scan_commands_and_core_default_capability() {
    let report = audit_desktop_shell_read_only_surface();

    assert_eq!(report.commands, vec!["scan", "rescan"]);
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
    let commands_source = fs::read_to_string(manifest_dir.join("src").join("commands.rs"))
        .expect("commands.rs readable");
    let lib_source =
        fs::read_to_string(manifest_dir.join("src").join("lib.rs")).expect("lib.rs readable");
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

    let handler = "tauri::generate_handler![commands::scan, commands::rescan]";
    if !lib_source.contains(handler) {
        command_findings.push("invoke handler must expose only scan and rescan".to_string());
    }

    for forbidden in [
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
    ] {
        if commands_source.contains(forbidden) || lib_source.contains(forbidden) {
            command_findings.push(format!(
                "desktop command surface contains forbidden mutation pattern `{forbidden}`"
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
