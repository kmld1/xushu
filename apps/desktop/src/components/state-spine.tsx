import type { TaskTransition } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { TASK_STATUS_META } from "@/lib/task-display";
import { cn } from "@/lib/utils";

export function StateSpine({ transitions }: { transitions: TaskTransition[] }) {
  if (transitions.length === 0) {
    return <p className="text-sm text-muted-foreground">尚无状态记录。</p>;
  }

  return (
    <ol className="relative ml-1.5 space-y-5 border-l border-border pl-5">
      {transitions.map((transition) => {
        const meta = TASK_STATUS_META[transition.toStatus];
        const Icon = meta.icon;

        return (
          <li key={transition.id} className="relative">
            <span
              className={cn(
                "absolute -left-[27px] top-1 grid size-4 place-items-center rounded-full border bg-card",
                meta.className,
              )}
              aria-hidden="true"
            >
              <Icon className="size-2.5" />
            </span>
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <p className="text-sm font-medium">
                {transition.fromStatus
                  ? `${TASK_STATUS_META[transition.fromStatus].label} → ${meta.label}`
                  : `创建为${meta.label}`}
              </p>
              <time
                className="text-xs text-muted-foreground"
                dateTime={transition.createdAt}
                title={transition.createdAt}
              >
                {formatDateTime(transition.createdAt)}
              </time>
            </div>
            <p className="mt-1 text-xs text-muted-foreground">
              修订 {transition.taskRevision}
              {transition.reason ? ` · ${transition.reason}` : ""}
            </p>
          </li>
        );
      })}
    </ol>
  );
}
