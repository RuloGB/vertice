//! Client installation detection: three independent probe slots (Claude
//! Code npm, Claude Code bundled in Claude Desktop, OpenCode npm), each
//! resolved independently into zero, one, or (for the bundled slot) many
//! [`crate::model::ClientInstallation`] values, or an explicit "not
//! detected"/"broken" [`ScanIssue`].
//!
//! The bundled slot is a *resolver*, not a fixed path: Claude Desktop ships
//! as an MSIX package, so its payload lives under a per-install,
//! per-machine `AppData/Local/Packages/Claude_<publisherhash>/...` path. This
//! module enumerates the direct children of `AppData/Local/Packages`
//! matching the `Claude_*` prefix and probes each one, plus the classic
//! `AppData/Roaming/Claude/claude-code/` path as a fallback for non-packaged
//! (legacy) installs. Every candidate root is resolved independently and
//! never merged with another, even the two npm/bundled Claude Code slots
//! (CA-7); an absent slot yields exactly one "not detected" `Warning`, never
//! an `Error` (CA-11).
//!
//! Windows only for T7; macOS/Linux path tables are T16. `roots::probe`
//! stays private and untouched — this module carries its own local `exists`
//! helper. `model/` is unmodified: the not-detected signal is carried
//! entirely through `ScanIssue`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::jsonc::{self, JsonValue};
use crate::model::{ClientInstallation, ClientKind, IssueSeverity, ScanIssue};

/// Owned result of one client-installation scan. A distinct type from
/// `SkillScan`, `AgentScan` and `OpenCodeAgentScan`, and deliberately
/// WITHOUT a `roots` field.
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
    /// The ONLY compile-target branch in this module. `cfg!` is an
    /// EXPRESSION, never an attribute: under `#[cfg]`,
    /// `windows_install_probes` would not be compiled on the Linux/macOS CI
    /// legs, and the unconditional call to it from `scan`/`scan_for` would
    /// fail to compile there — a hard build break, not a coverage gap.
    pub(crate) fn current() -> Self {
        if cfg!(target_os = "windows") {
            HostPlatform::Windows
        } else {
            HostPlatform::Unsupported
        }
    }
}

/// Scan for installed clients under `home`, dispatching on the compiled
/// target. Infallible, mirroring the component adapters.
pub fn scan(home: &Path) -> InstallationScan {
    scan_for(home, HostPlatform::current())
}

