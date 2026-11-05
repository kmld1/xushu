use tauri::State;
use xushu_core::{
    AcceptanceCriterion, AcceptanceReport, ArchiveTaskInput, ConfirmManualInput,
    CreateCriterionInput, CreateTaskInput, Evidence, ImportProjectInput, ManualConfirmation,
    Project, ProjectGitStatus, ProjectService, ReopenTaskInput, SaveVerificationSpecInput,
    StartVerificationInput, Task, TaskDetail, TaskFilter, TaskService, TransitionTaskInput,
    UpdateCriterionInput, UpdateProjectInput, UpdateTaskInput, VerificationCheck,
    VerificationPreview, VerificationQueueItem, VerificationRun, VerificationRunSummary,
    VerificationService, VerificationSpec, VerificationTemplate, WorkspaceSummary,
};

use crate::error::ApiError;
use crate::state::AppState;

#[tauri::command]
pub fn import_project(
    state: State<'_, AppState>,
    input: ImportProjectInput,
) -> Result<Project, ApiError> {
    state.with_database(|database| ProjectService::new(database).import(input))
}

#[tauri::command]
pub fn list_projects(
    state: State<'_, AppState>,
    include_archived: Option<bool>,
) -> Result<Vec<Project>, ApiError> {
    state.with_database(|database| {
        ProjectService::new(database).list(include_archived.unwrap_or(false))
    })
}

#[tauri::command]
pub fn get_project(state: State<'_, AppState>, id: String) -> Result<Project, ApiError> {
    state.with_database(|database| ProjectService::new(database).get(&id))
}

#[tauri::command]
pub fn update_project(
    state: State<'_, AppState>,
    input: UpdateProjectInput,
) -> Result<Project, ApiError> {
    state.with_database(|database| ProjectService::new(database).rename(input))
}

#[tauri::command]
pub fn archive_project(state: State<'_, AppState>, id: String) -> Result<Project, ApiError> {
    state.with_database(|database| ProjectService::new(database).archive(&id))
}

#[tauri::command]
pub fn restore_project(state: State<'_, AppState>, id: String) -> Result<Project, ApiError> {
    state.with_database(|database| ProjectService::new(database).restore(&id))
}

#[tauri::command]
pub fn get_project_git_status(
    state: State<'_, AppState>,
    id: String,
) -> Result<ProjectGitStatus, ApiError> {
    state.with_database(|database| ProjectService::new(database).git_status(&id))
}

#[tauri::command]
pub fn create_task(state: State<'_, AppState>, input: CreateTaskInput) -> Result<Task, ApiError> {
    state.with_database(|database| TaskService::new(database).create_task(input))
}

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>, filter: TaskFilter) -> Result<Vec<Task>, ApiError> {
    state.with_database(|database| TaskService::new(database).list_tasks(filter))
}

#[tauri::command]
pub fn get_task(state: State<'_, AppState>, id: String) -> Result<TaskDetail, ApiError> {
    state.with_database(|database| TaskService::new(database).get_task(&id))
}

#[tauri::command]
pub fn update_task(state: State<'_, AppState>, input: UpdateTaskInput) -> Result<Task, ApiError> {
    state.with_database(|database| TaskService::new(database).update_task(input))
}

#[tauri::command]
pub fn archive_task(state: State<'_, AppState>, input: ArchiveTaskInput) -> Result<Task, ApiError> {
    state.with_database(|database| TaskService::new(database).archive_task(input))
}

#[tauri::command]
pub fn restore_task(state: State<'_, AppState>, input: ArchiveTaskInput) -> Result<Task, ApiError> {
    state.with_database(|database| TaskService::new(database).restore_task(input))
}

#[tauri::command]
pub fn transition_task_status(
    state: State<'_, AppState>,
    input: TransitionTaskInput,
) -> Result<Task, ApiError> {
    state.with_database(|database| TaskService::new(database).transition_task_status(input))
}

#[tauri::command]
pub fn reopen_task(state: State<'_, AppState>, input: ReopenTaskInput) -> Result<Task, ApiError> {
    state.with_database(|database| TaskService::new(database).reopen_task(input))
}

