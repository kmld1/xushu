import type {
  ArchiveMemoryInput,
  ConfirmDecisionInput,
  ConfirmMemoryInput,
  CreateDecisionInput,
  CreateMemoryInput,
  DecisionDetail,
  DecisionFilter,
  DecisionView,
  DeprecateDecisionInput,
  MemoryDetail,
  MemoryFilter,
  ProjectMemoryView,
  UpdateDecisionInput,
  UpdateMemoryInput,
} from "@/lib/domain";
import { invokeCommand } from "@/lib/ipc";

export function listMemories(filter: MemoryFilter): Promise<ProjectMemoryView[]> {
  return invokeCommand<ProjectMemoryView[]>("list_project_memories", { filter });
}

export function getMemory(projectId: string, memoryId: string): Promise<MemoryDetail> {
  return invokeCommand<MemoryDetail>("get_project_memory", { projectId, memoryId });
}

export function createMemory(input: CreateMemoryInput): Promise<ProjectMemoryView> {
  return invokeCommand<ProjectMemoryView>("create_project_memory", { input });
}

export function updateMemory(input: UpdateMemoryInput): Promise<ProjectMemoryView> {
  return invokeCommand<ProjectMemoryView>("update_project_memory", { input });
}

export function confirmMemory(input: ConfirmMemoryInput): Promise<ProjectMemoryView> {
  return invokeCommand<ProjectMemoryView>("confirm_project_memory", { input });
}

export function archiveMemory(input: ArchiveMemoryInput): Promise<ProjectMemoryView> {
  return invokeCommand<ProjectMemoryView>("archive_project_memory", { input });
}

export function restoreMemory(input: ArchiveMemoryInput): Promise<ProjectMemoryView> {
  return invokeCommand<ProjectMemoryView>("restore_project_memory", { input });
}

export function listDecisions(filter: DecisionFilter): Promise<DecisionView[]> {
  return invokeCommand<DecisionView[]>("list_project_decisions", { filter });
}

export function getDecision(projectId: string, decisionId: string): Promise<DecisionDetail> {
  return invokeCommand<DecisionDetail>("get_project_decision", { projectId, decisionId });
}

export function createDecision(input: CreateDecisionInput): Promise<DecisionView> {
  return invokeCommand<DecisionView>("create_project_decision", { input });
}

export function updateDecision(input: UpdateDecisionInput): Promise<DecisionView> {
  return invokeCommand<DecisionView>("update_project_decision", { input });
}

export function confirmDecision(input: ConfirmDecisionInput): Promise<DecisionView> {
  return invokeCommand<DecisionView>("confirm_project_decision", { input });
}

export function deprecateDecision(input: DeprecateDecisionInput): Promise<DecisionView> {
  return invokeCommand<DecisionView>("deprecate_project_decision", { input });
}
