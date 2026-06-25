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

/// Resolve the `hyperspell` daemon binary, in priority order:
///
/// 1. TODO(bundling): the bundled relocatable-CPython venv shipped inside the
///    app (Option A — the venv is materialized OUTSIDE the signed bundle, at
///    `~/.hyperspell/venv`, so the daemon's wheel self-update doesn't break the
///    app's code signature). Resolved from the Tauri resource dir once bundling
///    lands.
/// 2. The existing installer's venv at `~/.hyperspell/venv/bin/hyperspell` — so
///    the app drives a daemon installed via the current curl|bash path today.
/// 3. A `hyperspell` on `PATH` (dev installs).
pub fn daemon_bin() -> String {
    let venv_bin = hyperspell_dir().join("venv").join("bin").join("hyperspell");
    if venv_bin.is_file() {
        return venv_bin.to_string_lossy().into_owned();
    }
    "hyperspell".to_string()
}

/// Whether a daemon binary the app can drive is present on this machine.
/// Distinct from "is the daemon running" (that's the supervisor's job).
pub fn daemon_installed() -> bool {
    hyperspell_dir()
        .join("venv")
        .join("bin")
        .join("hyperspell")
        .is_file()
        || which_on_path("hyperspell")
}

/// Minimal PATH lookup (avoids pulling in a crate for one call).
fn which_on_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let p = dir.join(bin);
                p.is_file()
            })
        })
        .unwrap_or(false)
}
