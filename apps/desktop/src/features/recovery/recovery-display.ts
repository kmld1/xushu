import {
  BadgeCheck,
  CircleHelp,
  CircleSlash,
  History,
  UserRoundCheck,
  Wand2,
  type LucideIcon,
} from "lucide-react";

import type { RecoveryFreshnessState, RecoveryGitState, RecoveryNextStepKind } from "@/lib/domain";

export const RECOVERY_FRESHNESS_META: Record<
  RecoveryFreshnessState,
  {
    label: string;
    description: string;
    icon: LucideIcon;
    variant: "success" | "warning" | "outline";
  }
> = {
  fresh: {
    label: "当前",
    description: "快照指纹与当前数据一致",
    icon: BadgeCheck,
    variant: "success",
  },
  stale: {
    label: "已过期",
    description: "生成后项目数据已变化",
    icon: History,
    variant: "warning",
  },
  no_snapshot: {
    label: "尚未生成",
    description: "还没有保存过恢复包",
    icon: CircleSlash,
    variant: "outline",
  },
};

export const NEXT_STEP_KIND_META: Record<
  RecoveryNextStepKind,
  { label: string; description: string; icon: LucideIcon; variant: "success" | "warning" | "info" }
> = {
  user_selected: {
    label: "用户指定",
    description: "你在 XUSHU 中明确选择的下一步",
    icon: UserRoundCheck,
    variant: "success",
  },
  suggested: {
    label: "系统建议",
    description: "根据任务状态推断，需要人工确认",
    icon: Wand2,
    variant: "info",
  },
  needs_confirmation: {
    label: "需要人工确认",
    description: "没有足够依据推断下一步",
    icon: CircleHelp,
    variant: "warning",
  },
};

export const GIT_STATE_META: Record<
  RecoveryGitState,
  { label: string; variant: "success" | "warning" }
> = {
  available: { label: "可用", variant: "success" },
  unknown: { label: "UNKNOWN", variant: "warning" },
};

const CONFLICT_LABELS: Record<string, string> = {
  done_evidence_invalid: "已完成任务缺少有效证据",
  next_task_unavailable: "指定的下一步不可用",
  memory_linked_task_inactive: "记忆关联任务已失效",
};

export function conflictLabel(kind: string): string {
  return CONFLICT_LABELS[kind] ?? "需要复核";
}

export function shortFingerprint(fingerprint: string): string {
  const value = fingerprint.startsWith("sha256:") ? fingerprint.slice(7) : fingerprint;
  return value.length > 16 ? `${value.slice(0, 16)}…` : value;
}
