import { Badge } from "@/components/ui/badge";
import type { RecoveryFreshnessState, RecoveryGitState, RecoveryNextStepKind } from "@/lib/domain";
import { cn } from "@/lib/utils";

import { GIT_STATE_META, NEXT_STEP_KIND_META, RECOVERY_FRESHNESS_META } from "./recovery-display";

export function RecoveryFreshnessBadge({
  state,
  className,
}: {
  state: RecoveryFreshnessState;
  className?: string;
}) {
  const meta = RECOVERY_FRESHNESS_META[state];
  const Icon = meta.icon;
  return (
    <Badge variant={meta.variant} className={cn("gap-1", className)} title={meta.description}>
      <Icon className="size-3" aria-hidden="true" />
      {meta.label}
    </Badge>
  );
}

export function NextStepKindBadge({
  kind,
  className,
}: {
  kind: RecoveryNextStepKind;
  className?: string;
}) {
  const meta = NEXT_STEP_KIND_META[kind];
  const Icon = meta.icon;
  return (
    <Badge variant={meta.variant} className={cn("gap-1", className)} title={meta.description}>
      <Icon className="size-3" aria-hidden="true" />
      {meta.label}
    </Badge>
  );
}

export function RecoveryGitBadge({
  state,
  className,
}: {
  state: RecoveryGitState;
  className?: string;
}) {
  const meta = GIT_STATE_META[state];
  return (
    <Badge variant={meta.variant} className={cn("gap-1", className)}>
      {meta.label}
    </Badge>
  );
}
