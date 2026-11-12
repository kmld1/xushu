import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

type PageHeaderProps = {
  title: string;
  description?: string;
  meta?: ReactNode;
  actions?: ReactNode;
  className?: string;
};

export function PageHeader({ title, description, meta, actions, className }: PageHeaderProps) {
  return (
    <header className={cn("flex flex-wrap items-start justify-between gap-x-4 gap-y-2", className)}>
      <div className="min-w-0">
        <div className="flex min-h-12 items-center gap-3">
          <h1 className="truncate text-xl font-semibold tracking-[-0.015em]">{title}</h1>
        </div>
        {meta || description ? (
          <div className="-mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 pb-1 text-xs text-muted-foreground">
            {meta}
            {meta && description ? <span aria-hidden="true">·</span> : null}
            {description ? <p className="max-w-3xl leading-5">{description}</p> : null}
          </div>
        ) : null}
      </div>
      {actions ? (
        <div className="flex min-h-12 shrink-0 flex-wrap items-center justify-end gap-2">
          {actions}
        </div>
      ) : null}
    </header>
  );
}
