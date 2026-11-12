import {
  ArrowRight,
  CheckCircle2,
  Circle,
  FileDown,
  FolderGit2,
  LoaderCircle,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router";
import { toast } from "sonner";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { SectionHeader } from "@/components/section";
import { PriorityBadge, StatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
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
          "w-full border-l-2 px-3 py-2.5 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          selected
            ? "border-primary bg-accent/40"
            : "border-transparent hover:border-line-strong hover:bg-hover/60",
        )}
      >
        <div className="flex items-start justify-between gap-2">
          <p className="min-w-0 break-words text-[13px] font-medium leading-5">{item.task.title}</p>
          {item.runningCount > 0 ? (
            <LoaderCircle
              className="mt-0.5 size-3.5 shrink-0 animate-spin text-primary"
              aria-label="验证运行中"
            />
          ) : null}
        </div>
        <p className="mt-0.5 truncate text-[10px] text-muted-foreground">{item.projectName}</p>
        <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px]">
          <span className="text-muted-foreground">
            自动{" "}
            <span className="font-mono text-foreground">
              {item.autoSatisfied}/{item.autoTotal}
            </span>
          </span>
          {item.manualPending > 0 ? (
            <span className="text-warning">人工待确认 {item.manualPending}</span>
          ) : null}
          {item.runningCount > 0 ? <span className="text-primary">运行中</span> : null}
        </div>
      </button>
    </li>
  );
}

function ManualCriterionRow({ view, taskId }: { view: CriterionView; taskId: string }) {
  return (
    <li className="flex items-start gap-3 py-3">
      <span className="mt-0.5 shrink-0" aria-hidden="true">
        {view.satisfied ? (
          <CheckCircle2 className="size-4 text-success" />
        ) : (
          <Circle className="size-4 text-muted-foreground" />
        )}
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <p className="break-words text-[13px] font-medium">{view.criterion.description}</p>
          <span
            className={cn(
              "text-[10px]",
              view.criterion.required ? "text-warning" : "text-muted-foreground",
            )}
          >
            {view.criterion.required ? "必需" : "可选"}
          </span>
          <span className="text-[10px] text-muted-foreground">人工确认</span>
          <span className="font-mono text-[10px] text-muted-foreground">
            r{view.criterion.revision}
          </span>
        </div>
        <p className="mt-1 text-[11px] leading-5 text-muted-foreground">
          {view.confirmation
            ? `确认于 ${formatDateTime(view.confirmation.confirmedAt)}${
                view.confirmation.note ? ` · ${view.confirmation.note}` : ""
              }`
            : "人工条件需要在任务详情中由你确认。"}
        </p>
        {!view.satisfied ? (
          <Button asChild size="sm" variant="ghost" className="mt-1 -ml-2">
            <Link to={`/tasks/${taskId}`}>
              前往任务详情确认
              <ArrowRight aria-hidden="true" />
            </Link>
          </Button>
        ) : null}
      </div>
    </li>
  );
}

