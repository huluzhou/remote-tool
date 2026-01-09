// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod ssh;
mod query;
mod export;
mod deploy;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::ssh_connect,
            commands::ssh_disconnect,
            commands::execute_query,
            commands::export_to_csv,
            commands::check_deploy_status,
            commands::deploy_application,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
