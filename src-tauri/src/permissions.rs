//! The five consent toggles the daemon enforces, stored as flat `perm_*` keys in
//! `~/.hyperspell/config.toml`. The app authors them (after the consent screen);
//! the daemon's `reconcile_agent_configs()` enforces them on its next sync tick —
//! writing granted integrations, stripping revoked ones. Default-deny: an unset
//! key reads as `false`.

use crate::config::read_table;
use crate::daemon_paths::config_path;
use serde::{Deserialize, Serialize};

/// Canonical key list — must match the daemon's `config.Permissions` keys.
pub const KEYS: [&str; 5] = [
    "perm_claude_code",
    "perm_codex",
    "perm_cursor",
    "perm_claude_desktop",
    "perm_visible_mirror",
];

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub claude_code: bool,
    pub codex: bool,
    pub cursor: bool,
    pub claude_desktop: bool,
    pub visible_mirror: bool,
}

pub fn load() -> Permissions {
    from_table(&read_table())
}

/// Resolve permissions from a config table — default-deny (an unset or
/// non-boolean key reads as `false`). Pure, so it's unit-testable.
pub fn from_table(t: &toml::Table) -> Permissions {
    let g = |k: &str| t.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    Permissions {
        claude_code: g("perm_claude_code"),
        codex: g("perm_codex"),
        cursor: g("perm_cursor"),
        claude_desktop: g("perm_claude_desktop"),
        visible_mirror: g("perm_visible_mirror"),
    }
}

/// Set one consent key, preserving every other key in config.toml. The daemon
/// reconciles the filesystem to match on its next sync tick.
pub fn set(key: &str, value: bool) -> Result<(), String> {
    if !KEYS.contains(&key) {
        return Err(format!("unknown permission key: {key}"));
    }
    let mut t = read_table();
    t.insert(key.to_string(), toml::Value::Boolean(value));
    let body = toml::to_string(&t).map_err(|e| e.to_string())?;
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, body).map_err(|e| e.to_string())?;
    // Match the daemon's 0600 on config.toml — it holds the auth token.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_denies_everything() {
        let p = from_table(&toml::Table::new());
        assert!(!p.claude_code && !p.codex && !p.cursor);
        assert!(!p.claude_desktop && !p.visible_mirror);
    }

    #[test]
    fn only_set_keys_are_granted() {
        let t: toml::Table = "perm_claude_code = true\nperm_cursor = true\n"
            .parse()
            .unwrap();
        let p = from_table(&t);
        assert!(p.claude_code && p.cursor);
        assert!(!p.codex && !p.claude_desktop && !p.visible_mirror);
    }

    #[test]
    fn non_boolean_value_reads_as_denied() {
        let t: toml::Table = "perm_codex = \"yes\"\n".parse().unwrap();
        assert!(!from_table(&t).codex);
    }
}
