import { useQuery } from "@tanstack/react-query";

import { getAppInfo, getDatabaseHealth } from "./api";

export const systemQueryKeys = {
  appInfo: ["system", "app-info"] as const,
  health: ["system", "health"] as const,
};

export function useAppInfo() {
  return useQuery({
    queryKey: systemQueryKeys.appInfo,
    queryFn: getAppInfo,
    staleTime: 60_000,
  });
}

export function useDatabaseHealth() {
  return useQuery({
    queryKey: systemQueryKeys.health,
    queryFn: getDatabaseHealth,
    staleTime: 5_000,
  });
}
