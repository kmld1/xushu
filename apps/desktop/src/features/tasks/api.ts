import type {
  AcceptanceCriterion,
  ArchiveTaskInput,
  ConfirmManualInput,
  CreateCriterionInput,
  CreateTaskInput,
  ManualConfirmation,
  ReopenTaskInput,
  Task,
  TaskDetail,
  TaskFilter,
  TransitionTaskInput,
  UpdateCriterionInput,
  UpdateTaskInput,
  WorkspaceSummary,
} from "@/lib/domain";
import { invokeCommand } from "@/lib/ipc";

export function createTask(input: CreateTaskInput): Promise<Task> {
  return invokeCommand<Task>("create_task", { input });
}

export function listTasks(filter: TaskFilter): Promise<Task[]> {
  return invokeCommand<Task[]>("list_tasks", { filter });
}

export function getTask(id: string): Promise<TaskDetail> {
  return invokeCommand<TaskDetail>("get_task", { id });
}

export function updateTask(input: UpdateTaskInput): Promise<Task> {
  return invokeCommand<Task>("update_task", { input });
}

export function archiveTask(input: ArchiveTaskInput): Promise<Task> {
  return invokeCommand<Task>("archive_task", { input });
}

export function restoreTask(input: ArchiveTaskInput): Promise<Task> {
  return invokeCommand<Task>("restore_task", { input });
}

export function transitionTask(input: TransitionTaskInput): Promise<Task> {
  return invokeCommand<Task>("transition_task_status", { input });
}

export function reopenTask(input: ReopenTaskInput): Promise<Task> {
  return invokeCommand<Task>("reopen_task", { input });
}

export function getWorkspaceSummary(): Promise<WorkspaceSummary> {
  return invokeCommand<WorkspaceSummary>("get_workspace_summary");
}

export function createCriterion(input: CreateCriterionInput): Promise<AcceptanceCriterion> {
  return invokeCommand<AcceptanceCriterion>("create_acceptance_criterion", { input });
}

export function listCriteria(taskId: string): Promise<AcceptanceCriterion[]> {
  return invokeCommand<AcceptanceCriterion[]>("list_acceptance_criteria", { taskId });
}

export function updateCriterion(input: UpdateCriterionInput): Promise<AcceptanceCriterion> {
  return invokeCommand<AcceptanceCriterion>("update_acceptance_criterion", { input });
}

export function removeCriterion(input: {
  criterionId: string;
  expectedTaskRevision: number;
  expectedCriterionRevision: number;
}): Promise<void> {
  return invokeCommand<void>("remove_acceptance_criterion", { input });
}

export function confirmManualCriterion(input: ConfirmManualInput): Promise<ManualConfirmation> {
  return invokeCommand<ManualConfirmation>("confirm_manual_criterion", { input });
}
