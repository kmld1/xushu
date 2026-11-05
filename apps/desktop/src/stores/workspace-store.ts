import { create } from "zustand";
import { persist } from "zustand/middleware";

type WorkspaceState = {
  selectedProjectId: string | null;
  setSelectedProjectId: (projectId: string | null) => void;
};

export const useWorkspaceStore = create<WorkspaceState>()(
  persist(
    (set) => ({
      selectedProjectId: null,
      setSelectedProjectId: (selectedProjectId) => set({ selectedProjectId }),
    }),
    {
      name: "xushu-workspace",
      version: 1,
    },
  ),
);
