//! Defensive response parsing and the live HTTP call (design §6, §9, §12).
//! `parse_npm_latest`/`parse_github_latest_release` are pure functions over
//! bytes — every test in this module runs against recorded fixture
//! payloads, never the network. [`fetch_reference`] is the one function
//! that actually performs a request; it is exercised only by the
//! `#[ignore]`d live test (task 6.8), never by `cargo test --workspace`.

use std::time::Duration;

use vertice_core::freshness::ReferenceLookup;

use super::upstream::UpstreamIdentity;

/// Responses are untrusted input (design §12): the ceiling is enforced
/// before any parsing is attempted. It exists to bound memory against a
/// hostile or broken endpoint — **not** to be tight. A ceiling calibrated
/// so finely that a legitimate upstream trips it does not protect the
/// user; it just reports `Unknown` forever, which is the failure this
/// feature exists to remove.
///
/// The ceilings are per upstream kind because the two payloads are not
/// remotely comparable:
///
/// - npm's `.../latest` is the manifest of a single version: ~2 KiB in
///   practice.
/// - GitHub's `releases/latest` embeds the release's **entire asset
///   array**. `openai/codex` publishes one binary per target triple plus
///   tooling — 160 assets, ~272 KiB — even though the only fields read
///   are `name` and `tag_name`, both of which appear within the first
///   ~1.7 KiB. A single shared 256 KiB ceiling rejected that response
///   before parsing, so Codex could never report anything but `Unknown`.
///   Chasing the asset count with a slightly larger shared number would
///   break again the next time OpenAI adds a target, so the GitHub
///   ceiling is set with roughly an order of magnitude of headroom.
pub const MAX_NPM_RESPONSE_BYTES: usize = 64 * 1024;
pub const MAX_GITHUB_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// Per-request budget (design §9): fast enough that three concurrent
/// requests still land the whole command around 5s wall-clock, not 15s.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
pub const TOTAL_TIMEOUT: Duration = Duration::from_secs(5);

fn unavailable(reason: impl Into<String>) -> ReferenceLookup {
    ReferenceLookup::Unavailable {
        reason: reason.into(),
    }
}

/// npm's `GET .../latest` shape: `{ "version": "1.2.3", ... }`. Only the
/// `.version` field is trusted; every other field is ignored.
pub fn parse_npm_latest(body: &[u8]) -> ReferenceLookup {
    if body.len() > MAX_NPM_RESPONSE_BYTES {
        return unavailable("npm response exceeds the 64 KiB size ceiling");
    }

    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(err) => return unavailable(format!("npm response is not valid JSON: {err}")),
    };

    match value.get("version").and_then(|v| v.as_str()) {
        Some(version) if semver::Version::parse(version).is_ok() => {
            ReferenceLookup::Found(version.to_string())
        }
        Some(_) => unavailable("npm response's \"version\" field is not a valid semver string"),
        None => unavailable("npm response is missing a string \"version\" field"),
    }
}

/// GitHub `releases/latest` shape (design §6): `name`, then `tag_name`
/// with a `rust-v`/`v` release-train prefix stripped, then `Unknown`. A
/// prefix-carrying tag is never compared as a version.
pub fn parse_github_latest_release(body: &[u8]) -> ReferenceLookup {
    if body.len() > MAX_GITHUB_RESPONSE_BYTES {
        return unavailable("GitHub response exceeds the 4 MiB size ceiling");
    }

    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(err) => return unavailable(format!("GitHub response is not valid JSON: {err}")),
    };

    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
        if semver::Version::parse(name).is_ok() {
            return ReferenceLookup::Found(name.to_string());
        }
    }

    if let Some(tag) = value.get("tag_name").and_then(|v| v.as_str()) {
        let stripped = tag
            .strip_prefix("rust-v")
            .or_else(|| tag.strip_prefix('v'))
            .unwrap_or(tag);
        if semver::Version::parse(stripped).is_ok() {
            return ReferenceLookup::Found(stripped.to_string());
        }
    }

    unavailable("neither \"name\" nor a de-prefixed \"tag_name\" yielded a parseable version")
}

