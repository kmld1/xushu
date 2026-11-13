import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { PerformRestoreInput, UpdateBackupPolicyInput } from "@/lib/domain";

import {
  applyBackupCleanup,
  cancelBackup,
  checkDataIntegrity,
  createBackup,
  deleteBackup,
  dismissRestoreNotice,
  getDataProtectionOverview,
  performRestore,
  preflightRestore,
  previewBackupCleanup,
  runDueAutomaticBackup,
  updateBackupPolicy,
  verifyBackup,
} from "./api";

export const dataProtectionQueryKeys = {
  all: ["data-protection"] as const,
  overview: () => ["data-protection", "overview"] as const,
};

export function useDataProtectionOverview() {
  return useQuery({
    queryKey: dataProtectionQueryKeys.overview(),
    queryFn: getDataProtectionOverview,
    staleTime: 0,
    refetchOnWindowFocus: false,
    retry: 0,
  });
}

function useOverviewInvalidation() {
  const queryClient = useQueryClient();
  return async () => {
    await queryClient.invalidateQueries({ queryKey: dataProtectionQueryKeys.all });
  };
}

export function useCreateBackup() {
  const invalidate = useOverviewInvalidation();
  return useMutation({
    mutationFn: (targetDirectory: string) => createBackup(targetDirectory),
    onSuccess: invalidate,
  });
}

export function useCancelBackup() {
  return useMutation({ mutationFn: cancelBackup });
}

export function useVerifyBackup() {
  const invalidate = useOverviewInvalidation();
  return useMutation({
    mutationFn: (backupId: string) => verifyBackup(backupId),
    onSuccess: invalidate,
  });
}

export function useDeleteBackup() {
  const invalidate = useOverviewInvalidation();
  return useMutation({
    mutationFn: (backupId: string) => deleteBackup(backupId, true),
    onSuccess: invalidate,
  });
}

export function useUpdateBackupPolicy() {
  const invalidate = useOverviewInvalidation();
  return useMutation({
    mutationFn: (input: UpdateBackupPolicyInput) => updateBackupPolicy(input),
    onSuccess: invalidate,
  });
}

export function usePreviewBackupCleanup() {
  return useMutation({ mutationFn: previewBackupCleanup });
}

export function useApplyBackupCleanup() {
  const invalidate = useOverviewInvalidation();
  return useMutation({
    mutationFn: () => applyBackupCleanup(true),
    onSuccess: invalidate,
  });
}

export function useRunDueAutomaticBackup() {
  const invalidate = useOverviewInvalidation();
  return useMutation({
    mutationFn: runDueAutomaticBackup,
    onSuccess: invalidate,
  });
}

export function useCheckDataIntegrity() {
  return useMutation({ mutationFn: checkDataIntegrity });
}

export function usePreflightRestore() {
  return useMutation({
    mutationFn: (backupDirectory: string) => preflightRestore(backupDirectory),
  });
}

export function usePerformRestore() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: PerformRestoreInput) => performRestore(input),
    onSuccess: async () => {
      await queryClient.invalidateQueries();
    },
  });
}

export function useDismissRestoreNotice() {
  const invalidate = useOverviewInvalidation();
  return useMutation({
    mutationFn: dismissRestoreNotice,
    onSuccess: invalidate,
  });
}
