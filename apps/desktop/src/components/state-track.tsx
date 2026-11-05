import { Check } from "lucide-react";

import type { TaskStatus } from "@/lib/domain";
import { TASK_STATUS_META, TASK_STATUS_ORDER } from "@/lib/task-display";
import { cn } from "@/lib/utils";

export function StateTrack({ status }: { status: TaskStatus }) {
  const currentIndex = TASK_STATUS_ORDER.indexOf(status);

  return (
    <div className="overflow-x-auto">
      <ol className="flex min-w-[560px] items-start px-1" aria-label="任务状态推进">
        {TASK_STATUS_ORDER.map((step, index) => {
          const meta = TASK_STATUS_META[step];
          const Icon = meta.icon;
          const completed = currentIndex >= 0 && index < currentIndex;
          const current = index === currentIndex;

          return (
            <li
              key={step}
              className="flex flex-1 flex-col items-center gap-2"
              aria-current={current ? "step" : undefined}
            >
              <div className="flex w-full items-center">
                <span
                  className={cn(
                    "h-px flex-1",
                    index === 0
                      ? "opacity-0"
                      : completed || current
                        ? "bg-primary/45"
                        : "bg-border",
                  )}
                />
                <span
                  className={cn(
                    "grid size-6 shrink-0 place-items-center rounded-full border text-[10px] font-semibold",
                    completed && "border-status-done/40 bg-status-done/15 text-status-done",
                    current && meta.className,
                    !completed &&
                      !current &&
                      "border-border bg-surface-raised text-muted-foreground",
                  )}
                >
                  {completed ? (
                    <Check className="size-3.5" aria-hidden="true" />
                  ) : (
                    <Icon className="size-3" aria-hidden="true" />
                  )}
                </span>
                <span
                  className={cn(
                    "h-px flex-1",
                    index === TASK_STATUS_ORDER.length - 1
                      ? "opacity-0"
                      : completed
                        ? "bg-primary/45"
                        : "bg-border",
                  )}
                />
              </div>
              <span
                className={cn(
                  "text-center text-[11px] leading-4",
                  current ? "font-medium text-foreground" : "text-muted-foreground",
                )}
              >
                {meta.label}
              </span>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
