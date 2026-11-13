import { ArrowLeft, Brain, History, Plus, Scale, X } from "lucide-react";
import { useMemo, useRef, useState, type MouseEvent } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { Section } from "@/components/section";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { DecisionDetailSheet } from "@/features/knowledge/decision-detail-sheet";
import { DecisionFormDialog } from "@/features/knowledge/decision-form-dialog";
import {
  DecisionStatusBadge,
  MemoryArchivedBadge,
  MemoryConfirmationBadge,
} from "@/features/knowledge/knowledge-badges";
import {
  DECISION_STATUS_META,
  MEMORY_CATEGORIES,
  MEMORY_CATEGORY_META,
  categoryLabel,
} from "@/features/knowledge/knowledge-display";
import { MemoryDetailSheet } from "@/features/knowledge/memory-detail-sheet";
import { MemoryFormDialog } from "@/features/knowledge/memory-form-dialog";
import { useDecisions, useMemories } from "@/features/knowledge/queries";
import { useProject } from "@/features/projects/queries";
import { useTasks } from "@/features/tasks/queries";
import type { DecisionStatus, DecisionView, MemoryCategory, ProjectMemoryView } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { useWorkspaceStore } from "@/stores/workspace-store";

const ALL_CATEGORIES = "__all__";
const ALL_STATUSES = "__all__";

