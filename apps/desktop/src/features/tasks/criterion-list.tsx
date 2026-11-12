import { CheckCircle2, Circle, ListChecks, Pencil, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { EmptyState } from "@/components/empty-state";
import { SectionHeader } from "@/components/section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ConfirmManualDialog } from "@/features/tasks/confirm-manual-dialog";
import { CriterionDialog } from "@/features/tasks/criterion-dialog";
import { useRemoveCriterion } from "@/features/tasks/queries";
import { CriterionVerification } from "@/features/verification/criterion-verification";
import type { AcceptanceCriterion, CriterionView, TaskDetail } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";

export function CriterionList({ detail, readOnly }: { detail: TaskDetail; readOnly: boolean }) {
  const removeCriterion = useRemoveCriterion();
  const [addOpen, setAddOpen] = useState(false);
  const [editing, setEditing] = useState<AcceptanceCriterion | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<CriterionView | null>(null);
  const [confirmTarget, setConfirmTarget] = useState<CriterionView | null>(null);

  const task = detail.task;
  const canConfirm = !readOnly && task.status === "awaiting_verification";

  const deleteCriterion = async () => {
    if (!deleteTarget) {
      return;
    }
    try {
      await removeCriterion.mutateAsync({
        criterionId: deleteTarget.criterion.id,
        expectedTaskRevision: task.revision,
        expectedCriterionRevision: deleteTarget.criterion.revision,
      });
      toast.success("验收条件已删除");
      setDeleteTarget(null);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <section className="space-y-3">
      <SectionHeader
        title="验收条件"
        meta={`${detail.criteria.length} 项`}
        actions={
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() => setAddOpen(true)}
            disabled={readOnly}
          >
            <Plus aria-hidden="true" />
            添加条件
          </Button>
        }
      />

      {detail.criteria.length === 0 ? (
        <EmptyState
          icon={ListChecks}
          title="尚无验收条件"
          description="至少添加一个必需验收条件后，任务才能进入就绪并最终完成。"
          action={
            <Button type="button" size="sm" onClick={() => setAddOpen(true)} disabled={readOnly}>
              <Plus aria-hidden="true" />
              添加验收条件
            </Button>
          }
        />
      ) : (
        <ul className="divide-y divide-line/70 border-y border-line/70">
          {detail.criteria.map((view) => {
            const criterion = view.criterion;
            const manual = criterion.kind === "manual";

            return (
              <li key={criterion.id} className="py-3.5">
                <div className="flex gap-3">
                  <span className="mt-0.5 shrink-0" aria-hidden="true">
                    {view.satisfied ? (
                      <CheckCircle2 className="size-4 text-success" />
                    ) : (
                      <Circle className="size-4 text-muted-foreground" />
                    )}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="break-words text-[13px] font-medium">{criterion.description}</p>
                      <Badge variant={manual ? "outline" : "info"}>
                        {manual ? "人工" : "自动"}
                      </Badge>
                      <Badge variant={criterion.required ? "warning" : "secondary"}>
                        {criterion.required ? "必需" : "可选"}
                      </Badge>
                      <span className="font-mono text-[10px] text-muted-foreground">
                        r{criterion.revision}
                      </span>
                    </div>

                    <div className="mt-1 text-[11px] leading-5 text-muted-foreground">
                      {view.satisfied && view.confirmation ? (
                        <p>
                          已确认 ·{" "}
                          <time dateTime={view.confirmation.confirmedAt}>
                            {formatDateTime(view.confirmation.confirmedAt)}
                          </time>
                          {view.confirmation.note ? ` · ${view.confirmation.note}` : ""}
                        </p>
                      ) : criterion.kind === "auto" ? (
                        <p>自动验收由你批准后运行的真实验证命令判定。</p>
                      ) : (
                        <p>{canConfirm ? "等待你人工确认。" : "处于待验证状态时才能人工确认。"}</p>
                      )}
                    </div>

                    {criterion.kind === "auto" ? (
                      <CriterionVerification task={task} view={view} readOnly={readOnly} />
                    ) : null}

                    <div
                      className={cn(
                        "mt-2 flex flex-wrap gap-1",
                        manual && !view.satisfied && canConfirm ? "mb-1" : null,
                      )}
                    >
                      {manual && !view.satisfied && canConfirm ? (
                        <Button type="button" size="sm" onClick={() => setConfirmTarget(view)}>
                          <CheckCircle2 aria-hidden="true" />
                          确认人工验收
                        </Button>
                      ) : null}
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        onClick={() => setEditing(criterion)}
                        disabled={readOnly}
                      >
                        <Pencil aria-hidden="true" />
                        编辑
                      </Button>
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        className="text-destructive hover:text-destructive"
                        onClick={() => setDeleteTarget(view)}
                        disabled={readOnly}
                      >
                        <Trash2 aria-hidden="true" />
                        删除
                      </Button>
                    </div>
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      {addOpen ? <CriterionDialog open onOpenChange={setAddOpen} task={task} /> : null}
      {editing ? (
        <CriterionDialog
          open
          onOpenChange={(open) => {
            if (!open) {
              setEditing(null);
            }
          }}
          task={task}
          criterion={editing}
        />
      ) : null}
      {confirmTarget ? (
        <ConfirmManualDialog
          open
          onOpenChange={(open) => {
            if (!open) {
              setConfirmTarget(null);
            }
          }}
          task={task}
          criterionView={confirmTarget}
        />
      ) : null}
      <ConfirmationDialog
        open={Boolean(deleteTarget)}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteTarget(null);
          }
        }}
        title="删除验收条件"
        description="删除后会同时移除该条件关联的人工确认记录。此操作不可撤销。"
        confirmLabel="删除条件"
        destructive
        pending={removeCriterion.isPending}
        onConfirm={() => void deleteCriterion()}
      />
    </section>
  );
}
