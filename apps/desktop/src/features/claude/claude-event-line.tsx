import { ClaudeEventStatus } from "./claude-badges";
import {
  endReasonLabel,
  eventTypeMeta,
  failureKindLabel,
  startSourceLabel,
  toolCategoryLabel,
} from "./claude-display";
import { formatDateTime, formatDuration } from "@/lib/format";
import type { ClaudeActivityEvent } from "@/lib/domain";
import { cn } from "@/lib/utils";

const timeFormatter = new Intl.DateTimeFormat("zh-CN", {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
  hour12: false,
});

function timeOnly(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : timeFormatter.format(date);
}

export function ClaudeEventText({
  event,
  compact = false,
}: {
  event: ClaudeActivityEvent;
  compact?: boolean;
}) {
  const meta = eventTypeMeta(event.eventType);
  const Icon = meta.icon;
  const details: string[] = [];

  if (event.toolName) {
    details.push(
      `${event.toolName}${event.toolCategory ? ` · ${toolCategoryLabel(event.toolCategory)}` : ""}`,
    );
  } else if (event.eventType === "session_start" && event.source) {
    details.push(startSourceLabel(event.source));
  } else if (event.eventType === "session_end") {
    details.push(endReasonLabel(event.endReason));
  }

  if (event.failureKind) {
    details.push(failureKindLabel(event.failureKind));
  }
  if (event.durationMs !== null) {
    details.push(formatDuration(event.durationMs));
  }
  if (!compact) {
    details.push(`会话 ${event.sessionRef}`);
  }

  return (
    <div className="min-w-0">
      <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
        <Icon className={cn("size-3.5 shrink-0", meta.className)} aria-hidden="true" />
        <span className="text-[13px] font-medium">{meta.label}</span>
        {event.eventType === "pre_tool_use" ||
        event.eventType === "post_tool_use" ||
        event.eventType === "post_tool_use_failure" ? (
          <ClaudeEventStatus status={event.status} />
        ) : null}
        <time
          dateTime={event.occurredAt}
          title={formatDateTime(event.occurredAt)}
          className="ml-auto shrink-0 font-mono text-[11px] tabular-nums text-muted-foreground"
        >
          {timeOnly(event.occurredAt)}
        </time>
      </div>
      {details.length > 0 ? (
        <p
          className="mt-0.5 min-w-0 truncate text-[11px] text-muted-foreground"
          title={details.join(" · ")}
        >
          {details.join(" · ")}
        </p>
      ) : null}
    </div>
  );
}

export function ClaudeEventRow({
  event,
  compact = false,
}: {
  event: ClaudeActivityEvent;
  compact?: boolean;
}) {
  return (
    <div className={cn("min-w-0 py-2.5", compact && "py-2")}>
      <ClaudeEventText event={event} compact={compact} />
    </div>
  );
}
