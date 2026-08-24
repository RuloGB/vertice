//! `ClientInstallSlot` — the machine-readable, non-display discriminator
//! naming which probe slot a `ClientPresence` record describes.
//!
//! Promoted from `installations.rs`'s private `InstallSlot`
//! (`add-client-version-freshness` design §2): a consumer now exists
//! (`component-freshness`) that must dispatch on slot identity to resolve
//! an upstream registry/repository without parsing `label` prose — the same
//! two-part condition that previously kept this type private. Plain data
//! only, per `model/`'s import allow-list: `serde`, `ts_rs`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One probe slot's identity, independent of and never inferred from
/// `ClientPresence.label`. Closed, exhaustively matchable — mirrors the
/// `Scope`/`ClientPresenceStatus` pattern. Growing this set follows
/// platform/adapter growth (T16), never speculative generality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum ClientInstallSlot {
    ClaudeCodeNpm,
    ClaudeCodeBundled,
    OpenCodeNpm,
    CodexStandalone,
}

impl ClientInstallSlot {
    /// The settled label used in `"{label} not detected"` and in this
    /// slot's `Error` reasons. Display-only copy — never keyed on for
    /// identity (`client-installation-detector` spec).
    pub fn label(self) -> &'static str {
        match self {
            ClientInstallSlot::ClaudeCodeNpm => "Claude Code CLI (npm)",
            ClientInstallSlot::ClaudeCodeBundled => "Claude Code (bundled in Claude Desktop)",
            ClientInstallSlot::OpenCodeNpm => "OpenCode (npm)",
            ClientInstallSlot::CodexStandalone => "Codex CLI (standalone)",
        }
    }
}
