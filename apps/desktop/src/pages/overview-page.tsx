import { useQueries } from "@tanstack/react-query";
import { ArrowRight, CheckCircle2, FolderGit2, Plus, ShieldCheck } from "lucide-react";
import { useMemo, useState } from "react";
import { Link } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Section, SectionHeader } from "@/components/section";
import { PriorityBadge, StatusBadge, VerificationBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { ImportProjectDialog } from "@/features/projects/import-project-dialog";
import { ProjectGitSummary } from "@/features/projects/project-git-summary";
import { CreateTaskDialog } from "@/features/tasks/create-task-dialog";
import { getTask } from "@/features/tasks/api";
import { taskQueryKeys, useTasks } from "@/features/tasks/queries";
import { SystemHealthCard } from "@/features/system/system-health-card";
import { useCurrentProject } from "@/features/workspace/use-current-project";
import { SignalRail } from "@/features/workspace/signal-rail";
import type {
  CriterionView,
  Task,
  TaskCounts,
  TaskDetail,
  VerificationRunSummary,
} from "@/lib/domain";
import { formatDateTime, formatDuration } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { VERIFICATION_META } from "@/lib/task-display";
import { EXECUTION_STATUS_META } from "@/lib/verification-display";

type Issue = {
  reason: string;
  action: string;
  to: string;
};

function emptyCounts(total = 0): TaskCounts {
  return {
    total,
    draft: 0,
    ready: 0,
    running: 0,
    awaitingVerification: 0,
    done: 0,
    blocked: 0,
    cancelled: 0,
  };
}

function countTasks(tasks: Task[]): TaskCounts {
  const counts = emptyCounts(tasks.length);
  for (const task of tasks) {
    switch (task.status) {
      case "draft":
        counts.draft += 1;
        break;
      case "ready":
        counts.ready += 1;
        break;
      case "running":
        counts.running += 1;
        break;
      case "awaiting_verification":
        counts.awaitingVerification += 1;
        break;
      case "done":
        counts.done += 1;
        break;
      case "blocked":
        counts.blocked += 1;
        break;
      case "cancelled":
        counts.cancelled += 1;
        break;
    }
  }
  return counts;
}

function issueFor(task: Task, detail: TaskDetail | undefined): Issue {
  if (task.status === "blocked") {
    return {
      reason: task.blockReason ?? "任务已阻塞，需要先解除阻塞。",
      action: "打开任务",
      to: `/tasks/${task.id}`,
    };
  }

  const running = detail?.criteria.find(
    (view) => view.verification?.latestRun?.executionStatus === "running",
  );
  if (running) {
    return {
      reason: `验证运行中：${running.criterion.description}`,
      action: "查看运行",
      to: `/verification?task=${task.id}`,
    };
  }

  const problem = detail?.criteria.find(
    (view) =>
      view.verification && ["stale", "fail", "inconclusive"].includes(view.verification.status),
  );
  if (problem?.verification) {
    return {
      reason: `${VERIFICATION_META[problem.verification.status].label}：${
        problem.verification.staleReason ??
        problem.verification.detail ??
        problem.criterion.description
      }`,
      action: "处理验收",
      to: `/verification?task=${task.id}`,
    };
  }

  if (task.status === "awaiting_verification") {
    return {
      reason: detail?.completion.blockers[0] ?? "等待你确认验收条件。",
      action: "进入验收",
      to: `/verification?task=${task.id}`,
    };
  }

  return {
    reason: "最近更新，尚无阻塞。",
    action: "打开任务",
    to: `/tasks/${task.id}`,
  };
}

function NextAction({
  task,
  detail,
  onCreate,
}: {
  task: Task | null;
  detail: TaskDetail | undefined;
  onCreate: () => void;
}) {
  if (!task) {
    return (
      <section
        aria-label="下一步"
        className="flex flex-wrap items-center justify-between gap-3 border-l-2 border-primary bg-surface-raised px-4 py-3"
      >
        <div>
          <h2 className="text-xs font-medium text-primary">下一步</h2>
          <p className="mt-1 text-lg font-semibold tracking-[-0.01em]">当前项目还没有任务</p>
          <p className="mt-0.5 text-xs text-muted-foreground">
            先定义一个目标与必需验收条件，再开始执行。
          </p>
        </div>
        <Button type="button" size="sm" onClick={onCreate}>
          <Plus aria-hidden="true" />
          新建任务
        </Button>
      </section>
    );
  }

  const issue = issueFor(task, detail);

  return (
    <section
      aria-label="下一步"
      className="grid min-w-0 items-center gap-3 border-l-2 border-primary bg-surface-raised px-4 py-3 min-[600px]:grid-cols-[minmax(0,1fr)_auto]"
    >
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <h2 className="text-xs font-medium text-primary">下一步</h2>
          <StatusBadge status={task.status} />
        </div>
        <p className="mt-1 break-words text-lg font-semibold tracking-[-0.01em]">{task.title}</p>
        <p className="mt-1 max-w-2xl break-words text-xs leading-5 text-muted-foreground">
          {issue.reason}
        </p>
      </div>
      <Button asChild size="sm" className="justify-self-start">
        <Link to={issue.to}>{issue.action}</Link>
      </Button>
    </section>
  );
}

function AttentionRow({ task, detail }: { task: Task; detail: TaskDetail | undefined }) {
  const issue = issueFor(task, detail);

  return (
    <li className="flex flex-wrap items-start gap-x-3 gap-y-1 px-2 py-2 transition-colors duration-120 hover:bg-hover/70 focus-within:bg-hover/70">
      <div className="min-w-0 flex-1">
        <Link
          to={`/tasks/${task.id}`}
          className="block truncate text-[13px] font-medium underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          title={task.title}
        >
          {task.title}
        </Link>
        <p className="mt-1 break-words text-xs leading-5 text-muted-foreground">{issue.reason}</p>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <StatusBadge status={task.status} appearance="inline" />
        <Button asChild variant="ghost" size="sm">
          <Link to={issue.to}>{issue.action}</Link>
        </Button>
      </div>
    </li>
  );
}

function RecentTaskRow({ task }: { task: Task }) {
  return (
    <li className="flex min-h-10 items-center gap-3 px-2 py-2 transition-colors duration-120 hover:bg-hover/70 focus-within:bg-hover/70">
      <Link
        to={`/tasks/${task.id}`}
        className="min-w-0 flex-1 truncate text-[13px] font-medium underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        title={task.title}
      >
        {task.title}
      </Link>
      <StatusBadge status={task.status} appearance="inline" className="shrink-0" />
      <PriorityBadge priority={task.priority} className="hidden sm:inline-flex" />
      <time
        dateTime={task.updatedAt}
        className="hidden shrink-0 text-xs tabular-nums text-muted-foreground md:inline"
      >
        {formatDateTime(task.updatedAt)}
      </time>
    </li>
  );
}

function VerificationActivityRow({
  task,
  view,
  run,
}: {
  task: Task;
  view: CriterionView;
  run: VerificationRunSummary;
}) {
  const running = run.executionStatus === "running";
  const execution = EXECUTION_STATUS_META[run.executionStatus];
  const ExecutionIcon = execution.icon;

  return (
    <li className="grid min-w-0 gap-x-4 gap-y-1 px-2 py-2.5 transition-colors duration-120 hover:bg-hover/70 focus-within:bg-hover/70 sm:grid-cols-[minmax(0,1fr)_auto]">
      <div className="flex min-w-0 items-center gap-2">
        {running || !run.result ? (
          <span
            className={`inline-flex shrink-0 items-center gap-1 text-xs ${execution.className}`}
          >
            <ExecutionIcon
              className={`size-3.5 ${running ? "animate-spin" : ""}`}
              aria-hidden="true"
            />
            {execution.label}
          </span>
        ) : run.result ? (
          <VerificationBadge status={run.result} className="shrink-0" />
        ) : null}
        <Link
          to={`/tasks/${task.id}`}
          className="min-w-0 flex-1 truncate text-sm font-medium underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          title={view.criterion.description}
        >
          {view.criterion.description}
        </Link>
      </div>
      <div className="flex items-center gap-3 font-mono text-xs tabular-nums text-muted-foreground">
        <span>退出码 {run.exitCode ?? "—"}</span>
        <span>{formatDuration(run.durationMs)}</span>
      </div>
      <div className="flex min-w-0 flex-wrap items-baseline gap-x-3 gap-y-1 text-xs text-muted-foreground sm:col-span-2">
        <span className="min-w-0 truncate" title={task.title}>
          {task.title}
        </span>
        <time dateTime={run.startedAt} className="shrink-0 tabular-nums">
          {formatDateTime(run.startedAt)}
        </time>
      </div>
    </li>
  );
}

export function OverviewPage() {
  const {
    activeProjects,
    currentProject,
    isPending: projectsPending,
    isError: projectsError,
    error: projectsErrorValue,
  } = useCurrentProject();
  const [importOpen, setImportOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);

  const tasksQuery = useTasks(
    { projectId: currentProject?.id ?? null, includeArchived: false },
    Boolean(currentProject),
  );
  const tasks = useMemo(() => tasksQuery.data ?? [], [tasksQuery.data]);
  const counts = useMemo(() => countTasks(tasks), [tasks]);
  const attentionTasks = useMemo(() => {
    const priority = (task: Task) => (task.status === "blocked" ? 0 : 1);
    return tasks
      .filter((task) => task.status === "blocked" || task.status === "awaiting_verification")
      .sort(
        (left, right) =>
          priority(left) - priority(right) || right.updatedAt.localeCompare(left.updatedAt),
      )
      .slice(0, 4);
  }, [tasks]);
  const recentTasks = useMemo(() => tasks.slice(0, 6), [tasks]);

  const detailIds = useMemo(() => {
    const ids = [
      ...attentionTasks.slice(0, 3).map((task) => task.id),
      ...recentTasks.slice(0, 3).map((task) => task.id),
    ];
    return Array.from(new Set(ids)).slice(0, 5);
  }, [attentionTasks, recentTasks]);

  const detailQueries = useQueries({
    queries: detailIds.map((id) => ({
      queryKey: taskQueryKeys.detail(id),
      queryFn: () => getTask(id),
      staleTime: 5_000,
      enabled: Boolean(currentProject),
    })),
  });

  const detailById = useMemo(() => {
    const map = new Map<string, TaskDetail>();
    detailQueries.forEach((query, index) => {
      if (query.data) {
        map.set(detailIds[index], query.data);
      }
    });
    return map;
  }, [detailQueries, detailIds]);

  const verificationActivity = useMemo(() => {
    const events: Array<{ task: Task; view: CriterionView; run: VerificationRunSummary }> = [];
    for (const id of detailIds) {
      const detail = detailById.get(id);
      if (!detail) {
        continue;
      }
      for (const view of detail.criteria) {
        const run = view.verification?.latestRun;
        if (run) {
          events.push({ task: detail.task, view, run });
        }
      }
    }
    return events
      .sort((left, right) => right.run.startedAt.localeCompare(left.run.startedAt))
      .slice(0, 3);
  }, [detailById, detailIds]);

  const focusTask = attentionTasks[0] ?? recentTasks[0] ?? null;
  const focusDetail = focusTask ? detailById.get(focusTask.id) : undefined;
  const remainingAttentionTasks = attentionTasks.filter((task) => task.id !== focusTask?.id);
  const focusNeedsAttention = attentionTasks.some((task) => task.id === focusTask?.id);

  if (projectsPending) {
    return (
      <div className="space-y-5">
        <h1 className="sr-only">工作台</h1>
        <LoadingState rows={4} />
      </div>
    );
  }

  if (projectsError) {
    return (
      <div className="space-y-5">
        <h1 className="sr-only">工作台</h1>
        <ErrorState
          title="无法读取项目"
          message={getErrorMessage(projectsErrorValue)}
          onRetry={() => window.location.reload()}
        />
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <h1 className="sr-only">工作台</h1>

      {activeProjects.length === 0 ? (
        <div className="space-y-4">
          <EmptyState
            icon={FolderGit2}
            title="还没有可管理的项目"
            description="导入一个本地 Git 工作树，定义任务与验收条件，再由你批准运行真实验证。XUSHU 只读 Git。"
            action={
              <Button type="button" onClick={() => setImportOpen(true)}>
                <FolderGit2 aria-hidden="true" />
                导入第一个项目
              </Button>
            }
          />
          <ol className="mx-auto flex max-w-2xl flex-wrap items-center justify-center gap-x-2 gap-y-1 text-[11px] text-muted-foreground">
            <li>1. 导入本地 Git 项目</li>
            <li aria-hidden="true">→</li>
            <li>2. 定义任务与必需验收条件</li>
            <li aria-hidden="true">→</li>
            <li>3. 在验收中心批准并运行验证</li>
          </ol>
        </div>
      ) : tasksQuery.isPending ? (
        <LoadingState rows={5} />
      ) : tasksQuery.isError ? (
        <ErrorState
          title="无法读取工作台数据"
          message={getErrorMessage(tasksQuery.error)}
          onRetry={() => void tasksQuery.refetch()}
        />
      ) : tasks.length === 0 ? (
        <EmptyState
          icon={CheckCircle2}
          title="当前项目还没有任务"
          description="创建第一个任务，写清目标与必需验收条件。完成必须由真实证据或人工确认支撑。"
          action={
            <Button type="button" size="sm" onClick={() => setCreateOpen(true)}>
              <Plus aria-hidden="true" />
              新建任务
            </Button>
          }
        />
      ) : (
        <>
          <SignalRail counts={counts} />

          <NextAction task={focusTask} detail={focusDetail} onCreate={() => setCreateOpen(true)} />

          <Section aria-label="最近验证" className="space-y-2">
            <SectionHeader
              title="最近验证"
              meta={verificationActivity.length > 0 ? `${verificationActivity.length} 次` : "暂无"}
              actions={
                <Button asChild variant="ghost" size="sm">
                  <Link to="/verification">
                    验收中心
                    <ArrowRight aria-hidden="true" />
                  </Link>
                </Button>
              }
            />
            {verificationActivity.length === 0 ? (
              <p className="border-y border-line/70 py-3 text-xs text-muted-foreground">
                当前项目最近任务还没有验证运行记录。
              </p>
            ) : (
              <ul className="divide-y divide-line/70 border-y border-line/70">
                {verificationActivity.map(({ task, view, run }) => (
                  <VerificationActivityRow key={run.id} task={task} view={view} run={run} />
                ))}
              </ul>
            )}
          </Section>

          <div className="grid min-w-0 items-start gap-6 xl:grid-cols-[minmax(0,1.6fr)_minmax(280px,0.85fr)]">
            <div className="min-w-0 space-y-6">
              <Section aria-label="需要处理" className="space-y-2">
                <SectionHeader
                  title="需要处理"
                  meta={
                    focusNeedsAttention
                      ? `另 ${remainingAttentionTasks.length} 项 · 焦点任务见上方`
                      : "当前无阻塞"
                  }
                  actions={
                    <Button asChild variant="ghost" size="sm">
                      <Link to="/tasks">全部任务</Link>
                    </Button>
                  }
                />
                {remainingAttentionTasks.length === 0 ? (
                  <p className="border-y border-line/70 py-3 text-xs text-muted-foreground">
                    {focusNeedsAttention
                      ? "待处理任务已列在「下一步」。"
                      : "没有阻塞或待验收任务。"}
                  </p>
                ) : (
                  <ul className="divide-y divide-line/70 border-y border-line/70">
                    {remainingAttentionTasks.map((task) => (
                      <AttentionRow key={task.id} task={task} detail={detailById.get(task.id)} />
                    ))}
                  </ul>
                )}
              </Section>

              <Section>
                <SectionHeader
                  title="最近任务"
                  meta={`${tasks.length} 个`}
                  actions={
                    <>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => setCreateOpen(true)}
                      >
                        <Plus aria-hidden="true" />
                        新建
                      </Button>
                      <Button asChild variant="ghost" size="sm">
                        <Link to="/tasks">
                          查看全部
                          <ArrowRight aria-hidden="true" />
                        </Link>
                      </Button>
                    </>
                  }
                />
                <ul className="divide-y divide-line/70 border-y border-line/70">
                  {recentTasks.map((task) => (
                    <RecentTaskRow key={task.id} task={task} />
                  ))}
                </ul>
              </Section>
            </div>

            <aside className="min-w-0 space-y-6 xl:border-l xl:border-line/70 xl:pl-6">
              <Section>
                <SectionHeader
                  title="当前仓库"
                  meta={currentProject?.name}
                  actions={
                    <Button asChild variant="ghost" size="sm">
                      <Link to={`/projects/${currentProject?.id}`}>
                        打开
                        <ArrowRight aria-hidden="true" />
                      </Link>
                    </Button>
                  }
                />
                {currentProject ? (
                  <div className="space-y-2 border-t border-line/70 pt-3">
                    <ProjectGitSummary projectId={currentProject.id} compact />
                    <p
                      className="truncate font-mono text-[10px] text-muted-foreground"
                      title={currentProject.rootPath}
                    >
                      {currentProject.rootPath}
                    </p>
                    <p className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                      <ShieldCheck className="size-3.5 text-primary" aria-hidden="true" />
                      Git 只读，不修改仓库。
                    </p>
                  </div>
                ) : null}
              </Section>

              <SystemHealthCard compact />

              <Section>
                <SectionHeader title="验收中心" meta="待验证任务与证据" />
                <Button asChild variant="outline" size="sm" className="w-full justify-between">
                  <Link to="/verification">
                    打开验收中心
                    <ArrowRight aria-hidden="true" />
                  </Link>
                </Button>
              </Section>
            </aside>
          </div>
        </>
      )}

      {importOpen ? <ImportProjectDialog open onOpenChange={setImportOpen} /> : null}
      {createOpen && currentProject ? (
        <CreateTaskDialog
          open
          onOpenChange={setCreateOpen}
          projectId={currentProject.id}
          projectName={currentProject.name}
        />
      ) : null}
    </div>
  );
}
