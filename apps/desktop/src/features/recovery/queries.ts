import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { RecoveryOverview, SetNextTaskInput } from "@/lib/domain";

import {
  generateRecoveryPackage,
  getRecoveryOverview,
  getRecoverySnapshot,
  listRecoverySnapshots,
  setRecoveryNextTask,
} from "./api";

export const recoveryQueryKeys = {
  all: ["recovery"] as const,
  overview: (projectId: string) => ["recovery", "overview", projectId] as const,
  snapshots: (projectId: string) => ["recovery", "snapshots", projectId] as const,
  snapshot: (projectId: string, snapshotId: string) =>
    ["recovery", "snapshot", projectId, snapshotId] as const,
};

export function useRecoveryOverview(projectId: string | undefined) {
  return useQuery({
    queryKey: recoveryQueryKeys.overview(projectId ?? ""),
    queryFn: () => getRecoveryOverview(projectId as string),
    enabled: Boolean(projectId),
    staleTime: 0,
    refetchOnWindowFocus: false,
    retry: 0,
  });
}

export function useGenerateRecovery(projectId: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => generateRecoveryPackage(projectId as string),
    onSuccess: async (snapshot) => {
      if (projectId) {
        queryClient.setQueryData<RecoveryOverview>(recoveryQueryKeys.overview(projectId), {
          latest: snapshot,
          freshness: {
            state: "fresh",
            checkedAt: snapshot.generatedAt,
            changed: [],
          },
        });
        await queryClient.invalidateQueries({
          queryKey: recoveryQueryKeys.snapshots(projectId),
        });
      }
    },
  });
}

export function useRecoverySnapshots(projectId: string | undefined, enabled = true) {
  return useQuery({
    queryKey: recoveryQueryKeys.snapshots(projectId ?? ""),
    queryFn: () => listRecoverySnapshots(projectId as string),
    enabled: Boolean(projectId) && enabled,
    staleTime: 5_000,
    retry: 0,
  });
}

export function useRecoverySnapshot(
  projectId: string | undefined,
  snapshotId: string | undefined,
  enabled = true,
) {
  return useQuery({
    queryKey: recoveryQueryKeys.snapshot(projectId ?? "", snapshotId ?? ""),
    queryFn: () => getRecoverySnapshot(projectId as string, snapshotId as string),
    enabled: Boolean(projectId) && Boolean(snapshotId) && enabled,
    staleTime: 60_000,
    retry: 0,
  });
}

export function useSetRecoveryNextTask(projectId: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: SetNextTaskInput) => setRecoveryNextTask(input),
    onSuccess: async () => {
      if (projectId) {
        await queryClient.invalidateQueries({
          queryKey: recoveryQueryKeys.all,
        });
      }
    },
  });
}
