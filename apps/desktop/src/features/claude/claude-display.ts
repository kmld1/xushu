import {
  Ban,
  CheckCircle2,
  CircleDot,
  CircleHelp,
  CircleSlash,
  CircleX,
  CloudOff,
  Flag,
  History,
  Loader2,
  OctagonAlert,
  Play,
  Wrench,
  type LucideIcon,
} from "lucide-react";

import type { ClaudeInstallState, ClaudeToolCallSummary } from "@/lib/domain";

export const CLAUDE_STATE_META: Record<
  ClaudeInstallState,
  {
    label: string;
    icon: LucideIcon;
    variant: "secondary" | "success" | "warning" | "destructive" | "info" | "outline";
    textClassName: string;
  }
> = {
  not_configured: {
    label: "未配置",
    icon: CloudOff,
    variant: "secondary",
    textClassName: "text-muted-foreground",
  },
  disabled: {
    label: "已停用",
    icon: Ban,
    variant: "secondary",
    textClassName: "text-muted-foreground",
  },
  installed: {
    label: "已启用",
    icon: CircleDot,
    variant: "success",
    textClassName: "text-success",
  },
  partial: {
    label: "部分安装",
    icon: Wrench,
    variant: "warning",
    textClassName: "text-warning",
  },
  needs_repair: {
    label: "需要修复",
    icon: Wrench,
    variant: "warning",
    textClassName: "text-warning",
  },
  configuration_error: {
    label: "配置异常",
    icon: OctagonAlert,
    variant: "destructive",
    textClassName: "text-destructive",
  },
  receiver_unavailable: {
    label: "接收器不可用",
    icon: CircleSlash,
    variant: "destructive",
    textClassName: "text-destructive",
  },
  project_unavailable: {
    label: "项目不可用",
    icon: OctagonAlert,
    variant: "destructive",
    textClassName: "text-destructive",
  },
  desynchronized: {
    label: "状态不一致",
    icon: History,
    variant: "warning",
    textClassName: "text-warning",
  },
};

const EVENT_STATUS_META: Record<
  string,
  { label: string; icon: LucideIcon; className: string; spin?: boolean }
> = {
  running: {
    label: "执行中",
    icon: Loader2,
    className: "text-primary",
    spin: true,
  },
  succeeded: { label: "成功", icon: CheckCircle2, className: "text-success" },
  failed: { label: "失败", icon: CircleX, className: "text-destructive" },
  unknown: { label: "状态未知", icon: CircleHelp, className: "text-warning" },
  observed: { label: "已观察", icon: CircleDot, className: "text-info" },
  ended: { label: "已结束", icon: CircleSlash, className: "text-muted-foreground" },
};

export function eventStatusMeta(status: string) {
  return (
    EVENT_STATUS_META[status] ?? {
      label: "状态未知",
      icon: CircleHelp,
      className: "text-warning",
    }
  );
}

export const SESSION_STATE_META: Record<
  string,
  { label: string; icon: LucideIcon; className: string }
> = {
  active: { label: "进行中", icon: CircleDot, className: "text-primary" },
  ended: { label: "已结束", icon: CheckCircle2, className: "text-success" },
  unknown: { label: "结束未确认", icon: CircleHelp, className: "text-warning" },
};

export function sessionStateMeta(state: string) {
  return (
    SESSION_STATE_META[state] ?? {
      label: "状态未知",
      icon: CircleHelp,
      className: "text-warning",
    }
  );
}

export const EVENT_TYPE_META: Record<
  string,
  { label: string; icon: LucideIcon; className: string }
> = {
  session_start: { label: "会话开始", icon: CircleDot, className: "text-info" },
  pre_tool_use: { label: "工具调用开始", icon: Play, className: "text-primary" },
  post_tool_use: { label: "工具调用成功", icon: CheckCircle2, className: "text-success" },
  post_tool_use_failure: { label: "工具调用失败", icon: CircleX, className: "text-destructive" },
  stop: { label: "一轮回答结束", icon: Flag, className: "text-muted-foreground" },
  session_end: { label: "会话结束", icon: CircleSlash, className: "text-muted-foreground" },
};

export function settingsEventLabel(eventName: string): string {
  return (
    {
      SessionStart: "会话开始",
      PreToolUse: "工具调用开始",
      PostToolUse: "工具调用成功",
      PostToolUseFailure: "工具调用失败",
      Stop: "一轮回答结束",
      SessionEnd: "会话结束",
    }[eventName] ?? eventName
  );
}

export function eventTypeMeta(eventType: string) {
  return (
    EVENT_TYPE_META[eventType] ?? {
      label: "未知事件",
      icon: CircleHelp,
      className: "text-warning",
    }
  );
}

export function toolCategoryLabel(category: string | null): string {
  if (!category) {
    return "未知类别";
  }
  return (
    {
      shell: "命令",
      file_read: "读取",
      file_write: "写入",
      search: "搜索",
      web: "网络",
      agent: "代理",
      mcp: "MCP",
      task: "任务",
      other: "其他",
    }[category] ?? "其他"
  );
}

export function failureKindLabel(kind: string | null): string {
  if (!kind) {
    return "原因未知";
  }
  return (
    {
      exit_code_nonzero: "命令退出码非 0",
      command_not_started: "命令未能启动",
      tool_error: "工具错误",
      timeout: "执行超时",
      interrupted: "执行被中断",
      unknown: "原因未知",
    }[kind] ?? "原因未知"
  );
}

export function startSourceLabel(source: string): string {
  return (
    {
      startup: "正常启动",
      resume: "恢复会话",
      clear: "清空后开始",
      compact: "上下文压缩后继续",
      fork: "分叉会话",
      unknown: "开始来源未知",
    }[source] ?? "开始来源未知"
  );
}

export function endReasonLabel(reason: string | null): string {
  if (!reason) {
    return "结束原因未知";
  }
  return (
    {
      clear: "清空会话",
      resume: "切换到其他会话",
      logout: "退出登录",
      prompt_input_exit: "退出输入",
      other: "其他原因",
      unknown: "结束原因未知",
    }[reason] ?? "结束原因未知"
  );
}

export function toolStatusLabel(status: ClaudeToolCallSummary["status"]): string {
  return eventStatusMeta(status).label;
}
