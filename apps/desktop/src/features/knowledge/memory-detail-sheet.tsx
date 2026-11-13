import { Archive, ArchiveRestore, BadgeCheck, Pencil } from "lucide-react";
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
import type { ProjectMemoryView } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

import { MemoryArchivedBadge, MemoryConfirmationBadge } from "./knowledge-badges";
import { MEMORY_CATEGORY_META, MEMORY_SOURCE_LABEL, categoryLabel } from "./knowledge-display";
import { useArchiveMemory, useConfirmMemory, useMemoryDetail, useRestoreMemory } from "./queries";

type ConfirmAction = "confirm" | "archive" | "restore" | null;

export function MemoryDetailSheet({
  projectId,
  memoryId,
  open,
  onOpenChange,
  readOnly,
  onEdit,
  onChanged,
  restoreFocus,
}: {
  projectId: string;
  memoryId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  readOnly: boolean;
  onEdit: (memory: ProjectMemoryView) => void;
  onChanged?: () => void;
  restoreFocus: () => void;
}) {
  const detail = useMemoryDetail(projectId, memoryId ?? undefined, open && Boolean(memoryId));
  const confirmMemory = useConfirmMemory();
  const archiveMemory = useArchiveMemory();
  const restoreMemory = useRestoreMemory();
  const [confirmAction, setConfirmAction] = useState<ConfirmAction>(null);
  const pending = confirmMemory.isPending || archiveMemory.isPending || restoreMemory.isPending;
  const memory = detail.data?.memory.memory;
  const view = detail.data?.memory;
  const CategoryIcon = memory ? MEMORY_CATEGORY_META[memory.category].icon : null;

  async function runAction() {
    if (!view || !confirmAction) {
      return;
    }
    try {
      if (confirmAction === "confirm") {
        await confirmMemory.mutateAsync({
          projectId,
          memoryId: view.memory.id,
          expectedRevision: view.memory.revision,
        });
        toast.success("记忆已确认", {
          description: "该记忆现在可以进入恢复包。",
        });
      } else if (confirmAction === "archive") {
        await archiveMemory.mutateAsync({
          projectId,
          memoryId: view.memory.id,
          expectedRevision: view.memory.revision,
          reason: null,
        });
        toast.success("记忆已归档", {
          description: "默认列表与恢复包不再包含该记忆；历史保留。",
        });
      } else {
        await restoreMemory.mutateAsync({
          projectId,
          memoryId: view.memory.id,
          expectedRevision: view.memory.revision,
          reason: null,
        });
        toast.success("记忆已恢复");
      }
      setConfirmAction(null);
      onChanged?.();
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
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
          <SheetTitle>项目记忆</SheetTitle>
          <SheetDescription>
            {memory ? memory.title : "读取记忆详情"} · 记忆不会改变任务状态或验收门禁。
          </SheetDescription>
        </SheetHeader>

        <SheetBody
          role="region"
          aria-label="项目记忆详情"
          tabIndex={0}
          className="space-y-5 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
        >
          {detail.isPending ? (
            <LoadingState rows={5} label="正在读取记忆详情" />
          ) : detail.isError ? (
            <ErrorState
              title="无法读取记忆"
              message={getErrorMessage(detail.error)}
              onRetry={() => void detail.refetch()}
            />
          ) : detail.data && view && memory && CategoryIcon ? (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
                  <CategoryIcon className="size-3.5" aria-hidden="true" />
                  {categoryLabel(memory.category)}
                </span>
                <MemoryConfirmationBadge status={memory.confirmationStatus} />
                {memory.validityStatus === "archived" ? <MemoryArchivedBadge /> : null}
                <span className="ml-auto font-mono text-[11px] text-muted-foreground">
                  r{memory.revision}
                </span>
              </div>

              {readOnly ? (
                <Alert variant="warning">
                  <AlertTitle>项目已归档</AlertTitle>
                  <AlertDescription>恢复项目后才能编辑、确认或归档记忆。</AlertDescription>
                </Alert>
              ) : null}

              <div className="border-l-2 border-primary/60 pl-4">
                <p className="whitespace-pre-wrap break-words text-sm leading-6">
                  {memory.content}
                </p>
              </div>

              <section aria-label="记忆元数据" className="space-y-2">
                <h3 className="text-[13px] font-semibold">元数据</h3>
                <dl className="divide-y divide-line/60 border-y border-line/70 text-xs">
                  {[
                    ["来源", MEMORY_SOURCE_LABEL[memory.sourceType]],
                    ["创建时间", formatDateTime(memory.createdAt)],
                    ["更新时间", formatDateTime(memory.updatedAt)],
                    ["确认时间", formatDateTime(memory.confirmedAt)],
                    ["有效性", memory.validityStatus === "archived" ? "已归档" : "有效"],
                    ["关联证据", memory.linkedEvidenceId ?? "无"],
                  ].map(([label, value]) => (
                    <div key={label} className="flex items-start justify-between gap-4 py-2">
                      <dt className="shrink-0 text-muted-foreground">{label}</dt>
                      <dd className="min-w-0 break-all text-right font-mono text-[11px]">
                        {value}
                      </dd>
                    </div>
                  ))}
                </dl>
                {view.linkedTask ? (
                  <p className="text-xs text-muted-foreground">
                    关联任务：
                    <Link
                      to={`/tasks/${view.linkedTask.id}`}
                      className="font-medium text-foreground underline-offset-4 hover:text-primary hover:underline"
                    >
                      {view.linkedTask.title}
                    </Link>
                    {view.linkedTask.archived ? "（已归档）" : ""}
                  </p>
                ) : null}
              </section>

              <section aria-label="修订历史" className="space-y-2">
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
                          {formatDateTime(revision.changedAt)}
                        </span>
                      </summary>
                      <div className="mt-2 space-y-2 pl-6 text-[11px] text-muted-foreground">
                        {revision.changeNote ? <p>变更：{revision.changeNote}</p> : null}
                        <p>
                          状态：{revision.confirmationStatus === "confirmed" ? "已确认" : "未确认"}
                          {" · "}
                          {revision.validityStatus === "archived" ? "已归档" : "有效"}
                        </p>
                        <pre className="max-h-56 overflow-auto whitespace-pre-wrap break-words rounded-sm bg-surface-raised p-2 font-mono text-[11px] leading-5 text-foreground">
                          {revision.content}
                        </pre>
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
          {view && !readOnly ? (
            <>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => onEdit(view)}
                disabled={pending || view.memory.validityStatus === "archived"}
              >
                <Pencil aria-hidden="true" />
                编辑
              </Button>
              {view.memory.validityStatus === "active" &&
              view.memory.confirmationStatus === "unconfirmed" ? (
                <Button
                  type="button"
                  size="sm"
                  onClick={() => setConfirmAction("confirm")}
                  disabled={pending}
                >
                  <BadgeCheck aria-hidden="true" />
                  确认记忆
                </Button>
              ) : null}
              {view.memory.validityStatus === "active" ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => setConfirmAction("archive")}
                  disabled={pending}
                >
                  <Archive aria-hidden="true" />
                  归档
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setConfirmAction("restore")}
                  disabled={pending}
                >
                  <ArchiveRestore aria-hidden="true" />
                  恢复
                </Button>
              )}
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
        title="确认这条记忆？"
        description="确认后，该记忆可进入跨会话恢复包。它仍然不会覆盖任务状态或验收证据。"
        confirmLabel="确认记忆"
        pending={confirmMemory.isPending}
        onConfirm={() => void runAction()}
      />
      <ConfirmationDialog
        open={confirmAction === "archive"}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) {
            setConfirmAction(null);
          }
        }}
        title="归档这条记忆？"
        description="归档后默认列表和恢复包不再包含它，历史与关联关系仍然保留，可随时恢复。"
        confirmLabel="归档记忆"
        pending={archiveMemory.isPending}
        onConfirm={() => void runAction()}
      />
      <ConfirmationDialog
        open={confirmAction === "restore"}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) {
            setConfirmAction(null);
          }
        }}
        title="恢复这条记忆？"
        description="恢复后该记忆重新进入有效状态；如果是已确认记忆，会重新进入恢复包。"
        confirmLabel="恢复记忆"
        pending={restoreMemory.isPending}
        onConfirm={() => void runAction()}
      />
    </Sheet>
  );
}
