//! Core domain model — plain data only, zero disk I/O.
//!
//! Every type in this module is `Serialize`/`Deserialize`/`TS`-derived plain
//! data with no behavior beyond `ComponentId::derive`'s deterministic string
//! composition. Nothing here performs disk, network, environment, or clock
//! access.
//!
//! # Purity invariant
//!
//! Import allow-list for the whole module: `std::path`, `std::time::Duration`,
//! `serde`, `ts_rs`, `thiserror`, `unicode_normalization`. Forbidden:
//! `std::fs`, `std::io`, `std::env`, `std::time::SystemTime`/`Instant`. Any
//! actual scanning, path resolution, or clock read belongs to a later phase
//! (T3-T9), never to this module. `ScanReport::duration_ms` is a value
//! *passed in* by the caller, never measured here.
//!
//! # TypeScript bindings
//!
//! Every public type here derives `TS` and exports to
//! `frontend/src/bindings/` via `#[ts(export)]`. Regenerate with
//! `cargo test -p vertice-core`; CI fails the build if the checked-in
//! bindings drift from this source (see `.github/workflows/ci.yml`).

mod component;
mod error;
mod freshness;
mod identity;
mod installation;
mod location;
mod mcp;
mod presence;
mod prompt;
mod report;
mod settings;
mod slot;

pub use component::{Component, ComponentKind, Scope};
pub use error::ScanError;
pub use freshness::{Freshness, FreshnessCheck, FreshnessReport, FreshnessSubject};
pub use identity::ComponentId;
pub use installation::{ClientInstallation, ClientKind};
pub use location::{
    Location, LocationOrigin, SearchRoot, SearchRootId, SearchRootKind, SearchRootStatus,
};
pub use mcp::McpTransport;
pub use presence::{ClientPresence, ClientPresenceStatus};
pub use prompt::{Prompt, PromptDraft, PromptError, PromptUpdate};
pub use report::{IssueSeverity, ScanIssue, ScanReport};
pub use settings::UserSettings;
pub use slot::ClientInstallSlot;
