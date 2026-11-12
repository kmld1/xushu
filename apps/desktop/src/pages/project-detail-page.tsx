import { ArrowLeft, FolderGit2, Plus, Star } from "lucide-react";
import { useState } from "react";
import { Link, useParams } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { Section, SectionHeader } from "@/components/section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ClaudeIntegrationSection } from "@/features/claude/claude-integration-section";
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
      <div className="space-y-5">
        <PageHeader title="项目详情" />
        <LoadingState rows={4} />
      </div>
    );
  }

  if (project.isError || !project.data) {
    return (
      <div className="space-y-5">
        <Button asChild variant="ghost" size="sm" className="-ml-2">
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
    <div className="space-y-5">
      <div>
        <Button asChild variant="ghost" size="sm" className="-ml-2 mb-1">
          <Link to="/projects">
            <ArrowLeft aria-hidden="true" />
            返回项目列表
          </Link>
        </Button>
        <PageHeader
          title={current.name}
          meta={
            <>
              {isCurrent ? <Badge variant="secondary">当前项目</Badge> : null}
              {archived ? <Badge variant="secondary">已归档</Badge> : null}
              <span
                className="max-w-[min(70vw,640px)] truncate font-mono text-[10px]"
                title={current.rootPath}
              >
                {current.rootPath}
              </span>
            </>
          }
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
      </div>

      <div className="border-y border-line/70 py-3">
        <SectionHeader title="仓库" meta="只读 Git" />
        <div className="mt-2">
          <ProjectGitSummary projectId={current.id} />
        </div>
      </div>

      <ClaudeIntegrationSection project={current} archived={archived} />

      <div className="grid min-w-0 items-start gap-6 xl:grid-cols-[minmax(0,1fr)_280px]">
        <Section className="min-w-0">
          <SectionHeader
            title="项目任务"
            meta={archived ? "只读" : "按最近更新排序"}
            actions={
              !archived ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setCreateOpen(true)}
                >
                  <Plus aria-hidden="true" />
                  新建任务
                </Button>
              ) : null
            }
          />

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
            <div className="overflow-hidden rounded-md border border-line/70 bg-card">
              <TaskTable
                tasks={tasks.data}
                projects={projects.data ?? [current]}
                readOnly={archived}
              />
            </div>
          )}
        </Section>

        <aside className="min-w-0 xl:border-l xl:border-line/70 xl:pl-6">
          <Section>
            <SectionHeader title="项目信息" />
            <dl className="divide-y divide-line/60 border-t border-line/70 text-xs">
              {[
                ["项目 ID", current.id],
                ["创建时间", formatDateTime(current.createdAt)],
                ["更新时间", formatDateTime(current.updatedAt)],
                ["归档状态", archived ? "已归档" : "进行中"],
              ].map(([label, value]) => (
                <div key={label} className="flex items-start justify-between gap-4 py-2">
                  <dt className="shrink-0 text-muted-foreground">{label}</dt>
                  <dd className="min-w-0 break-all text-right font-mono text-[11px]">{value}</dd>
                </div>
              ))}
            </dl>
            <p className="pt-1 text-[11px] leading-5 text-muted-foreground">
              XUSHU 只读检查分支、HEAD 与工作区，不执行 checkout、reset、clean 等写操作。
            </p>
          </Section>
        </aside>
      </div>

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
