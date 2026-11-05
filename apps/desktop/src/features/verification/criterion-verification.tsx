import { useQueryClient } from "@tanstack/react-query";
import {
  Ban,
  FileSearch,
  History,
  LoaderCircle,
  Play,
  Settings2,
  ShieldCheck,
  TerminalSquare,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { VerificationBadge } from "@/components/status-badge";
import { EvidenceDialog } from "@/features/verification/evidence-dialog";
import { VerificationApprovalDialog } from "@/features/verification/verification-approval-dialog";
import { VerificationConfigDialog } from "@/features/verification/verification-config-dialog";
import { VerificationHistoryDialog } from "@/features/verification/verification-history-dialog";
import {
  useCancelVerification,
  useVerificationRun,
  verificationQueryKeys,
} from "@/features/verification/queries";
import { taskQueryKeys } from "@/features/tasks/queries";
import { formatDateTime, formatDuration } from "@/lib/format";
import type { CriterionView, Task } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";
import { EXECUTION_STATUS_META, formatCommand } from "@/lib/verification-display";

export function CriterionVerification({
  task,
  view,
  readOnly,
}: {
  task: Task;
  view: CriterionView;
  readOnly: boolean;
}) {
  const queryClient = useQueryClient();
  const verification = view.verification;
  const spec = verification?.spec ?? null;
  const latestRun = verification?.latestRun ?? null;
  const latestCheck = verification?.latestCheck ?? null;
  const [configOpen, setConfigOpen] = useState(false);
  const [approvalOpen, setApprovalOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [evidenceId, setEvidenceId] = useState<string | null>(null);
  const [startedRunId, setStartedRunId] = useState<string | null>(null);

  const activeRunId =
    startedRunId ?? (latestRun?.executionStatus === "running" ? latestRun.id : null);
  const liveRun = useVerificationRun(activeRunId);
  const cancelVerification = useCancelVerification();
  const previousStatus = useRef<string | null>(null);

  const executionStatus = liveRun.data?.executionStatus ?? latestRun?.executionStatus ?? null;
  const running = Boolean(startedRunId) || executionStatus === "running";

  useEffect(() => {
    const status = liveRun.data?.executionStatus ?? null;
    if (previousStatus.current === "running" && status && status !== "running") {
      void queryClient.invalidateQueries({ queryKey: taskQueryKeys.all });
      void queryClient.invalidateQueries({ queryKey: verificationQueryKeys.queue });
      setStartedRunId(null);
    }
    previousStatus.current = status;
  }, [liveRun.data?.executionStatus, queryClient]);

  if (!verification) {
    return null;
  }

  const status = verification.status;
  const canConfigure = !readOnly;
  const canRun = !readOnly && task.status === "awaiting_verification" && spec !== null && !running;
  const evidenceIdForLatest = latestCheck?.evidenceId ?? latestRun?.evidenceId ?? null;
  const execution = executionStatus ? EXECUTION_STATUS_META[executionStatus] : null;
  const ExecutionIcon = execution?.icon;

  const cancel = async () => {
    if (!activeRunId) {
      return;
    }
    try {
      await cancelVerification.mutateAsync(activeRunId);
      toast.success("已请求取消验证");
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <div className="mt-3 space-y-3 border-t pt-3">
      <div className="flex flex-wrap items-center gap-2">
        <VerificationBadge status={status} />
        {execution && ExecutionIcon ? (
          <span className={`inline-flex items-center gap-1 text-xs ${execution.className}`}>
            <ExecutionIcon
              className={`size-3.5 ${running ? "animate-spin" : ""}`}
              aria-hidden="true"
            />
            {execution.label}
          </span>
        ) : null}
        {spec ? (
          <span className="font-mono text-[11px] text-muted-foreground">配置 r{spec.revision}</span>
        ) : null}
        {latestRun ? (
          <span className="text-xs text-muted-foreground">
            {formatDateTime(latestRun.startedAt)}
            {latestRun.exitCode !== null ? ` · 退出码 ${latestRun.exitCode}` : ""}
            {latestRun.durationMs !== null ? ` · ${formatDuration(latestRun.durationMs)}` : ""}
          </span>
        ) : null}
      </div>

      {spec ? (
        <div className="rounded-md border bg-surface-raised/40">
          <div className="flex items-center gap-1.5 border-b px-3 py-1.5 text-[11px] font-medium text-muted-foreground">
            <TerminalSquare className="size-3" aria-hidden="true" />
            验证命令
          </div>
          <pre className="overflow-x-auto whitespace-pre-wrap break-all px-3 py-2 font-mono text-xs leading-5">
            {formatCommand(spec.executable, spec.args)}
          </pre>
          <p
            className="truncate border-t px-3 py-1.5 font-mono text-[11px] text-muted-foreground"
            title={spec.cwd}
          >
            cwd: {spec.cwd}
          </p>
        </div>
      ) : (
        <p className="rounded-md border border-dashed px-3 py-2 text-xs leading-5 text-muted-foreground">
          尚未配置验证命令。配置后才能运行自动验收。
        </p>
      )}

      {verification.staleReason ? (
        <p className="text-xs leading-5 text-verification-stale">
          过期原因：{verification.staleReason}
        </p>
      ) : verification.detail ? (
        <p className="text-xs leading-5 text-muted-foreground">{verification.detail}</p>
      ) : null}

      {running ? (
        <div className="flex flex-wrap items-center gap-2 rounded-md border border-primary/35 bg-primary/5 px-3 py-2">
          <LoaderCircle className="size-4 animate-spin text-primary" aria-hidden="true" />
          <span className="text-xs text-primary">验证正在后台运行，输出将在结束后写入证据。</span>
          <Button
            type="button"
            size="sm"
            variant="outline"
            className="ml-auto"
            onClick={() => void cancel()}
            disabled={cancelVerification.isPending}
          >
            <Ban aria-hidden="true" />
            {cancelVerification.isPending ? "取消中…" : "取消运行"}
          </Button>
        </div>
      ) : null}

      {task.status !== "awaiting_verification" && spec && !running ? (
        <p className="text-xs leading-5 text-muted-foreground">
          任务进入待验证状态后才能启动验证。
        </p>
      ) : null}

      <div className="flex flex-wrap gap-2">
        {canConfigure ? (
          <Button type="button" size="sm" variant="ghost" onClick={() => setConfigOpen(true)}>
            <Settings2 aria-hidden="true" />
            {spec ? "编辑验证配置" : "配置验证"}
          </Button>
        ) : null}
        {canRun ? (
          <Button type="button" size="sm" onClick={() => setApprovalOpen(true)}>
            <Play aria-hidden="true" />
            运行验证
          </Button>
        ) : null}
        {evidenceIdForLatest ? (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={() => setEvidenceId(evidenceIdForLatest)}
          >
            <FileSearch aria-hidden="true" />
            查看证据
          </Button>
        ) : null}
        <Button type="button" size="sm" variant="ghost" onClick={() => setHistoryOpen(true)}>
          <History aria-hidden="true" />
          验证历史
        </Button>
        {verification.validPassRunId ? (
          <span className="ml-auto inline-flex items-center gap-1 text-xs text-verification-pass">
            <ShieldCheck className="size-3.5" aria-hidden="true" />
            当前有效 PASS
          </span>
        ) : null}
      </div>

      {configOpen ? (
        <VerificationConfigDialog
          open
          onOpenChange={setConfigOpen}
          task={task}
          criterion={view.criterion}
          spec={spec}
        />
      ) : null}

      {approvalOpen && spec ? (
        <VerificationApprovalDialog
          open
          onOpenChange={setApprovalOpen}
          task={task}
          criterion={view.criterion}
          onStarted={(run) => setStartedRunId(run.id)}
        />
      ) : null}

      <VerificationHistoryDialog
        open={historyOpen}
        onOpenChange={setHistoryOpen}
        taskId={task.id}
        criterionId={view.criterion.id}
        onViewEvidence={(id) => {
          setHistoryOpen(false);
          setEvidenceId(id);
        }}
      />

      <EvidenceDialog
        open={Boolean(evidenceId)}
        onOpenChange={(open) => {
          if (!open) {
            setEvidenceId(null);
          }
        }}
        evidenceId={evidenceId}
      />
    </div>
  );
}
