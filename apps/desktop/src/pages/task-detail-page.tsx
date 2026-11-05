import { Archive, ArchiveRestore, ArrowLeft, FolderGit2, Pencil, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { PageHeader } from "@/components/page-header";
import { StateSpine } from "@/components/state-spine";
import { StateTrack } from "@/components/state-track";
import { PriorityBadge, StatusBadge } from "@/components/status-badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { CriterionList } from "@/features/tasks/criterion-list";
import { EditTaskDialog } from "@/features/tasks/edit-task-dialog";
import { useArchiveTask, useRestoreTask, useTaskDetail } from "@/features/tasks/queries";
import { TransitionPanel } from "@/features/tasks/transition-panel";
import { useProject } from "@/features/projects/queries";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

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
              : "先处理未通过的验收条件；必需自动条件需要等待 Phase 3。"
            : status === "blocked"
              ? blockReason
                ? `解除阻塞：${blockReason}`
                : "补充阻塞原因并解除阻塞。"
              : status === "cancelled"
                ? "如仍需推进，请使用“重新开启任务”并填写原因。"
                : "任务已完成。如需返工，请创建新任务。";

  return <p className="text-sm leading-6 text-muted-foreground">{text}</p>;
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
      <div className="space-y-6">
        <PageHeader title="任务详情" description="加载任务与验收条件…" />
        <LoadingState rows={5} />
      </div>
    );
  }

  if (detail.isError || !detail.data) {
    return (
      <div className="space-y-6">
        <Button asChild variant="ghost" size="sm">
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
        description: "任务数据保留，可随时恢复。",
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
        <Button asChild variant="ghost" size="sm" className="-ml-2 mb-2">
          <Link to="/tasks">
            <ArrowLeft aria-hidden="true" />
            返回任务列表
          </Link>
        </Button>
        <PageHeader
          title={task.title}
          description={`${project.data?.name ?? "未知项目"} · 修订 ${task.revision} · 更新于 ${formatDateTime(task.updatedAt)}`}
          actions={
            <>
              {!archived && !terminal ? (
                <Button type="button" variant="outline" size="sm" onClick={() => setEditOpen(true)}>
                  <Pencil aria-hidden="true" />
                  编辑任务
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
                  恢复任务
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setArchiveOpen(true)}
                >
                  <Archive aria-hidden="true" />
                  归档任务
                </Button>
              )}
            </>
          }
        />
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <StatusBadge status={task.status} />
          <PriorityBadge priority={task.priority} />
          <Button asChild variant="link" size="sm" className="h-auto p-0">
            <Link to={`/projects/${task.projectId}`}>
              <FolderGit2 aria-hidden="true" />
              打开项目
            </Link>
          </Button>
          {archived ? <span className="text-xs text-muted-foreground">已归档</span> : null}
          {projectArchived ? (
            <span className="text-xs text-muted-foreground">所属项目已归档</span>
          ) : null}
        </div>
      </div>

      <Card>
        <CardContent className="p-4">
          <StateTrack status={task.status} />
        </CardContent>
      </Card>

      <div className="grid items-start gap-5 xl:grid-cols-[1.55fr_1fr]">
        <div className="space-y-5">
          <Card>
            <CardHeader>
              <CardTitle>目标</CardTitle>
              <CardDescription>任务的执行范围与背景。</CardDescription>
            </CardHeader>
            <CardContent>
              {task.description ? (
                <p className="whitespace-pre-wrap break-words text-sm leading-6">
                  {task.description}
                </p>
              ) : (
                <p className="text-sm text-muted-foreground">尚未填写任务描述。</p>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardContent className="p-5">
              <CriterionList detail={detail.data} readOnly={criteriaReadOnly} />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>状态历史</CardTitle>
              <CardDescription>状态变化、原因与对应任务修订。</CardDescription>
            </CardHeader>
            <CardContent>
              <StateSpine transitions={detail.data.transitions} />
            </CardContent>
          </Card>
        </div>

        <div className="space-y-5">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ShieldCheck className="size-4 text-primary" aria-hidden="true" />
                完成门禁
              </CardTitle>
              <CardDescription>只有必需验收条件全部满足时才允许完成。</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between gap-3 text-sm">
                <span className="text-muted-foreground">必需条件</span>
                <span className="font-mono">
                  {detail.data.completion.satisfiedRequiredCount}/
                  {detail.data.completion.requiredCount}
                </span>
              </div>

              {detail.data.completion.canComplete ? (
                <Alert variant="success">
                  <ShieldCheck className="mt-0.5 size-4" aria-hidden="true" />
                  <AlertTitle>满足完成条件</AlertTitle>
                  <AlertDescription>
                    {task.status === "awaiting_verification"
                      ? "可以标记完成。"
                      : "所有门禁已满足，但任务仍需按状态机推进。"}
                  </AlertDescription>
                </Alert>
              ) : (
                <Alert variant="warning">
                  <ShieldCheck className="mt-0.5 size-4" aria-hidden="true" />
                  <AlertTitle>尚不能完成</AlertTitle>
                  <AlertDescription>
                    <ul className="list-disc space-y-1 pl-4">
                      {detail.data.completion.blockers.map((blocker) => (
                        <li key={blocker}>{blocker}</li>
                      ))}
                    </ul>
                  </AlertDescription>
                </Alert>
              )}

              <div className="rounded-md border bg-surface-raised/40 p-3">
                <p className="text-xs font-medium text-muted-foreground">下一步</p>
                <div className="mt-1">
                  <NextStep
                    status={task.status}
                    canComplete={detail.data.completion.canComplete}
                    blockReason={task.blockReason}
                  />
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>状态操作</CardTitle>
              <CardDescription>所有转换均由 Rust 校验并写入历史。</CardDescription>
            </CardHeader>
            <CardContent>
              <TransitionPanel detail={detail.data} projectArchived={projectArchived} />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>元数据</CardTitle>
            </CardHeader>
            <CardContent>
              <dl className="space-y-3 text-sm">
                {[
                  ["任务 ID", task.id],
                  ["修订号", String(task.revision)],
                  ["创建时间", formatDateTime(task.createdAt)],
                  ["更新时间", formatDateTime(task.updatedAt)],
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
              {task.blockReason ? (
                <div className="mt-4 rounded-md border border-destructive/35 bg-destructive/5 p-3">
                  <p className="text-xs font-medium text-destructive">当前阻塞原因</p>
                  <p className="mt-1 whitespace-pre-wrap text-xs leading-5 text-destructive">
                    {task.blockReason}
                  </p>
                </div>
              ) : null}
            </CardContent>
          </Card>
        </div>
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
