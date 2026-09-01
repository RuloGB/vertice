//! Client installation detection: five independent probe slots (Claude
//! Code npm, Claude Code bundled in Claude Desktop, OpenCode npm, OpenCode
//! desktop app, Codex CLI standalone), each resolved independently into a
//! typed
//! [`crate::model::ClientPresence`] record carrying zero, one, or (for the
//! bundled and Codex standalone slots) many [`crate::model::ClientInstallation`]
//! values, plus a "broken" [`ScanIssue`] where applicable.
//!
//! The bundled slot is a *resolver*, not a fixed path: Claude Desktop ships
//! as an MSIX package, so its payload lives under a per-install,
//! per-machine `AppData/Local/Packages/Claude_<publisherhash>/...` path. This
//! module enumerates the direct children of `AppData/Local/Packages`
//! matching the `Claude_*` prefix and probes each one, plus the classic
//! `AppData/Roaming/Claude/claude-code/` path as a fallback for non-packaged
//! (legacy) installs. Every candidate root is resolved independently and
//! never merged with another, even the two npm/bundled Claude Code slots
//! (CA-7); an absent slot yields `ClientPresenceStatus::NotDetected`, never
//! an `Error` (CA-11).
//!
//! Windows only for T7; macOS/Linux path tables are T16. `roots::probe`
//! stays private and untouched — this module carries its own local `exists`
//! helper. `model/` now carries the absence signal through the typed
//! `ClientPresence`/`ClientPresenceStatus` record, not through `ScanIssue`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::asar;
use crate::jsonc::{self, JsonValue};
use crate::model::{
    ClientInstallSlot, ClientInstallation, ClientKind, ClientPresence, ClientPresenceStatus,
    IssueSeverity, ScanIssue,
};

/// Owned result of one client-installation scan. A distinct type from
/// `SkillScan`, `AgentScan` and `OpenCodeAgentScan`, and deliberately
/// WITHOUT a `roots` field.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallationScan {
    /// One [`ClientPresence`] record per probe slot the platform's table
    /// defines, or `None` when the platform has no probe table at all.
    /// `installations` below is a derived flattening of these records,
    /// computed by [`flatten_presence`] — the only producer.
    pub presence: Option<Vec<ClientPresence>>,
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
    let mut issues = Vec::new();

    let presence = match platform {
        HostPlatform::Windows => {
            let probes = windows_install_probes(home, &mut issues);
            let records = group_probes_by_slot(&probes)
                .into_iter()
                .map(|(slot, candidates)| resolve_slot(slot, &candidates, &mut issues))
                .collect();
            Some(records)
        }
        HostPlatform::Unsupported => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Warning,
                path: None,
                reason: "client installation detection is not implemented on this platform"
                    .to_string(),
            });
            None
        }
    };

    let installations = flatten_presence(&presence);

    InstallationScan {
        presence,
        installations,
        issues,
    }
}

/// The only producer of `InstallationScan.installations`: `resolve_slot`
/// never pushes into it directly. `None` -> empty; `Some` -> concatenation
/// of each record's `installations`, in record order, so ordering matches
/// today's output exactly (slot order = probe-table order, candidates
/// sorted byte-wise, legacy last).
fn flatten_presence(presence: &Option<Vec<ClientPresence>>) -> Vec<ClientInstallation> {
    match presence {
        None => Vec::new(),
        Some(records) => records
            .iter()
            .flat_map(|record| record.installations.clone())
            .collect(),
    }
}

// --- private ---

