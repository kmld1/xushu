import { ListChecks, Plus, Search, X } from "lucide-react";
import { useDeferredValue, useState } from "react";
import { Link } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Button } from "@/components/ui/button";
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
import { TASK_STATUS_META } from "@/lib/task-display";

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
      <h1 className="sr-only">任务</h1>

      <div className="flex flex-wrap items-center gap-2 border-y border-line/70 py-2.5">
        <div className="relative w-full sm:w-64">
          <Search
            className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="搜索任务标题或描述"
            className="pl-8"
            aria-label="搜索任务"
          />
        </div>

        <Select value={projectId} onValueChange={setProjectId}>
          <SelectTrigger className="w-40" aria-label="按项目筛选">
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
          <SelectTrigger className="w-36" aria-label="按状态筛选">
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

        <label className="flex items-center gap-2 whitespace-nowrap px-1 text-[11px] text-muted-foreground">
          <input
            type="checkbox"
            checked={includeArchived}
            onChange={(event) => setIncludeArchived(event.target.checked)}
            className="size-3.5 accent-primary"
          />
          包含归档
        </label>

        {hasFilters ? (
          <Button type="button" variant="ghost" size="sm" onClick={clearFilters}>
            <X aria-hidden="true" />
            清除
          </Button>
        ) : null}

        <div className="ml-auto flex items-center gap-2">
          <p className="text-[11px] text-muted-foreground">
            {tasks.isPending ? "正在读取任务…" : `匹配 ${tasks.data?.length ?? 0} 个任务`}
          </p>
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
        </div>
      </div>

      {allProjects.length === 0 ? (
        <EmptyState
          icon={ListChecks}
          title="还没有任务"
          description="先导入一个项目，再创建任务与验收条件。"
          action={
            <Button asChild size="sm">
              <Link to="/projects">前往项目</Link>
            </Button>
          }
        />
      ) : tasks.isPending ? (
        <LoadingState rows={5} />
      ) : tasks.isError ? (
        <ErrorState
          title="无法读取任务"
          message={getErrorMessage(tasks.error)}
          onRetry={() => void tasks.refetch()}
        />
      ) : tasks.data.length === 0 ? (
        <EmptyState
          icon={ListChecks}
          title={hasFilters ? "没有匹配的任务" : "还没有任务"}
          description={
            hasFilters
              ? "尝试清除筛选条件，或创建一个新任务。"
              : "从创建第一个任务开始，定义目标与验收条件。"
          }
          action={
            <Button
              type="button"
              size="sm"
              onClick={() => setCreateOpen(true)}
              disabled={!currentProject}
            >
              <Plus aria-hidden="true" />
              新建任务
            </Button>
          }
        />
      ) : (
        <div className="overflow-hidden rounded-md border border-line/70 bg-card">
          <TaskTable tasks={tasks.data} projects={allProjects} />
        </div>
      )}

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
