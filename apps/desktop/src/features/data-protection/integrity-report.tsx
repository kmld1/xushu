import { BadgeCheck, ShieldAlert } from "lucide-react";

import type { DataIntegrityReport, IntegrityCheck } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";

import { IntegrityStatusBadge } from "./data-protection-badges";

export function IntegrityCheckList({
  checks,
  emptyLabel = "没有可显示的检查项。",
}: {
  checks: IntegrityCheck[];
  emptyLabel?: string;
}) {
  if (checks.length === 0) {
    return <p className="text-[11px] text-muted-foreground">{emptyLabel}</p>;
  }

  return (
    <ul aria-label="完整性检查项" className="divide-y divide-line/60 border-y border-line/70">
      {checks.map((check) => (
        <li key={check.id} className="flex flex-wrap items-start gap-x-3 gap-y-1 py-2">
          <IntegrityStatusBadge status={check.status} />
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium">{check.label}</p>
            {check.detail ? (
              <p className="mt-0.5 break-words text-[11px] leading-5 text-muted-foreground">
                {check.detail}
              </p>
            ) : null}
          </div>
        </li>
      ))}
    </ul>
  );
}

export function IntegrityReportSummary({
  report,
  compact = false,
}: {
  report: DataIntegrityReport;
  compact?: boolean;
}) {
  const Icon = report.ok ? BadgeCheck : ShieldAlert;
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        <Icon
          className={cn("size-4", report.ok ? "text-success" : "text-destructive")}
          aria-hidden="true"
        />
        <p className={cn("text-xs font-medium", report.ok ? "text-success" : "text-destructive")}>
          {report.ok ? "结构与业务关系检查通过" : "检查发现问题"}
        </p>
        <span className="font-mono text-[11px] text-muted-foreground">
          Schema v{report.schemaVersion} / 预期 v{report.expectedSchemaVersion}
        </span>
        <span className="text-[11px] text-muted-foreground">
          检查时间 {formatDateTime(report.checkedAt)}
        </span>
      </div>
      <p className="break-words text-[11px] leading-5 text-muted-foreground">
        {report.integrityMessage}
      </p>
      {!compact ? (
        <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-[11px] text-muted-foreground sm:grid-cols-4">
          <span>项目 {report.counts.projects}</span>
          <span>任务 {report.counts.tasks}</span>
          <span>验收条件 {report.counts.acceptanceCriteria}</span>
          <span>验证运行 {report.counts.verificationRuns}</span>
          <span>证据 {report.counts.evidence}</span>
          <span>Claude 事件 {report.counts.claudeEvents}</span>
          <span>记忆 {report.counts.projectMemories}</span>
          <span>决策 {report.counts.decisions}</span>
        </div>
      ) : null}
      <IntegrityCheckList checks={report.checks} />
      <p className="text-[11px] leading-5 text-muted-foreground">
        完整性检查不能替代任务验收证据，也不会改变任务完成门禁。
      </p>
    </div>
  );
}
