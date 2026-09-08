// Prevents an extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod core;
mod db;
mod models;

use core::system_monitor::SystemMonitor;
use db::Db;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let conn = db::open().expect("failed to open local database");
            app.manage(Db(Mutex::new(conn)));
            app.manage(Mutex::new(SystemMonitor::new()));

            // Minimal tray: show/hide + quit. Keeping the app runnable from
            // the tray is what makes "dauerhaft laufen ohne zu stören"
            // (section 21) possible.
            let show = MenuItem::with_id(app, "show", "Dashboard anzeigen", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            core::update_manager::spawn(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_system_stats,
            commands::get_weather,
            commands::add_source,
            commands::list_sources,
            commands::remove_source,
            commands::refresh_source,
            commands::get_cached_news,
            commands::refresh_all_sources,
            commands::openai_sign_in_with_api_key,
            commands::openai_sign_in_oauth,
            commands::openai_sign_out,
            commands::openai_is_signed_in,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the application");
}
