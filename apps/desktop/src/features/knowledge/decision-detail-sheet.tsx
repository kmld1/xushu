import { ArrowRightLeft, BadgeCheck, Ban, Pencil } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetBody,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { ReasonDialog } from "@/features/tasks/reason-dialog";
import type { DecisionView } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

import { DecisionStatusBadge } from "./knowledge-badges";
import { decisionStatusLabel } from "./knowledge-display";
import { useConfirmDecision, useDecisionDetail, useDeprecateDecision } from "./queries";

type ConfirmAction = "confirm" | "deprecate" | null;

export function DecisionDetailSheet({
  projectId,
  decisionId,
  open,
  onOpenChange,
  readOnly,
  onEdit,
  onSupersede,
  onChanged,
  restoreFocus,
}: {
  projectId: string;
  decisionId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  readOnly: boolean;
  onEdit: (decision: DecisionView) => void;
  onSupersede: (decision: DecisionView) => void;
  onChanged?: () => void;
  restoreFocus: () => void;
}) {
  const detail = useDecisionDetail(projectId, decisionId ?? undefined, open && Boolean(decisionId));
  const confirmDecision = useConfirmDecision();
  const deprecateDecision = useDeprecateDecision();
  const [confirmAction, setConfirmAction] = useState<ConfirmAction>(null);
  const view = detail.data?.decision;
  const decision = view?.decision;

  async function runConfirm() {
    if (!view) {
      return;
    }
    try {
      await confirmDecision.mutateAsync({
        projectId,
        decisionId: view.decision.id,
        expectedRevision: view.decision.revision,
      });
      toast.success("决策已确认", {
        description: view.supersedesTitle
          ? `「${view.supersedesTitle}」已标记为已被取代。`
          : "该决策现在可以进入恢复包。",
      });
      setConfirmAction(null);
      onChanged?.();
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  }

  async function runDeprecate(reason: string) {
    if (!view) {
      return;
    }
    await deprecateDecision.mutateAsync({
      projectId,
      decisionId: view.decision.id,
      expectedRevision: view.decision.revision,
      reason,
    });
    toast.success("决策已废弃", {
      description: "历史与关联关系保留，新的恢复包不再把它作为当前约束。",
    });
    onChanged?.();
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocus();
        }}
      >
        <SheetHeader>
          <SheetTitle>开发决策</SheetTitle>
          <SheetDescription>
            {decision ? decision.title : "读取决策详情"} · 决策记录不会修改任务状态。
          </SheetDescription>
        </SheetHeader>

        <SheetBody
          role="region"
          aria-label="开发决策详情"
          tabIndex={0}
          className="space-y-5 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
        >
          {detail.isPending ? (
            <LoadingState rows={5} label="正在读取决策详情" />
          ) : detail.isError ? (
            <ErrorState
              title="无法读取决策"
              message={getErrorMessage(detail.error)}
              onRetry={() => void detail.refetch()}
            />
          ) : detail.data && decision && view ? (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <DecisionStatusBadge status={decision.status} />
                <span className="font-mono text-[11px] text-muted-foreground">
                  r{decision.revision}
                </span>
                <span className="text-[11px] text-muted-foreground">
                  更新于 {formatDateTime(decision.updatedAt)}
                </span>
              </div>

              {readOnly ? (
                <Alert variant="warning">
                  <AlertTitle>项目已归档</AlertTitle>
                  <AlertDescription>恢复项目后才能确认、废弃或创建取代决策。</AlertDescription>
                </Alert>
              ) : null}

              {decision.status === "draft" ? (
                <Alert variant="info">
                  <AlertTitle>草稿不会进入恢复包</AlertTitle>
                  <AlertDescription>
                    确认后，该决策才会作为当前有效约束进入跨会话恢复上下文。
                  </AlertDescription>
                </Alert>
              ) : null}

              <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
                {view.linkedTask ? (
                  <span>
                    关联任务：
                    <Link
                      to={`/tasks/${view.linkedTask.id}`}
                      className="font-medium text-foreground underline-offset-4 hover:text-primary hover:underline"
                    >
                      {view.linkedTask.title}
                    </Link>
                  </span>
                ) : (
                  <span>未关联任务</span>
                )}
                {view.supersedesTitle ? <span>取代了「{view.supersedesTitle}」</span> : null}
                {view.supersededByTitle ? (
                  <span className="text-warning">已被「{view.supersededByTitle}」取代</span>
                ) : null}
              </div>

              <section className="space-y-3">
                <h3 className="text-[13px] font-semibold">问题</h3>
                <p className="whitespace-pre-wrap break-words text-sm leading-6">
                  {decision.problem}
                </p>
              </section>

              <section className="space-y-3 border-t border-line/70 pt-4">
                <h3 className="text-[13px] font-semibold">背景与约束</h3>
                <p className="whitespace-pre-wrap break-words text-sm leading-6">
                  {decision.context}
                </p>
              </section>

              {decision.alternatives.length > 0 ? (
                <section className="space-y-2 border-t border-line/70 pt-4">
                  <h3 className="text-[13px] font-semibold">备选方案</h3>
                  <ul className="list-disc space-y-1 pl-4 text-sm leading-6">
                    {decision.alternatives.map((alternative) => (
                      <li key={alternative} className="break-words">
                        {alternative}
                      </li>
                    ))}
                  </ul>
                </section>
              ) : null}

              <section className="space-y-2 border-t border-line/70 pt-4">
                <h3 className="text-[13px] font-semibold">最终采用的方案</h3>
                <p className="whitespace-pre-wrap break-words border-l-2 border-primary/60 pl-4 text-sm leading-6">
                  {decision.decision}
                </p>
              </section>

              <section className="space-y-2 border-t border-line/70 pt-4">
                <h3 className="text-[13px] font-semibold">采用原因</h3>
                <p className="whitespace-pre-wrap break-words text-sm leading-6">
                  {decision.rationale}
                </p>
              </section>

              <section className="space-y-2 border-t border-line/70 pt-4">
                <h3 className="text-[13px] font-semibold">已知代价与风险</h3>
                <p className="whitespace-pre-wrap break-words text-sm leading-6">
                  {decision.risks}
                </p>
              </section>

              <section aria-label="决策修订历史" className="space-y-2 border-t border-line/70 pt-4">
                <div className="flex items-baseline justify-between gap-2">
                  <h3 className="text-[13px] font-semibold">修订历史</h3>
                  <span className="text-[11px] text-muted-foreground">
                    {detail.data.revisions.length} 个版本
                  </span>
                </div>
                <div className="border-y border-line/70">
                  {detail.data.revisions.map((revision) => (
                    <details
                      key={revision.id}
                      className="border-b border-line/60 py-2 last:border-0"
                    >
                      <summary className="flex cursor-pointer flex-wrap items-center gap-x-2 gap-y-1 text-xs">
                        <span className="font-mono text-[11px]">r{revision.revision}</span>
                        <span className="min-w-0 flex-1 truncate font-medium">
                          {revision.title}
                        </span>
                        <span className="text-[11px] text-muted-foreground">
                          {decisionStatusLabel(revision.status)} ·{" "}
                          {formatDateTime(revision.changedAt)}
                        </span>
                      </summary>
                      <div className="mt-2 space-y-2 pl-6 text-[11px] text-muted-foreground">
                        {revision.changeNote ? <p>变更：{revision.changeNote}</p> : null}
                        <p className="whitespace-pre-wrap break-words">
                          最终方案：{revision.decision}
                        </p>
                      </div>
                    </details>
                  ))}
                </div>
              </section>
            </>
          ) : null}
        </SheetBody>

        <SheetFooter>
          <SheetClose asChild>
            <Button type="button" variant="outline" size="sm">
              关闭
            </Button>
          </SheetClose>
          {view && decision && !readOnly ? (
            <>
              {decision.status === "draft" ? (
                <>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => onEdit(view)}
                    disabled={confirmDecision.isPending || deprecateDecision.isPending}
                  >
                    <Pencil aria-hidden="true" />
                    编辑草稿
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    onClick={() => setConfirmAction("confirm")}
                    disabled={confirmDecision.isPending || deprecateDecision.isPending}
                  >
                    <BadgeCheck aria-hidden="true" />
                    确认决策
                  </Button>
                </>
              ) : null}
              {decision.status === "confirmed" ? (
                <Button
                  type="button"
                  size="sm"
                  onClick={() => onSupersede(view)}
                  disabled={confirmDecision.isPending || deprecateDecision.isPending}
                >
                  <ArrowRightLeft aria-hidden="true" />
                  创建取代决策
                </Button>
              ) : null}
              {decision.status === "draft" || decision.status === "confirmed" ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => setConfirmAction("deprecate")}
                  disabled={confirmDecision.isPending || deprecateDecision.isPending}
                >
                  <Ban aria-hidden="true" />
                  废弃
                </Button>
              ) : null}
            </>
          ) : null}
        </SheetFooter>
      </SheetContent>

      <ConfirmationDialog
        open={confirmAction === "confirm"}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) {
            setConfirmAction(null);
          }
        }}
        title="确认这条开发决策？"
        description={
          view?.supersedesTitle
            ? `确认后将把「${view.supersedesTitle}」标记为已被取代，并保留双向引用。决策不会修改任务状态。`
            : "确认后，该决策将作为当前有效约束进入恢复包。已确认决策不能原地改写。"
        }
        confirmLabel="确认决策"
        pending={confirmDecision.isPending}
        onConfirm={() => void runConfirm()}
      />
      <ReasonDialog
        open={confirmAction === "deprecate"}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) {
            setConfirmAction(null);
          }
        }}
        title="废弃这条决策"
        description="决策会保留历史与关联关系，但不再作为当前约束进入新的恢复包。"
        confirmLabel="废弃决策"
        pending={deprecateDecision.isPending}
        error={deprecateDecision.error}
        onConfirm={runDeprecate}
      />
    </Sheet>
  );
}
