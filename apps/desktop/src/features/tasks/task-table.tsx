import { Link } from "react-router";

import { PriorityBadge, StatusBadge } from "@/components/status-badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { Project, Task } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";

export function TaskTable({ tasks, projects }: { tasks: Task[]; projects: Project[] }) {
  const projectNames = new Map(projects.map((project) => [project.id, project.name]));

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>任务</TableHead>
          <TableHead>状态</TableHead>
          <TableHead>优先级</TableHead>
          <TableHead>项目</TableHead>
          <TableHead className="text-right">修订</TableHead>
          <TableHead>更新时间</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {tasks.map((task) => (
          <TableRow key={task.id}>
            <TableCell className="max-w-[360px]">
              <Link
                to={`/tasks/${task.id}`}
                className="block truncate font-medium text-foreground underline-offset-4 hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                title={task.title}
              >
                {task.title}
              </Link>
              {task.archivedAt ? (
                <span className="text-[11px] text-muted-foreground">已归档</span>
              ) : null}
            </TableCell>
            <TableCell>
              <StatusBadge status={task.status} />
            </TableCell>
            <TableCell>
              <PriorityBadge priority={task.priority} />
            </TableCell>
            <TableCell className="max-w-[180px] truncate text-xs text-muted-foreground">
              {projectNames.get(task.projectId) ?? "—"}
            </TableCell>
            <TableCell className="text-right font-mono text-xs text-muted-foreground">
              {task.revision}
            </TableCell>
            <TableCell className="whitespace-nowrap text-xs text-muted-foreground">
              <time dateTime={task.updatedAt} title={task.updatedAt}>
                {formatDateTime(task.updatedAt)}
              </time>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
