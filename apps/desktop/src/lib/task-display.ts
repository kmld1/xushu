import {
  ArrowDown,
  ArrowUp,
  BadgeCheck,
  Ban,
  CheckCircle2,
  CircleDot,
  CircleHelp,
  CircleSlash,
  CircleX,
  FilePenLine,
  Flame,
  History,
  Hourglass,
  Minus,
  OctagonAlert,
  Play,
  type LucideIcon,
} from "lucide-react";

import type { TaskPriority, TaskStatus } from "@/lib/domain";

export const TASK_STATUS_ORDER: TaskStatus[] = [
  "draft",
  "ready",
  "running",
  "awaiting_verification",
  "done",
];

export const TASK_STATUS_META: Record<
  TaskStatus,
  { label: string; icon: LucideIcon; className: string }
> = {
  draft: {
    label: "草稿",
    icon: FilePenLine,
    className: "border-status-draft/35 bg-status-draft/10 text-status-draft",
  },
  ready: {
    label: "就绪",
    icon: CircleDot,
    className: "border-status-ready/35 bg-status-ready/10 text-status-ready",
  },
  running: {
    label: "执行中",
    icon: Play,
    className: "border-status-running/35 bg-status-running/10 text-status-running",
  },
  awaiting_verification: {
    label: "待验证",
    icon: Hourglass,
    className: "border-status-awaiting/40 bg-status-awaiting/10 text-status-awaiting",
  },
  done: {
    label: "已完成",
    icon: CheckCircle2,
    className: "border-status-done/35 bg-status-done/10 text-status-done",
  },
  blocked: {
    label: "已阻塞",
    icon: OctagonAlert,
    className: "border-status-blocked/40 bg-status-blocked/10 text-status-blocked",
  },
  cancelled: {
    label: "已取消",
    icon: Ban,
    className: "border-status-cancelled/35 bg-status-cancelled/10 text-status-cancelled",
  },
};

export const TASK_PRIORITY_META: Record<
  TaskPriority,
  { label: string; icon: LucideIcon; className: string }
> = {
  low: { label: "低", icon: ArrowDown, className: "text-muted-foreground" },
  medium: { label: "中", icon: Minus, className: "text-info" },
  high: { label: "高", icon: ArrowUp, className: "text-warning" },
  urgent: { label: "紧急", icon: Flame, className: "text-destructive" },
};

export type VerificationStatus = "pass" | "fail" | "not_run" | "inconclusive" | "stale";

export const VERIFICATION_META: Record<
  VerificationStatus,
  { label: string; icon: LucideIcon; className: string }
> = {
  pass: {
    label: "通过",
    icon: BadgeCheck,
    className: "border-verification-pass/35 bg-verification-pass/10 text-verification-pass",
  },
  fail: {
    label: "失败",
    icon: CircleX,
    className: "border-verification-fail/35 bg-verification-fail/10 text-verification-fail",
  },
  not_run: {
    label: "未运行",
    icon: CircleSlash,
    className:
      "border-verification-not-run/35 bg-verification-not-run/10 text-verification-not-run",
  },
  inconclusive: {
    label: "无法判定",
    icon: CircleHelp,
    className:
      "border-verification-inconclusive/35 bg-verification-inconclusive/10 text-verification-inconclusive",
  },
  stale: {
    label: "已过期",
    icon: History,
    className: "border-verification-stale/35 bg-verification-stale/10 text-verification-stale",
  },
};
