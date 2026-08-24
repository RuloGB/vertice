//! `ClientPresence` and `ClientPresenceStatus` — a typed, per-probe-slot
//! presence record. Plain data only, per the module's import allow-list:
//! `std::path`, `serde`, `ts_rs`. No I/O, no clock.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::installation::ClientInstallation;
use super::slot::ClientInstallSlot;

/// One probe slot's verdict. Exactly one record per slot the platform's
/// table defines (three on Windows), emitted whether or not anything
/// resolved. A slot is a *place we look*, not an installation: one record
/// MAY carry many installations (CA-7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct ClientPresence {
    /// The machine-readable, non-display slot identity. The stable key a
    /// consumer (e.g. `component-freshness`) dispatches on; never inferred
    /// from `label` (`client-installation-detector` spec).
    pub slot: ClientInstallSlot,
    /// The slot's settled proper-noun label, e.g. "OpenCode (npm)".
    /// Core-owned, unique within a report, never localized. Display-only —
    /// never a stable identity.
    pub label: String,
    /// Every path probed for this slot, in deterministic order; the legacy
    /// bundled path is always last. Non-empty by construction. Carried, not
    /// displayed.
    pub probed_paths: Vec<PathBuf>,
    pub status: ClientPresenceStatus,
    /// Never `Option`, never reduced to "highest wins" (CA-7).
    pub installations: Vec<ClientInstallation>,
}

/// Whether a candidate root for this slot exists on disk. `Detected` means
/// "the slot exists", NOT "a usable version was extracted" — a `Detected`
/// record with empty `installations` (present but broken) is a
/// deliberately representable state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum ClientPresenceStatus {
    Detected,
    NotDetected,
}
