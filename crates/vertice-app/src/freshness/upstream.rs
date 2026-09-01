//! Per-slot upstream identity resolution (design §6). Pure table lookup, no
//! I/O: `ClientInstallSlot` -> `UpstreamIdentity`, or `None` when a slot has
//! no queryable upstream at all (`ClaudeCodeBundled`, permanently).

use vertice_core::model::ClientInstallSlot;

/// One upstream source a reference version can be fetched from. The
/// request URL and the cache key are both derived here so `fetch.rs` and
/// `cache.rs` never construct either by hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpstreamIdentity {
    Npm {
        package: &'static str,
    },
    GitHubReleases {
        owner: &'static str,
        repo: &'static str,
    },
}

impl UpstreamIdentity {
    /// The cache key: `npm:<package>` or `github:<owner>/<repo>` (design
    /// §8 — keyed on upstream identity, never on slot, so two slots that
    /// ever shared one upstream would not be refetched twice).
    pub fn cache_key(&self) -> String {
        match self {
            UpstreamIdentity::Npm { package } => format!("npm:{package}"),
            UpstreamIdentity::GitHubReleases { owner, repo } => {
                format!("github:{owner}/{repo}")
            }
        }
    }

    /// The exact request URL (design §6). No query parameters, no
    /// identifier — the URL alone is the entire request content beyond the
    /// mandatory `User-Agent` header.
    pub fn request_url(&self) -> String {
        match self {
            UpstreamIdentity::Npm { package } => {
                let encoded = package.replace('/', "%2f");
                format!("https://registry.npmjs.org/{encoded}/latest")
            }
            UpstreamIdentity::GitHubReleases { owner, repo } => {
                format!("https://api.github.com/repos/{owner}/{repo}/releases/latest")
            }
        }
    }
}

/// The §6 table. `None` means "no upstream at all" — `evaluate` must never
/// be told `Found`/`Unavailable` for such a slot, and no request is ever
/// built for it.
pub fn upstream_for(slot: ClientInstallSlot) -> Option<UpstreamIdentity> {
    match slot {
        ClientInstallSlot::ClaudeCodeNpm => Some(UpstreamIdentity::Npm {
            package: "@anthropic-ai/claude-code",
        }),
        ClientInstallSlot::OpenCodeNpm => Some(UpstreamIdentity::Npm {
            package: "opencode-ai",
        }),
        ClientInstallSlot::CodexStandalone => Some(UpstreamIdentity::GitHubReleases {
            owner: "openai",
            repo: "codex",
        }),
        ClientInstallSlot::ClaudeCodeBundled | ClientInstallSlot::OpenCodeDesktop => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// design §6's table, verbatim: each npm/GitHub slot maps to its exact
    /// identity and request URL; the bundled slot maps to no request at
    /// all.
    #[test]
    fn each_slot_maps_to_its_design_section_6_upstream_identity() {
        assert_eq!(
            upstream_for(ClientInstallSlot::ClaudeCodeNpm),
            Some(UpstreamIdentity::Npm {
                package: "@anthropic-ai/claude-code"
            })
        );
        assert_eq!(
            upstream_for(ClientInstallSlot::OpenCodeNpm),
            Some(UpstreamIdentity::Npm {
                package: "opencode-ai"
            })
        );
        assert_eq!(
            upstream_for(ClientInstallSlot::CodexStandalone),
            Some(UpstreamIdentity::GitHubReleases {
                owner: "openai",
                repo: "codex"
            })
        );
        assert_eq!(upstream_for(ClientInstallSlot::ClaudeCodeBundled), None);
        assert_eq!(upstream_for(ClientInstallSlot::OpenCodeDesktop), None);
    }

    #[test]
    fn claude_code_bundled_issues_no_request_by_construction() {
        // No request is ever built because no `UpstreamIdentity` exists to
        // build one from — this is the compile-time shape of "issues no
        // request", not a runtime check.
        assert!(upstream_for(ClientInstallSlot::ClaudeCodeBundled).is_none());
    }

    /// `detect-desktop-client-installs` design §4.3: `app-update.yml` names
    /// `anomalyco/opencode`, but it is unverified whether the desktop app
    /// and the `opencode-ai` CLI share a release-tag namespace — wiring it
    /// risks a false "outdated" badge, so this slot deliberately has no
    /// upstream, exactly like `ClaudeCodeBundled` (logged in
    /// `internal-docs/pendientes-desarrollo.md`, entry P17).
    #[test]
    fn opencode_desktop_issues_no_request_by_construction() {
        assert!(upstream_for(ClientInstallSlot::OpenCodeDesktop).is_none());
    }

    #[test]
    fn npm_request_urls_match_the_registry_latest_endpoint() {
        assert_eq!(
            UpstreamIdentity::Npm {
                package: "@anthropic-ai/claude-code"
            }
            .request_url(),
            "https://registry.npmjs.org/@anthropic-ai%2fclaude-code/latest"
        );
        assert_eq!(
            UpstreamIdentity::Npm {
                package: "opencode-ai"
            }
            .request_url(),
            "https://registry.npmjs.org/opencode-ai/latest"
        );
    }

    #[test]
    fn github_request_url_matches_the_releases_latest_endpoint() {
        assert_eq!(
            UpstreamIdentity::GitHubReleases {
                owner: "openai",
                repo: "codex"
            }
            .request_url(),
            "https://api.github.com/repos/openai/codex/releases/latest"
        );
    }

    #[test]
    fn cache_keys_are_stable_and_distinct_per_identity() {
        assert_eq!(
            UpstreamIdentity::Npm {
                package: "opencode-ai"
            }
            .cache_key(),
            "npm:opencode-ai"
        );
        assert_eq!(
            UpstreamIdentity::GitHubReleases {
                owner: "openai",
                repo: "codex"
            }
            .cache_key(),
            "github:openai/codex"
        );
    }
}
