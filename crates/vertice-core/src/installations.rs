//! Client installation detection: three independent probe slots (Claude
//! Code npm, Claude Code desktop, OpenCode npm), each resolved
//! independently into zero, one, or (for the desktop slot) many
//! [`crate::model::ClientInstallation`] values, or an explicit "not
//! detected"/"broken" [`ScanIssue`] (design §1/§8).
//!
//! Windows only for T7; macOS/Linux path tables are T16 (design §5.2,
//! §11). `roots::probe` stays private and untouched — this module carries
//! its own local `exists` helper (design §5.3). `model/` is unmodified: the
//! not-detected signal is carried entirely through `ScanIssue` (design §2).

use std::path::{Path, PathBuf};

use crate::jsonc::{self, JsonValue};
use crate::model::{ClientInstallation, ClientKind, IssueSeverity, ScanIssue};

/// Owned result of one client-installation scan. A distinct type from
/// `SkillScan`, `AgentScan` and `OpenCodeAgentScan`, and deliberately
/// WITHOUT a `roots` field (design §3).
#[derive(Debug, Clone, PartialEq)]
pub struct InstallationScan {
    pub installations: Vec<ClientInstallation>,
    pub issues: Vec<ScanIssue>,
}

/// Which OS path table to use. NOT a model type: no `Serialize`, no `TS`.
/// T16 replaces `Unsupported` with `MacOs` and `Linux`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPlatform {
    Windows,
    Unsupported,
}

impl HostPlatform {
    /// The ONLY compile-target branch in this change. `cfg!` is an
    /// EXPRESSION, never an attribute (design §5.2): under `#[cfg]`,
    /// `windows_install_probes` would not be compiled on the Linux/macOS CI
    /// legs, and the unconditional call to it from `scan`/`scan_for` would
    /// fail to compile there — a hard build break, not a coverage gap.
    fn current() -> Self {
        if cfg!(target_os = "windows") {
            HostPlatform::Windows
        } else {
            HostPlatform::Unsupported
        }
    }
}

/// Scan for installed clients under `home`, dispatching on the compiled
/// target (design §5.2). Infallible, mirroring the component adapters.
pub fn scan(home: &Path) -> InstallationScan {
    scan_for(home, HostPlatform::current())
}

/// Same scan against an explicit platform's path table. Public because it
/// is what makes the Windows table testable on the Linux and macOS CI legs
/// (design §5.2) — not a general-purpose knob.
pub fn scan_for(home: &Path, platform: HostPlatform) -> InstallationScan {
    let mut installations = Vec::new();
    let mut issues = Vec::new();

    match platform {
        HostPlatform::Windows => {
            for probe in windows_install_probes(home) {
                resolve(&probe, &mut installations, &mut issues);
            }
        }
        HostPlatform::Unsupported => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Warning,
                path: None,
                reason: "client installation detection is not implemented on this platform"
                    .to_string(),
            });
        }
    }

    InstallationScan {
        installations,
        issues,
    }
}

// --- private ---

/// One fixed probe slot: which client, which install kind, its hardcoded
/// path under `home`, and where its version comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallProbe {
    client: ClientKind,
    kind: InstallKind,
    path: PathBuf,
    version: VersionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallKind {
    Npm,
    Desktop,
}

impl InstallKind {
    /// The `{kind}` label in the §4 `reason` grammar. Never exposed as a
    /// public/`TS` type — this is the only place it reaches the outside
    /// world (design §5.1).
    fn label(self) -> &'static str {
        match self {
            InstallKind::Npm => "npm",
            InstallKind::Desktop => "desktop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionSource {
    PackageJson,
    DirectoryName,
}

/// The `{Client}` label in the §4 `reason` grammar.
fn client_label(client: ClientKind) -> &'static str {
    match client {
        ClientKind::ClaudeCode => "Claude Code",
        ClientKind::OpenCode => "OpenCode",
    }
}

/// Build the fixed, hardcoded Windows probe table under `home` (design
/// §11). Always exactly 3 entries, in a fixed order: Claude Code npm,
/// Claude Code desktop, OpenCode npm. Every path is `home` plus hardcoded
/// segments pushed one at a time — never `dirs`/`directories`, never an
/// env read (`plan-desarrollo-poc.md:179`).
fn windows_install_probes(home: &Path) -> [InstallProbe; 3] {
    let mut claude_npm = home.to_path_buf();
    for segment in [
        "AppData",
        "Roaming",
        "npm",
        "node_modules",
        "@anthropic-ai",
        "claude-code",
    ] {
        claude_npm.push(segment);
    }

    let mut claude_desktop = home.to_path_buf();
    for segment in ["AppData", "Roaming", "Claude", "claude-code"] {
        claude_desktop.push(segment);
    }

    let mut opencode_npm = home.to_path_buf();
    for segment in ["AppData", "Roaming", "npm", "node_modules", "opencode-ai"] {
        opencode_npm.push(segment);
    }

    [
        InstallProbe {
            client: ClientKind::ClaudeCode,
            kind: InstallKind::Npm,
            path: claude_npm,
            version: VersionSource::PackageJson,
        },
        InstallProbe {
            client: ClientKind::ClaudeCode,
            kind: InstallKind::Desktop,
            path: claude_desktop,
            version: VersionSource::DirectoryName,
        },
        InstallProbe {
            client: ClientKind::OpenCode,
            kind: InstallKind::Npm,
            path: opencode_npm,
            version: VersionSource::PackageJson,
        },
    ]
}

