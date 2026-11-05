mod commands;
mod error;
mod state;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_directory = app.path().app_data_dir()?;
            app.manage(AppState::initialize(&data_directory));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_database_health,
            commands::create_probe_record,
            commands::list_probe_records,
            commands::get_probe_record,
        ])
        .run(tauri::generate_context!())
        .expect("运行序枢 XUSHU 桌面程序失败");
}
