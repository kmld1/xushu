mod business_commands;
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
            business_commands::import_project,
            business_commands::list_projects,
            business_commands::get_project,
            business_commands::update_project,
            business_commands::archive_project,
            business_commands::restore_project,
            business_commands::get_project_git_status,
            business_commands::create_task,
            business_commands::list_tasks,
            business_commands::get_task,
            business_commands::update_task,
            business_commands::archive_task,
            business_commands::restore_task,
            business_commands::transition_task_status,
            business_commands::reopen_task,
            business_commands::get_workspace_summary,
            business_commands::create_acceptance_criterion,
            business_commands::list_acceptance_criteria,
            business_commands::update_acceptance_criterion,
            business_commands::remove_acceptance_criterion,
            business_commands::confirm_manual_criterion,
        ])
        .run(tauri::generate_context!())
        .expect("运行序枢 XUSHU 桌面程序失败");
}
