import { invokeCommand } from "@/lib/ipc";

import type { AppInfo, DatabaseHealth } from "./types";

export function getAppInfo(): Promise<AppInfo> {
  return invokeCommand<AppInfo>("get_app_info");
}

export function getDatabaseHealth(): Promise<DatabaseHealth> {
  return invokeCommand<DatabaseHealth>("get_database_health");
}
