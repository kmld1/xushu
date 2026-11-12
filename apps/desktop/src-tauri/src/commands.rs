use serde::Serialize;
use tauri::State;
use xushu_core::{CreateProbeInput, DatabaseHealth, ProbeRecord};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub tauri_version: String,
    pub phase: String,
    pub platform: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: xushu_core::APP_NAME.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        tauri_version: tauri::VERSION.to_owned(),
        phase: "Phase 4".to_owned(),
        platform: std::env::consts::OS.to_owned(),
    }
}

#[tauri::command]
pub fn get_database_health(state: State<'_, AppState>) -> Result<DatabaseHealth, ApiError> {
    state.with_database(|database| database.health())
}

#[tauri::command]
pub fn create_probe_record(
    state: State<'_, AppState>,
    input: CreateProbeInput,
) -> Result<ProbeRecord, ApiError> {
    state.with_database(|database| database.create_probe(input))
}

#[tauri::command]
pub fn list_probe_records(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<ProbeRecord>, ApiError> {
    state.with_database(|database| database.list_probes(limit))
}

#[tauri::command]
pub fn get_probe_record(state: State<'_, AppState>, id: String) -> Result<ProbeRecord, ApiError> {
    state
        .with_database(|database| database.get_probe(&id))?
        .ok_or_else(|| ApiError::new("NOT_FOUND", "未找到指定记录"))
}
