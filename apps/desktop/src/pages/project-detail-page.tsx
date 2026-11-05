import { ArrowLeft, FolderGit2, Plus, Star } from "lucide-react";
import { useState } from "react";
import { Link, useParams } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ProjectActionsMenu } from "@/features/projects/project-actions-menu";
import { ProjectGitSummary } from "@/features/projects/project-git-summary";
import { useProject, useProjects } from "@/features/projects/queries";
import { CreateTaskDialog } from "@/features/tasks/create-task-dialog";
import { useTasks } from "@/features/tasks/queries";
import { TaskTable } from "@/features/tasks/task-table";
import { useCurrentProject } from "@/features/workspace/use-current-project";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { useWorkspaceStore } from "@/stores/workspace-store";

export function ProjectDetailPage() {
  const { projectId } = useParams();
  const project = useProject(projectId);
  const projects = useProjects(true);
  const { currentProject } = useCurrentProject();
  const setSelectedProjectId = useWorkspaceStore((state) => state.setSelectedProjectId);
  const [createOpen, setCreateOpen] = useState(false);

  const archived = Boolean(project.data?.archivedAt);
  const tasks = useTasks(
    {
      projectId: projectId ?? "",
      includeArchived: archived,
    },
    Boolean(projectId),
  );

  if (project.isPending) {
    return (
      <div className="space-y-6">
        <PageHeader title="项目详情" description="加载项目信息…" />
        <LoadingState rows={4} />
      </div>
    );
  }

  if (project.isError || !project.data) {
    return (
      <div className="space-y-6">
        <Button asChild variant="ghost" size="sm">
          <Link to="/projects">
            <ArrowLeft aria-hidden="true" />
            返回项目列表
          </Link>
        </Button>
        <ErrorState
          title="无法读取项目"
          message={project.isError ? getErrorMessage(project.error) : "项目不存在"}
        />
      </div>
    );
  }

  const current = project.data;
  const isCurrent = currentProject?.id === current.id;

  return (
    <div className="space-y-6">
      <div>
        <Button asChild variant="ghost" size="sm" className="-ml-2 mb-2">
          <Link to="/projects">
            <ArrowLeft aria-hidden="true" />
            返回项目列表
          </Link>
        </Button>
        <PageHeader
          title={current.name}
          description={current.rootPath}
          actions={
            <>
              {!archived && !isCurrent ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setSelectedProjectId(current.id)}
                >
                  <Star aria-hidden="true" />
                  设为当前
                </Button>
              ) : null}
              {!archived ? (
                <Button type="button" size="sm" onClick={() => setCreateOpen(true)}>
                  <Plus aria-hidden="true" />
                  新建任务
                </Button>
              ) : null}
              <ProjectActionsMenu
                project={current}
                isCurrent={isCurrent}
                onSetCurrent={setSelectedProjectId}
              />
            </>
          }
        />
        <div className="mt-3 flex flex-wrap items-center gap-2">
          {isCurrent ? <Badge variant="secondary">当前项目</Badge> : null}
          {archived ? <Badge variant="secondary">已归档</Badge> : null}
          <span className="text-xs text-muted-foreground">
            更新于 {formatDateTime(current.updatedAt)}
          </span>
        </div>
      </div>

      <div className="grid items-start gap-5 xl:grid-cols-[1.4fr_1fr]">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <FolderGit2 className="size-4 text-primary" aria-hidden="true" />
              Git 状态
            </CardTitle>
            <CardDescription>
              XUSHU 只读检查分支、HEAD 与工作区；不会执行 checkout、reset、clean 等写操作。
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <ProjectGitSummary projectId={current.id} />
            <p className="truncate font-mono text-[11px] text-muted-foreground">
              {current.rootPath}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>项目数据</CardTitle>
            <CardDescription>仅保存 XUSHU 管理所需元数据。</CardDescription>
          </CardHeader>
          <CardContent>
            <dl className="space-y-3 text-sm">
              {[
                ["项目 ID", current.id],
                ["创建时间", formatDateTime(current.createdAt)],
                ["更新时间", formatDateTime(current.updatedAt)],
                ["归档状态", archived ? "已归档" : "进行中"],
              ].map(([label, value]) => (
                <div
                  key={label}
                  className="flex items-start justify-between gap-4 border-b pb-2 last:border-0"
                >
                  <dt className="shrink-0 text-muted-foreground">{label}</dt>
                  <dd className="min-w-0 break-all text-right font-mono text-xs">{value}</dd>
                </div>
              ))}
            </dl>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <CardTitle>项目任务</CardTitle>
              <CardDescription>
                {archived ? "项目已归档，任务只读展示。" : "按最近更新时间排序。"}
              </CardDescription>
            </div>
            {!archived ? (
              <Button type="button" variant="outline" size="sm" onClick={() => setCreateOpen(true)}>
                <Plus aria-hidden="true" />
                新建任务
              </Button>
            ) : null}
          </div>
        </CardHeader>
        <CardContent>
          {tasks.isPending ? (
            <LoadingState rows={3} />
          ) : tasks.isError ? (
            <ErrorState
              title="无法读取项目任务"
              message={getErrorMessage(tasks.error)}
              onRetry={() => void tasks.refetch()}
            />
          ) : tasks.data.length === 0 ? (
            <EmptyState
              icon={FolderGit2}
              title="该项目还没有任务"
              description={
                archived
                  ? "项目已归档，无法新建任务。"
                  : "新建任务并添加验收条件后，才能进入可信的完成流程。"
              }
              action={
                !archived ? (
                  <Button type="button" size="sm" onClick={() => setCreateOpen(true)}>
                    <Plus aria-hidden="true" />
                    新建任务
                  </Button>
                ) : undefined
              }
            />
          ) : (
            <TaskTable tasks={tasks.data} projects={projects.data ?? [current]} />
          )}
        </CardContent>
      </Card>

      {createOpen ? (
        <CreateTaskDialog
          open
          onOpenChange={setCreateOpen}
          projectId={current.id}
          projectName={current.name}
        />
      ) : null}
    </div>
  );
}
