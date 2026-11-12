import { FileSearch, History } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { LoadingState } from "@/components/loading-state";
import { VerificationBadge } from "@/components/status-badge";
import { useVerificationRuns } from "@/features/verification/queries";
import { formatDateTime, formatDuration } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { EXECUTION_STATUS_META, formatCommand } from "@/lib/verification-display";

export function VerificationHistorySheet({
  open,
  onOpenChange,
  taskId,
  criterionId,
  onViewEvidence,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  taskId: string;
  criterionId: string;
  onViewEvidence: (evidenceId: string) => void;
}) {
  const runs = useVerificationRuns(taskId, open);
  const filtered = (runs.data ?? []).filter((run) => run.criterionId === criterionId);

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            <History className="size-4 text-primary" aria-hidden="true" />
            验证历史
          </SheetTitle>
          <SheetDescription>
            历史结果保留在本地，不会自动删除；当前有效性以任务详情中的状态为准。
          </SheetDescription>
        </SheetHeader>

        <SheetBody>
          {runs.isPending ? (
            <LoadingState rows={5} />
          ) : runs.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {getErrorMessage(runs.error)}
            </p>
          ) : filtered.length === 0 ? (
            <p className="py-8 text-center text-xs text-muted-foreground">尚无验证运行记录。</p>
          ) : (
            <ul className="divide-y divide-line/70 border-y border-line/70">
              {filtered.map((run) => {
                const execution = EXECUTION_STATUS_META[run.executionStatus];
                const ExecutionIcon = execution.icon;

                return (
                  <li key={run.id} className="py-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <span
                        className={`inline-flex items-center gap-1 text-[11px] font-medium ${execution.className}`}
                      >
                        <ExecutionIcon
                          className={`size-3.5 ${run.executionStatus === "running" ? "animate-spin" : ""}`}
                          aria-hidden="true"
                        />
                        {execution.label}
                      </span>
                      {run.result ? <VerificationBadge status={run.result} /> : null}
                      <time
                        dateTime={run.startedAt}
                        className="text-xs tabular-nums text-muted-foreground"
                      >
                        {formatDateTime(run.startedAt)}
                      </time>
                      <span className="ml-auto font-mono text-xs tabular-nums text-muted-foreground">
                        退出码 {run.exitCode ?? "—"} · {formatDuration(run.durationMs)}
                      </span>
                    </div>
                    <p
                      className="mt-2 truncate font-mono text-xs text-foreground"
                      title={formatCommand(run.executable, run.args)}
                    >
                      {formatCommand(run.executable, run.args)}
                    </p>
                    <div className="mt-1 flex flex-wrap items-center gap-2">
                      <span className="font-mono text-xs tabular-nums text-muted-foreground">
                        task r{run.taskRevision} / criterion r{run.criterionRevision} / spec r
                        {run.specRevision}
                      </span>
                      {run.evidenceId ? (
                        <button
                          type="button"
                          className="ml-auto inline-flex min-h-7 items-center gap-1 rounded-md px-2 text-xs font-medium text-primary transition-colors duration-120 hover:bg-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                          onClick={() => onViewEvidence(run.evidenceId as string)}
                        >
                          <FileSearch className="size-3" aria-hidden="true" />
                          查看证据
                        </button>
                      ) : null}
                    </div>
                    {run.reason ? (
                      <p className="mt-1 break-words text-[11px] leading-5 text-muted-foreground">
                        {run.reason}
                      </p>
                    ) : null}
                    {run.sensitiveFlags.length > 0 ? (
                      <div className="mt-1.5 flex flex-wrap gap-1">
                        {run.sensitiveFlags.map((flag) => (
                          <Badge key={flag} variant="warning">
                            已脱敏
                          </Badge>
                        ))}
                      </div>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          )}
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}