/// Probe whether `path` exists on disk. `NotFound` is the only negative
/// answer; any other outcome (present, or a probe error of another kind)
/// is treated as "something is there" (mirrors `roots::probe`'s semantics,
/// design §5.3 — `roots::probe` itself stays private and untouched, this
/// is a deliberate 3-line local duplicate).
fn exists(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

/// Resolve one probe slot, pushing zero-to-N installations and at most one
/// issue for the npm slots, or the desktop slot's own N-installations shape
/// (design §5.1/§6). Never returns early on failure — every slot is
/// resolved independently (design §8/isolation).
fn resolve(
    probe: &InstallProbe,
    installations: &mut Vec<ClientInstallation>,
    issues: &mut Vec<ScanIssue>,
) {
    match probe.kind {
        InstallKind::Npm => resolve_npm(probe, installations, issues),
        InstallKind::Desktop => resolve_desktop(probe, installations, issues),
    }
}

/// Resolve an npm slot: absent directory -> `Warning` "not detected";
/// present directory -> read `package.json` through the `jsonc.rs` seam
/// and extract `"version"` (design §5.4/§8).
fn resolve_npm(
    probe: &InstallProbe,
    installations: &mut Vec<ClientInstallation>,
    issues: &mut Vec<ScanIssue>,
) {
    debug_assert_eq!(probe.version, VersionSource::PackageJson);

    if !exists(&probe.path) {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: Some(probe.path.clone()),
            reason: format!(
                "{} ({}) not detected",
                client_label(probe.client),
                probe.kind.label()
            ),
        });
        return;
    }

    let mut package_json = probe.path.clone();
    package_json.push("package.json");

    let contents = match std::fs::read_to_string(&package_json) {
        Ok(contents) => contents,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(package_json),
                reason: format!("could not read package.json: {err}"),
            });
            return;
        }
    };

    let parsed = match jsonc::parse(&contents) {
        Ok(parsed) => parsed,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(package_json),
                reason: format!("could not parse package.json: {err}"),
            });
            return;
        }
    };

    let JsonValue::Object(root_map) = parsed else {
        issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(package_json),
            reason: "package.json is not a JSON object".to_string(),
        });
        return;
    };

    match extract_package_json_version(&JsonValue::Object(root_map)) {
        Some(version) => installations.push(ClientInstallation {
            client: probe.client,
            version,
            path: probe.path.clone(),
        }),
        None => issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(package_json),
            reason: "package.json has no \"version\" string".to_string(),
        }),
    }
}

