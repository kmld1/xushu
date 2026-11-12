import { Activity, Loader2, RefreshCw } from "lucide-react";
import { toast } from "sonner";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
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
import type { ClaudeToolCallSummary, Project } from "@/lib/domain";
import { formatDateTime, formatDuration } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

import { ClaudeSessionState } from "./claude-badges";
import { ClaudeEventText } from "./claude-event-line";
import { failureKindLabel, startSourceLabel, toolStatusLabel } from "./claude-display";
import { useClaudeActivity, useRefreshClaudeEvents } from "./queries";

export function ClaudeActivitySheet({
  project,
  open,
  onOpenChange,
  restoreFocus,
}: {
  project: Project;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  restoreFocus: () => void;
}) {
  const activity = useClaudeActivity(project.id, open);
  const refresh = useRefreshClaudeEvents(project.id);

  async function handleRefresh() {
    try {
      const report = await refresh.mutateAsync();
      toast.success("已刷新 Claude Code 事件", {
        description:
          report.imported > 0
            ? `新导入 ${report.imported} 条，重复 ${report.duplicates} 条。`
            : report.ignored > 0
              ? "接入已停用，新事件不会导入。"
              : "没有新的待导入事件。",
      });
      await activity.refetch();
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  }

  const unfinished =
    activity.data?.toolCalls.filter(
      (call) => call.status === "running" || call.status === "unknown",
    ) ?? [];

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocus();
        }}
      >
        <SheetHeader>
          <SheetTitle>Claude Code 活动</SheetTitle>
          <SheetDescription>
            {project.name} · 仅展示已授权的最小事件元数据；观察到工具成功不等于验收通过。
          </SheetDescription>
        </SheetHeader>
        <SheetBody
          role="region"
          aria-label="Claude Code 活动内容"
          tabIndex={0}
          className="space-y-5 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
        >
          {activity.isPending ? (
            <LoadingState rows={5} label="正在读取 Claude Code 活动" />
          ) : activity.isError ? (
            <ErrorState
              title="无法读取 Claude Code 活动"
              message={getErrorMessage(activity.error)}
              onRetry={() => void activity.refetch()}
            />
          ) : activity.data && activity.data.events.length === 0 ? (
            <EmptyState
              icon={Activity}
              title="尚未收到 Claude Code 事件"
              description="启用接入后，在项目目录运行 Claude Code；事件会先写入本地收件箱，再在应用打开时导入。"
            />
          ) : activity.data ? (
            <>
              {activity.data.pendingEventCount > 0 ? (
                <p className="text-xs text-warning">
                  本地收件箱还有 {activity.data.pendingEventCount} 条事件待导入。
                </p>
              ) : null}

              {unfinished.length > 0 ? (
                <section aria-label="未结束的工具调用" className="space-y-2">
                  <div className="flex items-baseline justify-between gap-2">
                    <h3 className="text-[13px] font-semibold">未结束或未确认</h3>
                    <span className="text-[11px] text-muted-foreground">
                      {unfinished.length} 项
                    </span>
                  </div>
                  <div className="divide-y divide-line/60 border-y border-line/70">
                    {unfinished.slice(0, 6).map((call) => (
                      <UnfinishedToolRow key={call.id} call={call} />
                    ))}
                  </div>
                </section>
              ) : null}

              <section aria-label="最近的会话" className="space-y-2">
                <div className="flex items-baseline justify-between gap-2">
                  <h3 className="text-[13px] font-semibold">最近的会话</h3>
                  <span className="text-[11px] text-muted-foreground">
                    {activity.data.sessions.length} 个
                  </span>
                </div>
                <div className="divide-y divide-line/60 border-y border-line/70">
                  {activity.data.sessions.map((session) => (
                    <div key={session.id} className="py-2.5">
                      <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                        <span className="font-mono text-[11px] text-foreground">
                          {session.sessionRef}
                        </span>
                        <ClaudeSessionState state={session.activityState} />
                        <span className="ml-auto font-mono text-[11px] tabular-nums text-muted-foreground">
                          {formatDateTime(session.lastEventAt)}
                        </span>
                      </div>
                      <p className="mt-0.5 text-[11px] text-muted-foreground">
                        {startSourceLabel(session.startSource)} · {session.eventCount} 事件 ·{" "}
                        {session.toolCallCount} 次工具调用
                        {session.runningToolCount > 0
                          ? ` · ${session.runningToolCount} 个未结束`
                          : ""}
                        {session.failedToolCount > 0 ? ` · ${session.failedToolCount} 个失败` : ""}
                        {session.unknownToolCount > 0
                          ? ` · ${session.unknownToolCount} 个未确认`
                          : ""}
                      </p>
                    </div>
                  ))}
                </div>
              </section>

              <section aria-label="Claude Code 事件时间线" className="space-y-2">
                <div className="flex items-baseline justify-between gap-2">
                  <h3 className="text-[13px] font-semibold">事件时间线</h3>
                  <span className="text-[11px] text-muted-foreground">
                    最近 {activity.data.events.length} 条
                  </span>
                </div>
                <ol className="border-y border-line/70">
                  {activity.data.events.map((event, index) => (
                    <li key={event.id} className="grid grid-cols-[16px_minmax(0,1fr)] gap-3 py-2.5">
                      <div className="relative flex justify-center">
                        {index < activity.data.events.length - 1 ? (
                          <span
                            className="absolute bottom-[-11px] top-3 w-px bg-line/70"
                            aria-hidden="true"
                          />
                        ) : null}
                        <span
                          className="mt-1.5 size-1.5 rounded-full bg-line-strong"
                          aria-hidden="true"
                        />
                      </div>
                      <ClaudeEventText event={event} />
                    </li>
                  ))}
                </ol>
              </section>

              <p className="text-[11px] leading-5 text-muted-foreground">
                XUSHU 不会读取 transcript、工具输入输出或会话正文。事件顺序按接收时间排列；
                缺失的结束事件会显示为“状态未知”，不会被当作成功。
              </p>
            </>
          ) : null}
        </SheetBody>
        <SheetFooter>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void handleRefresh()}
            disabled={refresh.isPending}
          >
            {refresh.isPending ? (
              <Loader2 className="animate-spin" aria-hidden="true" />
            ) : (
              <RefreshCw aria-hidden="true" />
            )}
            刷新
          </Button>
          <SheetClose asChild>
            <Button type="button" size="sm">
              关闭
            </Button>
          </SheetClose>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}

function UnfinishedToolRow({ call }: { call: ClaudeToolCallSummary }) {
  return (
    <div className="py-2.5">
      <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
        <span className="max-w-full wrap-anywhere text-[13px] font-medium">{call.toolName}</span>
        <span className="text-[11px] text-muted-foreground">{toolStatusLabel(call.status)}</span>
        <span className="ml-auto font-mono text-[11px] tabular-nums text-muted-foreground">
          {call.startedAt ? formatDateTime(call.startedAt) : "开始时间未知"}
        </span>
      </div>
      <p className="mt-0.5 text-[11px] text-muted-foreground">
        会话 {call.sessionRef} · {call.startObserved ? "已观察开始" : "未观察开始"} ·{" "}
        {call.finishObserved ? "已观察结束" : "未观察结束"}
        {call.conflict ? " · 事件冲突" : ""}
        {call.failureKind ? ` · ${failureKindLabel(call.failureKind)}` : ""}
        {call.durationMs !== null ? ` · ${formatDuration(call.durationMs)}` : ""}
      </p>
    </div>
  );
}
