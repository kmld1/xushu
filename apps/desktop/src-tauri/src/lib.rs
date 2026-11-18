mod business_commands;
mod claude_commands;
mod commands;
mod data_protection_commands;
mod error;
mod knowledge_commands;
mod logging;
mod recovery_commands;
mod state;
mod window_chrome;

use tauri::{Manager, RunEvent};

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::install_panic_hook();
    let app = tauri::Builder::default()
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
                if window_chrome::install(&window).is_err() {
                    logging::record("WARN", "window_snap_overlay_install_failed");
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
            knowledge_commands::list_project_memories,
            knowledge_commands::get_project_memory,
            knowledge_commands::create_project_memory,
            knowledge_commands::update_project_memory,
            knowledge_commands::confirm_project_memory,
            knowledge_commands::archive_project_memory,
            knowledge_commands::restore_project_memory,
            knowledge_commands::list_project_decisions,
            knowledge_commands::get_project_decision,
            knowledge_commands::create_project_decision,
            knowledge_commands::update_project_decision,
            knowledge_commands::confirm_project_decision,
            knowledge_commands::deprecate_project_decision,
            recovery_commands::get_recovery_overview,
            recovery_commands::generate_recovery_package,
            recovery_commands::list_recovery_snapshots,
            recovery_commands::get_recovery_snapshot,
            recovery_commands::set_recovery_next_task,
            data_protection_commands::get_data_protection_overview,
            data_protection_commands::dismiss_restore_notice,
            data_protection_commands::create_backup,
            data_protection_commands::cancel_backup,
            data_protection_commands::list_backups,
            data_protection_commands::verify_backup,
            data_protection_commands::delete_backup,
            data_protection_commands::get_backup_policy,
            data_protection_commands::update_backup_policy,
            data_protection_commands::preview_backup_cleanup,
            data_protection_commands::apply_backup_cleanup,
            data_protection_commands::run_due_automatic_backup,
            data_protection_commands::check_data_integrity,
            data_protection_commands::preflight_restore,
            data_protection_commands::perform_restore,
        ])
        .build(tauri::generate_context!())
        .expect("运行序枢 XUSHU 桌面程序失败");

    app.run(|_, event| {
        if let RunEvent::Exit = event {
            logging::record("INFO", "app_exit");
        }
    });
}