/// Which client a slot belongs to and where its version comes from.
/// `ClientInstallSlot` itself — the closed identity enum and its `label()`
/// — now lives in `model/slot.rs`, promoted to a public type
/// (`add-client-version-freshness` design §2) because `component-freshness`
/// needs to dispatch on slot identity outside this crate. This impl block
/// carries the detection-only behavior that stays private to this module:
/// which `ClientKind` a slot belongs to, and which of the four (now five,
/// with `AsarPackageJson` — `detect-desktop-client-installs` design §4.2)
/// version sources it reads from.
impl ClientInstallSlot {
    fn client(self) -> ClientKind {
        match self {
            ClientInstallSlot::ClaudeCodeNpm | ClientInstallSlot::ClaudeCodeBundled => {
                ClientKind::ClaudeCode
            }
            ClientInstallSlot::OpenCodeNpm | ClientInstallSlot::OpenCodeDesktop => {
                ClientKind::OpenCode
            }
            ClientInstallSlot::CodexStandalone => ClientKind::Codex,
        }
    }

    fn version_source(self) -> VersionSource {
        match self {
            ClientInstallSlot::ClaudeCodeNpm | ClientInstallSlot::OpenCodeNpm => {
                VersionSource::PackageJson
            }
            ClientInstallSlot::ClaudeCodeBundled => VersionSource::DirectoryName,
            ClientInstallSlot::OpenCodeDesktop => VersionSource::AsarPackageJson,
            ClientInstallSlot::CodexStandalone => VersionSource::ReleaseDirectoryName,
        }
    }
}

/// One candidate path for a slot. The bundled slot MAY contribute more than
/// one entry (one per resolved `Claude_*` package, plus the legacy path);
/// the npm slots always contribute exactly one.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallProbe {
    slot: ClientInstallSlot,
    path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionSource {
    PackageJson,
    DirectoryName,
    ReleaseDirectoryName,
    /// A version read from an `app.asar` header's root `package.json`
    /// (`detect-desktop-client-installs` design §4.2). A new sibling
    /// variant rather than a reuse of `PackageJson`: `PackageJson`'s
    /// contract is "`<root>/package.json` is a loose file on disk", which
    /// is false for the OpenCode desktop app — bending it would hide a
    /// per-slot branch inside `resolve_npm_slot` (T7CD §3.1's reasoning
    /// replayed).
    AsarPackageJson,
}

/// Target triples Codex publishes standalone releases for. MANUAL
/// MAINTENANCE, exactly like `agents::EMBEDDED_CLAUDE_AGENTS`: a triple
/// OpenAI adds is invisible to Vertice until this table is extended.
/// Windows-only for T7; T16 adds the macOS/Linux triples here and nowhere
/// else.
const CODEX_TARGET_TRIPLES: [&str; 2] = ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"];

/// Strip the longest suffix that is `-` followed by an exact member of
/// [`CODEX_TARGET_TRIPLES`]. What remains, if non-empty, is the version.
/// Pure, no I/O — unit-testable without a fixture (design §3.2).
fn split_release_dir_name(name: &str) -> Option<&str> {
    for triple in CODEX_TARGET_TRIPLES {
        let suffix = format!("-{triple}");
        if let Some(version) = name.strip_suffix(suffix.as_str()) {
            if !version.is_empty() {
                return Some(version);
            }
        }
    }
    None
}

