import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  bordered = false,
  className,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
  action?: ReactNode;
  /** 仅用于首次进入等需要明确边界的场景；默认不套边框。 */
  bordered?: boolean;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center px-6 py-10 text-center",
        bordered && "rounded-lg border border-dashed border-line bg-surface-raised/30",
        className,
      )}
    >
      <div className="grid size-9 place-items-center rounded-md bg-surface-raised text-muted-foreground">
        <Icon className="size-4" aria-hidden="true" />
      </div>
      <p className="mt-3 text-[13px] font-medium">{title}</p>
      <p className="mt-1 max-w-md text-xs leading-5 text-muted-foreground">{description}</p>
      {action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}