/// Parse a raw response body per `identity`'s shape.
pub fn parse_response(identity: &UpstreamIdentity, body: &[u8]) -> ReferenceLookup {
    match identity {
        UpstreamIdentity::Npm { .. } => parse_npm_latest(body),
        UpstreamIdentity::GitHubReleases { .. } => parse_github_latest_release(body),
    }
}

/// The one and only header value this crate sends that is not required by
/// HTTP itself: a static product token. Extracted from [`build_client`] so
/// the "no identifying content" requirement is *asserted by a test*, not
/// merely visible on inspection — it is a privacy guarantee, and
/// `reqwest::Client` exposes no way to read back the headers it was built
/// with.
pub fn user_agent() -> String {
    format!("vertice/{}", env!("CARGO_PKG_VERSION"))
}

/// Build the shared client used for every reference-version request:
/// zero retries (design §9), the connect/total budget above, and the
/// mandatory anonymous `User-Agent` (design §6 — no query params, no
/// identifier, no auth token).
pub fn build_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .user_agent(user_agent())
        .build()
}

/// Perform the one live request for `identity`. Never panics, never
/// returns an `Err` variant to its caller (design §12): every failure —
/// transport, timeout, rate limit, malformed body — degrades to
/// [`ReferenceLookup::Unavailable`].
pub async fn fetch_reference(
    client: &reqwest::Client,
    identity: &UpstreamIdentity,
) -> ReferenceLookup {
    let response = match client.get(identity.request_url()).send().await {
        Ok(response) => response,
        Err(err) => return unavailable(format!("request failed: {err}")),
    };

    let status = response.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status == reqwest::StatusCode::FORBIDDEN
    {
        return unavailable(format!("rate limited by upstream (status {status})"));
    }
    if !status.is_success() {
        return unavailable(format!("upstream returned status {status}"));
    }

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => return unavailable(format!("failed to read response body: {err}")),
    };

    parse_response(identity, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NPM_HAPPY_PATH: &str = r#"{"name":"opencode-ai","version":"1.18.21"}"#;
    const GITHUB_HAPPY_PATH_NAME: &str = r#"{"name":"0.149.1","tag_name":"rust-v0.149.1"}"#;
    const GITHUB_TAG_ONLY: &str = r#"{"tag_name":"rust-v0.149.1"}"#;
    const GITHUB_PLAIN_V_TAG: &str = r#"{"tag_name":"v0.149.1"}"#;

    /// Regression, reported by the user: Codex always reported "cannot
    /// validate the version". `openai/codex`'s `releases/latest` embeds
    /// 160 assets and weighs ~272 KiB, which the previous shared 256 KiB
    /// ceiling rejected *before parsing* — so a repository that publishes
    /// releases perfectly well could never produce a verdict. The fields
    /// actually read sit in the first ~1.7 KiB; the rest is an asset array
    /// this code never looks at.
    #[test]
    fn a_github_release_with_a_large_asset_array_still_yields_its_version() {
        let mut assets = String::new();
        for index in 0..160 {
            if index > 0 {
                assets.push(',');
            }
            assets.push_str(&format!(
                r#"{{"name":"codex-{index}-x86_64-pc-windows-msvc.zip","browser_download_url":"https://example.invalid/{index}/{filler}"}}"#,
                filler = "d".repeat(1800)
            ));
        }
        let body =
            format!(r#"{{"name":"0.149.1","tag_name":"rust-v0.149.1","assets":[{assets}]}}"#);

        assert!(
            body.len() > 256 * 1024,
            "fixture must exceed the old shared ceiling to be a regression test, got {} bytes",
            body.len()
        );
        assert_eq!(
            parse_github_latest_release(body.as_bytes()),
            ReferenceLookup::Found("0.149.1".to_string())
        );
    }

    #[test]
    fn npm_happy_path_extracts_the_version_field() {
        assert_eq!(
            parse_npm_latest(NPM_HAPPY_PATH.as_bytes()),
            ReferenceLookup::Found("1.18.21".to_string())
        );
    }

    #[test]
    fn npm_missing_version_field_is_unavailable() {
        let body = r#"{"name":"opencode-ai"}"#;
        match parse_npm_latest(body.as_bytes()) {
            ReferenceLookup::Unavailable { .. } => {}
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn npm_version_field_wrong_type_is_unavailable() {
        let body = r#"{"version": 123}"#;
        match parse_npm_latest(body.as_bytes()) {
            ReferenceLookup::Unavailable { .. } => {}
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn npm_truncated_json_is_unavailable_never_a_panic() {
        let body = r#"{"version": "1.2.3""#; // missing closing brace
        match parse_npm_latest(body.as_bytes()) {
            ReferenceLookup::Unavailable { .. } => {}
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn npm_oversize_body_is_rejected_before_parsing() {
        let oversized = vec![b'a'; MAX_NPM_RESPONSE_BYTES + 1];
        match parse_npm_latest(&oversized) {
            ReferenceLookup::Unavailable { reason } => {
                assert!(reason.contains("64 KiB"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn github_prefers_the_name_field_when_it_parses() {
        assert_eq!(
            parse_github_latest_release(GITHUB_HAPPY_PATH_NAME.as_bytes()),
            ReferenceLookup::Found("0.149.1".to_string())
        );
    }

    #[test]
    fn github_falls_through_to_tag_name_with_rust_v_prefix_stripped() {
        assert_eq!(
            parse_github_latest_release(GITHUB_TAG_ONLY.as_bytes()),
            ReferenceLookup::Found("0.149.1".to_string())
        );
    }

    #[test]
    fn github_falls_through_to_tag_name_with_plain_v_prefix_stripped() {
        assert_eq!(
            parse_github_latest_release(GITHUB_PLAIN_V_TAG.as_bytes()),
            ReferenceLookup::Found("0.149.1".to_string())
        );
    }

    #[test]
    fn github_raw_prefix_carrying_tag_is_never_used_as_is() {
        // No "name" field, and a tag whose stripped form is NOT valid semver
        // (the strip only removes an exact recognised prefix) must never
        // fall back to the raw, prefix-carrying string.
        let body = r#"{"tag_name":"nightly-build-42"}"#;
        match parse_github_latest_release(body.as_bytes()) {
            ReferenceLookup::Unavailable { .. } => {}
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn github_missing_both_fields_is_unavailable() {
        let body = r#"{"draft": false}"#;
        match parse_github_latest_release(body.as_bytes()) {
            ReferenceLookup::Unavailable { .. } => {}
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn github_wrong_type_field_falls_through_rather_than_panicking() {
        let body = r#"{"name": 42, "tag_name": "rust-v0.149.1"}"#;
        assert_eq!(
            parse_github_latest_release(body.as_bytes()),
            ReferenceLookup::Found("0.149.1".to_string())
        );
    }

    #[test]
    fn github_truncated_json_is_unavailable_never_a_panic() {
        let body = r#"{"tag_name": "rust-v0.149.1""#;
        match parse_github_latest_release(body.as_bytes()) {
            ReferenceLookup::Unavailable { .. } => {}
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn github_oversize_body_is_rejected_before_parsing() {
        let oversized = vec![b'a'; MAX_GITHUB_RESPONSE_BYTES + 1];
        match parse_github_latest_release(&oversized) {
            ReferenceLookup::Unavailable { reason } => {
                assert!(reason.contains("4 MiB"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn parse_response_dispatches_on_identity_shape() {
        let npm = UpstreamIdentity::Npm {
            package: "opencode-ai",
        };
        assert_eq!(
            parse_response(&npm, NPM_HAPPY_PATH.as_bytes()),
            ReferenceLookup::Found("1.18.21".to_string())
        );

        let github = UpstreamIdentity::GitHubReleases {
            owner: "openai",
            repo: "codex",
        };
        assert_eq!(
            parse_response(&github, GITHUB_HAPPY_PATH_NAME.as_bytes()),
            ReferenceLookup::Found("0.149.1".to_string())
        );
    }

    #[test]
    fn build_client_succeeds_with_the_designed_timeout_budget() {
        build_client().expect("client with a fixed timeout budget must always build");
    }

    /// `component-freshness`: "an outbound request carries no identifying
    /// content". The product token is the only thing we volunteer, and it
    /// must be the crate version and nothing else — no OS string, no
    /// machine name, no account name, no unique id.
    #[test]
    fn the_user_agent_carries_the_product_version_and_nothing_else() {
        let agent = user_agent();

        assert_eq!(agent, format!("vertice/{}", env!("CARGO_PKG_VERSION")));
        let version = agent
            .strip_prefix("vertice/")
            .expect("user agent must be the bare product token");
        assert!(
            !version.is_empty()
                && version
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '.'),
            "version segment must be digits and dots only, got {version:?}"
        );
        assert!(
            !agent.contains(' '),
            "a multi-token user agent would leak platform detail: {agent:?}"
        );

        for variable in ["USERNAME", "USER", "COMPUTERNAME", "HOSTNAME"] {
            if let Ok(value) = std::env::var(variable) {
                if !value.is_empty() {
                    assert!(
                        !agent.contains(&value),
                        "user agent must not embed {variable}"
                    );
                }
            }
        }
    }

    /// Same requirement, applied to the URL: every request this crate can
    /// build addresses a public package/release endpoint by name only —
    /// no query string, no fragment, nothing derived from the machine.
    #[test]
    fn no_request_url_carries_a_query_fragment_or_machine_identifier() {
        use crate::freshness::upstream::upstream_for;
        use vertice_core::model::ClientInstallSlot;

        let slots = [
            ClientInstallSlot::ClaudeCodeNpm,
            ClientInstallSlot::ClaudeCodeBundled,
            ClientInstallSlot::OpenCodeNpm,
            ClientInstallSlot::CodexStandalone,
        ];

        let mut built = 0usize;
        for slot in slots {
            let Some(identity) = upstream_for(slot) else {
                continue;
            };
            let url = identity.request_url();
            built += 1;

            assert!(url.starts_with("https://"), "must be HTTPS: {url:?}");
            assert!(!url.contains('?'), "no query string allowed: {url:?}");
            assert!(!url.contains('#'), "no fragment allowed: {url:?}");

            for variable in ["USERNAME", "USER", "COMPUTERNAME", "HOSTNAME"] {
                if let Ok(value) = std::env::var(variable) {
                    if !value.is_empty() {
                        assert!(
                            !url.contains(&value),
                            "url must not embed {variable}: {url:?}"
                        );
                    }
                }
            }
        }

        assert!(built > 0, "at least one slot must resolve to an upstream");
    }

    /// Live, manual-only check against the real registries (task 6.8, CA-17):
    /// catches upstream schema drift on demand. `#[ignore]`d so it never
    /// runs under `cargo test --workspace` or in CI — run explicitly with
    /// `cargo test -p vertice-app --lib -- --ignored freshness_live`.
    #[test]
    #[ignore = "hits the real npm/GitHub network; run manually only, never in CI (CA-17)"]
    fn freshness_live_upstream_endpoints_still_match_the_documented_shape() {
        let client = build_client().expect("client must build");

        let outcome = tauri::async_runtime::block_on(fetch_reference(
            &client,
            &UpstreamIdentity::Npm {
                package: "opencode-ai",
            },
        ));
        match outcome {
            ReferenceLookup::Found(version) => {
                assert!(!version.is_empty());
            }
            other => panic!("live npm lookup did not resolve a version: {other:?}"),
        }

        let outcome = tauri::async_runtime::block_on(fetch_reference(
            &client,
            &UpstreamIdentity::GitHubReleases {
                owner: "openai",
                repo: "codex",
            },
        ));
        match outcome {
            ReferenceLookup::Found(version) => {
                assert!(!version.is_empty());
            }
            other => panic!("live GitHub lookup did not resolve a version: {other:?}"),
        }
    }
}
