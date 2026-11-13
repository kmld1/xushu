import type {
  RecoveryOverview,
  RecoveryPreference,
  RecoverySnapshot,
  RecoverySnapshotSummary,
  SetNextTaskInput,
} from "@/lib/domain";
import { invokeCommand } from "@/lib/ipc";

export function getRecoveryOverview(projectId: string): Promise<RecoveryOverview> {
  return invokeCommand<RecoveryOverview>("get_recovery_overview", { projectId });
}

export function generateRecoveryPackage(projectId: string): Promise<RecoverySnapshot> {
  return invokeCommand<RecoverySnapshot>("generate_recovery_package", { projectId });
}

export function listRecoverySnapshots(projectId: string): Promise<RecoverySnapshotSummary[]> {
  return invokeCommand<RecoverySnapshotSummary[]>("list_recovery_snapshots", { projectId });
}

export function getRecoverySnapshot(
  projectId: string,
  snapshotId: string,
): Promise<RecoverySnapshot> {
  return invokeCommand<RecoverySnapshot>("get_recovery_snapshot", {
    projectId,
    snapshotId,
  });
}

export function setRecoveryNextTask(input: SetNextTaskInput): Promise<RecoveryPreference> {
  return invokeCommand<RecoveryPreference>("set_recovery_next_task", { input });
}