export function VerificationPage() {
  const queue = useVerificationQueue();
  const [searchParams, setSearchParams] = useSearchParams();
  const taskParam = searchParams.get("task");
  const [selectedId, setSelectedId] = useState<string | null>(taskParam);
  const selected =
    queue.data?.find((item) => item.task.id === selectedId) ?? queue.data?.[0] ?? null;
  const detail = useTaskDetail(selected?.task.id);
  const exportReport = useExportAcceptanceReport();
  const [report, setReport] = useState<AcceptanceReport | null>(null);

  useEffect(() => {
    if (taskParam) {
      setSelectedId(taskParam);
    }
  }, [taskParam]);

  const select = (id: string) => {
    setSelectedId(id);
    setSearchParams({ task: id }, { replace: true });
  };

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
      <h1 className="sr-only">验收中心</h1>

      {queue.isPending ? (
        <LoadingState rows={5} />
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
        <div className="grid min-w-0 items-start gap-0 xl:grid-cols-[300px_minmax(0,1fr)]">
          <aside className="min-w-0 border-b border-line/70 pb-4 xl:border-b-0 xl:border-r xl:pr-5">
            <SectionHeader title="待验收" meta={`${queue.data?.length ?? 0} 个`} className="mb-1" />
            <ul className="divide-y divide-line/70 border-y border-line/70">
              {(queue.data ?? []).map((item) => (
                <QueueRow
                  key={item.task.id}
                  item={item}
                  selected={selected?.task.id === item.task.id}
                  onSelect={() => select(item.task.id)}
                />
              ))}
            </ul>
            <p className="pt-2 text-[10px] leading-4 text-muted-foreground">
              队列每 3 秒刷新；运行中的验证会显示旋转图标。
            </p>
          </aside>

          {selected ? (
            detail.isPending ? (
              <div className="pt-4 xl:pl-6 xl:pt-0">
                <LoadingState rows={6} />
              </div>
            ) : detail.isError || !detail.data ? (
              <div className="pt-4 xl:pl-6 xl:pt-0">
                <ErrorState
                  title="无法读取任务验收详情"
                  message={detail.isError ? getErrorMessage(detail.error) : "任务不存在"}
                  onRetry={() => void detail.refetch()}
                />
              </div>
            ) : (
              <section className="min-w-0 pt-4 xl:pl-6 xl:pt-0">
                <div className="border-b border-line/70 pb-3">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <h2 className="break-words text-lg font-semibold tracking-[-0.01em]">
                        {detail.data.task.title}
                      </h2>
                      <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-muted-foreground">
                        <span className="inline-flex items-center gap-1">
                          <FolderGit2 className="size-3" aria-hidden="true" />
                          {selected.projectName}
                        </span>
                        <span aria-hidden="true">·</span>
                        <span className="font-mono">r{detail.data.task.revision}</span>
                        <span aria-hidden="true">·</span>
                        <span>更新于 {formatDateTime(detail.data.task.updatedAt)}</span>
                      </div>
                      <div className="mt-2 flex flex-wrap items-center gap-2">
                        <StatusBadge status={detail.data.task.status} />
                        <PriorityBadge priority={detail.data.task.priority} />
                      </div>
                    </div>
                    <div className="flex shrink-0 flex-wrap items-center gap-2">
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => void runExport()}
                        disabled={exportReport.isPending}
                      >
                        <FileDown aria-hidden="true" />
                        {exportReport.isPending ? "生成中…" : "导出报告"}
                      </Button>
                      <Button asChild variant="outline" size="sm">
                        <Link to={`/tasks/${detail.data.task.id}`}>
                          任务详情
                          <ArrowRight aria-hidden="true" />
                        </Link>
                      </Button>
                    </div>
                  </div>
                </div>

                <dl className="grid grid-cols-3 divide-x divide-line/70 border-b border-line/70">
                  <div className="px-3 py-2.5 first:pl-0">
                    <dt className="text-[10px] text-muted-foreground">自动条件</dt>
                    <dd className="mt-0.5 font-mono text-[15px] font-medium">
                      {selected.autoSatisfied}/{selected.autoTotal}
                    </dd>
                  </div>
                  <div className="px-3 py-2.5">
                    <dt className="text-[10px] text-muted-foreground">人工待确认</dt>
                    <dd className="mt-0.5 font-mono text-[15px] font-medium">
                      {selected.manualPending}
                    </dd>
                  </div>
                  <div className="px-3 py-2.5">
                    <dt className="text-[10px] text-muted-foreground">完成门禁</dt>
                    <dd
                      className={cn(
                        "mt-0.5 flex items-center gap-1.5 text-[13px] font-medium",
                        selected.completion.canComplete ? "text-success" : "text-warning",
                      )}
                    >
                      {selected.completion.canComplete ? (
                        <CheckCircle2 className="size-4" aria-hidden="true" />
                      ) : (
                        <ShieldCheck className="size-4" aria-hidden="true" />
                      )}
                      {selected.completion.canComplete ? "可完成" : "尚有阻塞"}
                    </dd>
                  </div>
                </dl>

                {selected.completion.blockers.length > 0 ? (
                  <div className="border-b border-line/70 bg-warning/5 px-3 py-2.5">
                    <ul className="list-disc space-y-0.5 pl-4 text-[11px] leading-5 text-warning">
                      {selected.completion.blockers.map((blocker) => (
                        <li key={blocker}>{blocker}</li>
                      ))}
                    </ul>
                  </div>
                ) : null}

                <div className="pt-4">
                  <SectionHeader title="验收条件" meta={`${detail.data.criteria.length} 项`} />
                  {detail.data.criteria.length === 0 ? (
                    <p className="border-y border-line/70 py-3 text-xs text-muted-foreground">
                      该任务没有验收条件。
                    </p>
                  ) : (
                    <ul className="divide-y divide-line/70 border-y border-line/70">
                      {detail.data.criteria.map((view) =>
                        view.criterion.kind === "auto" ? (
                          <li key={view.criterion.id} className="py-3.5">
                            <div className="flex flex-wrap items-center gap-2">
                              <p className="break-words text-[13px] font-medium">
                                {view.criterion.description}
                              </p>
                              <span
                                className={cn(
                                  "text-[10px]",
                                  view.criterion.required
                                    ? "text-warning"
                                    : "text-muted-foreground",
                                )}
                              >
                                {view.criterion.required ? "必需" : "可选"}
                              </span>
                              <span className="text-[10px] text-muted-foreground">自动验证</span>
                              <span className="font-mono text-[10px] text-muted-foreground">
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
                </div>
              </section>
            )
          ) : null}
        </div>
      )}

      <Sheet
        open={Boolean(report)}
        onOpenChange={(open) => {
          if (!open) {
            setReport(null);
          }
        }}
      >
        <SheetContent>
          <SheetHeader>
            <SheetTitle>验收报告</SheetTitle>
            <SheetDescription>
              {report?.fileName} ·
              报告只记录本地真实验证结果，不代表代码绝对正确；可能包含本地命令、工作目录与脱敏输出，分享前请检查。
            </SheetDescription>
          </SheetHeader>
          <SheetBody>
            <div className="mb-3 flex justify-end">
              <Button type="button" size="sm" variant="outline" onClick={() => void copyReport()}>
                复制 Markdown
              </Button>
            </div>
            <pre className="whitespace-pre-wrap break-words rounded-md border border-code-border bg-code p-4 font-mono text-[11px] leading-5 text-code-foreground">
              {report?.markdown}
            </pre>
          </SheetBody>
        </SheetContent>
      </Sheet>
    </div>
  );
}
