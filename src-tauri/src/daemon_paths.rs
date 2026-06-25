//! Filesystem locations the daemon and the app share. All consent/state lives in
//! the daemon's `~/.hyperspell` so the CLI and the app stay in lockstep.

use std::path::PathBuf;

/// `~/.hyperspell` — the daemon's private state dir.
pub fn hyperspell_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hyperspell")
}

/// `~/.hyperspell/config.toml` — auth + the `perm_*` consent keys.
pub fn config_path() -> PathBuf {
    hyperspell_dir().join("config.toml")
}

/// `~/.hyperspell/.actions.jsonl` — the daemon's append-only activity log.
pub fn actions_log_path() -> PathBuf {
    hyperspell_dir().join(".actions.jsonl")
}

/// Resolve the `hyperspell` daemon binary.
///
/// TODO(bundling): once the app bundles a relocatable CPython + the daemon wheel
/// (Option A — venv kept OUTSIDE the signed bundle so wheel self-update doesn't
/// break the signature), return the bundled venv's `bin/hyperspell` resolved
/// from the app resource dir. For now fall back to a `hyperspell` on PATH so the
/// scaffold is exercisable against a dev install of the daemon.
pub fn daemon_bin() -> String {
    "hyperspell".to_string()
}