/// Same scan against an explicit platform's path table. Public because it
/// is what makes the Windows table testable on the Linux and macOS CI
/// legs — not a general-purpose knob.
pub fn scan_for(home: &Path, platform: HostPlatform) -> InstallationScan {
    let mut installations = Vec::new();
    let mut issues = Vec::new();

    match platform {
        HostPlatform::Windows => {
            let probes = windows_install_probes(home, &mut issues);
            for (slot, candidates) in group_probes_by_slot(&probes) {
                resolve_slot(slot, &candidates, &mut installations, &mut issues);
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

/// Which client a slot belongs to, its not-detected label, and where its
/// version comes from. Replaces the old `(ClientKind, InstallKind)` pair,
/// which could express nonsense combinations and forced a single
/// `{Client} ({kind})` string template that cannot produce
/// `"Claude Code CLI (npm)"` and `"Claude Code (bundled in Claude Desktop)"`
/// from the same grammar. Never exposed as a public/`TS` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallSlot {
    ClaudeCodeNpm,
    ClaudeCodeBundled,
    OpenCodeNpm,
}

impl InstallSlot {
    fn client(self) -> ClientKind {
        match self {
            InstallSlot::ClaudeCodeNpm | InstallSlot::ClaudeCodeBundled => ClientKind::ClaudeCode,
            InstallSlot::OpenCodeNpm => ClientKind::OpenCode,
        }
    }

    /// The settled label used in `"{label} not detected"` and in this
    /// slot's `Error` reasons.
    fn label(self) -> &'static str {
        match self {
            InstallSlot::ClaudeCodeNpm => "Claude Code CLI (npm)",
            InstallSlot::ClaudeCodeBundled => "Claude Code (bundled in Claude Desktop)",
            InstallSlot::OpenCodeNpm => "OpenCode (npm)",
        }
    }

    fn version_source(self) -> VersionSource {
        match self {
            InstallSlot::ClaudeCodeNpm | InstallSlot::OpenCodeNpm => VersionSource::PackageJson,
            InstallSlot::ClaudeCodeBundled => VersionSource::DirectoryName,
        }
    }
}

/// One candidate path for a slot. The bundled slot MAY contribute more than
/// one entry (one per resolved `Claude_*` package, plus the legacy path);
/// the npm slots always contribute exactly one.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallProbe {
    slot: InstallSlot,
    path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionSource {
    PackageJson,
    DirectoryName,
}

/// Build the Windows probe list under `home`: the two npm slots (always
/// exactly one candidate each, `home` plus hardcoded segments — never
/// `dirs`/`directories`, never an env read, `plan-desarrollo-poc.md:179`),
/// and the bundled slot (1..N candidates from a bounded, read-only,
/// one-level-deep `Claude_*`-prefix-filtered listing of
/// `home/AppData/Local/Packages`, plus the hardcoded legacy path, always
/// appended last). Enumeration-level issues (an unreadable `Packages`
/// directory, a broken `DirEntry`) are pushed to `issues` here, since this
/// is the only place that performs the enumeration; per-candidate-root
/// issues (an empty version directory, a non-UTF-8 version name) are the
/// responsibility of `resolve_slot`.
fn windows_install_probes(home: &Path, issues: &mut Vec<ScanIssue>) -> Vec<InstallProbe> {
    let mut probes = Vec::new();

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
    probes.push(InstallProbe {
        slot: InstallSlot::ClaudeCodeNpm,
        path: claude_npm,
    });

    for candidate in bundled_candidates(home, issues) {
        probes.push(InstallProbe {
            slot: InstallSlot::ClaudeCodeBundled,
            path: candidate,
        });
    }

    let mut opencode_npm = home.to_path_buf();
    for segment in ["AppData", "Roaming", "npm", "node_modules", "opencode-ai"] {
        opencode_npm.push(segment);
    }
    probes.push(InstallProbe {
        slot: InstallSlot::OpenCodeNpm,
        path: opencode_npm,
    });

    probes
}

/// Candidate roots for the bundled slot: one per resolved `Claude_*`
/// package under `home/AppData/Local/Packages`, sorted byte-wise on the
/// package directory name (never locale collation), followed
/// unconditionally by the legacy path. An absent `Packages` directory is
/// not an event (CA-11: the "no candidate" verdict is `resolve_slot`'s
/// job). A present-but-unreadable `Packages` directory, or a broken
/// `DirEntry` mid-iteration, is an `Error` here and never blocks the legacy
/// fallback, since the legacy candidate is always appended regardless.
fn bundled_candidates(home: &Path, issues: &mut Vec<ScanIssue>) -> Vec<PathBuf> {
    let mut packages_dir = home.to_path_buf();
    for segment in ["AppData", "Local", "Packages"] {
        packages_dir.push(segment);
    }

    let mut candidates = Vec::new();

    match std::fs::read_dir(&packages_dir) {
        Ok(entries) => {
            let mut names = Vec::new();
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(err) => {
                        issues.push(ScanIssue {
                            severity: IssueSeverity::Error,
                            path: Some(packages_dir.clone()),
                            reason: format!("could not read a Packages directory entry: {err}"),
                        });
                        continue;
                    }
                };

                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }

                names.push(entry.file_name());
            }

            for name in filter_and_sort_claude_packages(names) {
                let mut candidate = packages_dir.clone();
                candidate.push(&name);
                for segment in ["LocalCache", "Roaming", "Claude", "claude-code"] {
                    candidate.push(segment);
                }
                candidates.push(candidate);
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            // Absence is not an event; resolve_slot decides the verdict.
        }
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(packages_dir.clone()),
                reason: format!("could not read the Packages directory: {err}"),
            });
        }
    }

    let mut legacy = home.to_path_buf();
    for segment in ["AppData", "Roaming", "Claude", "claude-code"] {
        legacy.push(segment);
    }
    candidates.push(legacy);

    candidates
}

