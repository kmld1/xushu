import {
  Activity,
  AlertTriangle,
  ArrowRight,
  Brain,
  ClipboardCopy,
  FileText,
  FolderGit2,
  GitBranch,
  History,
  Loader2,
  RefreshCw,
  Scale,
  ShieldAlert,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Section, SectionHeader } from "@/components/section";
import { StatusBadge, VerificationBadge } from "@/components/status-badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DecisionStatusBadge } from "@/features/knowledge/knowledge-badges";
import { categoryLabel } from "@/features/knowledge/knowledge-display";
import { buildClipboardText, copyTextToClipboard } from "@/features/recovery/copy-recovery";
import {
  NextStepKindBadge,
  RecoveryFreshnessBadge,
  RecoveryGitBadge,
} from "@/features/recovery/recovery-badges";
import { conflictLabel, shortFingerprint } from "@/features/recovery/recovery-display";
import { RecoveryHistorySheet } from "@/features/recovery/recovery-history-sheet";
import { RecoveryInstructionsSheet } from "@/features/recovery/recovery-instructions-sheet";
import {
  useGenerateRecovery,
  useRecoveryOverview,
  useRecoverySnapshot,
  useSetRecoveryNextTask,
} from "@/features/recovery/queries";
import { useTasks } from "@/features/tasks/queries";
import { useCurrentProject } from "@/features/workspace/use-current-project";
import type {
  RecoveryFreshness,
  RecoveryMemoryItem,
  RecoverySnapshot,
  RecoveryTaskItem,
  TaskStatus,
} from "@/lib/domain";
import { formatDateTime, shortHead } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { TASK_STATUS_META } from "@/lib/task-display";
import { cn } from "@/lib/utils";

const NO_TASK = "__none__";

type SelectableTask = {
  id: string;
  title: string;
  status: TaskStatus;
};

function taskOptions(items: RecoveryTaskItem[]): RecoveryTaskItem[] {
  const seen = new Set<string>();
  return items.filter((item) => {
    if (seen.has(item.id)) {
      return false;
    }
    seen.add(item.id);
    return true;
  });
}

