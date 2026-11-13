import type {
  AutomaticBackupOutcome,
  BackupPolicy,
  BackupRecord,
  BackupVerification,
  CleanupOutcome,
  DataIntegrityReport,
  DataProtectionOverview,
  PerformRestoreInput,
  RestoreOutcome,
  RestorePreflight,
  UpdateBackupPolicyInput,
} from "@/lib/domain";
import { invokeCommand } from "@/lib/ipc";

export function getDataProtectionOverview(): Promise<DataProtectionOverview> {
  return invokeCommand<DataProtectionOverview>("get_data_protection_overview");
}

export function createBackup(targetDirectory: string): Promise<BackupRecord> {
  return invokeCommand<BackupRecord>("create_backup", { targetDirectory });
}

export function cancelBackup(): Promise<void> {
  return invokeCommand<void>("cancel_backup");
}

export function verifyBackup(backupId: string): Promise<BackupVerification> {
  return invokeCommand<BackupVerification>("verify_backup", { backupId });
}

export function deleteBackup(backupId: string, approved: boolean): Promise<void> {
  return invokeCommand<void>("delete_backup", { backupId, approved });
}

export function getBackupPolicy(): Promise<BackupPolicy> {
  return invokeCommand<BackupPolicy>("get_backup_policy");
}

export function updateBackupPolicy(input: UpdateBackupPolicyInput): Promise<BackupPolicy> {
  return invokeCommand<BackupPolicy>("update_backup_policy", { input });
}

export function previewBackupCleanup(): Promise<BackupRecord[]> {
  return invokeCommand<BackupRecord[]>("preview_backup_cleanup");
}

export function applyBackupCleanup(approved: boolean): Promise<CleanupOutcome> {
  return invokeCommand<CleanupOutcome>("apply_backup_cleanup", { approved });
}

export function runDueAutomaticBackup(): Promise<AutomaticBackupOutcome> {
  return invokeCommand<AutomaticBackupOutcome>("run_due_automatic_backup");
}

export function checkDataIntegrity(): Promise<DataIntegrityReport> {
  return invokeCommand<DataIntegrityReport>("check_data_integrity");
}

export function preflightRestore(backupDirectory: string): Promise<RestorePreflight> {
  return invokeCommand<RestorePreflight>("preflight_restore", {
    input: { backupDirectory },
  });
}

export function performRestore(input: PerformRestoreInput): Promise<RestoreOutcome> {
  return invokeCommand<RestoreOutcome>("perform_restore", { input });
}

export function dismissRestoreNotice(): Promise<void> {
  return invokeCommand<void>("dismiss_restore_notice");
}
