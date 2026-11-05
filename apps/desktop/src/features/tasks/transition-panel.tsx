import { AlertCircle, Ban, CheckCircle2, ChevronRight, RotateCcw } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { StatusBadge } from "@/components/status-badge";
import { TASK_STATUS_META } from "@/lib/task-display";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useReopenTask, useTransitionTask } from "@/features/tasks/queries";
import { ReasonDialog } from "@/features/tasks/reason-dialog";
import type { TaskDetail, TaskStatus } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

const ACTION_LABELS: Partial<Record<TaskStatus, string>> = {
  ready: "进入就绪",
  running: "开始执行",
  awaiting_verification: "进入待验证",
  done: "标记完成",
  blocked: "标记阻塞",
  cancelled: "取消任务",
};

export function TransitionPanel({
  detail,
  projectArchived = false,
}: {
  detail: TaskDetail;
  projectArchived?: boolean;
}) {
  const transition = useTransitionTask();
  const reopen = useReopenTask();
  const [reasonTarget, setReasonTarget] = useState<TaskStatus | null>(null);
  const [reopenOpen, setReopenOpen] = useState(false);

  const task = detail.task;
  const readOnly = Boolean(task.archivedAt) || projectArchived;
  const pending = transition.isPending || reopen.isPending;

  const runDirectTransition = async (target: TaskStatus) => {
    try {
      await transition.mutateAsync({
        taskId: task.id,
        expectedRevision: task.revision,
        toStatus: target,
        reason: undefined,
      });
      toast.success("任务状态已更新", {
        description: TASK_STATUS_META[target].label,
      });
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  const confirmReasonTransition = async (reason: string) => {
    if (!reasonTarget) {
      return;
    }
    await transition.mutateAsync({
      taskId: task.id,
      expectedRevision: task.revision,
      toStatus: reasonTarget,
      reason,
    });
    toast.success("任务状态已更新", {
      description: TASK_STATUS_META[reasonTarget].label,
    });
    setReasonTarget(null);
  };

  const confirmReopen = async (reason: string) => {
    await reopen.mutateAsync({
      taskId: task.id,
      expectedRevision: task.revision,
      reason,
    });
    toast.success("任务已重新开启");
    setReopenOpen(false);
  };

  if (readOnly) {
    return (
      <Alert>
        <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
        <AlertTitle>只读任务</AlertTitle>
        <AlertDescription>
          {task.archivedAt
            ? "任务已归档，恢复后才能修改状态或验收条件。"
            : "所属项目已归档，恢复项目后才能继续操作任务。"}
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <StatusBadge status={task.status} />
        {detail.allowedTransitions.length === 0 ? (
          <span className="text-xs text-muted-foreground">当前状态没有可用的状态转换。</span>
        ) : null}
      </div>

      {detail.allowedTransitions.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {detail.allowedTransitions.map((target) => {
            const meta = TASK_STATUS_META[target];
            const needsReason = target === "blocked" || target === "cancelled";
            const isDone = target === "done";
            const disabled = pending || (isDone && !detail.completion.canComplete);

            return (
              <Button
                key={target}
                type="button"
                variant={isDone ? "default" : target === "cancelled" ? "destructive" : "outline"}
                size="sm"
                disabled={disabled}
                title={
                  isDone && !detail.completion.canComplete
                    ? detail.completion.blockers.join("；")
                    : undefined
                }
                onClick={() => {
                  if (needsReason) {
                    setReasonTarget(target);
                  } else {
                    void runDirectTransition(target);
                  }
                }}
              >
                {isDone ? (
                  <CheckCircle2 aria-hidden="true" />
                ) : target === "cancelled" ? (
                  <Ban aria-hidden="true" />
                ) : (
                  <ChevronRight aria-hidden="true" />
                )}
                {ACTION_LABELS[target] ?? meta.label}
              </Button>
            );
          })}
        </div>
      ) : null}

      {task.status === "cancelled" ? (
        <Button type="button" variant="outline" size="sm" onClick={() => setReopenOpen(true)}>
          <RotateCcw aria-hidden="true" />
          重新开启任务
        </Button>
      ) : null}

      {task.status === "awaiting_verification" && !detail.completion.canComplete ? (
        <div className="rounded-md border border-warning/35 bg-warning/5 p-3 text-xs leading-5 text-warning">
          <p className="font-medium">完成门禁未满足</p>
          <ul className="mt-1 list-disc space-y-1 pl-4">
            {detail.completion.blockers.map((blocker) => (
              <li key={blocker}>{blocker}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {transition.error ? (
        <Alert variant="destructive">
          <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>状态转换失败</AlertTitle>
          <AlertDescription>{getErrorMessage(transition.error)}</AlertDescription>
        </Alert>
      ) : null}

      {reasonTarget ? (
        <ReasonDialog
          open
          onOpenChange={(open) => {
            if (!open) {
              setReasonTarget(null);
            }
          }}
          title={reasonTarget === "blocked" ? "标记任务为阻塞" : "取消任务"}
          description={
            reasonTarget === "blocked"
              ? "阻塞必须有明确原因；解除阻塞后可继续执行。"
              : "取消是终止状态。如需继续，之后必须通过显式“重新开启”操作。"
          }
          confirmLabel={reasonTarget === "blocked" ? "确认阻塞" : "确认取消"}
          pending={transition.isPending}
          error={transition.error}
          onConfirm={confirmReasonTransition}
        />
      ) : null}

      {reopenOpen ? (
        <ReasonDialog
          open
          onOpenChange={setReopenOpen}
          title="重新开启任务"
          description="任务将回到草稿状态，修订号增加，旧的人工确认失效。"
          confirmLabel="重新开启"
          pending={reopen.isPending}
          error={reopen.error}
          onConfirm={confirmReopen}
        />
      ) : null}
    </div>
  );
}
