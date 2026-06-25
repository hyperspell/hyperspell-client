//! The commands the web UI invokes — thin wrappers over the modules below.

use crate::permissions::Permissions;
use crate::supervisor::Supervisor;
use crate::{actions_log, auth, bootstrap, daemon_paths, permissions};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize)]
pub struct Status {
    /// A daemon binary the app can drive is present on this machine.
    pub daemon_installed: bool,
    /// The daemon process is currently running under our supervisor.
    pub daemon_running: bool,
    pub permissions: Permissions,
}

#[tauri::command]
pub fn get_status(supervisor: State<'_, Supervisor>) -> Status {
    Status {
        daemon_installed: daemon_paths::daemon_installed(),
        daemon_running: supervisor.is_running(),
        permissions: permissions::load(),
    }
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
    // If the app ships a bundled runtime, materialize ~/.hyperspell/venv from it
    // before supervising. In `tauri dev` (no bundled runtime) this is skipped and
    // the supervisor falls back to a daemon on PATH / in an existing venv.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let rt = bootstrap::runtime_from_resources(&resource_dir);
        if bootstrap::is_bundled(&rt) {
            bootstrap::ensure_venv(&rt)?;
        }
    }
    supervisor.start()
}