/// Pure filter+sort step, factored out of `bundled_candidates` so the
/// byte-exact `Claude_` prefix match and the byte-wise ordering are
/// unit-testable without touching the filesystem. Prefix match is byte-exact
/// on `OsStr::as_encoded_bytes()` — no UTF-8 requirement, no allocation for
/// the comparison, no locale-dependent case folding (which would itself be
/// an OS-convention inference, forbidden by `plan-desarrollo-poc.md:179`).
fn filter_and_sort_claude_packages(names: Vec<OsString>) -> Vec<OsString> {
    let mut matched: Vec<OsString> = names
        .into_iter()
        .filter(|name| name.as_encoded_bytes().starts_with(b"Claude_"))
        .collect();
    matched.sort_by(|a, b| a.as_encoded_bytes().cmp(b.as_encoded_bytes()));
    matched
}

/// Group a flat probe list into `(slot, candidate paths)` pairs. Probes for
/// the same slot are always contiguous (`windows_install_probes` emits them
/// slot-by-slot), so a simple run-length grouping is enough — no sorting or
/// hashing needed, and grouping never reorders candidates within a slot.
fn group_probes_by_slot(probes: &[InstallProbe]) -> Vec<(InstallSlot, Vec<PathBuf>)> {
    let mut groups: Vec<(InstallSlot, Vec<PathBuf>)> = Vec::new();
    for probe in probes {
        match groups.last_mut() {
            Some((slot, paths)) if *slot == probe.slot => paths.push(probe.path.clone()),
            _ => groups.push((probe.slot, vec![probe.path.clone()])),
        }
    }
    groups
}

/// Probe whether `path` exists on disk. `NotFound` is the only negative
/// answer; any other outcome (present, or a probe error of another kind) is
/// treated as "something is there" (mirrors `roots::probe`'s semantics —
/// `roots::probe` itself stays private and untouched, this is a deliberate
/// 3-line local duplicate).
fn exists(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

/// Resolve one slot's verdict from its candidate paths, pushing zero-to-N
/// installations and issues. Every slot is resolved independently — a
/// broken candidate in one slot never blocks another slot or another
/// candidate in the same slot (isolation).
fn resolve_slot(
    slot: InstallSlot,
    candidates: &[PathBuf],
    installations: &mut Vec<ClientInstallation>,
    issues: &mut Vec<ScanIssue>,
) {
    match slot.version_source() {
        VersionSource::PackageJson => resolve_npm_slot(slot, &candidates[0], installations, issues),
        VersionSource::DirectoryName => {
            resolve_bundled_slot(slot, candidates, installations, issues)
        }
    }
}

/// Resolve an npm slot: absent directory -> `Warning` "not detected";
/// present directory -> read `package.json` through the `jsonc.rs` seam and
/// extract `"version"`.
fn resolve_npm_slot(
    slot: InstallSlot,
    path: &Path,
    installations: &mut Vec<ClientInstallation>,
    issues: &mut Vec<ScanIssue>,
) {
    debug_assert_eq!(slot.version_source(), VersionSource::PackageJson);

    if !exists(path) {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: Some(path.to_path_buf()),
            reason: format!("{} not detected", slot.label()),
        });
        return;
    }

    let mut package_json = path.to_path_buf();
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
            client: slot.client(),
            version,
            path: path.to_path_buf(),
        }),
        None => issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(package_json),
            reason: "package.json has no \"version\" string".to_string(),
        }),
    }
}

