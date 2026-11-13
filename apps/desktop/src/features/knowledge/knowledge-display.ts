import {
  BadgeCheck,
  Ban,
  BookOpen,
  CircleHelp,
  FilePenLine,
  History,
  ListTodo,
  Lock,
  Network,
  ScrollText,
  ShieldAlert,
  Target,
  type LucideIcon,
} from "lucide-react";

import type {
  DecisionStatus,
  MemoryCategory,
  MemoryConfirmationStatus,
  MemorySourceType,
} from "@/lib/domain";

export const MEMORY_CATEGORIES: MemoryCategory[] = [
  "goal",
  "constraint",
  "architecture",
  "convention",
  "fact",
  "risk",
  "follow_up",
  "knowledge",
];

export const MEMORY_CATEGORY_META: Record<
  MemoryCategory,
  { label: string; description: string; icon: LucideIcon }
> = {
  goal: {
    label: "项目目标",
    description: "项目要解决的问题与成功标准",
    icon: Target,
  },
  constraint: {
    label: "开发约束",
    description: "不能违反的边界、依赖与安全要求",
    icon: Lock,
  },
  architecture: {
    label: "技术架构决策",
    description: "已经采用的架构与关键取舍",
    icon: Network,
  },
  convention: {
    label: "开发规范",
    description: "代码风格、流程与协作约定",
    icon: ScrollText,
  },
  fact: {
    label: "已确认事实",
    description: "经过确认、可复用的重要事实",
    icon: BadgeCheck,
  },
  risk: {
    label: "风险与已知问题",
    description: "仍然有效的影响、缺陷与隐患",
    icon: ShieldAlert,
  },
  follow_up: {
    label: "后续待处理",
    description: "尚未关闭、需要继续跟进的事项",
    icon: ListTodo,
  },
  knowledge: {
    label: "项目知识",
    description: "用户主动记录的项目背景与经验",
    icon: BookOpen,
  },
};

export const MEMORY_CONFIRMATION_META: Record<
  MemoryConfirmationStatus,
  { label: string; icon: LucideIcon; variant: "success" | "warning" }
> = {
  confirmed: { label: "已确认", icon: BadgeCheck, variant: "success" },
  unconfirmed: { label: "未确认", icon: CircleHelp, variant: "warning" },
};

export const MEMORY_SOURCE_LABEL: Record<MemorySourceType, string> = {
  user: "用户记录",
  system: "XUSHU 系统",
  git: "Git 状态",
  verification: "独立验证",
  hook: "Claude Code Hook",
  inference: "AI 推断",
};

export const DECISION_STATUS_META: Record<
  DecisionStatus,
  { label: string; icon: LucideIcon; variant: "info" | "success" | "warning" | "outline" }
> = {
  draft: { label: "草稿", icon: FilePenLine, variant: "info" },
  confirmed: { label: "已确认", icon: BadgeCheck, variant: "success" },
  superseded: { label: "已被取代", icon: History, variant: "warning" },
  deprecated: { label: "已废弃", icon: Ban, variant: "outline" },
};

export function categoryLabel(category: MemoryCategory): string {
  return MEMORY_CATEGORY_META[category].label;
}

export function decisionStatusLabel(status: DecisionStatus): string {
  return DECISION_STATUS_META[status].label;
}
