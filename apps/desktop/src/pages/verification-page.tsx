import {
  ArrowRight,
  ClipboardCheck,
  FileDown,
  FolderGit2,
  LoaderCircle,
  ShieldCheck,
} from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { PriorityBadge, StatusBadge } from "@/components/status-badge";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useTaskDetail } from "@/features/tasks/queries";
import { CriterionVerification } from "@/features/verification/criterion-verification";
import { useExportAcceptanceReport, useVerificationQueue } from "@/features/verification/queries";
import { formatDateTime } from "@/lib/format";
import type { AcceptanceReport, CriterionView, VerificationQueueItem } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";

function QueueRow({
  item,
  selected,
  onSelect,
}: {
  item: VerificationQueueItem;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <li>
      <button
        type="button"
        aria-current={selected ? "true" : undefined}
        onClick={onSelect}
        className={cn(
          "w-full rounded-md border px-3 py-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          selected
            ? "border-primary/50 bg-accent"
            : "border-transparent hover:border-line hover:bg-surface-raised",
        )}
      >
        <div className="flex items-start justify-between gap-2">
          <p className="min-w-0 break-words text-sm font-medium leading-5">{item.task.title}</p>
          {item.runningCount > 0 ? (
            <LoaderCircle
              className="mt-0.5 size-4 shrink-0 animate-spin text-primary"
              aria-label="验证运行中"
            />
          ) : null}
        </div>
        <p className="mt-1 truncate text-xs text-muted-foreground">{item.projectName}</p>
        <div className="mt-2 flex flex-wrap items-center gap-1.5">
          <Badge variant="outline">
            自动 {item.autoSatisfied}/{item.autoTotal}
          </Badge>
          {item.manualPending > 0 ? (
            <Badge variant="warning">人工待确认 {item.manualPending}</Badge>
          ) : null}
          {item.completion.canComplete ? <Badge variant="success">门禁已满足</Badge> : null}
        </div>
      </button>
    </li>
  );
}

function ManualCriterionRow({ view, taskId }: { view: CriterionView; taskId: string }) {
  return (
    <li className="rounded-lg border bg-card p-4">
      <div className="flex flex-wrap items-center gap-2">
        <p className="break-words text-sm font-medium">{view.criterion.description}</p>
        <Badge variant={view.criterion.required ? "warning" : "secondary"}>
          {view.criterion.required ? "必需" : "可选"}
        </Badge>
        <Badge variant="outline">人工确认</Badge>
        <Badge variant={view.satisfied ? "success" : "secondary"}>
          {view.satisfied ? "已确认" : "待确认"}
        </Badge>
        <span className="font-mono text-[11px] text-muted-foreground">
          r{view.criterion.revision}
        </span>
      </div>
      <p className="mt-2 text-xs leading-5 text-muted-foreground">
        {view.confirmation
          ? `确认于 ${formatDateTime(view.confirmation.confirmedAt)}${view.confirmation.note ? ` · ${view.confirmation.note}` : ""}`
          : "人工条件需要在任务详情中由你确认。"}
      </p>
      {!view.satisfied ? (
        <Button asChild size="sm" variant="ghost" className="mt-2">
          <Link to={`/tasks/${taskId}`}>
            前往任务详情确认
            <ArrowRight aria-hidden="true" />
          </Link>
        </Button>
      ) : null}
    </li>
  );
}

