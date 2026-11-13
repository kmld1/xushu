import type {
  BackupIntegrityStatus,
  BackupOrigin,
  IntegrityCheckStatus,
  RestoreOperationSummary,
  RestoreOutcome,
} from "@/lib/domain";

export const BACKUP_ORIGIN_LABEL: Record<BackupOrigin, string> = {
  manual: "手动",
  automatic: "自动",
};

export const BACKUP_INTEGRITY_LABEL: Record<BackupIntegrityStatus, string> = {
  verified: "已验证",
  failed: "校验失败",
  unverified: "未校验",
  missing: "目录缺失",
};

export const INTEGRITY_CHECK_LABEL: Record<IntegrityCheckStatus, string> = {
  pass: "通过",
  fail: "失败",
  not_run: "未运行",
};

export const RESTORE_OUTCOME_LABEL: Record<string, string> = {
  completed: "恢复完成",
  rolled_back: "已回退",
  failed: "恢复失败",
  rollback_failed: "回退失败",
};

export function formatSize(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) {
    return "—";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  if (bytes < 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function shortHash(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }
  return value.length > 16 ? `${value.slice(0, 16)}…` : value;
}

export function restoreOutcomeTone(
  outcome: RestoreOperationSummary["outcome"],
): "success" | "warning" | "destructive" {
  if (outcome === "completed") {
    return "success";
  }
  if (outcome === "rolled_back") {
    return "warning";
  }
  return "destructive";
}

export function restoreResultTone(
  result: RestoreOutcome | null,
): "success" | "warning" | "destructive" {
  if (!result) {
    return "warning";
  }
  if (result.status === "completed") {
    return "success";
  }
  if (result.status === "rolled_back") {
    return "warning";
  }
  return "destructive";
}

export const RESTORE_STEPS = [
  "选择备份",
  "兼容性",
  "影响范围",
  "隔离验证",
  "明确确认",
  "执行恢复",
  "验证结果",
] as const;

export type RestoreWizardStage = "choose" | "preflight" | "confirm" | "executing" | "result";

export function restoreStepIndex(stage: RestoreWizardStage, ready: boolean): number {
  switch (stage) {
    case "choose":
      return 0;
    case "preflight":
      return ready ? 3 : 1;
    case "confirm":
      return 4;
    case "executing":
      return 5;
    case "result":
      return 6;
  }
}

export function restoreStepState(
  index: number,
  current: number,
  failed: boolean,
): "done" | "active" | "pending" | "failed" {
  if (failed && index === current) {
    return "failed";
  }
  if (index < current) {
    return "done";
  }
  if (index === current) {
    return "active";
  }
  return "pending";
}
