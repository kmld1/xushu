import { ListChecks, Plus, Search, X } from "lucide-react";
import { useDeferredValue, useState } from "react";
import { Link } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { TASK_STATUS_META } from "@/lib/task-display";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { CreateTaskDialog } from "@/features/tasks/create-task-dialog";
import { useTasks } from "@/features/tasks/queries";
import { TaskTable } from "@/features/tasks/task-table";
import { useCurrentProject } from "@/features/workspace/use-current-project";
import type { TaskFilter, TaskStatus } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

const ALL_STATUSES: TaskStatus[] = [
  "draft",
  "ready",
  "running",
  "awaiting_verification",
  "done",
  "blocked",
  "cancelled",
];

export function TasksPage() {
  const { allProjects, currentProject } = useCurrentProject();
  const [projectId, setProjectId] = useState("all");
  const [status, setStatus] = useState<"all" | TaskStatus>("all");
  const [search, setSearch] = useState("");
  const [includeArchived, setIncludeArchived] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const deferredSearch = useDeferredValue(search);

  const filter: TaskFilter = {
    projectId: projectId === "all" ? null : projectId,
    status: status === "all" ? null : status,
    search: deferredSearch.trim() || null,
    includeArchived,
  };
  const tasks = useTasks(filter);

  const hasFilters =
    projectId !== "all" || status !== "all" || search.trim() !== "" || includeArchived;

  const clearFilters = () => {
    setProjectId("all");
    setStatus("all");
    setSearch("");
    setIncludeArchived(false);
  };

  return (
    <div className="space-y-5">
      <PageHeader
        title="任务"
        description="所有任务由 Rust 状态机管理；完成必须满足必需的验收条件。"
        actions={
          <Button
            type="button"
            size="sm"
            onClick={() => setCreateOpen(true)}
            disabled={!currentProject}
            title={!currentProject ? "请先导入并选择一个项目" : undefined}
          >
            <Plus aria-hidden="true" />
            新建任务
          </Button>
        }
      />

      <Card>
        <CardContent className="grid gap-3 p-4 md:grid-cols-2 xl:grid-cols-[1fr_180px_240px_auto]">
          <div className="relative">
            <Search
              className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
              aria-hidden="true"
            />
            <Input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="搜索任务标题或描述"
              className="pl-9"
              aria-label="搜索任务"
            />
          </div>

          <Select value={projectId} onValueChange={setProjectId}>
            <SelectTrigger aria-label="按项目筛选">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部项目</SelectItem>
              {allProjects.map((project) => (
                <SelectItem key={project.id} value={project.id}>
                  {project.name}
                  {project.archivedAt ? "（已归档）" : ""}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Select value={status} onValueChange={(value) => setStatus(value as "all" | TaskStatus)}>
            <SelectTrigger aria-label="按状态筛选">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部状态</SelectItem>
              {ALL_STATUSES.map((value) => (
                <SelectItem key={value} value={value}>
                  {TASK_STATUS_META[value].label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <div className="flex items-center gap-3">
            <label className="flex items-center gap-2 whitespace-nowrap text-xs text-muted-foreground">
              <input
                type="checkbox"
                checked={includeArchived}
                onChange={(event) => setIncludeArchived(event.target.checked)}
                className="size-4 accent-primary"
              />
              包含归档
            </label>
            {hasFilters ? (
              <Button type="button" variant="ghost" size="sm" onClick={clearFilters}>
                <X aria-hidden="true" />
                清除
              </Button>
            ) : null}
          </div>
        </CardContent>
      </Card>

      <div className="flex items-center justify-between gap-3">
        <p className="text-xs text-muted-foreground">
          {tasks.isPending ? "正在读取任务…" : `匹配 ${tasks.data?.length ?? 0} 个任务`}
        </p>
        {allProjects.length === 0 ? (
          <Button asChild variant="link" size="sm">
            <Link to="/projects">先导入项目</Link>
          </Button>
        ) : null}
      </div>

      <Card>
        <CardContent className="p-0">
          {tasks.isPending ? (
            <div className="p-5">
              <LoadingState rows={4} />
            </div>
          ) : tasks.isError ? (
            <div className="p-5">
              <ErrorState
                title="无法读取任务"
                message={getErrorMessage(tasks.error)}
                onRetry={() => void tasks.refetch()}
              />
            </div>
          ) : tasks.data.length === 0 ? (
            <div className="p-5">
              <EmptyState
                icon={ListChecks}
                title={hasFilters ? "没有匹配的任务" : "还没有任务"}
                description={
                  allProjects.length === 0
                    ? "导入项目后即可创建任务。"
                    : hasFilters
                      ? "尝试清除筛选条件，或创建一个新任务。"
                      : "从创建第一个任务开始，定义目标与验收条件。"
                }
                action={
                  allProjects.length === 0 ? (
                    <Button asChild size="sm">
                      <Link to="/projects">前往项目</Link>
                    </Button>
                  ) : (
                    <Button
                      type="button"
                      size="sm"
                      onClick={() => setCreateOpen(true)}
                      disabled={!currentProject}
                    >
                      <Plus aria-hidden="true" />
                      新建任务
                    </Button>
                  )
                }
              />
            </div>
          ) : (
            <TaskTable tasks={tasks.data} projects={allProjects} />
          )}
        </CardContent>
      </Card>

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
