import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  archiveProject,
  getProject,
  getProjectGitStatus,
  importProject,
  listProjects,
  restoreProject,
  updateProject,
} from "./api";

export const projectQueryKeys = {
  all: ["projects"] as const,
  list: (includeArchived: boolean) => ["projects", "list", includeArchived] as const,
  detail: (id: string) => ["projects", "detail", id] as const,
  git: (id: string) => ["projects", "git", id] as const,
};

export function useProjects(includeArchived = true) {
  return useQuery({
    queryKey: projectQueryKeys.list(includeArchived),
    queryFn: () => listProjects(includeArchived),
    staleTime: 15_000,
  });
}

export function useProject(id: string | undefined) {
  return useQuery({
    queryKey: projectQueryKeys.detail(id ?? ""),
    queryFn: () => getProject(id as string),
    enabled: Boolean(id),
  });
}

export function useProjectGitStatus(id: string | undefined) {
  return useQuery({
    queryKey: projectQueryKeys.git(id ?? ""),
    queryFn: () => getProjectGitStatus(id as string),
    enabled: Boolean(id),
    staleTime: 10_000,
    retry: 0,
  });
}

export function useImportProject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: importProject,
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: projectQueryKeys.all }),
        queryClient.invalidateQueries({ queryKey: ["workspace"] }),
      ]);
    },
  });
}

export function useUpdateProject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: updateProject,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: projectQueryKeys.all });
    },
  });
}

export function useArchiveProject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: archiveProject,
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: projectQueryKeys.all }),
        queryClient.invalidateQueries({ queryKey: ["tasks"] }),
        queryClient.invalidateQueries({ queryKey: ["workspace"] }),
      ]);
    },
  });
}

export function useRestoreProject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: restoreProject,
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: projectQueryKeys.all }),
        queryClient.invalidateQueries({ queryKey: ["tasks"] }),
        queryClient.invalidateQueries({ queryKey: ["workspace"] }),
      ]);
    },
  });
}
