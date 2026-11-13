use tauri::State;
use xushu_core::{
    AutomaticBackupOutcome, BackupPolicy, BackupRecord, BackupVerification, CleanupOutcome,
    DataIntegrityReport, DataProtectionOverview, PerformRestoreInput, RestoreOutcome,
    RestorePreflight, RestorePreflightInput, UpdateBackupPolicyInput,
};

use crate::error::ApiError;
use crate::state::AppState;

#[tauri::command]
pub fn get_data_protection_overview(
    state: State<'_, AppState>,
) -> Result<DataProtectionOverview, ApiError> {
    state.get_data_protection_overview()
}

#[tauri::command]
pub fn dismiss_restore_notice(state: State<'_, AppState>) -> Result<(), ApiError> {
    state.dismiss_restore_notice()
}

#[tauri::command]
pub async fn create_backup(
    state: State<'_, AppState>,
    target_directory: String,
) -> Result<BackupRecord, ApiError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.create_backup(target_directory))
        .await
        .map_err(|_| ApiError::new("BACKUP_ERROR", "备份任务未能完成，请重试"))?
}

#[tauri::command]
pub fn cancel_backup(state: State<'_, AppState>) -> Result<(), ApiError> {
    state.cancel_backup()
}

#[tauri::command]
pub fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupRecord>, ApiError> {
    state.list_backups()
}

#[tauri::command]
pub async fn verify_backup(
    state: State<'_, AppState>,
    backup_id: String,
) -> Result<BackupVerification, ApiError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.verify_backup(backup_id))
        .await
        .map_err(|_| ApiError::new("BACKUP_ERROR", "备份校验未能完成，请重试"))?
}

#[tauri::command]
pub fn delete_backup(
    state: State<'_, AppState>,
    backup_id: String,
    approved: bool,
) -> Result<(), ApiError> {
    state.delete_backup(backup_id, approved)
}

#[tauri::command]
pub fn get_backup_policy(state: State<'_, AppState>) -> Result<BackupPolicy, ApiError> {
    state.get_backup_policy()
}

#[tauri::command]
pub fn update_backup_policy(
    state: State<'_, AppState>,
    input: UpdateBackupPolicyInput,
) -> Result<BackupPolicy, ApiError> {
    state.update_backup_policy(input)
}

#[tauri::command]
pub fn preview_backup_cleanup(state: State<'_, AppState>) -> Result<Vec<BackupRecord>, ApiError> {
    state.preview_backup_cleanup()
}

#[tauri::command]
pub fn apply_backup_cleanup(
    state: State<'_, AppState>,
    approved: bool,
) -> Result<CleanupOutcome, ApiError> {
    state.apply_backup_cleanup(approved)
}

#[tauri::command]
pub async fn run_due_automatic_backup(
    state: State<'_, AppState>,
) -> Result<AutomaticBackupOutcome, ApiError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.run_due_automatic_backup())
        .await
        .map_err(|_| ApiError::new("BACKUP_ERROR", "自动备份检查未能完成，请重试"))?
}

#[tauri::command]
pub async fn check_data_integrity(
    state: State<'_, AppState>,
) -> Result<DataIntegrityReport, ApiError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.check_data_integrity())
        .await
        .map_err(|_| ApiError::new("DATABASE_ERROR", "数据完整性检查未能完成，请重试"))?
}

#[tauri::command]
pub async fn preflight_restore(
    state: State<'_, AppState>,
    input: RestorePreflightInput,
) -> Result<RestorePreflight, ApiError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.preflight_restore(input))
        .await
        .map_err(|_| ApiError::new("RESTORE_ERROR", "恢复预检未能完成，请重试"))?
}

#[tauri::command]
pub async fn perform_restore(
    state: State<'_, AppState>,
    input: PerformRestoreInput,
) -> Result<RestoreOutcome, ApiError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.perform_restore(input))
        .await
        .map_err(|_| ApiError::new("RESTORE_ERROR", "恢复任务未能完成，请重试"))?
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde_json::{json, Value};
    use tauri::{
        ipc::{CallbackFn, InvokeBody},
        test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY},
        webview::InvokeRequest,
    };

    use super::*;

    #[test]
    fn restore_authorization_is_enforced_by_tauri_ipc() {
        let directory = tempfile::TempDir::new().unwrap();
        let state = AppState::initialize(directory.path());
        let backup = state
            .create_backup(
                directory
                    .path()
                    .join("backups")
                    .to_string_lossy()
                    .into_owned(),
            )
            .unwrap();
        let app = mock_builder()
            .manage(state)
            .invoke_handler(tauri::generate_handler![preflight_restore, perform_restore])
            .build(mock_context(noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let invoke = |command: &str, input: Value| {
            get_ipc_response(
                &window,
                InvokeRequest {
                    cmd: command.to_owned(),
                    callback: CallbackFn(0),
                    error: CallbackFn(1),
                    url: if cfg!(windows) {
                        "http://tauri.localhost"
                    } else {
                        "tauri://localhost"
                    }
                    .parse()
                    .unwrap(),
                    body: InvokeBody::Json(json!({ "input": input })),
                    headers: Default::default(),
                    invoke_key: INVOKE_KEY.to_owned(),
                },
            )
            .map(|body| body.deserialize::<Value>().unwrap())
        };
        let preflight = || {
            invoke(
                "preflight_restore",
                json!({ "backupDirectory": backup.directory_path }),
            )
            .unwrap()
        };
        let perform = |token: &str, text: &str, ack: bool| {
            invoke(
                "perform_restore",
                json!({
                    "token": token, "confirmationText": text, "acknowledged": ack
                }),
            )
        };
        let valid_text = xushu_core::backup::RESTORE_CONFIRMATION_TEXT;

        assert_eq!(
            perform("invalid", valid_text, true).unwrap_err()["code"],
            "RESTORE_ERROR"
        );
        let first = preflight();
        let first_token = first["token"].as_str().unwrap();
        assert_eq!(first["ready"], true);
        let expiry =
            chrono::DateTime::parse_from_rfc3339(first["expiresAt"].as_str().unwrap()).unwrap();
        let ttl = expiry
            .signed_duration_since(chrono::Utc::now())
            .num_seconds();
        assert!(
            (890..=900).contains(&ttl),
            "unexpected token lifetime: {ttl}"
        );
        for error in [
            perform(first_token, valid_text, false),
            perform(first_token, "yes", true),
        ] {
            assert_eq!(error.unwrap_err()["code"], "RESTORE_CONFIRMATION_REQUIRED");
        }
        let manifest = Path::new(&backup.directory_path).join("manifest.json");
        let original = fs::read(&manifest).unwrap();
        let mut changed = original.clone();
        changed.push(b' ');
        fs::write(&manifest, changed).unwrap();
        assert_eq!(
            perform(first_token, valid_text, true).unwrap_err()["code"],
            "RESTORE_ERROR"
        );
        fs::write(&manifest, original).unwrap();
        assert_eq!(
            perform(first_token, valid_text, true).unwrap_err()["code"],
            "RESTORE_ERROR"
        );

        let fresh = preflight();
        let fresh_token = fresh["token"].as_str().unwrap();
        let outcome = perform(fresh_token, valid_text, true).unwrap();
        assert_eq!(outcome["status"], "completed");
        assert_eq!(outcome["switchVerified"], true);
        assert_eq!(
            perform(fresh_token, valid_text, true).unwrap_err()["code"],
            "RESTORE_ERROR"
        );
    }
}
