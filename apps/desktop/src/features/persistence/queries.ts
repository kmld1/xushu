import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { systemQueryKeys } from "@/features/system/queries";

import { createProbeRecord, listProbeRecords } from "./api";

export const persistenceQueryKeys = {
  list: ["persistence", "probes"] as const,
};

export function useProbeRecords() {
  return useQuery({
    queryKey: persistenceQueryKeys.list,
    queryFn: () => listProbeRecords(20),
    staleTime: 5_000,
  });
}

export function useCreateProbeRecord() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createProbeRecord,
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: persistenceQueryKeys.list }),
        queryClient.invalidateQueries({ queryKey: systemQueryKeys.health }),
      ]);
    },
  });
}
