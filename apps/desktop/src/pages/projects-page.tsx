import { FolderGit2, FolderPlus, Star } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
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
        "flex items-start gap-3 rounded-lg border bg-card p-4",
        isCurrent && "border-l-2 border-l-primary",
        archived && "opacity-80",
      )}
    >
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <Link
            to={`/projects/${project.id}`}
            className="truncate text-sm font-semibold underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            {project.name}
          </Link>
          {isCurrent ? (
            <Badge variant="secondary" className="gap-1">
              <Star className="size-3" aria-hidden="true" />
              当前
            </Badge>
          ) : null}
          {archived ? <Badge variant="secondary">已归档</Badge> : null}
        </div>
        <p
          className="mt-1 truncate font-mono text-[11px] text-muted-foreground"
          title={project.rootPath}
        >
          {project.rootPath}
        </p>
        <div className="mt-2">
          <ProjectGitSummary projectId={project.id} compact />
        </div>
      </div>
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
    <div className="space-y-6">
      <PageHeader
        title="项目"
        description="管理已导入的本地 Git 项目。XUSHU 只读 Git，不修改仓库文件、分支或历史。"
        actions={
          <Button type="button" size="sm" onClick={() => setImportOpen(true)}>
            <FolderPlus aria-hidden="true" />
            导入项目
          </Button>
        }
      />

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
          <TabsList aria-label="项目状态筛选">
            <TabsTrigger value="active">进行中 {activeProjects.length}</TabsTrigger>
            <TabsTrigger value="archived">已归档 {archivedProjects.length}</TabsTrigger>
          </TabsList>

          <TabsContent value="active">
            {activeProjects.length === 0 ? (
              <EmptyState
                icon={FolderGit2}
                title="还没有进行中的项目"
                description="导入一个本地 Git 工作树，开始创建任务并跟踪验收状态。"
                action={
                  <Button type="button" onClick={() => setImportOpen(true)}>
                    <FolderPlus aria-hidden="true" />
                    导入项目
                  </Button>
                }
              />
            ) : (
              <ul className="space-y-3">
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
              <Card>
                <CardContent className="p-6 text-sm text-muted-foreground">
                  没有已归档项目。归档项目会保留磁盘文件与任务数据，可随时恢复。
                </CardContent>
              </Card>
            ) : (
              <ul className="space-y-3">
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
