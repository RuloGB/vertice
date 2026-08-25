//! The durable, whole-document user settings type (`add-locale-persistence`)
//! — plain data only, per `model/`'s import allow-list: `serde`, `ts_rs`. No
//! I/O, no clock, no filesystem knowledge. `vertice-app` owns the single
//! `settings.json` document this type describes; this module only defines
//! its shape.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The single durable settings document's fields, crossing IPC.
///
/// `locale` is free-form on purpose: core has no business knowing which
/// catalogs the frontend ships. An unrecognised value is treated as "no
/// explicit choice" by the frontend, which then falls through to
/// `navigator.languages`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct UserSettings {
    pub locale: Option<String>,
    pub enabled: bool,
    pub disclosure_seen: bool,
}
