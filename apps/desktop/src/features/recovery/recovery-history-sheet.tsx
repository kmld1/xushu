import { History, Loader2 } from "lucide-react";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Badge } from "@/components/ui/badge";
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
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

import { useRecoverySnapshots } from "./queries";
import { shortFingerprint } from "./recovery-display";

export function RecoveryHistorySheet({
  projectId,
  open,
  onOpenChange,
  currentSnapshotId,
  pendingSnapshotId,
  onSelect,
  restoreFocus,
}: {
  projectId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  currentSnapshotId: string | null;
  pendingSnapshotId?: string | null;
  onSelect: (snapshotId: string) => void;
  restoreFocus: () => void;
}) {
  const snapshots = useRecoverySnapshots(projectId, open);

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocus();
        }}
      >
        <SheetHeader>
          <SheetTitle>历史恢复快照</SheetTitle>
          <SheetDescription>
            历史快照只反映生成时状态，不代表当前事实。最近一次生成的快照在恢复页中标记为“当前”。
          </SheetDescription>
        </SheetHeader>
        <SheetBody
          role="region"
          aria-label="历史恢复快照"
          tabIndex={0}
          className="space-y-3 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
        >
          {snapshots.isPending ? (
            <LoadingState rows={5} label="正在读取历史快照" />
          ) : snapshots.isError ? (
            <ErrorState
              title="无法读取历史快照"
              message={getErrorMessage(snapshots.error)}
              onRetry={() => void snapshots.refetch()}
            />
          ) : snapshots.data.length === 0 ? (
            <EmptyState
              icon={History}
              title="还没有历史快照"
              description="生成恢复包后，最近 20 份快照会保留在这里。"
            />
          ) : (
            <ul className="divide-y divide-line/60 border-y border-line/70">
              {snapshots.data.map((snapshot) => (
                <li key={snapshot.id} className="flex min-w-0 flex-wrap items-center gap-2 py-2.5">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-xs font-medium">
                        {formatDateTime(snapshot.generatedAt)}
                      </span>
                      {snapshot.id === currentSnapshotId ? (
                        <Badge variant="success">当前</Badge>
                      ) : (
                        <Badge variant="outline">历史</Badge>
                      )}
                    </div>
                    <p className="mt-0.5 break-all font-mono text-[11px] text-muted-foreground">
                      {shortFingerprint(snapshot.fingerprint)}
                    </p>
                    <p className="mt-0.5 text-[11px] text-muted-foreground">
                      任务 {snapshot.taskCount} · 记忆 {snapshot.memoryCount} · 决策{" "}
                      {snapshot.decisionCount}
                    </p>
                  </div>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => onSelect(snapshot.id)}
                    disabled={snapshot.id === pendingSnapshotId}
                  >
                    {snapshot.id === pendingSnapshotId ? (
                      <Loader2 className="animate-spin" aria-hidden="true" />
                    ) : null}
                    查看
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </SheetBody>
        <SheetFooter>
          <SheetClose asChild>
            <Button type="button" variant="outline" size="sm">
              关闭
            </Button>
          </SheetClose>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
