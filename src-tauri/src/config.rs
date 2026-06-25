//! Reads of the daemon's `~/.hyperspell/config.toml` the app needs (auth state,
//! and the shared table reader used by permissions.rs).

use crate::daemon_paths::config_path;

pub fn read_table() -> toml::Table {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| s.parse::<toml::Table>().ok())
        .unwrap_or_default()
}

/// Whether the daemon is authenticated (device token or API key present).
pub fn logged_in() -> bool {
    let t = read_table();
    let nonempty = |k: &str| t.get(k).and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty());
    nonempty("token") || nonempty("api_key")
}
