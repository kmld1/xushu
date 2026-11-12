import { useQueryClient } from "@tanstack/react-query";
import { Ban, FileSearch, History, LoaderCircle, Play, Settings2, ShieldCheck } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { VerificationBadge } from "@/components/status-badge";
import { CommandBlock } from "@/features/verification/command-block";
import { EvidenceSheet } from "@/features/verification/evidence-sheet";
import { VerificationApprovalDialog } from "@/features/verification/verification-approval-dialog";
import { VerificationConfigDialog } from "@/features/verification/verification-config-dialog";
import { VerificationHistorySheet } from "@/features/verification/verification-history-sheet";
import {
  useCancelVerification,
  useVerificationRun,
  verificationQueryKeys,
} from "@/features/verification/queries";
import { taskQueryKeys } from "@/features/tasks/queries";
import { formatDateTime, formatDuration } from "@/lib/format";
import type { CriterionView, Task } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";
import { EXECUTION_STATUS_META } from "@/lib/verification-display";
import { cn } from "@/lib/utils";

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
    <div className="mt-3 space-y-3 border-t border-line/60 pt-3">
      <div className="flex flex-wrap items-center gap-2">
        <VerificationBadge status={status} />
        {execution && ExecutionIcon ? (
          <span className={cn("inline-flex items-center gap-1 text-[11px]", execution.className)}>
            <ExecutionIcon
              className={cn("size-3.5", running && "animate-spin")}
              aria-hidden="true"
            />
            {execution.label}
          </span>
        ) : null}
        {spec ? (
          <span className="font-mono text-[10px] text-muted-foreground">配置 r{spec.revision}</span>
        ) : null}
        {latestRun ? (
          <span className="text-[11px] text-muted-foreground">
            {formatDateTime(latestRun.startedAt)}
            {latestRun.exitCode !== null ? ` · 退出码 ${latestRun.exitCode}` : ""}
            {latestRun.durationMs !== null ? ` · ${formatDuration(latestRun.durationMs)}` : ""}
          </span>
        ) : null}
      </div>

      {spec ? (
        <CommandBlock
          executable={spec.executable}
          args={spec.args}
          cwd={spec.cwd}
          label="验证命令"
        />
      ) : (
        <p className="border-l-2 border-line-strong bg-surface-raised/40 px-3 py-2 text-[11px] leading-5 text-muted-foreground">
          尚未配置验证命令。配置后才能运行自动验收。
        </p>
      )}

      {verification.staleReason ? (
        <p className="text-[11px] leading-5 text-verification-stale">
          过期原因：{verification.staleReason}
        </p>
      ) : verification.detail ? (
        <p className="text-[11px] leading-5 text-muted-foreground">{verification.detail}</p>
      ) : null}

      {running ? (
        <div className="flex flex-wrap items-center gap-2 border-l-2 border-primary bg-accent/35 px-3 py-2">
          <LoaderCircle className="size-4 animate-spin text-primary" aria-hidden="true" />
          <span className="text-[11px] text-primary">验证在后台运行，结束后写入证据。</span>
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
        <p className="text-[11px] leading-5 text-muted-foreground">
          任务进入待验证状态后才能启动验证。
        </p>
      ) : null}

      <div className="flex flex-wrap items-center gap-1">
        {canConfigure ? (
          <Button type="button" size="sm" variant="ghost" onClick={() => setConfigOpen(true)}>
            <Settings2 aria-hidden="true" />
            {spec ? "编辑配置" : "配置验证"}
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
          <span className="ml-auto inline-flex items-center gap-1 text-[11px] text-verification-pass">
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

      <VerificationHistorySheet
        open={historyOpen}
        onOpenChange={setHistoryOpen}
        taskId={task.id}
        criterionId={view.criterion.id}
        onViewEvidence={(id) => {
          setHistoryOpen(false);
          setEvidenceId(id);
        }}
      />

      <EvidenceSheet
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
