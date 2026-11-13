import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { taskQueryKeys } from "@/features/tasks/queries";

import {
  cancelVerification,
  exportAcceptanceReport,
  getEvidence,
  getVerificationRun,
  getVerificationSpec,
  listVerificationQueue,
  listVerificationRuns,
  listVerificationTemplates,
  previewVerification,
  saveVerificationSpec,
  startVerification,
} from "./api";

export const verificationQueryKeys = {
  all: ["verification"] as const,
  templates: ["verification", "templates"] as const,
  spec: (criterionId: string) => ["verification", "spec", criterionId] as const,
  preview: (criterionId: string) => ["verification", "preview", criterionId] as const,
  run: (runId: string) => ["verification", "run", runId] as const,
  runs: (taskId: string) => ["verification", "runs", taskId] as const,
  evidence: (evidenceId: string) => ["verification", "evidence", evidenceId] as const,
  queue: ["verification", "queue"] as const,
};

export function useVerificationTemplates() {
  return useQuery({
    queryKey: verificationQueryKeys.templates,
    queryFn: listVerificationTemplates,
    staleTime: 60_000,
  });
}

export function useVerificationSpec(criterionId: string | undefined, enabled = true) {
  return useQuery({
    queryKey: verificationQueryKeys.spec(criterionId ?? ""),
    queryFn: () => getVerificationSpec(criterionId as string),
    enabled: enabled && Boolean(criterionId),
    staleTime: 3_000,
  });
}

export function usePreviewVerification(criterionId: string | undefined, enabled = true) {
  return useQuery({
    queryKey: verificationQueryKeys.preview(criterionId ?? ""),
    queryFn: () => previewVerification(criterionId as string),
    enabled: enabled && Boolean(criterionId),
    staleTime: 0,
  });
}

export function useVerificationRuns(taskId: string | undefined, enabled = true) {
  return useQuery({
    queryKey: verificationQueryKeys.runs(taskId ?? ""),
    queryFn: () => listVerificationRuns(taskId as string),
    enabled: enabled && Boolean(taskId),
    staleTime: 3_000,
  });
}

export function useVerificationRun(runId: string | null | undefined) {
  return useQuery({
    queryKey: verificationQueryKeys.run(runId ?? ""),
    queryFn: () => getVerificationRun(runId as string),
    enabled: Boolean(runId),
    refetchInterval: (query) => (query.state.data?.executionStatus === "running" ? 1_000 : false),
  });
}

export function useEvidence(evidenceId: string | null | undefined, enabled = true) {
  return useQuery({
    queryKey: verificationQueryKeys.evidence(evidenceId ?? ""),
    queryFn: () => getEvidence(evidenceId as string),
    enabled: enabled && Boolean(evidenceId),
    staleTime: 30_000,
  });
}

export function useVerificationQueue() {
  return useQuery({
    queryKey: verificationQueryKeys.queue,
    queryFn: listVerificationQueue,
    refetchInterval: 3_000,
    staleTime: 2_000,
  });
}

async function invalidateVerificationData(
  queryClient: ReturnType<typeof useQueryClient>,
  taskId?: string,
) {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: verificationQueryKeys.queue }),
    queryClient.invalidateQueries({ queryKey: taskQueryKeys.all }),
    queryClient.invalidateQueries({ queryKey: taskQueryKeys.workspace }),
    queryClient.invalidateQueries({ queryKey: ["recovery"] }),
    taskId
      ? queryClient.invalidateQueries({ queryKey: verificationQueryKeys.runs(taskId) })
      : Promise.resolve(),
  ]);
}

export function useSaveVerificationSpec() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: saveVerificationSpec,
    onSuccess: async (spec) => {
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: verificationQueryKeys.spec(spec.criterionId),
        }),
        queryClient.invalidateQueries({
          queryKey: verificationQueryKeys.preview(spec.criterionId),
        }),
        invalidateVerificationData(queryClient),
      ]);
    },
  });
}

export function useStartVerification() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: startVerification,
    onSuccess: async (run) => {
      queryClient.setQueryData(verificationQueryKeys.run(run.id), run);
      await invalidateVerificationData(queryClient, run.taskId);
    },
  });
}

export function useCancelVerification() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: cancelVerification,
    onSuccess: async (_result, runId) => {
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: verificationQueryKeys.run(runId),
        }),
        queryClient.invalidateQueries({ queryKey: ["recovery"] }),
      ]);
    },
  });
}

export function useExportAcceptanceReport() {
  return useMutation({
    mutationFn: exportAcceptanceReport,
  });
}
