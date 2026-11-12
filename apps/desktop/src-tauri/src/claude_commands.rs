use tauri::State;
use xushu_core::{
    ClaudeActionInput, ClaudeActivitySnapshot, ClaudeHookPreview, ClaudeImportReport,
    ClaudeIntegrationStatus, InstallClaudeHooksInput,
};

use crate::error::ApiError;
use crate::state::AppState;

#[tauri::command]
pub fn get_claude_integration(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ClaudeIntegrationStatus, ApiError> {
    state.get_claude_status(&project_id)
}

#[tauri::command]
pub fn get_claude_activity(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ClaudeActivitySnapshot, ApiError> {
    state.get_claude_activity(&project_id)
}

#[tauri::command]
pub fn preview_claude_hooks(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ClaudeHookPreview, ApiError> {
    state.preview_claude_hooks(&project_id)
}

#[tauri::command]
pub fn install_claude_hooks(
    state: State<'_, AppState>,
    input: InstallClaudeHooksInput,
) -> Result<ClaudeIntegrationStatus, ApiError> {
    state.install_claude_hooks(input)
}

#[tauri::command]
pub fn disable_claude_hooks(
    state: State<'_, AppState>,
    input: ClaudeActionInput,
) -> Result<ClaudeIntegrationStatus, ApiError> {
    state.disable_claude_hooks(input)
}

#[tauri::command]
pub fn uninstall_claude_hooks(
    state: State<'_, AppState>,
    input: ClaudeActionInput,
) -> Result<ClaudeIntegrationStatus, ApiError> {
    state.uninstall_claude_hooks(input)
}

#[tauri::command]
pub fn refresh_claude_events(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ClaudeImportReport, ApiError> {
    state.refresh_claude_events(&project_id)
}
