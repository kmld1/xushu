import {
  Ban,
  CheckCircle2,
  CircleSlash,
  LoaderCircle,
  TimerOff,
  Unplug,
  type LucideIcon,
} from "lucide-react";

import type { VerificationExecutionStatus } from "@/lib/domain";

export const EXECUTION_STATUS_META: Record<
  VerificationExecutionStatus,
  { label: string; icon: LucideIcon; className: string }
> = {
  running: {
    label: "运行中",
    icon: LoaderCircle,
    className: "text-primary",
  },
  completed: {
    label: "已完成",
    icon: CheckCircle2,
    className: "text-muted-foreground",
  },
  cancelled: {
    label: "已取消",
    icon: Ban,
    className: "text-muted-foreground",
  },
  timed_out: {
    label: "已超时",
    icon: TimerOff,
    className: "text-verification-fail",
  },
  start_failed: {
    label: "启动失败",
    icon: CircleSlash,
    className: "text-verification-not-run",
  },
  interrupted: {
    label: "已中断",
    icon: Unplug,
    className: "text-verification-inconclusive",
  },
};

export function formatCommand(executable: string, args: string[]): string {
  const quote = (value: string) => {
    if (value.length === 0) {
      return '""';
    }
    return /[\s"'\\]/.test(value) ? `"${value.replace(/"/g, '\\"')}"` : value;
  };

  return [executable, ...args].map(quote).join(" ");
}

export function describeSuccessRule(rule: string): string {
  return rule === "exit_code_zero" ? "退出码为 0 才判定通过" : rule;
}

export function describeSensitiveFlag(flag: string): string {
  switch (flag) {
    case "private_key":
      return "私钥";
    case "authorization_header":
      return "Authorization";
    case "bearer_token":
      return "Bearer Token";
    case "credential_assignment":
      return "凭据赋值";
    case "api_key_prefix":
      return "API Key";
    case "connection_string":
      return "连接串";
    default:
      return flag;
  }
}
