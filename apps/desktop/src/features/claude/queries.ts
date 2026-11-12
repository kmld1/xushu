import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { ClaudeActionInput, InstallClaudeHooksInput } from "@/lib/domain";
import {
  disableClaudeHooks,
  getClaudeActivity,
  getClaudeIntegration,
  installClaudeHooks,
  previewClaudeHooks,
  refreshClaudeEvents,
  uninstallClaudeHooks,
} from "./api";

export const claudeQueryKeys = {
  all: ["claude"] as const,
  status: (projectId: string) => ["claude", "status", projectId] as const,
  activity: (projectId: string) => ["claude", "activity", projectId] as const,
  preview: (projectId: string) => ["claude", "preview", projectId] as const,
};

export function useClaudeStatus(projectId: string | undefined) {
  return useQuery({
    queryKey: claudeQueryKeys.status(projectId ?? ""),
    queryFn: () => getClaudeIntegration(projectId as string),
    enabled: Boolean(projectId),
    staleTime: 5_000,
    refetchInterval: 15_000,
    retry: 0,
  });
}

export function useClaudeActivity(projectId: string | undefined, enabled = true) {
  return useQuery({
    queryKey: claudeQueryKeys.activity(projectId ?? ""),
    queryFn: () => getClaudeActivity(projectId as string),
    enabled: Boolean(projectId) && enabled,
    staleTime: 5_000,
    refetchInterval: 15_000,
    retry: 0,
  });
}

export function useClaudePreview(projectId: string | undefined, open: boolean) {
  return useQuery({
    queryKey: claudeQueryKeys.preview(projectId ?? ""),
    queryFn: () => previewClaudeHooks(projectId as string),
    enabled: Boolean(projectId) && open,
    staleTime: 0,
    retry: 0,
  });
}

function invalidateClaude(queryClient: ReturnType<typeof useQueryClient>, projectId: string) {
  return Promise.all([
    queryClient.invalidateQueries({ queryKey: claudeQueryKeys.status(projectId) }),
    queryClient.invalidateQueries({ queryKey: claudeQueryKeys.activity(projectId) }),
    queryClient.invalidateQueries({ queryKey: claudeQueryKeys.preview(projectId) }),
  ]);
}

export function useInstallClaudeHooks(projectId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: InstallClaudeHooksInput) => installClaudeHooks(input),
    onSuccess: () => invalidateClaude(queryClient, projectId),
  });
}

export function useDisableClaudeHooks(projectId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: ClaudeActionInput) => disableClaudeHooks(input),
    onSuccess: () => invalidateClaude(queryClient, projectId),
  });
}

export function useUninstallClaudeHooks(projectId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: ClaudeActionInput) => uninstallClaudeHooks(input),
    onSuccess: () => invalidateClaude(queryClient, projectId),
  });
}

export function useRefreshClaudeEvents(projectId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => refreshClaudeEvents(projectId),
    onSuccess: () => invalidateClaude(queryClient, projectId),
  });
}
