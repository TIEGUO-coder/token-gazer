mod billing;
mod commands;
mod db;
mod quota;
mod settings;
mod usage;
mod window;

use commands::AppState;
use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;

fn app_db_path() -> PathBuf {
    let base = dirs::data_dir()
        .or_else(dirs::home_dir)
        .expect("data directory not found");
    let dir = base.join("ai-roi-pet");
    std::fs::create_dir_all(&dir).expect("failed to create app data directory");
    dir.join("usage.sqlite3")
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            db_path: app_db_path(),
            lock: Mutex::new(()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::sync_now,
            commands::detect_billing_periods,
            commands::refresh_codex_quota,
            commands::get_usage_summary,
            commands::get_subscriptions,
            commands::save_subscription,
            commands::get_app_config,
            commands::save_app_config,
            commands::start_window_drag
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                crate::window::set_pet_window_defaults(&window);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run AI ROI Pet");
}
