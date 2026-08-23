//! TOML deserialization seam.
//!
//! The ONLY module in `vertice-core` allowed to import the TOML parsing
//! crate directly (declared under a renamed dependency alias so the
//! containment test can be textual). Every other module MUST go through
//! [`from_str`]. Read-only by construction: no serialization function is
//! exposed, so no caller can acquire a write capability through this seam.
//! See `openspec/changes/2026-08-23-add-codex-client-support/design.md` §5.

use serde::de::DeserializeOwned;

/// Error returned when TOML input cannot be parsed or deserialized.
#[derive(Debug, thiserror::Error)]
pub enum TomlError {
    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml_seam::de::Error),
}

/// Deserialize a value of type `T` from a TOML string.
pub fn from_str<T: DeserializeOwned>(input: &str) -> Result<T, TomlError> {
    toml_seam::from_str(input).map_err(TomlError::from)
}
