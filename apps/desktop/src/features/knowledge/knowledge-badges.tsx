import { Archive } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import type { DecisionStatus, MemoryConfirmationStatus } from "@/lib/domain";
import { cn } from "@/lib/utils";

import { DECISION_STATUS_META, MEMORY_CONFIRMATION_META } from "./knowledge-display";

export function MemoryConfirmationBadge({
  status,
  className,
}: {
  status: MemoryConfirmationStatus;
  className?: string;
}) {
  const meta = MEMORY_CONFIRMATION_META[status];
  const Icon = meta.icon;

  return (
    <Badge variant={meta.variant} className={cn("gap-1", className)}>
      <Icon className="size-3" aria-hidden="true" />
      {meta.label}
    </Badge>
  );
}

export function MemoryArchivedBadge({ className }: { className?: string }) {
  return (
    <Badge variant="outline" className={cn("gap-1 text-muted-foreground", className)}>
      <Archive className="size-3" aria-hidden="true" />
      已归档
    </Badge>
  );
}

export function DecisionStatusBadge({
  status,
  className,
}: {
  status: DecisionStatus;
  className?: string;
}) {
  const meta = DECISION_STATUS_META[status];
  const Icon = meta.icon;

  return (
    <Badge variant={meta.variant} className={cn("gap-1", className)}>
      <Icon className="size-3" aria-hidden="true" />
      {meta.label}
    </Badge>
  );
}
