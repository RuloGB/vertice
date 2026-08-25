//! Freshness verdict and report types — the `component-freshness`
//! capability's core data model. Plain data only, per `model/`'s import
//! allow-list: `std::path`, `serde`, `ts_rs`. No I/O, no clock.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::slot::ClientInstallSlot;

/// Three-valued by settled decision (`component-freshness` design §7).
/// `Unknown` is a first-class outcome, not an error path. There is NO
/// fourth state: an installed version ahead of the latest reference is
/// `UpToDate`, never a separate "ahead"/"prerelease" variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum Freshness {
    UpToDate,
    Outdated { latest: String },
    Unknown { reason: String },
}

/// The discriminator AND the id in one closed enum. Today one variant;
/// skills and agents arrive later as `Skill { id: ComponentId }` etc.
/// (`component-freshness` design §3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum FreshnessSubject {
    ClientInstallation {
        slot: ClientInstallSlot,
        path: PathBuf,
    },
}

/// One subject's installed version paired with its verdict. `installed` is
/// denormalised deliberately: the badge renders beside a version, and
/// without it the frontend would have to re-join on `path` across two
/// payloads returned at different times.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct FreshnessCheck {
    pub subject: FreshnessSubject,
    pub installed: String,
    pub verdict: Freshness,
}

/// The collection returned to the frontend. `enabled: false` means the user
/// turned the check off and NO request was issued — distinct from
/// every-check-`Unknown`, which means an attempt was made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct FreshnessReport {
    pub enabled: bool,
    pub checks: Vec<FreshnessCheck>,
}
