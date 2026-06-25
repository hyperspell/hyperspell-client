//! The commands the web UI invokes — thin wrappers over the modules below.

use crate::permissions::Permissions;
use crate::supervisor::Supervisor;
use crate::{actions_log, auth, bootstrap, config, daemon_paths, permissions, uninstall};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize)]
pub struct Status {
    /// The daemon is authenticated (device token or API key present).
    pub logged_in: bool,
    /// A daemon binary the app can drive is present on this machine.
    pub daemon_installed: bool,
    /// The daemon process is currently running under our supervisor.
    pub daemon_running: bool,
    pub permissions: Permissions,
}

#[tauri::command]
pub fn get_status(supervisor: State<'_, Supervisor>) -> Status {
    Status {
        logged_in: config::logged_in(),
        daemon_installed: daemon_paths::daemon_installed(),
        daemon_running: supervisor.is_running(),
        permissions: permissions::load(),
    }
}

/// The trusted-install + supervise sequence, shared by the `start_daemon` command
/// and the auto-start on app launch. Idempotent.
pub fn launch_daemon(app: &AppHandle, supervisor: &Supervisor) -> Result<(), String> {
    let stage = |s: &str| {
        let _ = app.emit("setup", s);
    };

    // If the app ships a bundled runtime, do the trusted install before
    // supervising: build ~/.hyperspell/venv (the daemon), install the full
    // hyperbrain CLI, and expose both (+ uv) on PATH so every CLI ability works
    // from any terminal. In `tauri dev` (no bundled runtime) this is skipped and
    // the supervisor falls back to a daemon on PATH / in an existing venv.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let rt = bootstrap::runtime_from_resources(&resource_dir);
        if bootstrap::is_bundled(&rt) {
            stage("Setting up the runtime…");
            let daemon = bootstrap::ensure_venv(&rt)?;
            stage("Linking the CLIs onto your PATH…");
            bootstrap::expose_cli(&rt, &daemon)?;
            stage("Installing the brain CLI…");
            // Best-effort: needs PyPI on first run; the daemon also installs it
            // on its first sync, so a transient failure here isn't fatal.
            let _ = bootstrap::install_hyperbrain(&rt);
        }
    }
    stage("Starting sync…");
    let result = supervisor.start();
    stage(""); // done — clear the overlay
    result
}

#[tauri::command]
pub fn get_permissions() -> Permissions {
    permissions::load()
}

/// Flip one consent toggle, then return the full updated set. The daemon picks
/// the change up on its next sync tick and reconciles the filesystem.
#[tauri::command]
pub fn set_permission(key: String, value: bool) -> Result<Permissions, String> {
    permissions::set(&key, value)?;
    Ok(permissions::load())
}

#[tauri::command]
pub fn recent_actions(limit: Option<usize>) -> Vec<serde_json::Value> {
    actions_log::recent(limit.unwrap_or(20))
}

#[tauri::command]
pub fn start_login(app: AppHandle, app_slug: String, name: String) -> Result<(), String> {
    auth::start_login(app, app_slug, name)
}

#[tauri::command]
pub fn start_daemon(app: AppHandle, supervisor: State<'_, Supervisor>) -> Result<(), String> {
    launch_daemon(&app, &supervisor)
}

/// Remove Hyperspell: stop syncing, disable app login-autostart, and run the
/// teardown (agent configs, mirror, PATH symlinks/rc). `purge` also deletes
/// ~/.hyperspell.
#[tauri::command]
pub fn uninstall(
    app: AppHandle,
    supervisor: State<'_, Supervisor>,
    purge: bool,
) -> Result<(), String> {
    supervisor.stop();
    {
        use tauri_plugin_autostart::ManagerExt;
        let _ = app.autolaunch().disable();
    }
    uninstall::run(purge)
}
