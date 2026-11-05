import { invokeCommand } from "@/lib/ipc";

import type { CreateProbeInput, ProbeRecord } from "./types";

export function createProbeRecord(input: CreateProbeInput): Promise<ProbeRecord> {
  return invokeCommand<ProbeRecord>("create_probe_record", { input });
}

export function listProbeRecords(limit = 20): Promise<ProbeRecord[]> {
  return invokeCommand<ProbeRecord[]>("list_probe_records", { limit });
}

export function getProbeRecord(id: string): Promise<ProbeRecord> {
  return invokeCommand<ProbeRecord>("get_probe_record", { id });
}
