import type { ComponentProps, ReactNode } from "react";

import { cn } from "@/lib/utils";

export function Section({ children, className, ...props }: ComponentProps<"section">) {
  return (
    <section className={cn("space-y-3", className)} {...props}>
      {children}
    </section>
  );
}

export function SectionHeader({
  title,
  meta,
  actions,
  className,
}: {
  title: string;
  meta?: ReactNode;
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex min-h-8 flex-wrap items-center justify-between gap-2", className)}>
      <div className="flex min-w-0 items-baseline gap-2">
        <h2 className="truncate text-[13px] font-semibold tracking-[-0.01em]">{title}</h2>
        {meta ? <span className="text-[11px] text-muted-foreground">{meta}</span> : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-1.5">{actions}</div> : null}
    </div>
  );
}
