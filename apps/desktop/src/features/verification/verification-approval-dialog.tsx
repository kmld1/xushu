import { CircleAlert, Play, ShieldCheck, TerminalSquare } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { LoadingState } from "@/components/loading-state";
import { usePreviewVerification, useStartVerification } from "@/features/verification/queries";
import { formatBytes, formatDuration } from "@/lib/format";
import type { AcceptanceCriterion, Task, VerificationRun } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";
import { describeSuccessRule, formatCommand } from "@/lib/verification-display";

export function VerificationApprovalDialog({
  open,
  onOpenChange,
  task,
  criterion,
  onStarted,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  task: Task;
  criterion: AcceptanceCriterion;
  onStarted?: (run: VerificationRun) => void;
}) {
  const preview = usePreviewVerification(criterion.id, open);
  const startVerification = useStartVerification();
  const [approved, setApproved] = useState(false);

  const start = async () => {
    if (!preview.data) {
      return;
    }
    try {
      const run = await startVerification.mutateAsync({
        criterionId: criterion.id,
        expectedTaskRevision: preview.data.taskRevision,
        expectedCriterionRevision: preview.data.criterionRevision,
        expectedSpecRevision: preview.data.specRevision,
        approved: true,
      });
      toast.success("验证已启动", {
        description: "运行在后台进行；可在此查看状态或取消。",
      });
      setApproved(false);
      onOpenChange(false);
      onStarted?.(run);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>批准并运行验证</DialogTitle>
          <DialogDescription>
            {task.title} · {criterion.description}
          </DialogDescription>
        </DialogHeader>

        {preview.isPending ? (
          <LoadingState rows={4} />
        ) : preview.isError || !preview.data ? (
          <Alert variant="destructive">
            <CircleAlert className="mt-0.5 size-4" aria-hidden="true" />
            <AlertTitle>无法准备验证预览</AlertTitle>
            <AlertDescription>
              {preview.isError ? getErrorMessage(preview.error) : "验证配置不可用"}
            </AlertDescription>
          </Alert>
        ) : (
          <div className="space-y-4">
            <div className="rounded-lg border bg-surface-raised/40">
              <div className="flex items-center gap-2 border-b px-4 py-2.5 text-xs font-medium text-muted-foreground">
                <TerminalSquare className="size-3.5" aria-hidden="true" />
                将要执行的命令
              </div>
              <pre className="overflow-x-auto whitespace-pre-wrap break-all p-4 font-mono text-xs leading-6">
                {formatCommand(preview.data.executable, preview.data.args)}
              </pre>
            </div>

            <dl className="grid gap-3 text-sm sm:grid-cols-2">
              <div className="space-y-1">
                <dt className="text-xs text-muted-foreground">工作目录</dt>
                <dd className="break-all font-mono text-xs">{preview.data.cwd}</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-xs text-muted-foreground">解析到的程序</dt>
                <dd className="break-all font-mono text-xs">
                  {preview.data.resolvedExecutable ?? "未解析到（运行将记录为未运行）"}
                </dd>
              </div>
              <div className="space-y-1">
                <dt className="text-xs text-muted-foreground">超时</dt>
                <dd>{formatDuration(preview.data.timeoutSeconds * 1_000)}</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-xs text-muted-foreground">输出上限</dt>
                <dd>{formatBytes(preview.data.outputLimitBytes)} / 每个输出流</dd>
              </div>
              <div className="space-y-1 sm:col-span-2">
                <dt className="text-xs text-muted-foreground">成功规则</dt>
                <dd>{describeSuccessRule(preview.data.successRule)}</dd>
              </div>
            </dl>

            <Alert variant="warning">
              <ShieldCheck className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>执行前确认</AlertTitle>
              <AlertDescription>
                <ul className="list-disc space-y-1 pl-4">
                  {preview.data.warnings.map((warning) => (
                    <li key={warning}>{warning}</li>
                  ))}
                </ul>
              </AlertDescription>
            </Alert>

            <label className="flex items-start gap-2 rounded-md border p-3 text-sm">
              <input
                type="checkbox"
                className="mt-0.5 size-4 accent-primary"
                checked={approved}
                onChange={(event) => setApproved(event.target.checked)}
              />
              <span>我确认该项目可信，并批准 XUSHU 以上述程序、参数和工作目录执行这次验证。</span>
            </label>
          </div>
        )}

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button
            type="button"
            onClick={() => void start()}
            disabled={!approved || !preview.data || startVerification.isPending}
          >
            <Play aria-hidden="true" />
            {startVerification.isPending ? "启动中…" : "批准并运行"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
