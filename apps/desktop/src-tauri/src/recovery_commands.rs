use tauri::State;
use xushu_core::{
    RecoveryOverview, RecoveryPreference, RecoveryService, RecoverySnapshot,
    RecoverySnapshotSummary, SetNextTaskInput,
};

use crate::error::ApiError;
use crate::state::AppState;

#[tauri::command]
pub fn get_recovery_overview(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<RecoveryOverview, ApiError> {
    state.with_database(|database| RecoveryService::new(database).overview(&project_id))
}

#[tauri::command]
pub fn generate_recovery_package(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<RecoverySnapshot, ApiError> {
    state.with_database(|database| RecoveryService::new(database).generate(&project_id))
}

#[tauri::command]
pub fn list_recovery_snapshots(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<RecoverySnapshotSummary>, ApiError> {
    state.with_database(|database| RecoveryService::new(database).list_snapshots(&project_id))
}

#[tauri::command]
pub fn get_recovery_snapshot(
    state: State<'_, AppState>,
    project_id: String,
    snapshot_id: String,
) -> Result<RecoverySnapshot, ApiError> {
    state.with_database(|database| {
        RecoveryService::new(database).get_snapshot(&project_id, &snapshot_id)
    })
}

#[tauri::command]
pub fn set_recovery_next_task(
    state: State<'_, AppState>,
    input: SetNextTaskInput,
) -> Result<RecoveryPreference, ApiError> {
    state.with_database(|database| RecoveryService::new(database).set_next_task(input))
}