function TaskGroup({
  title,
  tasks,
  tone = "default",
}: {
  title: string;
  tasks: RecoveryTaskItem[];
  tone?: "default" | "warning";
}) {
  if (tasks.length === 0) {
    return null;
  }

  return (
    <section aria-label={title} className="space-y-1">
      <h3 className="text-[11px] font-medium text-muted-foreground">{title}</h3>
      <div className="divide-y divide-line/60 border-y border-line/70">
        {tasks.map((task) => (
          <div key={task.id} className="py-2">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <StatusBadge status={task.status} appearance="inline" />
              <Link
                to={`/tasks/${task.id}`}
                className="min-w-0 flex-1 truncate text-xs font-medium underline-offset-4 hover:text-primary hover:underline"
                title={task.title}
              >
                {task.title}
              </Link>
              <span className="font-mono text-[11px] text-muted-foreground">r{task.revision}</span>
            </div>
            <p className="mt-0.5 text-[11px] text-muted-foreground">
              验收 {task.satisfiedRequiredCount}/{task.requiredCount} 必需条件
            </p>
            {task.blockReason ? (
              <p className="mt-1 whitespace-pre-wrap break-words text-[11px] leading-5 text-destructive">
                阻塞原因：{task.blockReason}
              </p>
            ) : null}
            {task.blockers.length > 0 ? (
              <ul
                className={cn(
                  "mt-1 list-disc space-y-0.5 pl-4 text-[11px] leading-5",
                  tone === "warning" ? "text-warning" : "text-muted-foreground",
                )}
              >
                {task.blockers.map((blocker) => (
                  <li key={blocker} className="break-words">
                    {blocker}
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        ))}
      </div>
    </section>
  );
}

function MemoryPreviewRow({ memory }: { memory: RecoveryMemoryItem }) {
  return (
    <div className="py-2">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <span className="text-[11px] text-muted-foreground">{categoryLabel(memory.category)}</span>
        <span className="min-w-0 flex-1 truncate text-xs font-medium" title={memory.title}>
          {memory.title}
        </span>
        <span className="font-mono text-[11px] text-muted-foreground">r{memory.revision}</span>
      </div>
      <p className="mt-0.5 line-clamp-2 whitespace-pre-wrap break-words text-[11px] leading-5 text-muted-foreground">
        {memory.content}
      </p>
    </div>
  );
}

export function RecoveryPage() {
  const {
    currentProject,
    isPending: projectsPending,
    isError: projectsError,
    error: projectsErrorValue,
  } = useCurrentProject();
  const projectId = currentProject?.id;
  const overview = useRecoveryOverview(projectId);
  const generate = useGenerateRecovery(projectId);
  const setNextTask = useSetRecoveryNextTask(projectId);
  const [instructionsOpen, setInstructionsOpen] = useState(false);
  const [instructionsSnapshot, setInstructionsSnapshot] = useState<RecoverySnapshot | null>(null);
  const [instructionsHistorical, setInstructionsHistorical] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [historySnapshotId, setHistorySnapshotId] = useState<string | null>(null);
  const [copying, setCopying] = useState(false);
  const [copyRequest, setCopyRequest] = useState<{
    snapshot: RecoverySnapshot;
    freshness: RecoveryFreshness;
    historical: boolean;
  } | null>(null);
  const [nextTaskSelection, setNextTaskSelection] = useState<string>(NO_TASK);
  const instructionsOpener = useRef<HTMLButtonElement>(null);
  const historyOpener = useRef<HTMLButtonElement>(null);
  const copyOpener = useRef<HTMLButtonElement>(null);
  const historySnapshot = useRecoverySnapshot(
    projectId,
    historySnapshotId ?? undefined,
    Boolean(historySnapshotId),
  );

  useEffect(() => {
    if (historySnapshot.data) {
      setInstructionsSnapshot(historySnapshot.data);
      setInstructionsHistorical(true);
      setHistoryOpen(false);
      setHistorySnapshotId(null);
      setInstructionsOpen(true);
    }
  }, [historySnapshot.data]);

  useEffect(() => {
    if (historySnapshot.error) {
      toast.error(getErrorMessage(historySnapshot.error));
      setHistorySnapshotId(null);
    }
  }, [historySnapshot.error]);

  const latest = overview.data?.latest ?? null;
  const freshness = overview.data?.freshness;
  const projectTasks = useTasks({ projectId, includeArchived: false }, Boolean(projectId));

  const activeOptions = useMemo(() => {
    if (!latest) {
      return [];
    }
    return taskOptions([
      ...latest.package.tasks.blocked,
      ...latest.package.tasks.awaitingVerification,
      ...latest.package.tasks.inProgress,
      ...latest.package.tasks.ready,
    ]);
  }, [latest]);

  const selectableTasks = useMemo<SelectableTask[]>(() => {
    if (projectTasks.data) {
      return projectTasks.data
        .filter((task) => task.status !== "done" && task.status !== "cancelled")
        .map((task) => ({
          id: task.id,
          title: task.title,
          status: task.status,
        }));
    }
    return activeOptions.map((task) => ({
      id: task.id,
      title: task.title,
      status: task.status,
    }));
  }, [projectTasks.data, activeOptions]);

  useEffect(() => {
    const preferred = latest?.package.nextStep.taskId;
    const available =
      preferred && projectTasks.data
        ? projectTasks.data.some(
            (task) =>
              task.id === preferred && task.status !== "done" && task.status !== "cancelled",
          )
        : false;
    setNextTaskSelection(available ? (preferred ?? NO_TASK) : NO_TASK);
  }, [latest?.id, latest?.package.nextStep.taskId, projectTasks.data]);

  async function handleGenerate() {
    try {
      const snapshot = await generate.mutateAsync();
      toast.success(latest ? "恢复包已重新生成" : "恢复包已生成", {
        description: `指纹 ${shortFingerprint(snapshot.fingerprint)}；已保存为最近快照。`,
      });
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  }

  async function copySnapshot(
    snapshot: RecoverySnapshot,
    state: RecoveryFreshness,
    historical: boolean,
  ) {
    try {
      await copyTextToClipboard(buildClipboardText(snapshot, state, { historical }));
      toast.success("恢复指令已复制", {
        description: "请在新会话中明确读取建议文件，并重新核验真实状态。",
      });
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  }

  async function handleCopy(
    snapshot: RecoverySnapshot,
    opener: HTMLButtonElement,
    fromHistory = false,
  ) {
    if (copying || !freshness || snapshot.projectId !== projectId) {
      return;
    }
    copyOpener.current = opener;
    setCopying(true);
    try {
      if (fromHistory) {
        await copySnapshot(snapshot, freshness, true);
        return;
      }
      const { data } = await overview.refetch({ throwOnError: true, cancelRefetch: false });
      if (!data) {
        throw new Error("无法检查恢复快照，请重试。");
      }
      const historical = data.latest?.id !== snapshot.id;
      if (historical || data.freshness.state === "stale") {
        setCopyRequest({ snapshot, freshness: data.freshness, historical });
      } else {
        await copySnapshot(snapshot, data.freshness, false);
      }
    } catch (error) {
      toast.error("无法检查恢复快照", { description: getErrorMessage(error) });
    } finally {
      setCopying(false);
    }
  }

  function handlePreview() {
    if (!latest) {
      return;
    }
    setInstructionsSnapshot(latest);
    setInstructionsHistorical(false);
    setInstructionsOpen(true);
  }

  async function handleSetNextTask(taskId: string | null) {
    if (!projectId) {
      return;
    }
    try {
      await setNextTask.mutateAsync({ projectId, taskId });
      const regenerated = await generate.mutateAsync();
      await overview.refetch().catch(() => undefined);
      toast.success(taskId ? "已指定下一步任务" : "已清除下一步指定", {
        description: `恢复包已更新，指纹 ${shortFingerprint(regenerated.fingerprint)}。`,
      });
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  }

  if (projectsPending) {
    return (
      <div className="space-y-5">
        <h1 className="sr-only">恢复上下文</h1>
        <LoadingState rows={4} label="正在读取项目" />
      </div>
    );
  }

  if (projectsError) {
    return (
      <div className="space-y-5">
        <h1 className="sr-only">恢复上下文</h1>
        <ErrorState title="无法读取项目" message={getErrorMessage(projectsErrorValue)} />
      </div>
    );
  }

  if (!currentProject) {
    return (
      <div className="space-y-5">
        <h1 className="sr-only">恢复上下文</h1>
        <EmptyState
          icon={History}
          title="还没有可恢复的项目"
          description="导入一个本地 Git 项目后，XUSHU 才能从真实任务、验收证据与项目记忆中生成恢复包。"
          action={
            <Button asChild size="sm">
              <Link to="/projects">导入项目</Link>
            </Button>
          }
        />
      </div>
    );
  }

  if (overview.isPending) {
    return (
      <div className="space-y-5">
        <h1 className="sr-only">恢复上下文</h1>
        <LoadingState rows={5} label="正在检查恢复快照" />
      </div>
    );
  }

  if (overview.isError || !overview.data) {
    return (
      <div className="space-y-5">
        <h1 className="sr-only">恢复上下文</h1>
        <ErrorState
          title="无法读取恢复上下文"
          message={getErrorMessage(overview.error)}
          onRetry={() => void overview.refetch()}
        />
      </div>
    );
  }

  const snapshot = overview.data.latest;
  const state = overview.data.freshness;
  const previewHistorical = instructionsHistorical || instructionsSnapshot?.id !== snapshot?.id;
  const pendingCopy = copyRequest?.snapshot.projectId === projectId ? copyRequest : null;
  const pkg = snapshot?.package;
  const nextStep = pkg?.nextStep;
  const sourceSummary = pkg?.sources;

  return (
    <div className="space-y-5">
      <h1 className="sr-only">恢复上下文</h1>

      <section aria-label="恢复快照状态" className="space-y-3 border-y border-line/70 py-3">
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2">
          <RecoveryFreshnessBadge state={state.state} />
          {snapshot ? (
            <>
              <span className="text-xs text-muted-foreground">
                生成于 {formatDateTime(snapshot.generatedAt)}
              </span>
              <span
                className="font-mono text-[11px] text-muted-foreground"
                title={snapshot.fingerprint}
              >
                {shortFingerprint(snapshot.fingerprint)}
              </span>
              <span className="text-[11px] text-muted-foreground">
                任务 {snapshot.taskCount} · 记忆 {snapshot.memoryCount} · 决策{" "}
                {snapshot.decisionCount}
              </span>
            </>
          ) : (
            <span className="text-xs text-muted-foreground">
              尚未生成恢复包；生成过程不调用外部模型。
            </span>
          )}
          <span className="ml-auto text-[11px] text-muted-foreground">{currentProject.name}</span>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            type="button"
            size="sm"
            onClick={() => void handleGenerate()}
            disabled={generate.isPending || setNextTask.isPending}
          >
            {generate.isPending ? (
              <Loader2 className="animate-spin" aria-hidden="true" />
            ) : snapshot ? (
              <RefreshCw aria-hidden="true" />
            ) : (
              <History aria-hidden="true" />
            )}
            {generate.isPending ? "生成中…" : snapshot ? "重新生成" : "生成恢复包"}
          </Button>
          <Button
            ref={instructionsOpener}
            type="button"
            variant="outline"
            size="sm"
            onClick={handlePreview}
            disabled={!snapshot}
          >
            <FileText aria-hidden="true" />
            预览完整指令
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={(event) => {
              if (snapshot) void handleCopy(snapshot, event.currentTarget);
            }}
            disabled={!snapshot || copying}
          >
            {copying ? (
              <Loader2 className="animate-spin" aria-hidden="true" />
            ) : (
              <ClipboardCopy aria-hidden="true" />
            )}
            {copying ? "检查中…" : "复制恢复指令"}
          </Button>
          <Button
            ref={historyOpener}
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => setHistoryOpen(true)}
            disabled={!snapshot}
          >
            <History aria-hidden="true" />
            历史快照
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => void overview.refetch()}
            disabled={overview.isFetching}
          >
            {overview.isFetching ? (
              <Loader2 className="animate-spin" aria-hidden="true" />
            ) : (
              <RefreshCw aria-hidden="true" />
            )}
            检查是否过期
          </Button>
        </div>

        <p className="text-[11px] leading-5 text-muted-foreground">
          复制内容包含本地项目路径、任务与验证状态、已确认记忆和决策；不会上传到外部服务。恢复包不是
          Claude Code 的会话记忆，复制后仍需在新会话中明确读取文件并重新核验。
        </p>
      </section>

      {state.state === "stale" ? (
        <Alert variant="warning">
          <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>这份快照已经过期</AlertTitle>
          <AlertDescription>
            {state.changed.length > 0
              ? `生成后发生变化：${state.changed.join("；")}。`
              : "生成后项目数据已变化。"}
            旧状态不能当作当前事实；建议先重新生成，再复制给新会话。
          </AlertDescription>
        </Alert>
      ) : null}

      {!snapshot || !pkg || !nextStep || !sourceSummary ? (
        <div className="space-y-5">
          <EmptyState
            icon={History}
            title="尚未生成恢复包"
            description="恢复包会从本地数据库、只读 Git 与独立验收结果确定性生成，并保存快照。没有配置 AI Provider 也可以使用。"
          />
          <div className="grid gap-5 sm:grid-cols-2">
            <Section>
              <SectionHeader title="恢复包会包含" meta="确定性生成" />
              <ul className="list-disc space-y-1.5 pl-4 text-xs leading-5 text-muted-foreground">
                <li>项目身份、当前项目状态与开发约束。</li>
                <li>实时 Git 分支、HEAD、工作区状态与代码指纹；失败时显示 UNKNOWN。</li>
                <li>进行中、阻塞、待验证任务与尚未满足的验收条件。</li>
                <li>最近独立验证结果、证据引用与过期原因。</li>
                <li>已导入的 Claude Code 元数据与会话结束确认状态。</li>
                <li>已确认的项目记忆与开发决策，以及未确认/已取代提示。</li>
                <li>下一步依据、建议读取的项目文件、冲突与缺失信息。</li>
              </ul>
            </Section>
            <Section>
              <SectionHeader title="数据边界" meta="本地优先" />
              <ul className="list-disc space-y-1.5 pl-4 text-xs leading-5 text-muted-foreground">
                <li>不读取 Claude Code transcript、Prompt、工具输入输出或密钥。</li>
                <li>不扫描项目文件内容，只检查白名单协作文件是否存在。</li>
                <li>记忆与决策不能替代任务状态、验收证据或完成门禁。</li>
                <li>未导入的 Hook 事件不会出现在恢复包中。</li>
                <li>验证结果按生成时的任务修订与代码状态计算，之后可能过期。</li>
              </ul>
            </Section>
          </div>
        </div>
      ) : (
        <>
          <section
            aria-label="下一步"
            className="grid min-w-0 gap-3 border-l-2 border-primary bg-surface-raised px-4 py-3 min-[900px]:grid-cols-[minmax(0,1fr)_auto]"
          >
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-xs font-medium text-primary">下一步</h2>
                <NextStepKindBadge kind={nextStep.kind} />
                {nextStep.taskStatus ? <StatusBadge status={nextStep.taskStatus} /> : null}
                {nextStep.requiresConfirmation ? (
                  <span className="text-[11px] text-warning">需要你确认</span>
                ) : null}
              </div>
              <p className="mt-1 break-words text-lg font-semibold tracking-[-0.01em]">
                {nextStep.taskTitle ?? "尚未确定下一步"}
              </p>
              <p className="mt-1 max-w-2xl break-words text-xs leading-5 text-muted-foreground">
                {nextStep.reason}
              </p>
            </div>
            <div className="flex min-w-0 flex-wrap items-center gap-2 self-start">
              {selectableTasks.length > 0 ? (
                <>
                  <Select value={nextTaskSelection} onValueChange={setNextTaskSelection}>
                    <SelectTrigger className="h-7 w-[min(260px,60vw)]" aria-label="选择下一步任务">
                      <SelectValue placeholder="选择任务" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={NO_TASK}>不指定</SelectItem>
                      {selectableTasks.map((task) => (
                        <SelectItem key={task.id} value={task.id}>
                          {task.title}（{TASK_STATUS_META[task.status].label}）
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <Button
                    type="button"
                    size="sm"
                    disabled={
                      setNextTask.isPending ||
                      generate.isPending ||
                      nextTaskSelection === NO_TASK ||
                      (nextStep.kind === "user_selected" && nextTaskSelection === nextStep.taskId)
                    }
                    onClick={() => void handleSetNextTask(nextTaskSelection)}
                  >
                    设为下一步
                  </Button>
                </>
              ) : (
                <Button asChild variant="outline" size="sm">
                  <Link to="/tasks">打开任务列表</Link>
                </Button>
              )}
              {nextStep.kind === "user_selected" ||
              pkg.conflicts.some((conflict) => conflict.kind === "next_task_unavailable") ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  disabled={setNextTask.isPending || generate.isPending}
                  onClick={() => void handleSetNextTask(null)}
                >
                  清除指定
                </Button>
              ) : null}
            </div>
          </section>

          <div className="grid min-w-0 items-start gap-6 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
            <div className="min-w-0 space-y-6">
              <Section>
                <SectionHeader
                  title="项目与 Git"
                  meta={`数据读取 ${formatDateTime(sourceSummary.databaseReadAt)}`}
                />
                <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs">
                  <span className="inline-flex items-center gap-1.5">
                    <FolderGit2 className="size-3.5 text-muted-foreground" aria-hidden="true" />
                    <span className="font-medium">{pkg.project.name}</span>
                  </span>
                  <span
                    className="min-w-0 max-w-[min(100%,520px)] truncate font-mono text-[11px] text-muted-foreground"
                    title={pkg.project.rootPath}
                  >
                    {pkg.project.rootPath}
                  </span>
                  {pkg.project.archived ? (
                    <Badge variant="outline">已归档，只读</Badge>
                  ) : (
                    <Badge variant="success">进行中</Badge>
                  )}
                </div>
                <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-line/60 pt-2 text-xs">
                  <RecoveryGitBadge state={pkg.git.state} />
                  {pkg.git.state === "available" ? (
                    <>
                      <span className="inline-flex items-center gap-1.5">
                        <GitBranch className="size-3.5 text-muted-foreground" aria-hidden="true" />
                        <span className="font-mono text-[11px]">{pkg.git.branch ?? "UNKNOWN"}</span>
                      </span>
                      <span className="font-mono text-[11px] text-muted-foreground">
                        {shortHead(pkg.git.head)}
                      </span>
                      <span className={pkg.git.dirty ? "text-warning" : "text-success"}>
                        {pkg.git.dirty ? "工作区存在未提交修改" : "工作区干净"}
                      </span>
                    </>
                  ) : (
                    <span className="text-warning">无法读取 Git 状态</span>
                  )}
                  <span className="text-[11px] text-muted-foreground">
                    检查于 {formatDateTime(pkg.git.checkedAt)}
                  </span>
                </div>
                {pkg.git.workspaceFingerprint ? (
                  <p className="font-mono text-[11px] text-muted-foreground">
                    代码指纹 {shortFingerprint(pkg.git.workspaceFingerprint)}
                    {pkg.git.fingerprintComplete ? "（完整）" : "（不完整）"}
                  </p>
                ) : null}
                {pkg.git.note ? (
                  <p className="text-[11px] leading-5 text-warning">{pkg.git.note}</p>
                ) : null}
              </Section>

              <Section>
                <SectionHeader
                  title="任务"
                  meta={`${sourceSummary.activeTaskCount} 条未完成 · ${pkg.tasks.draftCount} 个草稿`}
                />
                <div className="space-y-3">
                  <TaskGroup title="执行中" tasks={pkg.tasks.inProgress} />
                  <TaskGroup title="已就绪" tasks={pkg.tasks.ready} />
                  <TaskGroup title="已阻塞" tasks={pkg.tasks.blocked} tone="warning" />
                  <TaskGroup title="待验证" tasks={pkg.tasks.awaitingVerification} />
                  <TaskGroup title="最近完成且证据有效" tasks={pkg.tasks.doneWithValidEvidence} />
                  <TaskGroup
                    title="最近完成但证据无效或已过期"
                    tasks={pkg.tasks.doneWithInvalidEvidence}
                    tone="warning"
                  />
                  {pkg.tasks.inProgress.length === 0 &&
                  pkg.tasks.ready.length === 0 &&
                  pkg.tasks.blocked.length === 0 &&
                  pkg.tasks.awaitingVerification.length === 0 &&
                  pkg.tasks.doneWithValidEvidence.length === 0 &&
                  pkg.tasks.doneWithInvalidEvidence.length === 0 ? (
                    <p className="text-xs text-muted-foreground">没有已记录的任务状态。</p>
                  ) : null}
                </div>
              </Section>

              <Section>
                <SectionHeader title="独立验证" meta="按生成时状态；之后可能过期" />
                {pkg.verification.length === 0 ? (
                  <p className="text-xs text-muted-foreground">
                    尚无独立验证运行记录；任何完成声明都需要证据。
                  </p>
                ) : (
                  <ul className="divide-y divide-line/60 border-y border-line/70">
                    {pkg.verification.map((item) => (
                      <li
                        key={`${item.runId ?? item.criterionId}-${item.startedAt ?? ""}`}
                        className="py-2"
                      >
                        <div className="flex min-w-0 flex-wrap items-center gap-2">
                          <VerificationBadge status={item.status} />
                          <Link
                            to={`/tasks/${item.taskId}`}
                            className="min-w-0 flex-1 truncate text-xs font-medium underline-offset-4 hover:text-primary hover:underline"
                            title={item.criterionDescription}
                          >
                            {item.criterionDescription}
                          </Link>
                          <span className="font-mono text-[11px] text-muted-foreground">
                            退出码 {item.exitCode ?? "—"}
                          </span>
                        </div>
                        <p className="mt-0.5 text-[11px] text-muted-foreground">
                          {item.taskTitle} · {formatDateTime(item.startedAt)}
                          {item.executionStatus ? ` · ${item.executionStatus}` : ""}
                        </p>
                        {(item.staleReason ?? item.detail) ? (
                          <p className="mt-0.5 whitespace-pre-wrap break-words text-[11px] leading-5 text-warning">
                            {item.staleReason ?? item.detail}
                          </p>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                )}
              </Section>
            </div>

            <div className="min-w-0 space-y-6">
              <Section>
                <SectionHeader title="Claude Code 活动" meta="仅已导入元数据" />
                <div className="flex flex-wrap items-center gap-2 text-xs">
                  <Badge variant="outline">{pkg.claude.eventCount} 条事件</Badge>
                  <Badge variant="outline">{pkg.claude.sessionCount} 个会话</Badge>
                  {pkg.claude.sessionCount > 0 ? (
                    pkg.claude.endConfirmed ? (
                      <Badge variant="success">结束已确认</Badge>
                    ) : (
                      <Badge variant="warning">结束未确认</Badge>
                    )
                  ) : (
                    <Badge variant="outline">尚无活动</Badge>
                  )}
                </div>
                {pkg.claude.lastSessionRef ? (
                  <p className="font-mono text-[11px] text-muted-foreground">
                    最近会话 {pkg.claude.lastSessionRef}
                    {pkg.claude.lastSessionLastEventAt
                      ? ` · ${formatDateTime(pkg.claude.lastSessionLastEventAt)}`
                      : ""}
                  </p>
                ) : null}
                <p className="text-[11px] text-muted-foreground">
                  失败 {pkg.claude.lastSessionFailedToolCount} · 未结束{" "}
                  {pkg.claude.lastSessionRunningToolCount} · 状态未知{" "}
                  {pkg.claude.lastSessionUnknownToolCount}
                </p>
                {pkg.claude.note ? (
                  <p className="text-[11px] leading-5 text-muted-foreground">{pkg.claude.note}</p>
                ) : null}
              </Section>

              <Section>
                <SectionHeader
                  title="已确认记忆"
                  meta={`${pkg.memories.length} 条 · 不替代事实`}
                  actions={
                    <Button asChild variant="ghost" size="sm">
                      <Link to={`/projects/${pkg.project.id}/knowledge?tab=memory`}>
                        <ArrowRight aria-hidden="true" />
                        管理
                      </Link>
                    </Button>
                  }
                />
                {pkg.memories.length === 0 ? (
                  <p className="text-xs text-muted-foreground">尚无已确认的项目记忆。</p>
                ) : (
                  <div className="divide-y divide-line/60 border-y border-line/70">
                    {pkg.memories.slice(0, 8).map((memory) => (
                      <MemoryPreviewRow key={memory.id} memory={memory} />
                    ))}
                  </div>
                )}
                {pkg.memories.length > 8 ? (
                  <p className="text-[11px] text-muted-foreground">
                    其余 {pkg.memories.length - 8} 条已确认记忆请在项目上下文中查看。
                  </p>
                ) : null}
                {pkg.unconfirmedMemories.length > 0 ? (
                  <Alert variant="warning">
                    <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
                    <AlertTitle>未确认记忆不作为事实</AlertTitle>
                    <AlertDescription>
                      {pkg.unconfirmedMemories
                        .slice(0, 5)
                        .map((memory) => memory.title)
                        .join("；")}
                      {pkg.unconfirmedMemories.length > 5 ? " 等" : ""}。它们未进入已确认上下文。
                    </AlertDescription>
                  </Alert>
                ) : null}
              </Section>

              <Section>
                <SectionHeader
                  title="开发决策"
                  meta={`${pkg.decisions.length} 条已确认`}
                  actions={
                    <Button asChild variant="ghost" size="sm">
                      <Link to={`/projects/${pkg.project.id}/knowledge?tab=decisions`}>
                        <ArrowRight aria-hidden="true" />
                        管理
                      </Link>
                    </Button>
                  }
                />
                {pkg.decisions.length === 0 ? (
                  <p className="text-xs text-muted-foreground">尚无已确认的开发决策。</p>
                ) : (
                  <div className="divide-y divide-line/60 border-y border-line/70">
                    {pkg.decisions.slice(0, 6).map((decision) => (
                      <div key={decision.id} className="py-2">
                        <div className="flex min-w-0 flex-wrap items-center gap-2">
                          <DecisionStatusBadge status={decision.status} />
                          <span
                            className="min-w-0 flex-1 truncate text-xs font-medium"
                            title={decision.title}
                          >
                            {decision.title}
                          </span>
                          <span className="font-mono text-[11px] text-muted-foreground">
                            r{decision.revision}
                          </span>
                        </div>
                        <p className="mt-0.5 line-clamp-2 whitespace-pre-wrap break-words text-[11px] leading-5 text-muted-foreground">
                          {decision.decision}
                        </p>
                      </div>
                    ))}
                  </div>
                )}
                {pkg.supersededDecisions.length > 0 ? (
                  <div className="space-y-1 border-t border-line/60 pt-2">
                    <p className="text-[11px] font-medium text-muted-foreground">
                      已被取代或废弃的决策
                    </p>
                    {pkg.supersededDecisions.slice(0, 5).map((notice) => (
                      <p key={notice.id} className="text-[11px] leading-5 text-muted-foreground">
                        {notice.title} · {notice.note}
                        {notice.supersededByTitle ? ` 新决策：${notice.supersededByTitle}。` : ""}
                      </p>
                    ))}
                  </div>
                ) : null}
              </Section>

              <Section>
                <SectionHeader title="建议读取的项目文件" meta="仅检查是否存在" />
                {pkg.filesToRead.length === 0 ? (
                  <p className="text-xs text-muted-foreground">
                    未在项目根目录发现常见协作文件；请手动指定。
                  </p>
                ) : (
                  <ul className="space-y-1 text-xs">
                    {pkg.filesToRead.map((file) => (
                      <li key={file} className="break-all font-mono text-[11px]">
                        {file}
                      </li>
                    ))}
                  </ul>
                )}
              </Section>

              <Section>
                <SectionHeader title="冲突与缺失信息" meta="优先呈现风险" />
                {pkg.conflicts.length > 0 ? (
                  <Alert variant="warning">
                    <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
                    <AlertTitle>需要复核的冲突</AlertTitle>
                    <AlertDescription>
                      <ul className="list-disc space-y-1 pl-4">
                        {pkg.conflicts.map((conflict) => (
                          <li key={`${conflict.kind}-${conflict.subject}`}>
                            {conflictLabel(conflict.kind)}：{conflict.subject} — {conflict.detail}
                          </li>
                        ))}
                      </ul>
                    </AlertDescription>
                  </Alert>
                ) : null}
                {pkg.missingInformation.length > 0 ? (
                  <Alert variant="info">
                    <ShieldCheck className="mt-0.5 size-4" aria-hidden="true" />
                    <AlertTitle>缺失信息与不确定性</AlertTitle>
                    <AlertDescription>
                      <ul className="list-disc space-y-1 pl-4">
                        {pkg.missingInformation.map((item) => (
                          <li key={item}>{item}</li>
                        ))}
                      </ul>
                    </AlertDescription>
                  </Alert>
                ) : (
                  <p className="text-xs text-muted-foreground">未记录到缺失信息。</p>
                )}
              </Section>
            </div>
          </div>

          <details className="border-y border-line/70 py-3">
            <summary className="cursor-pointer rounded-sm text-[13px] font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
              数据来源、包含上限与排除项
            </summary>
            <div className="mt-3 grid gap-5 text-xs sm:grid-cols-2">
              <div className="space-y-1 text-muted-foreground">
                <p>数据库读取：{formatDateTime(sourceSummary.databaseReadAt)}</p>
                <p>Git 检查：{formatDateTime(sourceSummary.gitCheckedAt)}</p>
                <p>
                  任务：全部 {sourceSummary.taskCandidateCount} 条；未完成{" "}
                  {sourceSummary.activeTaskCount}；已完成 {sourceSummary.doneTaskCount}
                </p>
                <p>
                  记忆：已确认 {sourceSummary.memoryConfirmedActiveCount}；尚未确认{" "}
                  {sourceSummary.memoryUnconfirmedActiveCount}；已归档{" "}
                  {sourceSummary.memoryArchivedCount}
                </p>
                <p>
                  决策：已确认 {sourceSummary.decisionConfirmedCount}；草稿{" "}
                  {sourceSummary.decisionDraftCount}；已被取代{" "}
                  {sourceSummary.decisionSupersededCount}；已废弃{" "}
                  {sourceSummary.decisionDeprecatedCount}
                </p>
                <p>
                  验证运行：{sourceSummary.verificationRunCount}；Claude Code 事件：{" "}
                  {sourceSummary.claudeEventCount}；会话：{sourceSummary.claudeSessionCount}
                </p>
              </div>
              <div className="space-y-2 text-muted-foreground">
                <div>
                  <p className="font-medium text-foreground">包含上限</p>
                  <ul className="mt-1 list-disc space-y-1 pl-4">
                    {sourceSummary.includedLimits.map((limit) => (
                      <li key={limit}>{limit}</li>
                    ))}
                  </ul>
                </div>
                <div>
                  <p className="font-medium text-foreground">排除项</p>
                  <ul className="mt-1 list-disc space-y-1 pl-4">
                    {sourceSummary.exclusions.map((exclusion) => (
                      <li key={exclusion}>{exclusion}</li>
                    ))}
                  </ul>
                </div>
              </div>
            </div>
          </details>

          <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-line/70 pt-3 text-[11px] text-muted-foreground">
            <span className="inline-flex items-center gap-1.5">
              <Brain className="size-3.5" aria-hidden="true" />
              记忆与决策是上下文，不是验收证据。
            </span>
            <span className="inline-flex items-center gap-1.5">
              <ShieldCheck className="size-3.5" aria-hidden="true" />
              完成状态仍以独立验证与人工确认为准。
            </span>
            <span className="inline-flex items-center gap-1.5">
              <Activity className="size-3.5" aria-hidden="true" />
              Claude Code 事件不代表任务完成。
            </span>
            <span className="inline-flex items-center gap-1.5">
              <Scale className="size-3.5" aria-hidden="true" />
              过期验证不会写成“当前通过”。
            </span>
          </div>
        </>
      )}

      {instructionsSnapshot ? (
        <RecoveryInstructionsSheet
          snapshot={instructionsSnapshot}
          freshness={state}
          open={instructionsOpen}
          onOpenChange={setInstructionsOpen}
          restoreFocus={() =>
            (instructionsHistorical ? historyOpener : instructionsOpener).current?.focus()
          }
          historical={previewHistorical}
          copying={copying}
          onCopy={(opener) => void handleCopy(instructionsSnapshot, opener, instructionsHistorical)}
        />
      ) : null}

      <RecoveryHistorySheet
        projectId={currentProject.id}
        open={historyOpen}
        onOpenChange={(open) => {
          setHistoryOpen(open);
          if (!open) {
            setHistorySnapshotId(null);
          }
        }}
        currentSnapshotId={snapshot?.id ?? null}
        pendingSnapshotId={historySnapshot.isFetching ? historySnapshotId : null}
        onSelect={(snapshotId) => setHistorySnapshotId(snapshotId)}
        restoreFocus={() => historyOpener.current?.focus()}
      />

      <ConfirmationDialog
        open={Boolean(pendingCopy)}
        onOpenChange={(open) => {
          if (!open) setCopyRequest(null);
        }}
        title={pendingCopy?.historical ? "复制历史恢复指令？" : "复制已过期的恢复指令？"}
        description={
          pendingCopy?.historical
            ? "这份快照已不是最新快照。复制文本会标注为历史状态。"
            : `${pendingCopy?.freshness.changed.join("；") || "生成后项目数据已变化"}。复制文本会标注为过期；建议先重新生成。`
        }
        confirmLabel="仍然复制"
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          copyOpener.current?.focus();
        }}
        onConfirm={() => {
          setCopyRequest(null);
          if (pendingCopy) {
            void copySnapshot(pendingCopy.snapshot, pendingCopy.freshness, pendingCopy.historical);
          }
        }}
      />
    </div>
  );
}
