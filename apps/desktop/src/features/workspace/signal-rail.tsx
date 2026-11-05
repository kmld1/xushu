import type { TaskCounts, TaskStatus } from "@/lib/domain";
import { TASK_STATUS_META } from "@/lib/task-display";
import { cn } from "@/lib/utils";

export function SignalRail({ counts }: { counts: TaskCounts }) {
  const items: Array<{ status: TaskStatus; value: number }> = [
    { status: "draft", value: counts.draft },
    { status: "ready", value: counts.ready },
    { status: "running", value: counts.running },
    { status: "awaiting_verification", value: counts.awaitingVerification },
    { status: "done", value: counts.done },
    { status: "blocked", value: counts.blocked },
    { status: "cancelled", value: counts.cancelled },
  ];

  return (
    <div
      className="grid overflow-hidden rounded-lg border bg-card sm:grid-cols-2 lg:grid-cols-4 xl:grid-cols-7"
      aria-label="任务状态分布"
    >
      {items.map((item, index) => {
        const meta = TASK_STATUS_META[item.status];
        const Icon = meta.icon;
        const isLast = index === items.length - 1;

        return (
          <div
            key={item.status}
            className={cn(
              "flex items-center gap-3 px-4 py-3",
              !isLast && "border-b sm:border-r lg:border-b-0",
              index === 4 && "xl:border-r",
            )}
          >
            <span
              className={cn(
                "grid size-7 shrink-0 place-items-center rounded-md border",
                meta.className,
              )}
              aria-hidden="true"
            >
              <Icon className="size-3.5" />
            </span>
            <div className="min-w-0">
              <p className="text-lg font-semibold leading-none tabular-nums">{item.value}</p>
              <p className="mt-1 truncate text-[11px] text-muted-foreground">{meta.label}</p>
            </div>
          </div>
        );
      })}
    </div>
  );
}
