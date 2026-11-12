mod business_commands;
mod claude_commands;
mod commands;
mod error;
mod state;
mod window_chrome;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_directory = app.path().app_data_dir()?;
            let isolated_build = app.config().identifier.starts_with("dev.xushu.e2e.");
            let expected_data = std::env::var_os("XUSHU_E2E_DATA_DIR");
            if isolated_build || expected_data.is_some() {
                let expected = expected_data.and_then(|path| std::fs::canonicalize(path).ok());
                if !isolated_build
                    || expected.is_none()
                    || std::fs::canonicalize(&data_directory).ok() != expected
                {
                    return Err(std::io::Error::other(
                        "desktop E2E data directory does not match the launcher",
                    )
                    .into());
                }
            }
            app.manage(AppState::initialize(&data_directory));

            if let Some(window) = app.get_webview_window("main") {
                if let Err(error) = window_chrome::install(&window) {
                    eprintln!("无法安装窗口 Snap 覆盖层：{error}");
                }
            }

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
            business_commands::save_verification_spec,
            business_commands::get_verification_spec,
            business_commands::list_verification_templates,
            business_commands::preview_verification,
            business_commands::start_verification,
            business_commands::cancel_verification,
            business_commands::get_verification_run,
            business_commands::list_verification_runs,
            business_commands::get_verification_check,
            business_commands::get_evidence,
            business_commands::list_verification_queue,
            business_commands::export_acceptance_report,
            claude_commands::get_claude_integration,
            claude_commands::get_claude_activity,
            claude_commands::preview_claude_hooks,
            claude_commands::install_claude_hooks,
            claude_commands::disable_claude_hooks,
            claude_commands::uninstall_claude_hooks,
            claude_commands::refresh_claude_events,
        ])
        .run(tauri::generate_context!())
        .expect("运行序枢 XUSHU 桌面程序失败");
}
