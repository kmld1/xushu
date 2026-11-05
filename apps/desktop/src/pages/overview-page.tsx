import { ArrowRight, FolderGit2, ListChecks, OctagonAlert, Plus, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { PriorityBadge, StatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ImportProjectDialog } from "@/features/projects/import-project-dialog";
import { ProjectGitSummary } from "@/features/projects/project-git-summary";
import { CreateTaskDialog } from "@/features/tasks/create-task-dialog";
import { useWorkspaceSummary } from "@/features/tasks/queries";
import { SystemHealthCard } from "@/features/system/system-health-card";
import { useCurrentProject } from "@/features/workspace/use-current-project";
import { SignalRail } from "@/features/workspace/signal-rail";
import type { Task } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

function CompactTaskList({
  tasks,
  emptyText,
  showBlockReason = false,
}: {
  tasks: Task[];
  emptyText: string;
  showBlockReason?: boolean;
}) {
  if (tasks.length === 0) {
    return (
      <p className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
        {emptyText}
      </p>
    );
  }

  return (
    <ul className="divide-y">
      {tasks.map((task) => (
        <li
          key={task.id}
          className="flex items-start justify-between gap-3 py-3 first:pt-0 last:pb-0"
        >
          <div className="min-w-0">
            <Link
              to={`/tasks/${task.id}`}
              className="block truncate text-sm font-medium underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              title={task.title}
            >
              {task.title}
            </Link>
            <div className="mt-1 flex flex-wrap items-center gap-2">
              <StatusBadge status={task.status} />
              <PriorityBadge priority={task.priority} />
              <span className="text-[11px] text-muted-foreground">
                {formatDateTime(task.updatedAt)}
              </span>
            </div>
            {showBlockReason && task.blockReason ? (
              <p className="mt-1 text-xs leading-5 text-destructive">{task.blockReason}</p>
            ) : null}
          </div>
          <ArrowRight className="mt-1 size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
        </li>
      ))}
    </ul>
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
  const summary = useWorkspaceSummary();
  const [importOpen, setImportOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);

  if (projectsPending) {
    return (
      <div className="space-y-6">
        <PageHeader title="工作台" description="目标、任务与验证状态的真实汇总。" />
        <LoadingState rows={4} />
      </div>
    );
  }

  if (projectsError) {
    return (
      <div className="space-y-6">
        <PageHeader title="工作台" description="目标、任务与验证状态的真实汇总。" />
        <ErrorState
          title="无法读取项目"
          message={getErrorMessage(projectsErrorValue)}
          onRetry={() => window.location.reload()}
        />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="工作台"
        description="当前项目、任务状态与需要处理的事项。所有数字来自本地 SQLite，不使用演示数据。"
        actions={
          currentProject ? (
            <>
              <Button type="button" variant="outline" size="sm" onClick={() => setImportOpen(true)}>
                <FolderGit2 aria-hidden="true" />
                导入项目
              </Button>
              <Button type="button" size="sm" onClick={() => setCreateOpen(true)}>
                <Plus aria-hidden="true" />
                新建任务
              </Button>
            </>
          ) : (
            <Button type="button" size="sm" onClick={() => setImportOpen(true)}>
              <FolderGit2 aria-hidden="true" />
              导入 Git 项目
            </Button>
          )
        }
      />

      {activeProjects.length === 0 ? (
        <>
          <EmptyState
            icon={FolderGit2}
            title="还没有可管理的项目"
            description="导入一个本地 Git 工作树后，即可创建任务、定义验收条件并跟踪可信状态。XUSHU 只做只读 Git 检查。"
            action={
              <Button type="button" onClick={() => setImportOpen(true)}>
                <FolderGit2 aria-hidden="true" />
                导入第一个项目
              </Button>
            }
          />
          <SystemHealthCard />
        </>
      ) : summary.isPending ? (
        <LoadingState rows={5} />
      ) : summary.isError ? (
        <ErrorState
          title="无法读取工作台数据"
          message={getErrorMessage(summary.error)}
          onRetry={() => void summary.refetch()}
        />
      ) : (
        <>
          <SignalRail counts={summary.data.taskCounts} />

          <div className="grid items-start gap-5 xl:grid-cols-[1.55fr_1fr]">
            <div className="space-y-5">
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <OctagonAlert className="size-4 text-warning" aria-hidden="true" />
                    需要关注
                  </CardTitle>
                  <CardDescription>阻塞任务与等待验证的任务。</CardDescription>
                </CardHeader>
                <CardContent className="space-y-5">
                  <div>
                    <p className="mb-2 text-xs font-medium text-muted-foreground">已阻塞</p>
                    <CompactTaskList
                      tasks={summary.data.blockedTasks}
                      emptyText="没有阻塞任务。"
                      showBlockReason
                    />
                  </div>
                  <div>
                    <p className="mb-2 text-xs font-medium text-muted-foreground">待验证</p>
                    <CompactTaskList
                      tasks={summary.data.awaitingTasks}
                      emptyText="没有等待验证的任务。"
                    />
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <CardTitle className="flex items-center gap-2">
                        <ListChecks className="size-4 text-primary" aria-hidden="true" />
                        最近任务
                      </CardTitle>
                      <CardDescription>按最近更新时间排序。</CardDescription>
                    </div>
                    <Button asChild variant="ghost" size="sm">
                      <Link to="/tasks">查看全部</Link>
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  <CompactTaskList tasks={summary.data.recentTasks} emptyText="还没有任务。" />
                </CardContent>
              </Card>
            </div>

            <div className="space-y-5">
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <FolderGit2 className="size-4 text-primary" aria-hidden="true" />
                    当前项目
                  </CardTitle>
                  <CardDescription>只读 Git 状态与项目入口。</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {currentProject ? (
                    <>
                      <div>
                        <Link
                          to={`/projects/${currentProject.id}`}
                          className="text-sm font-semibold underline-offset-4 hover:text-primary hover:underline"
                        >
                          {currentProject.name}
                        </Link>
                        <p
                          className="mt-1 truncate font-mono text-[11px] text-muted-foreground"
                          title={currentProject.rootPath}
                        >
                          {currentProject.rootPath}
                        </p>
                      </div>
                      <ProjectGitSummary projectId={currentProject.id} />
                      <p className="flex items-start gap-2 text-xs leading-5 text-muted-foreground">
                        <ShieldCheck
                          className="mt-0.5 size-3.5 shrink-0 text-primary"
                          aria-hidden="true"
                        />
                        Git 操作全程只读，不修改仓库文件或历史。
                      </p>
                    </>
                  ) : (
                    <p className="text-sm text-muted-foreground">没有可用的当前项目。</p>
                  )}
                </CardContent>
              </Card>

              <SystemHealthCard compact />
            </div>
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
