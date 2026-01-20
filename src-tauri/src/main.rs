// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod ssh;
mod query;
mod export;
mod deploy;

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
            commands::export_wide_table_direct,
            commands::export_demand_results_direct,
            commands::check_deploy_status,
            commands::deploy_application,
        ])
        .setup(|_app| {
            // SSH 连接在断开时会自动清理资源
            // 临时文件在查询过程中通过 Python 脚本和 trap 命令确保清理
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
