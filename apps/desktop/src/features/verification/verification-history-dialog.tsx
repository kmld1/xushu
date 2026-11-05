import { FileSearch, History } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { LoadingState } from "@/components/loading-state";
import { VerificationBadge } from "@/components/status-badge";
import { useVerificationRuns } from "@/features/verification/queries";
import { formatDateTime, formatDuration } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { EXECUTION_STATUS_META, formatCommand } from "@/lib/verification-display";

export function VerificationHistoryDialog({
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
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <History className="size-4 text-primary" aria-hidden="true" />
            验证历史
          </DialogTitle>
          <DialogDescription>
            历史结果保留在本地，不会自动删除；结果是否仍有效以任务详情中的状态为准。
          </DialogDescription>
        </DialogHeader>

        {runs.isPending ? (
          <LoadingState rows={4} />
        ) : runs.isError ? (
          <p role="alert" className="text-sm text-destructive">
            {getErrorMessage(runs.error)}
          </p>
        ) : filtered.length === 0 ? (
          <p className="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
            尚无验证运行记录。
          </p>
        ) : (
          <ul className="max-h-[55vh] space-y-2 overflow-y-auto pr-1">
            {filtered.map((run) => {
              const execution = EXECUTION_STATUS_META[run.executionStatus];
              const ExecutionIcon = execution.icon;
              return (
                <li key={run.id} className="rounded-lg border bg-card p-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <span
                      className={`inline-flex items-center gap-1 text-xs font-medium ${execution.className}`}
                    >
                      <ExecutionIcon
                        className={`size-3.5 ${run.executionStatus === "running" ? "animate-spin" : ""}`}
                        aria-hidden="true"
                      />
                      {execution.label}
                    </span>
                    {run.result ? <VerificationBadge status={run.result} /> : null}
                    <span className="text-xs text-muted-foreground">
                      {formatDateTime(run.startedAt)}
                    </span>
                    <span className="font-mono text-[11px] text-muted-foreground">
                      r{run.taskRevision}/{run.criterionRevision}/{run.specRevision}
                    </span>
                    <span className="ml-auto text-xs text-muted-foreground">
                      退出码 {run.exitCode ?? "—"} · {formatDuration(run.durationMs)}
                    </span>
                  </div>
                  <p
                    className="mt-1 truncate font-mono text-[11px] text-muted-foreground"
                    title={formatCommand(run.executable, run.args)}
                  >
                    {formatCommand(run.executable, run.args)}
                  </p>
                  <div className="mt-2 flex flex-wrap items-center gap-2">
                    {run.reason ? (
                      <p className="min-w-0 flex-1 break-words text-xs leading-5 text-muted-foreground">
                        {run.reason}
                      </p>
                    ) : null}
                    {run.evidenceId ? (
                      <button
                        type="button"
                        className="inline-flex items-center gap-1 rounded-sm text-xs font-medium text-primary underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                        onClick={() => onViewEvidence(run.evidenceId as string)}
                      >
                        <FileSearch className="size-3.5" aria-hidden="true" />
                        查看证据
                      </button>
                    ) : null}
                  </div>
                  {run.sensitiveFlags.length > 0 ? (
                    <div className="mt-2 flex flex-wrap gap-1">
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
      </DialogContent>
    </Dialog>
  );
}