/// Build the Windows probe list under `home`: the two npm slots (always
/// exactly one candidate each, `home` plus hardcoded segments — never
/// `dirs`/`directories`, never an env read),
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
        slot: ClientInstallSlot::ClaudeCodeNpm,
        path: claude_npm,
    });

    for candidate in bundled_candidates(home, issues) {
        probes.push(InstallProbe {
            slot: ClientInstallSlot::ClaudeCodeBundled,
            path: candidate,
        });
    }

    let mut opencode_npm = home.to_path_buf();
    for segment in ["AppData", "Roaming", "npm", "node_modules", "opencode-ai"] {
        opencode_npm.push(segment);
    }
    probes.push(InstallProbe {
        slot: ClientInstallSlot::OpenCodeNpm,
        path: opencode_npm,
    });

    let mut opencode_desktop = home.to_path_buf();
    for segment in ["AppData", "Local", "Programs", "@opencode-aidesktop"] {
        opencode_desktop.push(segment);
    }
    probes.push(InstallProbe {
        slot: ClientInstallSlot::OpenCodeDesktop,
        path: opencode_desktop,
    });

    let mut codex_releases = home.to_path_buf();
    for segment in [".codex", "packages", "standalone", "releases"] {
        codex_releases.push(segment);
    }
    probes.push(InstallProbe {
        slot: ClientInstallSlot::CodexStandalone,
        path: codex_releases,
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
/// an OS-convention inference, which path resolution here never makes).
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
fn group_probes_by_slot(probes: &[InstallProbe]) -> Vec<(ClientInstallSlot, Vec<PathBuf>)> {
    let mut groups: Vec<(ClientInstallSlot, Vec<PathBuf>)> = Vec::new();
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

/// Resolve one slot's verdict from its candidate paths into a
/// [`ClientPresence`] record, pushing zero-to-N issues. Every slot is
/// resolved independently — a broken candidate in one slot never blocks
/// another slot or another candidate in the same slot (isolation).
/// Emitting a presence record never itself pushes a `ScanIssue`.
fn resolve_slot(
    slot: ClientInstallSlot,
    candidates: &[PathBuf],
    issues: &mut Vec<ScanIssue>,
) -> ClientPresence {
    match slot.version_source() {
        VersionSource::PackageJson => resolve_npm_slot(slot, &candidates[0], issues),
        VersionSource::DirectoryName => resolve_bundled_slot(slot, candidates, issues),
        VersionSource::ReleaseDirectoryName => resolve_codex_slot(slot, &candidates[0], issues),
        VersionSource::AsarPackageJson => {
            resolve_opencode_desktop_slot(slot, &candidates[0], issues)
        }
    }
}

/// Resolve an npm slot: absent directory -> `status: NotDetected`; present
/// directory -> `status: Detected`, reading `package.json` through the
/// `jsonc.rs` seam and extracting `"version"`. A present-but-broken
/// directory still yields `Detected` with empty `installations`; the
/// underlying `Error` issue is unchanged.
fn resolve_npm_slot(
    slot: ClientInstallSlot,
    path: &Path,
    issues: &mut Vec<ScanIssue>,
) -> ClientPresence {
    debug_assert_eq!(slot.version_source(), VersionSource::PackageJson);

    let probed_paths = vec![path.to_path_buf()];

    if !exists(path) {
        return ClientPresence {
            slot,
            label: slot.label().to_string(),
            probed_paths,
            status: ClientPresenceStatus::NotDetected,
            installations: Vec::new(),
        };
    }

    let mut installations = Vec::new();
    let mut package_json = path.to_path_buf();
    package_json.push("package.json");

    match std::fs::read_to_string(&package_json) {
        Ok(contents) => match jsonc::parse(&contents) {
            Ok(JsonValue::Object(root_map)) => {
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
            Ok(_) => issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(package_json),
                reason: "package.json is not a JSON object".to_string(),
            }),
            Err(err) => issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(package_json),
                reason: format!("could not parse package.json: {err}"),
            }),
        },
        Err(err) => issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(package_json),
            reason: format!("could not read package.json: {err}"),
        }),
    }

    ClientPresence {
        slot,
        label: slot.label().to_string(),
        probed_paths,
        status: ClientPresenceStatus::Detected,
        installations,
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

/// Resolve the OpenCode desktop slot from its single candidate root
/// (`<home>/AppData/Local/Programs/@opencode-aidesktop`). Presence never
/// depends on version extraction (design §5.1): absent root -> `NotDetected`,
/// zero issues (CA-11); present root -> `Detected`, ALWAYS, whatever the
/// archive read does. `path` on the resulting `ClientInstallation` is the
/// install ROOT, never the `.asar` file — `path` answers "where is this
/// installation", and the root is what `probed_paths` already names and
/// what the user recognises (design §4.2).
fn resolve_opencode_desktop_slot(
    slot: ClientInstallSlot,
    root: &Path,
    issues: &mut Vec<ScanIssue>,
) -> ClientPresence {
    debug_assert_eq!(slot.version_source(), VersionSource::AsarPackageJson);

    let probed_paths = vec![root.to_path_buf()];

    if !exists(root) {
        return ClientPresence {
            slot,
            label: slot.label().to_string(),
            probed_paths,
            status: ClientPresenceStatus::NotDetected,
            installations: Vec::new(),
        };
    }

    let mut archive = root.to_path_buf();
    archive.push("resources");
    archive.push("app.asar");

    let mut installations = Vec::new();
    match asar::read_package_version(&archive) {
        Ok(version) => installations.push(ClientInstallation {
            client: slot.client(),
            version,
            path: root.to_path_buf(),
        }),
        Err(err) => {
            // The oversized-header ceiling is the one Vertice-side "we
            // chose not to look" branch (design §5.2): Warning, not Error,
            // and phrased as a deliberate skip rather than a defect.
            let (severity, verb) = match err {
                asar::AsarError::HeaderTooLarge { .. } => (IssueSeverity::Warning, "skipped"),
                _ => (IssueSeverity::Error, "could not read"),
            };
            issues.push(ScanIssue {
                severity,
                path: Some(archive),
                reason: format!("{verb} the {} version: {err}", slot.label()),
            });
        }
    }

    ClientPresence {
        slot,
        label: slot.label().to_string(),
        probed_paths,
        status: ClientPresenceStatus::Detected,
        installations,
    }
}

/// Resolve the bundled slot from 1..N candidate roots (one per resolved
/// `Claude_*` package, plus the legacy path always last): each existing
/// candidate root yields its own versioned subdirectories as independent
/// `ClientInstallation` values, never merged across candidate roots even on
/// matching version strings; an existing-but-empty candidate root is its
/// own `Error`; `status: NotDetected` fires only when NO candidate root
/// exists at all.
fn resolve_bundled_slot(
    slot: ClientInstallSlot,
    candidates: &[PathBuf],
    issues: &mut Vec<ScanIssue>,
) -> ClientPresence {
    debug_assert_eq!(slot.version_source(), VersionSource::DirectoryName);
    debug_assert!(
        !candidates.is_empty(),
        "the legacy path is always appended, so candidates is never empty"
    );

    let mut installations = Vec::new();
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

    ClientPresence {
        slot,
        label: slot.label().to_string(),
        probed_paths: candidates.to_vec(),
        status: if any_candidate_root_exists {
            ClientPresenceStatus::Detected
        } else {
            ClientPresenceStatus::NotDetected
        },
        installations,
    }
}

/// Resolve the Codex standalone slot from its single candidate root
/// (`<home>/.codex/packages/standalone/releases`), enumerated one level
/// deep, never following a symlink. A structural sibling of
/// `resolve_bundled_slot`, not a parameterization of it (design §3.1):
/// unlike the bundled slot, this slot has exactly one candidate root and a
/// distinct failure class (an unparseable release directory name).
fn resolve_codex_slot(
    slot: ClientInstallSlot,
    releases_dir: &Path,
    issues: &mut Vec<ScanIssue>,
) -> ClientPresence {
    debug_assert_eq!(slot.version_source(), VersionSource::ReleaseDirectoryName);

    let probed_paths = vec![releases_dir.to_path_buf()];

    if !exists(releases_dir) {
        return ClientPresence {
            slot,
            label: slot.label().to_string(),
            probed_paths,
            status: ClientPresenceStatus::NotDetected,
            installations: Vec::new(),
        };
    }

    let mut installations = Vec::new();

    let entries = match std::fs::read_dir(releases_dir) {
        Ok(entries) => entries,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(releases_dir.to_path_buf()),
                reason: format!("could not read the {} directory: {err}", slot.label()),
            });
            return ClientPresence {
                slot,
                label: slot.label().to_string(),
                probed_paths,
                status: ClientPresenceStatus::Detected,
                installations,
            };
        }
    };

    // Collected as (file-name-bytes, path) so the sort below is byte-wise,
    // never locale collation, regardless of the platform's own `read_dir`
    // order.
    let mut children: Vec<(OsString, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                issues.push(ScanIssue {
                    severity: IssueSeverity::Error,
                    path: Some(releases_dir.to_path_buf()),
                    reason: format!("could not read the {} directory: {err}", slot.label()),
                });
                continue;
            }
        };

        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        children.push((entry.file_name(), entry.path()));
    }
    children.sort_by(|a, b| a.0.as_encoded_bytes().cmp(b.0.as_encoded_bytes()));

    if children.is_empty() {
        issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(releases_dir.to_path_buf()),
            reason: format!(
                "expected at least one {} release directory, found none",
                slot.label()
            ),
        });
        return ClientPresence {
            slot,
            label: slot.label().to_string(),
            probed_paths,
            status: ClientPresenceStatus::Detected,
            installations,
        };
    }

    for (file_name, path) in children {
        match codex_installation_from_release_dir(slot, &file_name, path) {
            Ok(installation) => installations.push(installation),
            Err(issue) => issues.push(issue),
        }
    }

    ClientPresence {
        slot,
        label: slot.label().to_string(),
        probed_paths,
        status: ClientPresenceStatus::Detected,
        installations,
    }
}