/// Extract `"version"` from a parsed `package.json` document at the value
/// level: present and a non-empty `JsonValue::String` -> `Some`; absent,
/// non-string, or empty -> `None` (the collapsed row). No regex, no
/// `#[derive(Deserialize)]` DTO.
fn extract_package_json_version(document: &JsonValue) -> Option<String> {
    let JsonValue::Object(map) = document else {
        return None;
    };

    match map.get("version") {
        Some(JsonValue::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Resolve the bundled slot from 1..N candidate roots (one per resolved
/// `Claude_*` package, plus the legacy path always last): each existing
/// candidate root yields its own versioned subdirectories as independent
/// `ClientInstallation` values, never merged across candidate roots even on
/// matching version strings; an existing-but-empty candidate root is its
/// own `Error`; the overall "not detected" `Warning` fires exactly once,
/// only when NO candidate root exists at all, with `path` set to the legacy
/// fallback (always the last candidate).
fn resolve_bundled_slot(
    slot: InstallSlot,
    candidates: &[PathBuf],
    installations: &mut Vec<ClientInstallation>,
    issues: &mut Vec<ScanIssue>,
) {
    debug_assert_eq!(slot.version_source(), VersionSource::DirectoryName);
    debug_assert!(
        !candidates.is_empty(),
        "the legacy path is always appended, so candidates is never empty"
    );

    let mut any_candidate_root_exists = false;

    for candidate in candidates {
        if !exists(candidate) {
            // Not a candidate root at all: contributes nothing, no issue.
            continue;
        }
        any_candidate_root_exists = true;

        let entries = match std::fs::read_dir(candidate) {
            Ok(entries) => entries,
            Err(err) => {
                issues.push(ScanIssue {
                    severity: IssueSeverity::Error,
                    path: Some(candidate.clone()),
                    reason: format!("could not read the {} directory: {err}", slot.label()),
                });
                continue;
            }
        };

        // Collected as (file-name-bytes, path) so the sort below is
        // byte-wise, never locale collation, regardless of the platform's
        // own `read_dir` order.
        let mut versions: Vec<(OsString, PathBuf)> = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    issues.push(ScanIssue {
                        severity: IssueSeverity::Error,
                        path: Some(candidate.clone()),
                        reason: format!("could not read the {} directory: {err}", slot.label()),
                    });
                    continue;
                }
            };

            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }

            versions.push((entry.file_name(), entry.path()));
        }
        versions.sort_by(|a, b| a.0.as_encoded_bytes().cmp(b.0.as_encoded_bytes()));

        if versions.is_empty() {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(candidate.clone()),
                reason: format!(
                    "expected at least one {} version directory, found none",
                    slot.label()
                ),
            });
            continue;
        }

        for (file_name, path) in versions {
            match install_from_version_dir(slot, file_name, path) {
                Ok(installation) => installations.push(installation),
                Err(issue) => issues.push(issue),
            }
        }
    }

    if !any_candidate_root_exists {
        let legacy_path = candidates
            .last()
            .expect("the legacy path is always appended last")
            .clone();
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: Some(legacy_path),
            reason: format!("{} not detected", slot.label()),
        });
    }
}

