import {
  AlertCircle,
  Archive,
  ArchiveRestore,
  ArrowLeft,
  CheckCircle2,
  FolderGit2,
  Pencil,
  ShieldCheck,
} from "lucide-react";
import { useState } from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { Section, SectionHeader } from "@/components/section";
import { StateSpine } from "@/components/state-spine";
import { StateTrack } from "@/components/state-track";
import { PriorityBadge, StatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { TaskKnowledgeSection } from "@/features/knowledge/task-knowledge-section";
import { CriterionList } from "@/features/tasks/criterion-list";
import { EditTaskDialog } from "@/features/tasks/edit-task-dialog";
import { useArchiveTask, useRestoreTask, useTaskDetail } from "@/features/tasks/queries";
import { TransitionPanel } from "@/features/tasks/transition-panel";
import { useProject } from "@/features/projects/queries";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";

function NextStep({
  status,
  canComplete,
  blockReason,
}: {
  status: string;
  canComplete: boolean;
  blockReason: string | null;
}) {
  const text =
    status === "draft"
      ? "添加至少一个必需验收条件，然后将任务进入就绪。"
      : status === "ready"
        ? "开始执行任务，并将状态推进到执行中。"
        : status === "running"
          ? "执行完成后进入待验证。"
          : status === "awaiting_verification"
            ? canComplete
              ? "必需验收条件已满足，可以标记完成。"
              : "先处理未通过的验收条件；必需自动条件需要先配置并运行验证，取得当前有效的 PASS 证据。"
            : status === "blocked"
              ? blockReason
                ? `解除阻塞：${blockReason}`
                : "补充阻塞原因并解除阻塞。"
              : status === "cancelled"
                ? "如仍需推进，请使用“重新开启任务”并填写原因。"
                : "任务已完成。如需返工，请创建新任务。";

  return <p className="text-[11px] leading-5 text-muted-foreground">{text}</p>;
}

export function TaskDetailPage() {
  const { taskId } = useParams();
  const detail = useTaskDetail(taskId);
  const project = useProject(detail.data?.task.projectId);
  const archiveTask = useArchiveTask();
  const restoreTask = useRestoreTask();
  const [editOpen, setEditOpen] = useState(false);
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [restoreOpen, setRestoreOpen] = useState(false);

  if (detail.isPending) {
    return (
      <div className="space-y-5">
        <PageHeader title="任务详情" />
        <LoadingState rows={5} />
      </div>
    );
  }

  if (detail.isError || !detail.data) {
    return (
      <div className="space-y-5">
        <Button asChild variant="ghost" size="sm" className="-ml-2">
          <Link to="/tasks">
            <ArrowLeft aria-hidden="true" />
            返回任务列表
          </Link>
        </Button>
        <ErrorState
          title="无法读取任务"
          message={detail.isError ? getErrorMessage(detail.error) : "任务不存在"}
        />
      </div>
    );
  }

  const task = detail.data.task;
  const projectArchived = Boolean(project.data?.archivedAt);
  const archived = Boolean(task.archivedAt);
  const terminal = task.status === "done" || task.status === "cancelled";
  const criteriaReadOnly = archived || projectArchived || terminal;

  const archive = async () => {
    try {
      await archiveTask.mutateAsync({
        taskId: task.id,
        expectedRevision: task.revision,
      });
      toast.success("任务已归档", {
        description: "任务数据与验收历史保留，可随时恢复。",
      });
      setArchiveOpen(false);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  const restore = async () => {
    try {
      await restoreTask.mutateAsync({
        taskId: task.id,
        expectedRevision: task.revision,
      });
      toast.success("任务已恢复");
      setRestoreOpen(false);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <div className="space-y-5">
      <div>
        <Button asChild variant="ghost" size="sm" className="-ml-2 mb-1">
          <Link to="/tasks">
            <ArrowLeft aria-hidden="true" />
            返回任务列表
          </Link>
        </Button>
        <PageHeader
          title={task.title}
          meta={
            <>
              <Link
                to={`/projects/${task.projectId}`}
                className="inline-flex items-center gap-1 underline-offset-4 hover:text-primary hover:underline"
              >
                <FolderGit2 className="size-3" aria-hidden="true" />
                {project.data?.name ?? "未知项目"}
              </Link>
              <span aria-hidden="true">·</span>
              <StatusBadge status={task.status} />
              <PriorityBadge priority={task.priority} />
              <span className="font-mono text-[10px]">r{task.revision}</span>
              <span>更新于 {formatDateTime(task.updatedAt)}</span>
              {archived ? <span className="text-warning">已归档</span> : null}
              {projectArchived ? <span className="text-warning">项目已归档</span> : null}
            </>
          }
          actions={
            <>
              {task.status === "awaiting_verification" && !archived && !projectArchived ? (
                <Button asChild size="sm">
                  <Link to={`/verification?task=${task.id}`}>
                    <ShieldCheck aria-hidden="true" />
                    进入验收
                  </Link>
                </Button>
              ) : null}
              {!archived && !terminal ? (
                <Button type="button" variant="outline" size="sm" onClick={() => setEditOpen(true)}>
                  <Pencil aria-hidden="true" />
                  编辑
                </Button>
              ) : null}
              {archived ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setRestoreOpen(true)}
                >
                  <ArchiveRestore aria-hidden="true" />
                  恢复
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => setArchiveOpen(true)}
                >
                  <Archive aria-hidden="true" />
                  归档
                </Button>
              )}
            </>
          }
        />
      </div>

      <div className="border-y border-line/70 py-3">
        <StateTrack status={task.status} />
      </div>

      <div className="grid min-w-0 items-start gap-6 xl:grid-cols-[minmax(0,1.55fr)_minmax(280px,0.9fr)]">
        <div className="min-w-0 space-y-6">
          <Section>
            <SectionHeader title="目标" />
            {task.description ? (
              <p className="whitespace-pre-wrap break-words text-sm leading-6">
                {task.description}
              </p>
            ) : (
              <p className="text-xs text-muted-foreground">尚未填写任务描述。</p>
            )}
          </Section>

          <div className="border-t border-line/70 pt-5">
            <CriterionList detail={detail.data} readOnly={criteriaReadOnly} />
          </div>

          <div className="border-t border-line/70 pt-5">
            <TaskKnowledgeSection projectId={task.projectId} taskId={task.id} />
          </div>

          <Section className="border-t border-line/70 pt-5">
            <SectionHeader title="状态历史" meta={`${detail.data.transitions.length} 条`} />
            <StateSpine transitions={detail.data.transitions} />
          </Section>
        </div>

        <aside className="min-w-0 space-y-6 xl:border-l xl:border-line/70 xl:pl-6">
          <Section>
            <SectionHeader
              title="完成门禁"
              meta={`${detail.data.completion.satisfiedRequiredCount}/${detail.data.completion.requiredCount} 必需条件`}
            />
            <div
              className={cn(
                "border-l-2 px-4 py-3",
                detail.data.completion.canComplete
                  ? "border-success bg-success/5"
                  : "border-warning bg-warning/5",
              )}
            >
              <div className="flex items-center gap-2 text-[13px] font-medium">
                {detail.data.completion.canComplete ? (
                  <CheckCircle2 className="size-4 text-success" aria-hidden="true" />
                ) : (
                  <AlertCircle className="size-4 text-warning" aria-hidden="true" />
                )}
                <span
                  className={detail.data.completion.canComplete ? "text-success" : "text-warning"}
                >
                  {detail.data.completion.canComplete ? "满足完成条件" : "尚不能完成"}
                </span>
              </div>
              {detail.data.completion.canComplete ? (
                <p className="mt-1 text-[11px] leading-5 text-muted-foreground">
                  {task.status === "awaiting_verification"
                    ? "可以标记完成。"
                    : "门禁已满足，但仍需按状态机推进。"}
                </p>
              ) : (
                <ul className="mt-1 list-disc space-y-1 pl-4 text-[11px] leading-5 text-muted-foreground">
                  {detail.data.completion.blockers.map((blocker) => (
                    <li key={blocker}>{blocker}</li>
                  ))}
                </ul>
              )}
              <div className="mt-2 border-t border-line/60 pt-2">
                <NextStep
                  status={task.status}
                  canComplete={detail.data.completion.canComplete}
                  blockReason={task.blockReason}
                />
              </div>
            </div>
          </Section>

          <Section>
            <SectionHeader title="状态操作" meta="由 Rust 校验" />
            <TransitionPanel detail={detail.data} projectArchived={projectArchived} />
          </Section>

          <Section>
            <SectionHeader title="元数据" />
            <dl className="divide-y divide-line/60 border-t border-line/70 text-xs">
              {[
                ["任务 ID", task.id],
                ["修订号", String(task.revision)],
                ["创建时间", formatDateTime(task.createdAt)],
                ["更新时间", formatDateTime(task.updatedAt)],
              ].map(([label, value]) => (
                <div key={label} className="flex items-start justify-between gap-4 py-2">
                  <dt className="shrink-0 text-muted-foreground">{label}</dt>
                  <dd className="min-w-0 break-all text-right font-mono text-[11px]">{value}</dd>
                </div>
              ))}
            </dl>
            {task.blockReason ? (
              <div className="border-l-2 border-destructive bg-destructive/5 px-3 py-2">
                <p className="text-[10px] font-medium text-destructive">当前阻塞原因</p>
                <p className="mt-0.5 whitespace-pre-wrap text-[11px] leading-5 text-destructive">
                  {task.blockReason}
                </p>
              </div>
            ) : null}
          </Section>
        </aside>
      </div>

      {editOpen ? (
        <EditTaskDialog
          open
          onOpenChange={setEditOpen}
          task={task}
          onUpdated={() => void detail.refetch()}
        />
      ) : null}

      <ConfirmationDialog
        open={archiveOpen}
        onOpenChange={setArchiveOpen}
        title="归档任务"
        description="任务会从默认列表隐藏，但数据、验收条件与历史均保留，可随时恢复。"
        confirmLabel="归档任务"
        destructive
        pending={archiveTask.isPending}
        onConfirm={() => void archive()}
      />
      <ConfirmationDialog
        open={restoreOpen}
        onOpenChange={setRestoreOpen}
        title="恢复任务"
        description="任务会重新出现在默认列表中，原有状态与验收条件保持不变。"
        confirmLabel="恢复任务"
        pending={restoreTask.isPending}
        onConfirm={() => void restore()}
      />
    </div>
  );
}
