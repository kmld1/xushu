import { Badge } from "@/components/ui/badge";
import type { TaskPriority, TaskStatus } from "@/lib/domain";
import {
  TASK_PRIORITY_META,
  TASK_STATUS_META,
  VERIFICATION_META,
  type VerificationStatus,
} from "@/lib/task-display";
import { cn } from "@/lib/utils";

export function StatusBadge({
  status,
  className,
  appearance = "badge",
}: {
  status: TaskStatus;
  className?: string;
  appearance?: "badge" | "inline";
}) {
  const meta = TASK_STATUS_META[status];
  const Icon = meta.icon;

  if (appearance === "inline") {
    return (
      <span
        className={cn(
          "inline-flex items-center gap-1 whitespace-nowrap text-xs",
          meta.textClassName,
          className,
        )}
      >
        <Icon className="size-3 shrink-0" aria-hidden="true" />
        {meta.label}
      </span>
    );
  }

  return (
    <Badge variant="outline" className={cn("gap-1 border", meta.className, className)}>
      <Icon className="size-3" aria-hidden="true" />
      {meta.label}
    </Badge>
  );
}

export function PriorityBadge({
  priority,
  className,
}: {
  priority: TaskPriority;
  className?: string;
}) {
  const meta = TASK_PRIORITY_META[priority];
  const Icon = meta.icon;

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 text-xs font-medium",
        meta.className,
        className,
      )}
    >
      <Icon className="size-3.5" aria-hidden="true" />
      {meta.label}
    </span>
  );
}

export function VerificationBadge({
  status,
  className,
}: {
  status: VerificationStatus;
  className?: string;
}) {
  const meta = VERIFICATION_META[status];
  const Icon = meta.icon;

  return (
    <Badge variant="outline" className={cn("gap-1 border", meta.className, className)}>
      <Icon className="size-3" aria-hidden="true" />
      {meta.label}
    </Badge>
  );
}