/// Convert one bundled-slot version directory into a `ClientInstallation`,
/// or a `ScanIssue` if the directory's name is not valid UTF-8 (`path: None`
/// — there is no valid string to report; `file_name.to_string_lossy()`
/// still describes it in the `reason`). Factored out of
/// `resolve_bundled_slot` as a pure, I/O-free function so this branch is
/// unit-testable with a synthetic non-UTF-8 `OsString`, which is not
/// portable to express as a committed repository fixture.
fn install_from_version_dir(
    slot: InstallSlot,
    file_name: OsString,
    path: PathBuf,
) -> Result<ClientInstallation, ScanIssue> {
    match file_name.to_str() {
        Some(version) => Ok(ClientInstallation {
            client: slot.client(),
            version: version.to_string(),
            path,
        }),
        None => Err(ScanIssue {
            severity: IssueSeverity::Error,
            path: None,
            reason: format!(
                "a {} version directory's name is not valid UTF-8: {}",
                slot.label(),
                file_name.to_string_lossy()
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- HostPlatform --

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

    // -- install_from_version_dir (non-UTF-8 branch pin) --

    #[cfg(unix)]
    #[test]
    fn install_from_version_dir_rejects_non_utf8_name_as_error_with_no_path() {
        use std::os::unix::ffi::OsStringExt;

        let name = OsString::from_vec(vec![0x66, 0x6f, 0x80, 0x6f]);
        let path = PathBuf::from("/home/example/version-dir");

        let result = install_from_version_dir(InstallSlot::ClaudeCodeBundled, name, path);

        let issue = result.expect_err("a non-UTF-8 version directory name must be an Error");
        assert_eq!(issue.severity, IssueSeverity::Error);
        assert_eq!(
            issue.path, None,
            "a non-UTF-8 name has no valid path to report"
        );
        assert!(issue.reason.contains("not valid UTF-8"));
    }

    #[cfg(windows)]
    #[test]
    fn install_from_version_dir_rejects_unpaired_surrogate_as_error_with_no_path() {
        use std::os::windows::ffi::OsStringExt;

        // 0xD800 is an unpaired high surrogate: valid UTF-16 code unit, but
        // not representable as UTF-8 / a valid `&str`.
        let wide: Vec<u16> = vec![0x0066, 0x006f, 0xD800, 0x006f];
        let name = OsString::from_wide(&wide);
        let path = PathBuf::from(r"C:\example\version-dir");

        let result = install_from_version_dir(InstallSlot::ClaudeCodeBundled, name, path);

        let issue = result.expect_err("an unpaired surrogate must be an Error");
        assert_eq!(issue.severity, IssueSeverity::Error);
        assert_eq!(
            issue.path, None,
            "a non-UTF-8 name has no valid path to report"
        );
        assert!(issue.reason.contains("not valid UTF-8"));
    }

    // -- InstallSlot labels --

    #[test]
    fn slot_labels_match_the_settled_vocabulary() {
        assert_eq!(InstallSlot::ClaudeCodeNpm.label(), "Claude Code CLI (npm)");
        assert_eq!(
            InstallSlot::ClaudeCodeBundled.label(),
            "Claude Code (bundled in Claude Desktop)"
        );
        assert_eq!(InstallSlot::OpenCodeNpm.label(), "OpenCode (npm)");
    }

    #[test]
    fn slot_client_maps_both_claude_slots_to_claude_code() {
        assert_eq!(InstallSlot::ClaudeCodeNpm.client(), ClientKind::ClaudeCode);
        assert_eq!(
            InstallSlot::ClaudeCodeBundled.client(),
            ClientKind::ClaudeCode
        );
        assert_eq!(InstallSlot::OpenCodeNpm.client(), ClientKind::OpenCode);
    }

    #[test]
    fn slot_version_source_is_package_json_for_npm_and_directory_name_for_bundled() {
        assert_eq!(
            InstallSlot::ClaudeCodeNpm.version_source(),
            VersionSource::PackageJson
        );
        assert_eq!(
            InstallSlot::OpenCodeNpm.version_source(),
            VersionSource::PackageJson
        );
        assert_eq!(
            InstallSlot::ClaudeCodeBundled.version_source(),
            VersionSource::DirectoryName
        );
    }

    // -- byte-prefix filter + candidate ordering --

    #[test]
    fn filter_and_sort_claude_packages_keeps_only_the_claude_prefix_byte_exact() {
        let names = vec![
            OsString::from("Claude_zzz"),
            OsString::from("NotClaude_abc"),
            OsString::from("claude_lowercase"),
            OsString::from("Claude_aaa"),
        ];

        let filtered = filter_and_sort_claude_packages(names);

        assert_eq!(
            filtered,
            vec![OsString::from("Claude_aaa"), OsString::from("Claude_zzz")],
            "byte-exact prefix match rejects case-folded and unrelated names"
        );
    }

    #[test]
    fn filter_and_sort_claude_packages_orders_byte_wise_not_by_locale() {
        // 'Z' (0x5A) sorts before 'a' (0x61) byte-wise; a locale-aware sort
        // would put "Claude_apple" first.
        let names = vec![
            OsString::from("Claude_apple"),
            OsString::from("Claude_Zebra"),
        ];

        let filtered = filter_and_sort_claude_packages(names);

        assert_eq!(
            filtered,
            vec![
                OsString::from("Claude_Zebra"),
                OsString::from("Claude_apple")
            ]
        );
    }

    // -- version extraction --

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

    // -- probe/group plumbing --

    #[test]
    fn windows_install_probes_builds_npm_slots_as_home_plus_hardcoded_segments() {
        let home = PathBuf::from("/home/example");
        let mut issues = Vec::new();

        let probes = windows_install_probes(&home, &mut issues);

        let npm = probes
            .iter()
            .find(|p| p.slot == InstallSlot::ClaudeCodeNpm)
            .expect("npm slot present");
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
        assert_eq!(npm.path, expected_claude_npm);

        let opencode = probes
            .iter()
            .find(|p| p.slot == InstallSlot::OpenCodeNpm)
            .expect("opencode slot present");
        let mut expected_opencode_npm = home.clone();
        for segment in ["AppData", "Roaming", "npm", "node_modules", "opencode-ai"] {
            expected_opencode_npm.push(segment);
        }
        assert_eq!(opencode.path, expected_opencode_npm);
    }

    #[test]
    fn windows_install_probes_always_appends_the_legacy_bundled_candidate_last() {
        let home = PathBuf::from("/home/example");
        let mut issues = Vec::new();

        let probes = windows_install_probes(&home, &mut issues);

        let bundled: Vec<&InstallProbe> = probes
            .iter()
            .filter(|p| p.slot == InstallSlot::ClaudeCodeBundled)
            .collect();
        assert!(
            !bundled.is_empty(),
            "the legacy candidate is always appended"
        );

        let mut expected_legacy = home.clone();
        for segment in ["AppData", "Roaming", "Claude", "claude-code"] {
            expected_legacy.push(segment);
        }
        assert_eq!(bundled.last().unwrap().path, expected_legacy);
    }

    #[test]
    fn group_probes_by_slot_keeps_multiple_bundled_candidates_together() {
        let probes = vec![
            InstallProbe {
                slot: InstallSlot::ClaudeCodeNpm,
                path: PathBuf::from("npm"),
            },
            InstallProbe {
                slot: InstallSlot::ClaudeCodeBundled,
                path: PathBuf::from("pkg-a"),
            },
            InstallProbe {
                slot: InstallSlot::ClaudeCodeBundled,
                path: PathBuf::from("legacy"),
            },
            InstallProbe {
                slot: InstallSlot::OpenCodeNpm,
                path: PathBuf::from("opencode"),
            },
        ];

        let groups = group_probes_by_slot(&probes);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].0, InstallSlot::ClaudeCodeNpm);
        assert_eq!(groups[0].1, vec![PathBuf::from("npm")]);
        assert_eq!(groups[1].0, InstallSlot::ClaudeCodeBundled);
        assert_eq!(
            groups[1].1,
            vec![PathBuf::from("pkg-a"), PathBuf::from("legacy")]
        );
        assert_eq!(groups[2].0, InstallSlot::OpenCodeNpm);
        assert_eq!(groups[2].1, vec![PathBuf::from("opencode")]);
    }
}
