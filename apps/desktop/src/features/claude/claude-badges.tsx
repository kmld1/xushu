import { Badge } from "@/components/ui/badge";
import type { ClaudeInstallState } from "@/lib/domain";
import { cn } from "@/lib/utils";

import { CLAUDE_STATE_META, eventStatusMeta, sessionStateMeta } from "./claude-display";

export function ClaudeStateBadge({
  state,
  className,
}: {
  state: ClaudeInstallState;
  className?: string;
}) {
  const meta = CLAUDE_STATE_META[state];
  const Icon = meta.icon;
  return (
    <Badge variant={meta.variant} className={cn("gap-1 border", className)}>
      <Icon className="size-3" aria-hidden="true" />
      {meta.label}
    </Badge>
  );
}

export function ClaudeStateInline({
  state,
  className,
}: {
  state: ClaudeInstallState;
  className?: string;
}) {
  const meta = CLAUDE_STATE_META[state];
  const Icon = meta.icon;
  return (
    <span className={cn("inline-flex items-center gap-1 text-xs", meta.textClassName, className)}>
      <Icon className="size-3 shrink-0" aria-hidden="true" />
      {meta.label}
    </span>
  );
}

export function ClaudeEventStatus({
  status,
  className,
  appearance = "inline",
}: {
  status: string;
  className?: string;
  appearance?: "inline" | "badge";
}) {
  const meta = eventStatusMeta(status);
  const Icon = meta.icon;
  const icon = (
    <Icon className={cn("size-3 shrink-0", meta.spin && "animate-spin")} aria-hidden="true" />
  );

  if (appearance === "badge") {
    return (
      <Badge
        variant="outline"
        className={cn("gap-1 border border-line/70", meta.className, className)}
      >
        {icon}
        {meta.label}
      </Badge>
    );
  }

  return (
    <span className={cn("inline-flex items-center gap-1 text-xs", meta.className, className)}>
      {icon}
      {meta.label}
    </span>
  );
}

export function ClaudeSessionState({ state, className }: { state: string; className?: string }) {
  const meta = sessionStateMeta(state);
  const Icon = meta.icon;
  return (
    <span className={cn("inline-flex items-center gap-1 text-xs", meta.className, className)}>
      <Icon className="size-3 shrink-0" aria-hidden="true" />
      {meta.label}
    </span>
  );
}