export function VerificationPage() {
  const queue = useVerificationQueue();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected =
    queue.data?.find((item) => item.task.id === selectedId) ?? queue.data?.[0] ?? null;
  const detail = useTaskDetail(selected?.task.id);
  const exportReport = useExportAcceptanceReport();
  const [report, setReport] = useState<AcceptanceReport | null>(null);

  const runExport = async () => {
    if (!selected) {
      return;
    }
    try {
      const result = await exportReport.mutateAsync(selected.task.id);
      setReport(result);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  const copyReport = async () => {
    if (!report) {
      return;
    }
    try {
      await navigator.clipboard.writeText(report.markdown);
      toast.success("报告已复制到剪贴板");
    } catch {
      toast.error("无法复制报告，请手动选择文本");
    }
  };

  return (
    <div className="space-y-5">
      <PageHeader
        title="验收中心"
        description="对待验证任务运行真实命令、检查证据并处理过期结果。"
        actions={
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void runExport()}
            disabled={!selected || exportReport.isPending}
          >
            <FileDown aria-hidden="true" />
            {exportReport.isPending ? "生成中…" : "导出验收报告"}
          </Button>
        }
      />

      {queue.isPending ? (
        <LoadingState rows={4} />
      ) : queue.isError ? (
        <ErrorState
          title="无法读取验收队列"
          message={getErrorMessage(queue.error)}
          onRetry={() => void queue.refetch()}
        />
      ) : (queue.data ?? []).length === 0 ? (
        <EmptyState
          icon={ShieldCheck}
          title="当前没有待验收任务"
          description="任务进入待验证状态后会出现在这里。你也可以先在任务详情中配置自动条件的验证命令。"
          action={
            <Button asChild size="sm" variant="outline">
              <Link to="/tasks">
                查看任务
                <ArrowRight aria-hidden="true" />
              </Link>
            </Button>
          }
        />
      ) : (
        <div className="grid items-start gap-5 xl:grid-cols-[340px_minmax(0,1fr)]">
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="flex items-center gap-2">
                <ClipboardCheck className="size-4 text-primary" aria-hidden="true" />
                待验收任务
                <span className="ml-auto font-mono text-xs text-muted-foreground">
                  {queue.data?.length ?? 0}
                </span>
              </CardTitle>
              <CardDescription>按最近更新时间排序，每 3 秒刷新。</CardDescription>
            </CardHeader>
            <CardContent className="p-2 pt-0">
              <ul className="max-h-[65vh] space-y-1 overflow-y-auto">
                {(queue.data ?? []).map((item) => (
                  <QueueRow
                    key={item.task.id}
                    item={item}
                    selected={selected?.task.id === item.task.id}
                    onSelect={() => setSelectedId(item.task.id)}
                  />
                ))}
              </ul>
            </CardContent>
          </Card>

          {selected ? (
            detail.isPending ? (
              <LoadingState rows={5} />
            ) : detail.isError || !detail.data ? (
              <ErrorState
                title="无法读取任务验收详情"
                message={detail.isError ? getErrorMessage(detail.error) : "任务不存在"}
                onRetry={() => void detail.refetch()}
              />
            ) : (
              <div className="space-y-5">
                <Card>
                  <CardHeader>
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div className="min-w-0">
                        <CardTitle className="break-words">{detail.data.task.title}</CardTitle>
                        <CardDescription className="mt-1 flex flex-wrap items-center gap-2">
                          <span className="inline-flex items-center gap-1">
                            <FolderGit2 className="size-3.5" aria-hidden="true" />
                            {selected.projectName}
                          </span>
                          <span>修订 {detail.data.task.revision}</span>
                          <span>更新于 {formatDateTime(detail.data.task.updatedAt)}</span>
                        </CardDescription>
                      </div>
                      <Button asChild variant="outline" size="sm">
                        <Link to={`/tasks/${detail.data.task.id}`}>
                          打开任务详情
                          <ArrowRight aria-hidden="true" />
                        </Link>
                      </Button>
                    </div>
                    <div className="mt-2 flex flex-wrap items-center gap-2">
                      <StatusBadge status={detail.data.task.status} />
                      <PriorityBadge priority={detail.data.task.priority} />
                    </div>
                  </CardHeader>
                  <CardContent>
                    <dl className="grid gap-3 text-sm sm:grid-cols-3">
                      <div className="rounded-md border bg-surface-raised/40 p-3">
                        <dt className="text-xs text-muted-foreground">自动条件</dt>
                        <dd className="mt-1 font-mono text-lg">
                          {selected.autoSatisfied}/{selected.autoTotal}
                        </dd>
                      </div>
                      <div className="rounded-md border bg-surface-raised/40 p-3">
                        <dt className="text-xs text-muted-foreground">人工待确认</dt>
                        <dd className="mt-1 font-mono text-lg">{selected.manualPending}</dd>
                      </div>
                      <div className="rounded-md border bg-surface-raised/40 p-3">
                        <dt className="text-xs text-muted-foreground">完成门禁</dt>
                        <dd className="mt-1 flex items-center gap-1.5 text-sm font-medium">
                          <ShieldCheck
                            className={
                              selected.completion.canComplete
                                ? "size-4 text-verification-pass"
                                : "size-4 text-warning"
                            }
                            aria-hidden="true"
                          />
                          {selected.completion.canComplete ? "可完成" : "尚有阻塞"}
                        </dd>
                      </div>
                    </dl>
                    {selected.completion.blockers.length > 0 ? (
                      <ul className="mt-3 list-disc space-y-1 pl-5 text-xs leading-5 text-muted-foreground">
                        {selected.completion.blockers.map((blocker) => (
                          <li key={blocker}>{blocker}</li>
                        ))}
                      </ul>
                    ) : null}
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle>验收条件</CardTitle>
                    <CardDescription>
                      自动条件显示命令、当前判定与证据；人工条件仍需在任务详情确认。
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    {detail.data.criteria.length === 0 ? (
                      <p className="text-sm text-muted-foreground">该任务没有验收条件。</p>
                    ) : (
                      <ul className="space-y-3">
                        {detail.data.criteria.map((view) =>
                          view.criterion.kind === "auto" ? (
                            <li key={view.criterion.id} className="rounded-lg border bg-card p-4">
                              <div className="flex flex-wrap items-center gap-2">
                                <p className="break-words text-sm font-medium">
                                  {view.criterion.description}
                                </p>
                                <Badge variant={view.criterion.required ? "warning" : "secondary"}>
                                  {view.criterion.required ? "必需" : "可选"}
                                </Badge>
                                <Badge variant="info">自动验证</Badge>
                                <span className="font-mono text-[11px] text-muted-foreground">
                                  r{view.criterion.revision}
                                </span>
                              </div>
                              <CriterionVerification
                                task={detail.data.task}
                                view={view}
                                readOnly={false}
                              />
                            </li>
                          ) : (
                            <ManualCriterionRow
                              key={view.criterion.id}
                              view={view}
                              taskId={detail.data.task.id}
                            />
                          ),
                        )}
                      </ul>
                    )}
                  </CardContent>
                </Card>
              </div>
            )
          ) : null}
        </div>
      )}

      <Dialog
        open={Boolean(report)}
        onOpenChange={(open) => {
          if (!open) {
            setReport(null);
          }
        }}
      >
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>验收报告已生成</DialogTitle>
            <DialogDescription>
              文件已写入应用数据目录的 exports 文件夹：{report?.fileName}
              。报告只记录本地真实验证结果，不代表代码绝对正确；报告可能包含本地命令、工作目录与脱敏输出，分享前请检查。
            </DialogDescription>
          </DialogHeader>
          <div className="flex justify-end">
            <Button type="button" size="sm" variant="outline" onClick={() => void copyReport()}>
              复制 Markdown
            </Button>
          </div>
          <pre className="max-h-[55vh] overflow-auto whitespace-pre-wrap break-words rounded-md border bg-surface-raised/50 p-4 font-mono text-xs leading-6">
            {report?.markdown}
          </pre>
        </DialogContent>
      </Dialog>
    </div>
  );
}
