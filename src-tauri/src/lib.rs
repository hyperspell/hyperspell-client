//! Hyperspell desktop client — a menu-bar app that wraps and supervises the
//! existing Python sync daemon (`tools/hyperspell-sync` in the hyperspell
//! monorepo) and gives the user an explicit consent surface over what it touches.
//!
//! Architecture: the app does NOT reimplement sync. It bundles the daemon (a
//! relocatable CPython + the released wheel) and supervises `hyperspell sync
//! --supervised` as a child (supervisor.rs), drives `hyperspell login --json`
//! for auth (auth.rs), and writes the daemon's `perm_*` consent keys from a
//! consent UI (permissions.rs). See README.md.

mod actions_log;
mod auth;
mod bootstrap;
mod daemon_paths;
mod ipc;
mod permissions;
mod supervisor;

use supervisor::Supervisor;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(Supervisor::default())
        .invoke_handler(tauri::generate_handler![
            ipc::get_status,
            ipc::get_permissions,
            ipc::set_permission,
            ipc::recent_actions,
            ipc::start_login,
            ipc::start_daemon,
        ])
        .setup(|app| {
            // Menu-bar app: live in the tray, not the dock.
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Start at login so the company brain keeps syncing without the user
            // having to relaunch. Best-effort.
            {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
            }

            // Tray menu: Open + Quit.
            let open_i = MenuItem::with_id(app, "open", "Open Hyperspell", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Hyperspell", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &quit_i])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        // Quitting stops syncing — transparent ownership.
                        if let Some(sup) = app.try_state::<Supervisor>() {
                            sup.stop();
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it; the app keeps running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
