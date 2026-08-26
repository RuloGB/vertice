//! Claude Code agent discovery: a flat walk over `~/.claude/agents/*.md`
//! plus a fixed set of embedded agents that ship inside Claude Code with no
//! backing file.
//!
//! `scan` is infallible: it takes an already-resolved `home` and returns an
//! owned [`AgentScan`]. Unlike [`crate::skills`], the walk is deliberately
//! flat (`std::fs::read_dir`, no `walkdir`, never recursive) — see
//! `design.md` §6. `AgentFrontmatter` lives here rather than in
//! [`crate::frontmatter`] because its only consumer is this module
//! (design §5.3); `SkillScan`/`AgentScan` are deliberately separate types,
//! not a shared abstraction (design §5.4).

use std::path::Path;

use serde::Deserialize;

use crate::frontmatter;
use crate::model::{
    Component, ComponentId, ComponentKind, IssueSeverity, Location, LocationOrigin, ScanIssue,
    Scope, SearchRoot, SearchRootId,
};
use crate::roots::{self, ResolvedRoot};

/// The six agents Claude Code ships with no file behind them. Provenance:
/// `claude agents` on the reference machine, verified 2026-08-18
/// (`alcance-poc-vertice.md:118`, finding 4). MANUAL MAINTENANCE: if
/// Anthropic adds, removes or renames one, Vertice is silently wrong until
/// the T16 oracle contrast is re-run. One named const, never scattered.
const EMBEDDED_CLAUDE_AGENTS: [&str; 6] = [
    "Explore",
    "Plan",
    "general-purpose",
    "statusline-setup",
    "claude",
    "claude-code-guide",
];

/// Owned result of one agent scan. A distinct type from `SkillScan`, not an
/// alias and not a shared generic — see design §5.4.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentScan {
    /// Always the two roots `roots::agent_roots` resolves (design §3).
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

/// Frontmatter contract for a Claude Code agent `*.md`. `Deserialize`-only:
/// no `Serialize`, no `TS`, so it emits no binding (design §2).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AgentFrontmatter {
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    /// Comma-separated scalar (`tools: Read, Grep, Glob, Bash`), NEVER a
    /// sequence type — verified against the 17 real agent files on the
    /// reference machine (proposal, Approach).
    pub tools: Option<String>,
}

/// Scan the Claude Code agent roots under `home`. Read-only: `roots::probe`,
/// `std::fs::read_dir`, and `frontmatter::read`'s file reads are the
/// complete disk surface — no write of any kind, anywhere.
pub fn scan(home: &Path) -> AgentScan {
    let [agents_root, embedded_root] = roots::agent_roots(home);

    let mut components = Vec::new();
    let mut issues = Vec::new();

    walk_agents_root(&agents_root, &mut components, &mut issues);

    let embedded_status = embedded_root.root.status;
    if embedded_status == crate::model::SearchRootStatus::Found {
        emit_embedded_components(&embedded_root.root.id, &mut components);
    }

    AgentScan {
        roots: vec![agents_root.root, embedded_root.root],
        components,
        issues,
    }
}

/// Walk the on-disk agent root (flat, never recursive) and accumulate
/// components/issues into the caller's buffers.
fn walk_agents_root(
    resolved: &ResolvedRoot,
    components: &mut Vec<Component>,
    issues: &mut Vec<ScanIssue>,
) {
    // `roots::agent_roots` always gives the on-disk root exactly one scan
    // path.
    let Some(scan_path) = resolved.scan_paths.first() else {
        return;
    };
    let root_id = &resolved.root.id;

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

    // `read_dir` yields OS-dependent order — unlike `walkdir`'s
    // `sort_by_file_name()`. Collect then sort so component order is
    // identical on every CI platform (design §6).
    let mut entries: Vec<std::fs::DirEntry> = Vec::new();
    for entry in read_dir {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(err) => {
                // A bare `io::Error` carries no `DirEntry` and therefore no
                // path; attribute it to the root instead (design §8).
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

        if !file_type.is_file() || path.extension() != Some(std::ffi::OsStr::new("md")) {
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

        match frontmatter::read::<AgentFrontmatter>(&path) {
            Ok(fm) => components.push(Component {
                id: ComponentId::derive(ComponentKind::Agent, &fm.name),
                name: fm.name,
                kind: ComponentKind::Agent,
                description: fm.description,
                scope: Scope::User,
                locations: vec![Location {
                    path: Some(path),
                    root: root_id.clone(),
                    origin: LocationOrigin::File,
                    mcp_transport: None,
                }],
                provenance_hint: None,
            }),
            Err(issue) => issues.push(escalate(issue)),
        }
    }
}

/// Emit the six embedded Claude Code agents as components with
/// `origin: Embedded`, `path: None`. Caller MUST only invoke this when the
/// embedded pseudo-root's status is `Found` (design §4).
fn emit_embedded_components(embedded_root_id: &SearchRootId, components: &mut Vec<Component>) {
    for name in EMBEDDED_CLAUDE_AGENTS {
        components.push(Component {
            id: ComponentId::derive(ComponentKind::Agent, name),
            name: name.to_string(),
            kind: ComponentKind::Agent,
            description: None,
            scope: Scope::User,
            locations: vec![Location {
                path: None,
                root: embedded_root_id.clone(),
                origin: LocationOrigin::Embedded,
                mcp_transport: None,
            }],
            provenance_hint: None,
        });
    }
}

/// Every failure to parse a discovered agent `.md` is escalated to
/// `IssueSeverity::Error`, uniformly — `path`/`reason` untouched. Detection
/// under the agent root is "if there is a `.md` file, it is an agent", so
/// every failure to parse one is an agent missing from the user's inventory
/// (design §7). Structurally identical to `skills::escalate`, duplicated
/// rather than shared per design §5.4.
fn escalate(issue: ScanIssue) -> ScanIssue {
    ScanIssue {
        severity: IssueSeverity::Error,
        ..issue
    }
}

/// Check whether a discovered path is representable as UTF-8, returning the
/// lossy display string on failure. Split out from the walk loop so the
/// conversion itself is directly unit-testable (design §7.1) — no portable
/// fixture can produce a non-UTF-8 path on disk.
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
    use crate::model::IssueSeverity;

    /// Every T3 severity class, once escalated, surfaces as `Error` — the
    /// deferred-to-T16 BOM/`NoOpeningFence` case included (design §7).
    #[test]
    fn escalate_maps_every_severity_to_error() {
        for original in [IssueSeverity::Warning, IssueSeverity::Error] {
            let issue = ScanIssue {
                severity: original,
                path: Some(std::path::PathBuf::from("/some/agent.md")),
                reason: "no frontmatter block: file does not begin with a --- fence".to_string(),
            };

            let escalated = escalate(issue.clone());

            assert_eq!(escalated.severity, IssueSeverity::Error);
            assert_eq!(escalated.path, issue.path);
            assert_eq!(escalated.reason, issue.reason);
        }
    }

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
        let path = Path::new("/home/user/.claude/agents/reviewer.md");

        assert!(ensure_utf8_path(path).is_ok());
    }
}
