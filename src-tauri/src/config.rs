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
    logged_in_from(&read_table())
}

/// Pure form for testing.
fn logged_in_from(t: &toml::Table) -> bool {
    let nonempty = |k: &str| {
        t.get(k)
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
    };
    nonempty("token") || nonempty("api_key")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_is_not_logged_in() {
        assert!(!logged_in_from(&toml::Table::new()));
    }

    #[test]
    fn token_or_api_key_means_logged_in() {
        assert!(logged_in_from(&"token = \"abc\"".parse().unwrap()));
        assert!(logged_in_from(&"api_key = \"hs2-x\"".parse().unwrap()));
    }

    #[test]
    fn empty_string_token_is_not_logged_in() {
        assert!(!logged_in_from(&"token = \"\"".parse().unwrap()));
    }
}