/// Convert one Codex release directory into a `ClientInstallation`, or a
/// `ScanIssue` if its name is not valid UTF-8, matches no known target
/// triple, or strips to an empty version (design §3.3). Factored out so the
/// non-UTF-8 branch is unit-testable with a synthetic `OsString`.
fn codex_installation_from_release_dir(
    slot: ClientInstallSlot,
    file_name: &OsString,
    path: PathBuf,
) -> Result<ClientInstallation, ScanIssue> {
    let Some(name) = file_name.to_str() else {
        return Err(ScanIssue {
            severity: IssueSeverity::Error,
            path: None,
            reason: format!(
                "a {} release directory's name is not valid UTF-8: {}",
                slot.label(),
                file_name.to_string_lossy()
            ),
        });
    };

    match split_release_dir_name(name) {
        Some(version) => Ok(ClientInstallation {
            client: slot.client(),
            version: version.to_string(),
            path,
        }),
        None => Err(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(path),
            reason: format!(
                "could not read a version from the {} release directory name: {name}",
                slot.label()
            ),
        }),
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
    slot: ClientInstallSlot,
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

    // -- flatten_presence --

    #[test]
    fn flatten_presence_none_yields_empty() {
        assert_eq!(flatten_presence(&None), Vec::new());
    }

    #[test]
    fn flatten_presence_concatenates_in_record_order() {
        let make = |version: &str| ClientInstallation {
            client: ClientKind::ClaudeCode,
            version: version.to_string(),
            path: PathBuf::from(version),
        };
        let presence = Some(vec![
            ClientPresence {
                slot: ClientInstallSlot::ClaudeCodeNpm,
                label: "first".to_string(),
                probed_paths: vec![PathBuf::from("first")],
                status: ClientPresenceStatus::Detected,
                installations: vec![make("1.0.0")],
            },
            ClientPresence {
                slot: ClientInstallSlot::OpenCodeNpm,
                label: "second".to_string(),
                probed_paths: vec![PathBuf::from("second")],
                status: ClientPresenceStatus::Detected,
                installations: vec![make("2.0.0"), make("2.1.0")],
            },
        ]);

        let flattened = flatten_presence(&presence);

        let versions: Vec<&str> = flattened.iter().map(|i| i.version.as_str()).collect();
        assert_eq!(versions, vec!["1.0.0", "2.0.0", "2.1.0"]);
    }

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

        let result = install_from_version_dir(ClientInstallSlot::ClaudeCodeBundled, name, path);

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

        let result = install_from_version_dir(ClientInstallSlot::ClaudeCodeBundled, name, path);

        let issue = result.expect_err("an unpaired surrogate must be an Error");
        assert_eq!(issue.severity, IssueSeverity::Error);
        assert_eq!(
            issue.path, None,
            "a non-UTF-8 name has no valid path to report"
        );
        assert!(issue.reason.contains("not valid UTF-8"));
    }

    // -- ClientInstallSlot labels --

    #[test]
    fn slot_labels_match_the_settled_vocabulary() {
        assert_eq!(
            ClientInstallSlot::ClaudeCodeNpm.label(),
            "Claude Code CLI (npm)"
        );
        assert_eq!(
            ClientInstallSlot::ClaudeCodeBundled.label(),
            "Claude Code (bundled in Claude Desktop)"
        );
        assert_eq!(ClientInstallSlot::OpenCodeNpm.label(), "OpenCode (npm)");
        assert_eq!(
            ClientInstallSlot::OpenCodeDesktop.label(),
            "OpenCode (desktop app)"
        );
        assert_eq!(
            ClientInstallSlot::CodexStandalone.label(),
            "Codex CLI (standalone)"
        );
    }

    #[test]
    fn slot_client_maps_both_claude_slots_to_claude_code() {
        assert_eq!(
            ClientInstallSlot::ClaudeCodeNpm.client(),
            ClientKind::ClaudeCode
        );
        assert_eq!(
            ClientInstallSlot::ClaudeCodeBundled.client(),
            ClientKind::ClaudeCode
        );
        assert_eq!(
            ClientInstallSlot::OpenCodeNpm.client(),
            ClientKind::OpenCode
        );
        assert_eq!(
            ClientInstallSlot::OpenCodeDesktop.client(),
            ClientKind::OpenCode
        );
        assert_eq!(
            ClientInstallSlot::CodexStandalone.client(),
            ClientKind::Codex
        );
    }

    #[test]
    fn slot_version_source_is_package_json_for_npm_and_directory_name_for_bundled() {
        assert_eq!(
            ClientInstallSlot::ClaudeCodeNpm.version_source(),
            VersionSource::PackageJson
        );
        assert_eq!(
            ClientInstallSlot::OpenCodeNpm.version_source(),
            VersionSource::PackageJson
        );
        assert_eq!(
            ClientInstallSlot::ClaudeCodeBundled.version_source(),
            VersionSource::DirectoryName
        );
        assert_eq!(
            ClientInstallSlot::OpenCodeDesktop.version_source(),
            VersionSource::AsarPackageJson
        );
        assert_eq!(
            ClientInstallSlot::CodexStandalone.version_source(),
            VersionSource::ReleaseDirectoryName
        );
    }

    // -- split_release_dir_name (design §3.2) --

    #[test]
    fn split_release_dir_name_strips_the_known_triple_suffix() {
        assert_eq!(
            split_release_dir_name("0.149.0-x86_64-pc-windows-msvc"),
            Some("0.149.0")
        );
    }

    #[test]
    fn split_release_dir_name_is_prerelease_safe() {
        // The case that kills "split on the first `-`", which would yield
        // "0.150.0" and silently report a prerelease as a release.
        assert_eq!(
            split_release_dir_name("0.150.0-rc.1-x86_64-pc-windows-msvc"),
            Some("0.150.0-rc.1")
        );
    }

    #[test]
    fn split_release_dir_name_rejects_an_unknown_triple() {
        assert_eq!(
            split_release_dir_name("0.151.0-riscv64-unknown-linux-gnu"),
            None
        );
    }

    #[test]
    fn split_release_dir_name_rejects_the_bare_triple_with_no_version() {
        assert_eq!(split_release_dir_name("x86_64-pc-windows-msvc"), None);
    }

    #[test]
    fn split_release_dir_name_rejects_a_name_with_no_dash() {
        assert_eq!(split_release_dir_name("nightly"), None);
    }

    #[test]
    fn split_release_dir_name_strips_the_aarch64_triple_too() {
        assert_eq!(
            split_release_dir_name("0.149.0-aarch64-pc-windows-msvc"),
            Some("0.149.0")
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
            .find(|p| p.slot == ClientInstallSlot::ClaudeCodeNpm)
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
            .find(|p| p.slot == ClientInstallSlot::OpenCodeNpm)
            .expect("opencode slot present");
        let mut expected_opencode_npm = home.clone();
        for segment in ["AppData", "Roaming", "npm", "node_modules", "opencode-ai"] {
            expected_opencode_npm.push(segment);
        }
        assert_eq!(opencode.path, expected_opencode_npm);
    }

    #[test]
    fn windows_install_probes_builds_opencode_desktop_as_home_plus_hardcoded_segments() {
        let home = PathBuf::from("/home/example");
        let mut issues = Vec::new();

        let probes = windows_install_probes(&home, &mut issues);

        let opencode_desktop = probes
            .iter()
            .find(|p| p.slot == ClientInstallSlot::OpenCodeDesktop)
            .expect("opencode desktop slot present");
        let mut expected = home.clone();
        for segment in ["AppData", "Local", "Programs", "@opencode-aidesktop"] {
            expected.push(segment);
        }
        assert_eq!(opencode_desktop.path, expected);

        // Position is load-bearing (design §4.1): between OpenCodeNpm and
        // CodexStandalone.
        let slot_order: Vec<ClientInstallSlot> = probes.iter().map(|p| p.slot).collect();
        let npm_index = slot_order
            .iter()
            .position(|s| *s == ClientInstallSlot::OpenCodeNpm)
            .unwrap();
        let desktop_index = slot_order
            .iter()
            .position(|s| *s == ClientInstallSlot::OpenCodeDesktop)
            .unwrap();
        let codex_index = slot_order
            .iter()
            .position(|s| *s == ClientInstallSlot::CodexStandalone)
            .unwrap();
        assert!(npm_index < desktop_index);
        assert!(desktop_index < codex_index);
    }

    #[test]
    fn windows_install_probes_always_appends_the_legacy_bundled_candidate_last() {
        let home = PathBuf::from("/home/example");
        let mut issues = Vec::new();

        let probes = windows_install_probes(&home, &mut issues);

        let bundled: Vec<&InstallProbe> = probes
            .iter()
            .filter(|p| p.slot == ClientInstallSlot::ClaudeCodeBundled)
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
                slot: ClientInstallSlot::ClaudeCodeNpm,
                path: PathBuf::from("npm"),
            },
            InstallProbe {
                slot: ClientInstallSlot::ClaudeCodeBundled,
                path: PathBuf::from("pkg-a"),
            },
            InstallProbe {
                slot: ClientInstallSlot::ClaudeCodeBundled,
                path: PathBuf::from("legacy"),
            },
            InstallProbe {
                slot: ClientInstallSlot::OpenCodeNpm,
                path: PathBuf::from("opencode"),
            },
        ];

        let groups = group_probes_by_slot(&probes);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].0, ClientInstallSlot::ClaudeCodeNpm);
        assert_eq!(groups[0].1, vec![PathBuf::from("npm")]);
        assert_eq!(groups[1].0, ClientInstallSlot::ClaudeCodeBundled);
        assert_eq!(
            groups[1].1,
            vec![PathBuf::from("pkg-a"), PathBuf::from("legacy")]
        );
        assert_eq!(groups[2].0, ClientInstallSlot::OpenCodeNpm);
        assert_eq!(groups[2].1, vec![PathBuf::from("opencode")]);
    }
}
