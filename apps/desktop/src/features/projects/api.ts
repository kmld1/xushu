import type {
  ImportProjectInput,
  Project,
  ProjectGitStatus,
  UpdateProjectInput,
} from "@/lib/domain";
import { invokeCommand } from "@/lib/ipc";

export function listProjects(includeArchived = true): Promise<Project[]> {
  return invokeCommand<Project[]>("list_projects", { includeArchived });
}

export function getProject(id: string): Promise<Project> {
  return invokeCommand<Project>("get_project", { id });
}

export function importProject(input: ImportProjectInput): Promise<Project> {
  return invokeCommand<Project>("import_project", { input });
}

export function updateProject(input: UpdateProjectInput): Promise<Project> {
  return invokeCommand<Project>("update_project", { input });
}

export function archiveProject(id: string): Promise<Project> {
  return invokeCommand<Project>("archive_project", { id });
}

export function restoreProject(id: string): Promise<Project> {
  return invokeCommand<Project>("restore_project", { id });
}

export function getProjectGitStatus(id: string): Promise<ProjectGitStatus> {
  return invokeCommand<ProjectGitStatus>("get_project_git_status", { id });
}
