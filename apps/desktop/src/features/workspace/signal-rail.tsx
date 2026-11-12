import type { TaskCounts, TaskStatus } from "@/lib/domain";
import { TASK_STATUS_META } from "@/lib/task-display";
import { cn } from "@/lib/utils";

export function SignalRail({ counts }: { counts: TaskCounts }) {
  const items: Array<{ status: TaskStatus; value: number }> = [
    { status: "awaiting_verification", value: counts.awaitingVerification },
    { status: "blocked", value: counts.blocked },
    { status: "running", value: counts.running },
    { status: "ready", value: counts.ready },
    { status: "done", value: counts.done },
  ];

  return (
    <div
      className="flex flex-wrap items-center gap-x-5 gap-y-2 border-y border-line/70 py-2.5"
      aria-label="任务状态分布"
    >
      <span className="text-[11px] text-muted-foreground">
        共 <span className="font-mono text-foreground">{counts.total}</span> 个任务
      </span>
      {items.map((item) => {
        const meta = TASK_STATUS_META[item.status];
        const Icon = meta.icon;

        return (
          <span key={item.status} className="inline-flex items-center gap-1.5 text-xs">
            <Icon className={cn("size-3.5", meta.textClassName)} aria-hidden="true" />
            <span className="font-mono font-medium tabular-nums">{item.value}</span>
            <span className="text-[11px] text-muted-foreground">{meta.label}</span>
          </span>
        );
      })}
    </div>
  );
}
