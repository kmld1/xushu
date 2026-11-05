import { useEffect } from "react";

import { useProjects } from "@/features/projects/queries";
import { useWorkspaceStore } from "@/stores/workspace-store";

export function useCurrentProject() {
  const projectsQuery = useProjects(true);
  const selectedProjectId = useWorkspaceStore((state) => state.selectedProjectId);
  const setSelectedProjectId = useWorkspaceStore((state) => state.setSelectedProjectId);

  const allProjects = projectsQuery.data ?? [];
  const activeProjects = allProjects.filter((project) => !project.archivedAt);
  const currentProject =
    activeProjects.find((project) => project.id === selectedProjectId) ?? activeProjects[0] ?? null;

  useEffect(() => {
    if (currentProject && currentProject.id !== selectedProjectId) {
      setSelectedProjectId(currentProject.id);
    } else if (!currentProject && selectedProjectId !== null) {
      setSelectedProjectId(null);
    }
  }, [currentProject, selectedProjectId, setSelectedProjectId]);

  return {
    ...projectsQuery,
    allProjects,
    activeProjects,
    currentProject,
  };
}
