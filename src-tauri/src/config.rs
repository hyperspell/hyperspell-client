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

/// A human label for who's logged in (email → user_key → as_user), if any.
pub fn identity() -> Option<String> {
    identity_from(&read_table())
}

fn identity_from(t: &toml::Table) -> Option<String> {
    let s = |k: &str| {
        t.get(k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    s("email")
        .or_else(|| s("user_key"))
        .or_else(|| s("as_user"))
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

    #[test]
    fn identity_prefers_email_then_user_key() {
        let t = "user_key = \"david\"\nemail = \"d@x.com\"".parse().unwrap();
        assert_eq!(identity_from(&t).as_deref(), Some("d@x.com"));
        let t2 = "user_key = \"david\"".parse().unwrap();
        assert_eq!(identity_from(&t2).as_deref(), Some("david"));
        assert_eq!(identity_from(&toml::Table::new()), None);
    }
}
