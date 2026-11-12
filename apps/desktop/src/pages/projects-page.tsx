import { ArrowRight, FolderGit2, FolderPlus } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ImportProjectDialog } from "@/features/projects/import-project-dialog";
import { ProjectActionsMenu } from "@/features/projects/project-actions-menu";
import { ProjectGitSummary } from "@/features/projects/project-git-summary";
import { useProjects } from "@/features/projects/queries";
import { useCurrentProject } from "@/features/workspace/use-current-project";
import type { Project } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useWorkspaceStore } from "@/stores/workspace-store";

function ProjectRow({
  project,
  isCurrent,
  onSetCurrent,
}: {
  project: Project;
  isCurrent: boolean;
  onSetCurrent: (projectId: string) => void;
}) {
  const archived = Boolean(project.archivedAt);

  return (
    <li
      className={cn(
        "group flex items-center gap-3 py-3 first:pt-2 last:pb-2",
        archived && "opacity-70",
      )}
    >
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          {isCurrent ? (
            <span className="size-1.5 shrink-0 rounded-full bg-primary" aria-hidden="true" />
          ) : null}
          <Link
            to={`/projects/${project.id}`}
            className="truncate text-[13px] font-semibold underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            title={project.name}
          >
            {project.name}
          </Link>
          {isCurrent ? <span className="text-[10px] font-medium text-primary">当前</span> : null}
          {archived ? <span className="text-[10px] text-muted-foreground">已归档</span> : null}
        </div>
        <p
          className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground"
          title={project.rootPath}
        >
          {project.rootPath}
        </p>
        <div className="mt-1">
          <ProjectGitSummary projectId={project.id} compact />
        </div>
      </div>

      <Link
        to={`/projects/${project.id}`}
        className="inline-flex shrink-0 items-center gap-1 rounded-sm px-1.5 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-hover hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        aria-label={`打开项目 ${project.name}`}
      >
        打开
        <ArrowRight className="size-3" aria-hidden="true" />
      </Link>
      <ProjectActionsMenu project={project} isCurrent={isCurrent} onSetCurrent={onSetCurrent} />
    </li>
  );
}

export function ProjectsPage() {
  const projects = useProjects(true);
  const { activeProjects, currentProject } = useCurrentProject();
  const setSelectedProjectId = useWorkspaceStore((state) => state.setSelectedProjectId);
  const [importOpen, setImportOpen] = useState(false);
  const [tab, setTab] = useState("active");

  const archivedProjects = (projects.data ?? []).filter((project) => project.archivedAt);

  return (
    <div className="space-y-5">
      <h1 className="sr-only">项目</h1>

      {projects.isPending ? (
        <LoadingState rows={4} />
      ) : projects.isError ? (
        <ErrorState
          title="无法读取项目列表"
          message={getErrorMessage(projects.error)}
          onRetry={() => void projects.refetch()}
        />
      ) : (
        <Tabs value={tab} onValueChange={setTab}>
          <div className="flex items-center justify-between gap-3 border-b border-line/80">
            <TabsList aria-label="项目状态筛选" className="w-auto flex-1 border-b-0">
              <TabsTrigger value="active">进行中 {activeProjects.length}</TabsTrigger>
              <TabsTrigger value="archived">已归档 {archivedProjects.length}</TabsTrigger>
            </TabsList>
            <Button type="button" size="sm" onClick={() => setImportOpen(true)}>
              <FolderPlus aria-hidden="true" />
              导入项目
            </Button>
          </div>

          <TabsContent value="active">
            {activeProjects.length === 0 ? (
              <EmptyState
                icon={FolderGit2}
                title="还没有进行中的项目"
                description="导入一个本地 Git 工作树，开始定义任务与验收条件。"
                action={
                  <Button type="button" size="sm" onClick={() => setImportOpen(true)}>
                    <FolderPlus aria-hidden="true" />
                    导入项目
                  </Button>
                }
              />
            ) : (
              <ul className="divide-y divide-line/70 border-b border-line/70">
                {activeProjects.map((project) => (
                  <ProjectRow
                    key={project.id}
                    project={project}
                    isCurrent={project.id === currentProject?.id}
                    onSetCurrent={setSelectedProjectId}
                  />
                ))}
              </ul>
            )}
          </TabsContent>

          <TabsContent value="archived">
            {archivedProjects.length === 0 ? (
              <p className="border-b border-line/70 py-6 text-xs text-muted-foreground">
                没有已归档项目。归档会保留磁盘文件与任务数据，可随时恢复。
              </p>
            ) : (
              <ul className="divide-y divide-line/70 border-b border-line/70">
                {archivedProjects.map((project) => (
                  <ProjectRow
                    key={project.id}
                    project={project}
                    isCurrent={false}
                    onSetCurrent={setSelectedProjectId}
                  />
                ))}
              </ul>
            )}
          </TabsContent>
        </Tabs>
      )}

      {importOpen ? (
        <ImportProjectDialog
          open
          onOpenChange={setImportOpen}
          onImported={(project) => {
            setSelectedProjectId(project.id);
            setTab("active");
          }}
        />
      ) : null}
    </div>
  );
}
