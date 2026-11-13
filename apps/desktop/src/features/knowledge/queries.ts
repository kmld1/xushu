import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { QueryClient } from "@tanstack/react-query";

import type {
  ArchiveMemoryInput,
  ConfirmDecisionInput,
  ConfirmMemoryInput,
  CreateDecisionInput,
  CreateMemoryInput,
  DecisionFilter,
  DeprecateDecisionInput,
  MemoryFilter,
  UpdateDecisionInput,
  UpdateMemoryInput,
} from "@/lib/domain";

import {
  archiveMemory,
  confirmDecision,
  confirmMemory,
  createDecision,
  createMemory,
  deprecateDecision,
  getDecision,
  getMemory,
  listDecisions,
  listMemories,
  restoreMemory,
  updateDecision,
  updateMemory,
} from "./api";

export const knowledgeQueryKeys = {
  all: ["knowledge"] as const,
  memories: (filter: MemoryFilter) => ["knowledge", "memories", filter] as const,
  memory: (projectId: string, memoryId: string) =>
    ["knowledge", "memory", projectId, memoryId] as const,
  decisions: (filter: DecisionFilter) => ["knowledge", "decisions", filter] as const,
  decision: (projectId: string, decisionId: string) =>
    ["knowledge", "decision", projectId, decisionId] as const,
};

async function invalidateKnowledge(queryClient: QueryClient) {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: knowledgeQueryKeys.all }),
    queryClient.invalidateQueries({ queryKey: ["recovery"] }),
  ]);
}

export function useMemories(filter: MemoryFilter, enabled = true) {
  return useQuery({
    queryKey: knowledgeQueryKeys.memories(filter),
    queryFn: () => listMemories(filter),
    enabled: enabled && Boolean(filter.projectId),
    staleTime: 5_000,
  });
}

export function useMemoryDetail(
  projectId: string | undefined,
  memoryId: string | undefined,
  enabled = true,
) {
  return useQuery({
    queryKey: knowledgeQueryKeys.memory(projectId ?? "", memoryId ?? ""),
    queryFn: () => getMemory(projectId as string, memoryId as string),
    enabled: enabled && Boolean(projectId) && Boolean(memoryId),
    staleTime: 2_000,
  });
}

export function useDecisions(filter: DecisionFilter, enabled = true) {
  return useQuery({
    queryKey: knowledgeQueryKeys.decisions(filter),
    queryFn: () => listDecisions(filter),
    enabled: enabled && Boolean(filter.projectId),
    staleTime: 5_000,
  });
}

export function useDecisionDetail(
  projectId: string | undefined,
  decisionId: string | undefined,
  enabled = true,
) {
  return useQuery({
    queryKey: knowledgeQueryKeys.decision(projectId ?? "", decisionId ?? ""),
    queryFn: () => getDecision(projectId as string, decisionId as string),
    enabled: enabled && Boolean(projectId) && Boolean(decisionId),
    staleTime: 2_000,
  });
}

function useKnowledgeMutation<TInput, TResult>(mutationFn: (input: TInput) => Promise<TResult>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn,
    onSuccess: () => invalidateKnowledge(queryClient),
  });
}

export function useCreateMemory() {
  return useKnowledgeMutation<CreateMemoryInput, Awaited<ReturnType<typeof createMemory>>>(
    createMemory,
  );
}

export function useUpdateMemory() {
  return useKnowledgeMutation<UpdateMemoryInput, Awaited<ReturnType<typeof updateMemory>>>(
    updateMemory,
  );
}

export function useConfirmMemory() {
  return useKnowledgeMutation<ConfirmMemoryInput, Awaited<ReturnType<typeof confirmMemory>>>(
    confirmMemory,
  );
}

export function useArchiveMemory() {
  return useKnowledgeMutation<ArchiveMemoryInput, Awaited<ReturnType<typeof archiveMemory>>>(
    archiveMemory,
  );
}

export function useRestoreMemory() {
  return useKnowledgeMutation<ArchiveMemoryInput, Awaited<ReturnType<typeof restoreMemory>>>(
    restoreMemory,
  );
}

export function useCreateDecision() {
  return useKnowledgeMutation<CreateDecisionInput, Awaited<ReturnType<typeof createDecision>>>(
    createDecision,
  );
}

export function useUpdateDecision() {
  return useKnowledgeMutation<UpdateDecisionInput, Awaited<ReturnType<typeof updateDecision>>>(
    updateDecision,
  );
}

export function useConfirmDecision() {
  return useKnowledgeMutation<ConfirmDecisionInput, Awaited<ReturnType<typeof confirmDecision>>>(
    confirmDecision,
  );
}

export function useDeprecateDecision() {
  return useKnowledgeMutation<
    DeprecateDecisionInput,
    Awaited<ReturnType<typeof deprecateDecision>>
  >(deprecateDecision);
}
