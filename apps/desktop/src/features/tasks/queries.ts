import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { QueryClient } from "@tanstack/react-query";

import type { TaskFilter } from "@/lib/domain";

import {
  archiveTask,
  confirmManualCriterion,
  createCriterion,
  createTask,
  getTask,
  getWorkspaceSummary,
  listTasks,
  removeCriterion,
  reopenTask,
  restoreTask,
  transitionTask,
  updateCriterion,
  updateTask,
} from "./api";

export const taskQueryKeys = {
  all: ["tasks"] as const,
  list: (filter: TaskFilter) => ["tasks", "list", filter] as const,
  detail: (id: string) => ["tasks", "detail", id] as const,
  workspace: ["workspace", "summary"] as const,
};

async function invalidateBusinessData(queryClient: QueryClient) {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: taskQueryKeys.all }),
    queryClient.invalidateQueries({ queryKey: taskQueryKeys.workspace }),
  ]);
}

export function useTasks(filter: TaskFilter, enabled = true) {
  return useQuery({
    queryKey: taskQueryKeys.list(filter),
    queryFn: () => listTasks(filter),
    enabled,
    staleTime: 5_000,
  });
}

export function useTaskDetail(id: string | undefined) {
  return useQuery({
    queryKey: taskQueryKeys.detail(id ?? ""),
    queryFn: () => getTask(id as string),
    enabled: Boolean(id),
    staleTime: 3_000,
  });
}

export function useWorkspaceSummary() {
  return useQuery({
    queryKey: taskQueryKeys.workspace,
    queryFn: getWorkspaceSummary,
    staleTime: 5_000,
  });
}

function useTaskMutation<TInput, TResult>(mutationFn: (input: TInput) => Promise<TResult>) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn,
    onSuccess: async () => {
      await invalidateBusinessData(queryClient);
    },
  });
}

export function useCreateTask() {
  return useTaskMutation(createTask);
}

export function useUpdateTask() {
  return useTaskMutation(updateTask);
}

export function useArchiveTask() {
  return useTaskMutation(archiveTask);
}

export function useRestoreTask() {
  return useTaskMutation(restoreTask);
}

export function useTransitionTask() {
  return useTaskMutation(transitionTask);
}

export function useReopenTask() {
  return useTaskMutation(reopenTask);
}

export function useCreateCriterion() {
  return useTaskMutation(createCriterion);
}

export function useUpdateCriterion() {
  return useTaskMutation(updateCriterion);
}

export function useRemoveCriterion() {
  return useTaskMutation(removeCriterion);
}

export function useConfirmManualCriterion() {
  return useTaskMutation(confirmManualCriterion);
}
