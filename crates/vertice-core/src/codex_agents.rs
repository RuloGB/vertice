//! Codex agent discovery: a flat walk over `~/.codex/agents/*.toml`, one
//! agent per file, no recursion, no embedded pseudo-root (design §6.3).
//!
//! `scan` is infallible: it takes an already-resolved `home` and returns an
//! owned [`CodexAgentScan`]. Structurally `agents.rs`'s walk with three
//! deliberate differences (design §6.3): no embedded pseudo-root (there is
//! no verified list of agents Codex ships with no file), no `escalate`
//! function (`crate::toml::from_str` returns a `TomlError`, so every issue
//! here is constructed at the failure site with the severity already
//! correct), and the file extension is `.toml`, not `.md`.

use std::path::Path;

use serde::Deserialize;

use crate::model::{
    Component, ComponentId, ComponentKind, IssueSeverity, Location, LocationOrigin, ScanIssue,
    Scope, SearchRoot,
};
use crate::roots;

/// Contract for a Codex agent `*.toml`. `Deserialize`-only: no `Serialize`,
/// no `TS`, so it emits no binding. Permissive by design: unmodelled Codex
/// keys are ignored, never an error. Not named `…Frontmatter` — a Codex
/// agent file has no `---`-fenced block and no body (design §5.4).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CodexAgentDocument {
    /// Required. Absent or non-string => the whole file is an `Error`.
    pub name: String,
    pub description: Option<String>,
    /// Parsed and pinned by tests — the multiline `"""…"""` case the seam
    /// exists for — but deliberately NOT mapped onto `Component` (design
    /// §6.3).
    pub developer_instructions: Option<String>,
}

/// Owned result of one Codex agent scan. A distinct type from `AgentScan`
/// and `OpenCodeAgentScan`, not an alias, not a shared generic.
#[derive(Debug, Clone, PartialEq)]
pub struct CodexAgentScan {
    /// Always exactly one root.
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

/// Scan the Codex agent root under `home`. Read-only: `roots::probe`'s
/// `symlink_metadata` (via `roots::codex_agent_root`), `std::fs::read_dir`,
/// and `std::fs::read_to_string` are the COMPLETE disk surface — no write of
/// any kind, anywhere (CA-16).
pub fn scan(home: &Path) -> CodexAgentScan {
    let resolved = roots::codex_agent_root(home);

    let mut components = Vec::new();
    let mut issues = Vec::new();

    walk_agents_root(&resolved, &mut components, &mut issues);

    CodexAgentScan {
        roots: vec![resolved.root],
        components,
        issues,
    }
}

/// Walk the on-disk agent root (flat, never recursive) and accumulate
/// components/issues into the caller's buffers.
fn walk_agents_root(
    resolved: &roots::ResolvedRoot,
    components: &mut Vec<Component>,
    issues: &mut Vec<ScanIssue>,
) {
    // `roots::codex_agent_root` always gives the on-disk root exactly one
    // scan path.
    let Some(scan_path) = resolved.scan_paths.first() else {
        return;
    };
    let root_id = &resolved.root.id;
    let client = resolved.root.client;

    let metadata = match std::fs::symlink_metadata(scan_path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(scan_path.to_path_buf()),
                reason: format!("could not inspect search root: {err}"),
            });
            return;
        }
    };

    if !metadata.is_dir() {
        issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(scan_path.to_path_buf()),
            reason: "search root is not a directory".to_string(),
        });
        return;
    }

    let read_dir = match std::fs::read_dir(scan_path) {
        Ok(read_dir) => read_dir,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(scan_path.to_path_buf()),
                reason: format!("could not read search root: {err}"),
            });
            return;
        }
    };

    // `read_dir` yields OS-dependent order. Collect then sort so component
    // order is identical on every CI platform (design §6.3).
    let mut entries: Vec<std::fs::DirEntry> = Vec::new();
    for entry in read_dir {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(err) => {
                // A bare `io::Error` carries no `DirEntry` and therefore no
                // path; attribute it to the root instead.
                issues.push(ScanIssue {
                    severity: IssueSeverity::Error,
                    path: Some(scan_path.to_path_buf()),
                    reason: format!("could not read directory entry: {err}"),
                });
            }
        }
    }
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                issues.push(ScanIssue {
                    severity: IssueSeverity::Error,
                    path: Some(entry.path()),
                    reason: format!("could not read directory entry: {err}"),
                });
                continue;
            }
        };

        let path = entry.path();

        if !file_type.is_file() || path.extension() != Some(std::ffi::OsStr::new("toml")) {
            continue;
        }

        if let Err(lossy) = ensure_utf8_path(&path) {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: None,
                reason: format!("skipped a file whose path is not valid UTF-8: {lossy}"),
            });
            continue;
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) => {
                issues.push(ScanIssue {
                    severity: IssueSeverity::Error,
                    path: Some(path.clone()),
                    reason: format!("could not read Codex agent file: {err}"),
                });
                continue;
            }
        };

        match crate::toml::from_str::<CodexAgentDocument>(&contents) {
            Ok(document) => components.push(Component {
                id: ComponentId::derive(ComponentKind::Agent, &document.name),
                name: document.name,
                kind: ComponentKind::Agent,
                description: document.description,
                scope: Scope::User,
                locations: vec![Location {
                    path: Some(path),
                    root: root_id.clone(),
                    origin: LocationOrigin::File,
                    mcp_transport: None,
                    client,
                }],
                provenance_hint: None,
            }),
            Err(err) => issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(path),
                reason: format!("could not parse Codex agent file: {err}"),
            }),
        }
    }
}

/// Check whether a discovered path is representable as UTF-8, returning the
/// lossy display string on failure. Split out from the walk loop so the
/// conversion itself is directly unit-testable — no portable fixture can
/// produce a non-UTF-8 path on disk.
fn ensure_utf8_path(path: &Path) -> Result<(), String> {
    if path.to_str().is_some() {
        Ok(())
    } else {
        Err(path.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_component_fails_the_utf8_check() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let invalid = OsStr::from_bytes(&[0x66, 0x6f, 0x80, 0x6f]);
        let path = Path::new(invalid);

        assert!(ensure_utf8_path(path).is_err());
    }

    #[test]
    fn utf8_path_passes_the_utf8_check() {
        let path = Path::new("/home/user/.codex/agents/reviewer.toml");

        assert!(ensure_utf8_path(path).is_ok());
    }
}