#[tauri::command]
pub fn get_workspace_summary(state: State<'_, AppState>) -> Result<WorkspaceSummary, ApiError> {
    state.with_database(|database| TaskService::new(database).workspace_summary())
}

#[tauri::command]
pub fn create_acceptance_criterion(
    state: State<'_, AppState>,
    input: CreateCriterionInput,
) -> Result<AcceptanceCriterion, ApiError> {
    state.with_database(|database| TaskService::new(database).create_criterion(input))
}

#[tauri::command]
pub fn list_acceptance_criteria(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Vec<AcceptanceCriterion>, ApiError> {
    state.with_database(|database| TaskService::new(database).list_criteria(&task_id))
}

#[tauri::command]
pub fn update_acceptance_criterion(
    state: State<'_, AppState>,
    input: UpdateCriterionInput,
) -> Result<AcceptanceCriterion, ApiError> {
    state.with_database(|database| TaskService::new(database).update_criterion(input))
}

#[tauri::command]
pub fn remove_acceptance_criterion(
    state: State<'_, AppState>,
    input: xushu_core::RemoveCriterionInput,
) -> Result<(), ApiError> {
    state.with_database(|database| TaskService::new(database).remove_criterion(input))
}

#[tauri::command]
pub fn confirm_manual_criterion(
    state: State<'_, AppState>,
    input: ConfirmManualInput,
) -> Result<ManualConfirmation, ApiError> {
    state.with_database(|database| TaskService::new(database).confirm_manual_criterion(input))
}

#[tauri::command]
pub fn save_verification_spec(
    state: State<'_, AppState>,
    input: SaveVerificationSpecInput,
) -> Result<VerificationSpec, ApiError> {
    state.with_database(|database| VerificationService::new(database).save_spec(input))
}

#[tauri::command]
pub fn get_verification_spec(
    state: State<'_, AppState>,
    criterion_id: String,
) -> Result<Option<VerificationSpec>, ApiError> {
    state.with_database(|database| VerificationService::new(database).get_spec(&criterion_id))
}

#[tauri::command]
pub fn list_verification_templates(
    state: State<'_, AppState>,
) -> Result<Vec<VerificationTemplate>, ApiError> {
    state.with_database(|database| Ok(VerificationService::new(database).templates()))
}

#[tauri::command]
pub fn preview_verification(
    state: State<'_, AppState>,
    criterion_id: String,
) -> Result<VerificationPreview, ApiError> {
    state.with_database(|database| VerificationService::new(database).preview(&criterion_id))
}

#[tauri::command]
pub fn start_verification(
    state: State<'_, AppState>,
    input: StartVerificationInput,
) -> Result<VerificationRun, ApiError> {
    state.inner().clone().start_verification(input)
}

#[tauri::command]
pub fn cancel_verification(state: State<'_, AppState>, run_id: String) -> Result<(), ApiError> {
    state.cancel_verification(&run_id)
}

#[tauri::command]
pub fn get_verification_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<VerificationRun, ApiError> {
    state.with_database(|database| VerificationService::new(database).get_run(&run_id))
}

#[tauri::command]
pub fn list_verification_runs(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Vec<VerificationRunSummary>, ApiError> {
    state.with_database(|database| VerificationService::new(database).list_runs(&task_id))
}

#[tauri::command]
pub fn get_verification_check(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Option<VerificationCheck>, ApiError> {
    state.with_database(|database| VerificationService::new(database).get_check(&run_id))
}

#[tauri::command]
pub fn get_evidence(state: State<'_, AppState>, evidence_id: String) -> Result<Evidence, ApiError> {
    state.with_database(|database| VerificationService::new(database).get_evidence(&evidence_id))
}

#[tauri::command]
pub fn list_verification_queue(
    state: State<'_, AppState>,
) -> Result<Vec<VerificationQueueItem>, ApiError> {
    state.with_database(|database| TaskService::new(database).verification_queue())
}

#[tauri::command]
pub fn export_acceptance_report(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<AcceptanceReport, ApiError> {
    state.export_acceptance_report(&task_id)
}
