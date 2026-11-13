use tauri::State;
use xushu_core::{
    ArchiveMemoryInput, ConfirmDecisionInput, ConfirmMemoryInput, CreateDecisionInput,
    CreateMemoryInput, DecisionDetail, DecisionFilter, DecisionService, DecisionView,
    DeprecateDecisionInput, MemoryDetail, MemoryFilter, MemoryService, ProjectMemoryView,
    UpdateDecisionInput, UpdateMemoryInput,
};

use crate::error::ApiError;
use crate::state::AppState;

#[tauri::command]
pub fn list_project_memories(
    state: State<'_, AppState>,
    filter: MemoryFilter,
) -> Result<Vec<ProjectMemoryView>, ApiError> {
    state.with_database(|database| MemoryService::new(database).list(filter))
}

#[tauri::command]
pub fn get_project_memory(
    state: State<'_, AppState>,
    project_id: String,
    memory_id: String,
) -> Result<MemoryDetail, ApiError> {
    state.with_database(|database| MemoryService::new(database).get(&project_id, &memory_id))
}

#[tauri::command]
pub fn create_project_memory(
    state: State<'_, AppState>,
    input: CreateMemoryInput,
) -> Result<ProjectMemoryView, ApiError> {
    state.with_database(|database| MemoryService::new(database).create(input))
}

#[tauri::command]
pub fn update_project_memory(
    state: State<'_, AppState>,
    input: UpdateMemoryInput,
) -> Result<ProjectMemoryView, ApiError> {
    state.with_database(|database| MemoryService::new(database).update(input))
}

#[tauri::command]
pub fn confirm_project_memory(
    state: State<'_, AppState>,
    input: ConfirmMemoryInput,
) -> Result<ProjectMemoryView, ApiError> {
    state.with_database(|database| MemoryService::new(database).confirm(input))
}

#[tauri::command]
pub fn archive_project_memory(
    state: State<'_, AppState>,
    input: ArchiveMemoryInput,
) -> Result<ProjectMemoryView, ApiError> {
    state.with_database(|database| MemoryService::new(database).archive(input))
}

#[tauri::command]
pub fn restore_project_memory(
    state: State<'_, AppState>,
    input: ArchiveMemoryInput,
) -> Result<ProjectMemoryView, ApiError> {
    state.with_database(|database| MemoryService::new(database).restore(input))
}

#[tauri::command]
pub fn list_project_decisions(
    state: State<'_, AppState>,
    filter: DecisionFilter,
) -> Result<Vec<DecisionView>, ApiError> {
    state.with_database(|database| DecisionService::new(database).list(filter))
}

#[tauri::command]
pub fn get_project_decision(
    state: State<'_, AppState>,
    project_id: String,
    decision_id: String,
) -> Result<DecisionDetail, ApiError> {
    state.with_database(|database| DecisionService::new(database).get(&project_id, &decision_id))
}

#[tauri::command]
pub fn create_project_decision(
    state: State<'_, AppState>,
    input: CreateDecisionInput,
) -> Result<DecisionView, ApiError> {
    state.with_database(|database| DecisionService::new(database).create(input))
}

#[tauri::command]
pub fn update_project_decision(
    state: State<'_, AppState>,
    input: UpdateDecisionInput,
) -> Result<DecisionView, ApiError> {
    state.with_database(|database| DecisionService::new(database).update(input))
}

#[tauri::command]
pub fn confirm_project_decision(
    state: State<'_, AppState>,
    input: ConfirmDecisionInput,
) -> Result<DecisionView, ApiError> {
    state.with_database(|database| DecisionService::new(database).confirm(input))
}

#[tauri::command]
pub fn deprecate_project_decision(
    state: State<'_, AppState>,
    input: DeprecateDecisionInput,
) -> Result<DecisionView, ApiError> {
    state.with_database(|database| DecisionService::new(database).deprecate(input))
}
