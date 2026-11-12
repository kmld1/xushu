import { Archive, ArchiveRestore, MoreHorizontal, SquareArrowOutUpRight } from "lucide-react";
import { useState } from "react";
import { Link, useNavigate } from "react-router";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { PriorityBadge, StatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useArchiveTask, useRestoreTask } from "@/features/tasks/queries";
import type { Project, Task } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

function TaskActionsMenu({ task }: { task: Task }) {
  const navigate = useNavigate();
  const archiveTask = useArchiveTask();
  const restoreTask = useRestoreTask();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const archived = Boolean(task.archivedAt);

  const archive = async () => {
    try {
      await archiveTask.mutateAsync({ taskId: task.id, expectedRevision: task.revision });
      toast.success("任务已归档");
      setConfirmOpen(false);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  const restore = async () => {
    try {
      await restoreTask.mutateAsync({ taskId: task.id, expectedRevision: task.revision });
      toast.success("任务已恢复");
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7 text-muted-foreground"
            aria-label={`任务操作：${task.title}`}
          >
            <MoreHorizontal className="size-4" aria-hidden="true" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onSelect={() => navigate(`/tasks/${task.id}`)}>
            <SquareArrowOutUpRight className="size-3.5" aria-hidden="true" />
            打开任务
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          {archived ? (
            <DropdownMenuItem onSelect={() => void restore()}>
              <ArchiveRestore className="size-3.5" aria-hidden="true" />
              恢复任务
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem destructive onSelect={() => setConfirmOpen(true)}>
              <Archive className="size-3.5" aria-hidden="true" />
              归档任务
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      <ConfirmationDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title="归档任务"
        description="任务会从默认列表隐藏，但数据、验收条件、验证历史与证据均保留，可随时恢复。"
        confirmLabel="归档任务"
        destructive
        pending={archiveTask.isPending}
        onConfirm={() => void archive()}
      />
    </>
  );
}

export function TaskTable({
  tasks,
  projects,
  readOnly = false,
}: {
  tasks: Task[];
  projects: Project[];
  readOnly?: boolean;
}) {
  const projectNames = new Map(projects.map((project) => [project.id, project.name]));

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead className="min-w-[220px]">任务</TableHead>
          <TableHead className="w-[120px]">状态</TableHead>
          <TableHead className="w-[90px]">优先级</TableHead>
          <TableHead className="hidden w-[150px] lg:table-cell">项目</TableHead>
          <TableHead className="hidden w-[64px] text-right xl:table-cell">修订</TableHead>
          <TableHead className="hidden w-[150px] md:table-cell">更新时间</TableHead>
          <TableHead className="w-10">
            <span className="sr-only">操作</span>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {tasks.map((task) => (
          <TableRow key={task.id}>
            <TableCell className="max-w-[420px]">
              <Link
                to={`/tasks/${task.id}`}
                className="block truncate font-medium text-foreground underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                title={task.title}
              >
                {task.title}
              </Link>
              {task.archivedAt ? (
                <span className="text-[10px] text-muted-foreground">已归档</span>
              ) : null}
            </TableCell>
            <TableCell>
              <StatusBadge status={task.status} appearance="inline" />
            </TableCell>
            <TableCell>
              <PriorityBadge priority={task.priority} />
            </TableCell>
            <TableCell className="hidden max-w-[150px] truncate text-xs text-muted-foreground lg:table-cell">
              {projectNames.get(task.projectId) ?? "—"}
            </TableCell>
            <TableCell className="hidden text-right font-mono text-[11px] text-muted-foreground xl:table-cell">
              {task.revision}
            </TableCell>
            <TableCell className="hidden whitespace-nowrap text-[11px] text-muted-foreground md:table-cell">
              <time dateTime={task.updatedAt} title={task.updatedAt}>
                {formatDateTime(task.updatedAt)}
              </time>
            </TableCell>
            <TableCell className="px-2">
              {readOnly ? null : <TaskActionsMenu task={task} />}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