/// Extract `"version"` from a parsed `package.json` document at the value
/// level (design §5.4): present and a non-empty `JsonValue::String` ->
/// `Some`; absent, non-string, or empty -> `None` (the collapsed row,
/// design §8/V5). No regex, no `#[derive(Deserialize)]` DTO.
fn extract_package_json_version(document: &JsonValue) -> Option<String> {
    let JsonValue::Object(map) = document else {
        return None;
    };

    match map.get("version") {
        Some(JsonValue::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Resolve the Claude Code desktop slot: absent directory -> `Warning` "not
/// detected"; present directory with zero candidate subdirectories ->
/// `Error`; present directory with N >= 1 candidate subdirectories -> N
/// `ClientInstallation` values, one per candidate, sorted by file name
/// byte-wise (design §6/§7). A non-UTF-8 candidate name yields its own
/// `Error` and never blocks any other candidate.
fn resolve_desktop(
    probe: &InstallProbe,
    installations: &mut Vec<ClientInstallation>,
    issues: &mut Vec<ScanIssue>,
) {
    debug_assert_eq!(probe.version, VersionSource::DirectoryName);

    if !exists(&probe.path) {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: Some(probe.path.clone()),
            reason: format!(
                "{} ({}) not detected",
                client_label(probe.client),
                probe.kind.label()
            ),
        });
        return;
    }

    let entries = match std::fs::read_dir(&probe.path) {
        Ok(entries) => entries,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(probe.path.clone()),
                reason: format!("could not read the Claude Code desktop directory: {err}"),
            });
            return;
        }
    };

    // Collected as (file-name-bytes, path) so the sort below is byte-wise,
    // never locale collation (design §7), regardless of the platform's
    // own `read_dir` order.
    let mut candidates: Vec<(std::ffi::OsString, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                issues.push(ScanIssue {
                    severity: IssueSeverity::Error,
                    path: Some(probe.path.clone()),
                    reason: format!("could not read the Claude Code desktop directory: {err}"),
                });
                continue;
            }
        };

        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        candidates.push((entry.file_name(), entry.path()));
    }
    candidates.sort_by(|a, b| a.0.as_encoded_bytes().cmp(b.0.as_encoded_bytes()));

    if candidates.is_empty() {
        issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(probe.path.clone()),
            reason: "expected at least one Claude Code desktop version directory, found none"
                .to_string(),
        });
        return;
    }

    for (file_name, path) in candidates {
        match file_name.to_str() {
            Some(version) => installations.push(ClientInstallation {
                client: probe.client,
                version: version.to_string(),
                path,
            }),
            None => issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: None,
                reason: format!(
                    "a Claude Code desktop version directory's name is not valid UTF-8: {}",
                    file_name.to_string_lossy()
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- windows_install_probes (task 1.1) --

    #[test]
    fn windows_install_probes_returns_exactly_three_entries_in_fixed_order() {
        let home = PathBuf::from("/home/example");

        let probes = windows_install_probes(&home);

        assert_eq!(probes.len(), 3);
        assert_eq!(probes[0].client, ClientKind::ClaudeCode);
        assert_eq!(probes[0].kind, InstallKind::Npm);
        assert_eq!(probes[0].version, VersionSource::PackageJson);
        assert_eq!(probes[1].client, ClientKind::ClaudeCode);
        assert_eq!(probes[1].kind, InstallKind::Desktop);
        assert_eq!(probes[1].version, VersionSource::DirectoryName);
        assert_eq!(probes[2].client, ClientKind::OpenCode);
        assert_eq!(probes[2].kind, InstallKind::Npm);
        assert_eq!(probes[2].version, VersionSource::PackageJson);
    }

    #[test]
    fn windows_install_probes_paths_are_home_plus_hardcoded_segments() {
        let home = PathBuf::from("/home/example");

        let probes = windows_install_probes(&home);

        let mut expected_claude_npm = home.clone();
        for segment in [
            "AppData",
            "Roaming",
            "npm",
            "node_modules",
            "@anthropic-ai",
            "claude-code",
        ] {
            expected_claude_npm.push(segment);
        }
        assert_eq!(probes[0].path, expected_claude_npm);

        let mut expected_claude_desktop = home.clone();
        for segment in ["AppData", "Roaming", "Claude", "claude-code"] {
            expected_claude_desktop.push(segment);
        }
        assert_eq!(probes[1].path, expected_claude_desktop);

        let mut expected_opencode_npm = home.clone();
        for segment in ["AppData", "Roaming", "npm", "node_modules", "opencode-ai"] {
            expected_opencode_npm.push(segment);
        }
        assert_eq!(probes[2].path, expected_opencode_npm);
    }

    #[test]
    fn windows_install_probes_structure_is_identical_for_two_different_homes() {
        let alice = windows_install_probes(&PathBuf::from("/home/alice"));
        let bob = windows_install_probes(&PathBuf::from("/home/bob"));

        for (a, b) in alice.iter().zip(bob.iter()) {
            assert_eq!(a.client, b.client);
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.version, b.version);
        }
    }

    // -- HostPlatform (task 1.3) --

    #[test]
    fn host_platform_variants_are_constructible_and_inspectable_everywhere() {
        assert_eq!(HostPlatform::Windows, HostPlatform::Windows);
        assert_eq!(HostPlatform::Unsupported, HostPlatform::Unsupported);
        assert_ne!(HostPlatform::Windows, HostPlatform::Unsupported);
    }

    #[test]
    fn host_platform_current_uses_cfg_expression_matching_the_compiled_target() {
        let expected = if cfg!(target_os = "windows") {
            HostPlatform::Windows
        } else {
            HostPlatform::Unsupported
        };

        assert_eq!(HostPlatform::current(), expected);
    }

    // -- version extraction (task 2.1) --

    fn obj(pairs: &[(&str, JsonValue)]) -> JsonValue {
        let mut map = std::collections::BTreeMap::new();
        for (key, value) in pairs {
            map.insert((*key).to_string(), value.clone());
        }
        JsonValue::Object(map)
    }

    #[test]
    fn version_string_present_yields_some() {
        let doc = obj(&[("version", JsonValue::String("1.2.3".to_string()))]);

        assert_eq!(
            extract_package_json_version(&doc),
            Some("1.2.3".to_string())
        );
    }

    #[test]
    fn version_absent_yields_none() {
        let doc = obj(&[("name", JsonValue::String("pkg".to_string()))]);

        assert_eq!(extract_package_json_version(&doc), None);
    }

    #[test]
    fn version_non_string_yields_none() {
        let doc = obj(&[("version", JsonValue::Number("3".to_string()))]);

        assert_eq!(extract_package_json_version(&doc), None);
    }

    #[test]
    fn version_empty_string_yields_none() {
        let doc = obj(&[("version", JsonValue::String(String::new()))]);

        assert_eq!(extract_package_json_version(&doc), None);
    }

    #[test]
    fn version_empty_document_yields_none() {
        let doc = obj(&[]);

        assert_eq!(extract_package_json_version(&doc), None);
    }
}