export function ProjectKnowledgePage() {
  const { projectId } = useParams();
  const navigate = useNavigate();
  const setSelectedProjectId = useWorkspaceStore((state) => state.setSelectedProjectId);
  const project = useProject(projectId);
  const tasks = useTasks({ projectId: projectId ?? "", includeArchived: true }, Boolean(projectId));
  const [searchParams, setSearchParams] = useSearchParams();
  const taskFilter = searchParams.get("task");
  const activeTab = searchParams.get("tab") === "decisions" ? "decisions" : "memory";
  const [categoryFilter, setCategoryFilter] = useState<string>(ALL_CATEGORIES);
  const [statusFilter, setStatusFilter] = useState<string>(ALL_STATUSES);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [memoryDialogOpen, setMemoryDialogOpen] = useState(false);
  const [editingMemory, setEditingMemory] = useState<ProjectMemoryView | null>(null);
  const [selectedMemoryId, setSelectedMemoryId] = useState<string | null>(null);
  const [memorySheetOpen, setMemorySheetOpen] = useState(false);
  const [decisionDialogOpen, setDecisionDialogOpen] = useState(false);
  const [editingDecision, setEditingDecision] = useState<DecisionView | null>(null);
  const [initialSupersedesId, setInitialSupersedesId] = useState<string | undefined>();
  const [selectedDecisionId, setSelectedDecisionId] = useState<string | null>(null);
  const [decisionSheetOpen, setDecisionSheetOpen] = useState(false);
  const memoryOpener = useRef<HTMLElement | null>(null);
  const decisionOpener = useRef<HTMLElement | null>(null);

  const memoryFilter = useMemo(
    () => ({
      projectId: projectId ?? "",
      category: categoryFilter === ALL_CATEGORIES ? null : (categoryFilter as MemoryCategory),
      includeArchived,
      taskId: taskFilter,
    }),
    [projectId, categoryFilter, includeArchived, taskFilter],
  );
  const decisionFilter = useMemo(
    () => ({
      projectId: projectId ?? "",
      status: statusFilter === ALL_STATUSES ? null : (statusFilter as DecisionStatus),
      taskId: taskFilter,
    }),
    [projectId, statusFilter, taskFilter],
  );
  const memories = useMemories(memoryFilter, Boolean(projectId));
  const decisions = useDecisions(decisionFilter, Boolean(projectId));
  const confirmedDecisions = useDecisions(
    { projectId: projectId ?? "", status: "confirmed", taskId: null },
    Boolean(projectId),
  );
  const archived = Boolean(project.data?.archivedAt);
  const taskFilterTitle = taskFilter
    ? tasks.data?.find((task) => task.id === taskFilter)?.title
    : null;

  function setTab(value: string) {
    const next = new URLSearchParams(searchParams);
    next.set("tab", value);
    setSearchParams(next, { replace: true });
  }

  function clearTaskFilter() {
    const next = new URLSearchParams(searchParams);
    next.delete("task");
    setSearchParams(next, { replace: true });
  }

  function openMemory(memoryId: string, opener: HTMLElement) {
    memoryOpener.current = opener;
    setSelectedMemoryId(memoryId);
    setMemorySheetOpen(true);
  }

  function editMemory(view: ProjectMemoryView) {
    setMemorySheetOpen(false);
    setEditingMemory(view);
    setMemoryDialogOpen(true);
  }

  function createMemory(event: MouseEvent<HTMLButtonElement>) {
    memoryOpener.current = event.currentTarget;
    setEditingMemory(null);
    setMemoryDialogOpen(true);
  }

  function openDecision(decisionId: string, opener: HTMLElement) {
    decisionOpener.current = opener;
    setSelectedDecisionId(decisionId);
    setDecisionSheetOpen(true);
  }

  function editDecision(view: DecisionView) {
    setDecisionSheetOpen(false);
    setEditingDecision(view);
    setInitialSupersedesId(undefined);
    setDecisionDialogOpen(true);
  }

  function createDecision(event: MouseEvent<HTMLButtonElement>) {
    decisionOpener.current = event.currentTarget;
    setEditingDecision(null);
    setInitialSupersedesId(undefined);
    setDecisionDialogOpen(true);
  }

  function supersedeDecision(view: DecisionView) {
    setDecisionSheetOpen(false);
    setEditingDecision(null);
    setInitialSupersedesId(view.decision.id);
    setDecisionDialogOpen(true);
  }

  if (project.isPending) {
    return (
      <div className="space-y-5">
        <PageHeader title="记忆与决策" />
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
  const projectTasks = tasks.data ?? [];

  return (
    <div className="space-y-5">
      <div>
        <Button asChild variant="ghost" size="sm" className="-ml-2 mb-1">
          <Link to={`/projects/${current.id}`}>
            <ArrowLeft aria-hidden="true" />
            返回项目
          </Link>
        </Button>
        <PageHeader
          title="记忆与决策"
          meta={
            <>
              <span className="font-medium text-foreground">{current.name}</span>
              <span className="max-w-[min(60vw,560px)] truncate font-mono text-[10px]">
                {current.rootPath}
              </span>
              {archived ? <span className="text-warning">项目已归档，只读</span> : null}
            </>
          }
          description="项目记忆与开发决策是开发上下文，不会改变任务状态、验收证据或完成门禁。"
          actions={
            !archived ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  setSelectedProjectId(current.id);
                  navigate("/recovery");
                }}
              >
                <History aria-hidden="true" />
                生成恢复包
              </Button>
            ) : undefined
          }
        />
      </div>

      {taskFilter ? (
        <div className="flex flex-wrap items-center gap-2 border-l-2 border-info bg-info/5 px-3 py-2 text-xs text-info">
          <span>
            仅显示关联任务
            {taskFilterTitle ? `「${taskFilterTitle}」` : ""}的记忆与决策。
          </span>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="ml-auto text-info hover:text-info"
            onClick={clearTaskFilter}
          >
            <X aria-hidden="true" />
            清除筛选
          </Button>
        </div>
      ) : null}

      <Tabs value={activeTab} onValueChange={setTab}>
        <TabsList aria-label="记忆与决策">
          <TabsTrigger value="memory">项目记忆</TabsTrigger>
          <TabsTrigger value="decisions">开发决策</TabsTrigger>
        </TabsList>

        <TabsContent value="memory" className="mt-0">
          <Section>
            <div className="flex flex-wrap items-center gap-3 border-b border-line/70 pb-3">
              <div className="flex items-center gap-2">
                <label
                  htmlFor="memory-category-filter"
                  className="text-[11px] text-muted-foreground"
                >
                  类别
                </label>
                <Select value={categoryFilter} onValueChange={setCategoryFilter}>
                  <SelectTrigger id="memory-category-filter" className="h-7 w-[150px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={ALL_CATEGORIES}>全部类别</SelectItem>
                    {MEMORY_CATEGORIES.map((category) => (
                      <SelectItem key={category} value={category}>
                        {MEMORY_CATEGORY_META[category].label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <label className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
                <input
                  type="checkbox"
                  className="size-3.5 rounded-sm border-line-strong accent-primary"
                  checked={includeArchived}
                  onChange={(event) => setIncludeArchived(event.target.checked)}
                />
                包含已归档
              </label>
              <span className="text-[11px] text-muted-foreground">
                {memories.data ? `${memories.data.length} 条` : "—"}
              </span>
              {!archived ? (
                <Button type="button" size="sm" className="ml-auto" onClick={createMemory}>
                  <Plus aria-hidden="true" />
                  新建记忆
                </Button>
              ) : null}
            </div>

            {memories.isPending ? (
              <LoadingState rows={4} label="正在读取项目记忆" />
            ) : memories.isError ? (
              <ErrorState
                title="无法读取项目记忆"
                message={getErrorMessage(memories.error)}
                onRetry={() => void memories.refetch()}
              />
            ) : memories.data.length === 0 ? (
              <EmptyState
                icon={Brain}
                title="还没有项目记忆"
                description={
                  archived
                    ? "项目已归档，不能新增记忆。"
                    : "记录目标、约束、事实与风险。只有已确认且有效的记忆会进入恢复包。"
                }
                action={
                  !archived ? (
                    <Button type="button" size="sm" onClick={createMemory}>
                      <Plus aria-hidden="true" />
                      新建记忆
                    </Button>
                  ) : undefined
                }
              />
            ) : (
              <ul className="divide-y divide-line/60 border-y border-line/70">
                {memories.data.map((row) => {
                  const Icon = MEMORY_CATEGORY_META[row.memory.category].icon;
                  const archivedMemory = row.memory.validityStatus === "archived";
                  return (
                    <li
                      key={row.memory.id}
                      className="group flex min-w-0 flex-col gap-1.5 px-2 py-3 transition-colors duration-120 hover:bg-hover/70 focus-within:bg-hover/70"
                    >
                      <div className="flex min-w-0 flex-wrap items-center gap-2">
                        <span className="inline-flex shrink-0 items-center gap-1 text-[11px] text-muted-foreground">
                          <Icon className="size-3.5" aria-hidden="true" />
                          {categoryLabel(row.memory.category)}
                        </span>
                        <button
                          type="button"
                          className="min-w-0 flex-1 truncate text-left text-[13px] font-medium underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                          title={row.memory.title}
                          onClick={(event) => openMemory(row.memory.id, event.currentTarget)}
                        >
                          {row.memory.title}
                        </button>
                        <MemoryConfirmationBadge status={row.memory.confirmationStatus} />
                        {archivedMemory ? <MemoryArchivedBadge /> : null}
                      </div>
                      <p className="line-clamp-2 whitespace-pre-wrap break-words text-xs leading-5 text-muted-foreground">
                        {row.memory.content}
                      </p>
                      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                        <span className="font-mono">r{row.memory.revision}</span>
                        <span>更新于 {formatDateTime(row.memory.updatedAt)}</span>
                        {row.linkedTask ? (
                          <span className="min-w-0 truncate" title={row.linkedTask.title}>
                            关联任务：{row.linkedTask.title}
                          </span>
                        ) : null}
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="ml-auto"
                          onClick={(event) => openMemory(row.memory.id, event.currentTarget)}
                        >
                          查看
                        </Button>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </Section>
        </TabsContent>

        <TabsContent value="decisions" className="mt-0">
          <Section>
            <div className="flex flex-wrap items-center gap-3 border-b border-line/70 pb-3">
              <div className="flex items-center gap-2">
                <label
                  htmlFor="decision-status-filter"
                  className="text-[11px] text-muted-foreground"
                >
                  状态
                </label>
                <Select value={statusFilter} onValueChange={setStatusFilter}>
                  <SelectTrigger id="decision-status-filter" className="h-7 w-[140px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={ALL_STATUSES}>全部状态</SelectItem>
                    {(Object.keys(DECISION_STATUS_META) as DecisionStatus[]).map((status) => (
                      <SelectItem key={status} value={status}>
                        {DECISION_STATUS_META[status].label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <span className="text-[11px] text-muted-foreground">
                {decisions.data ? `${decisions.data.length} 条` : "—"}
              </span>
              {!archived ? (
                <Button type="button" size="sm" className="ml-auto" onClick={createDecision}>
                  <Plus aria-hidden="true" />
                  新建决策
                </Button>
              ) : null}
            </div>

            {decisions.isPending ? (
              <LoadingState rows={4} label="正在读取开发决策" />
            ) : decisions.isError ? (
              <ErrorState
                title="无法读取开发决策"
                message={getErrorMessage(decisions.error)}
                onRetry={() => void decisions.refetch()}
              />
            ) : decisions.data.length === 0 ? (
              <EmptyState
                icon={Scale}
                title="还没有开发决策"
                description={
                  archived
                    ? "项目已归档，不能新增决策。"
                    : "记录为什么采用当前方案、放弃了什么，以及仍然有效的代价。草稿不会进入恢复包。"
                }
                action={
                  !archived ? (
                    <Button type="button" size="sm" onClick={createDecision}>
                      <Plus aria-hidden="true" />
                      新建决策
                    </Button>
                  ) : undefined
                }
              />
            ) : (
              <ul className="divide-y divide-line/60 border-y border-line/70">
                {decisions.data.map((row) => {
                  const decision = row.decision;
                  return (
                    <li
                      key={decision.id}
                      className="group flex min-w-0 flex-col gap-1.5 px-2 py-3 transition-colors duration-120 hover:bg-hover/70 focus-within:bg-hover/70"
                    >
                      <div className="flex min-w-0 flex-wrap items-center gap-2">
                        <DecisionStatusBadge status={decision.status} />
                        <button
                          type="button"
                          className="min-w-0 flex-1 truncate text-left text-[13px] font-medium underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                          title={decision.title}
                          onClick={(event) => openDecision(decision.id, event.currentTarget)}
                        >
                          {decision.title}
                        </button>
                        <span className="font-mono text-[11px] text-muted-foreground">
                          r{decision.revision}
                        </span>
                      </div>
                      <p className="line-clamp-2 whitespace-pre-wrap break-words text-xs leading-5 text-muted-foreground">
                        {decision.decision}
                      </p>
                      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                        <span>{decision.alternatives.length} 个备选方案</span>
                        <span>更新于 {formatDateTime(decision.updatedAt)}</span>
                        {row.linkedTask ? (
                          <span className="min-w-0 truncate" title={row.linkedTask.title}>
                            关联任务：{row.linkedTask.title}
                          </span>
                        ) : null}
                        {row.supersededByTitle ? (
                          <span className="text-warning">已被「{row.supersededByTitle}」取代</span>
                        ) : null}
                        {decision.status === "draft" ? (
                          <span className="text-info">草稿不会进入恢复包</span>
                        ) : null}
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="ml-auto"
                          onClick={(event) => openDecision(decision.id, event.currentTarget)}
                        >
                          查看
                        </Button>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </Section>
        </TabsContent>
      </Tabs>

      <MemoryFormDialog
        open={memoryDialogOpen}
        restoreFocus={() => memoryOpener.current?.focus()}
        onOpenChange={(open) => {
          setMemoryDialogOpen(open);
          if (!open) {
            setEditingMemory(null);
          }
        }}
        projectId={current.id}
        projectName={current.name}
        tasks={projectTasks}
        memory={editingMemory ?? undefined}
      />

      <DecisionFormDialog
        open={decisionDialogOpen}
        restoreFocus={() => decisionOpener.current?.focus()}
        onOpenChange={(open) => {
          setDecisionDialogOpen(open);
          if (!open) {
            setEditingDecision(null);
            setInitialSupersedesId(undefined);
          }
        }}
        projectId={current.id}
        projectName={current.name}
        tasks={projectTasks}
        supersededCandidates={confirmedDecisions.data ?? []}
        decision={editingDecision ?? undefined}
        initialSupersedesId={initialSupersedesId}
      />

      <MemoryDetailSheet
        projectId={current.id}
        memoryId={selectedMemoryId}
        open={memorySheetOpen}
        onOpenChange={setMemorySheetOpen}
        readOnly={archived}
        onEdit={editMemory}
        restoreFocus={() => {
          if (!memoryDialogOpen) memoryOpener.current?.focus();
        }}
      />

      <DecisionDetailSheet
        projectId={current.id}
        decisionId={selectedDecisionId}
        open={decisionSheetOpen}
        onOpenChange={setDecisionSheetOpen}
        readOnly={archived}
        onEdit={editDecision}
        onSupersede={supersedeDecision}
        restoreFocus={() => {
          if (!decisionDialogOpen) decisionOpener.current?.focus();
        }}
      />
    </div>
  );
}
