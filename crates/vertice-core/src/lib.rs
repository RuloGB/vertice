//! `vertice-core`: pure, Tauri-agnostic domain library for Vertice.
//!
//! This crate MUST NOT depend on `tauri` or any `tauri-*` crate, directly or
//! transitively (enforced by `deny.toml` in the workspace root). It exists so
//! a future CLI binary can reuse the same logic as the desktop app.

pub mod model;
pub mod yaml;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Ping {
        message: String,
    }

    #[test]
    fn yaml_seam_is_reachable_from_the_crate_root() {
        let parsed: Ping = yaml::from_str("message: pong\n").expect("valid YAML should parse");

        assert_eq!(parsed.message, "pong");
    }
}
