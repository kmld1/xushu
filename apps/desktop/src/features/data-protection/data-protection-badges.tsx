import {
  BadgeCheck,
  CircleHelp,
  CircleSlash,
  CircleX,
  CheckCircle2,
  OctagonAlert,
  Timer,
  Undo2,
  UserRound,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import type {
  BackupIntegrityStatus,
  BackupOrigin,
  IntegrityCheckStatus,
  RestoreOutcome,
} from "@/lib/domain";
import { cn } from "@/lib/utils";

import {
  BACKUP_INTEGRITY_LABEL,
  BACKUP_ORIGIN_LABEL,
  INTEGRITY_CHECK_LABEL,
  RESTORE_OUTCOME_LABEL,
} from "./data-protection-display";

const integrityTone: Record<BackupIntegrityStatus, string> = {
  verified: "border-success/40 bg-success/10 text-success",
  failed: "border-destructive/40 bg-destructive/10 text-destructive",
  unverified: "border-warning/40 bg-warning/10 text-warning",
  missing: "border-line bg-muted text-muted-foreground",
};

export function BackupIntegrityBadge({ status }: { status: BackupIntegrityStatus }) {
  const Icon =
    status === "verified"
      ? BadgeCheck
      : status === "failed"
        ? CircleX
        : status === "missing"
          ? CircleSlash
          : CircleHelp;
  return (
    <Badge variant="outline" className={cn("gap-1 border", integrityTone[status])}>
      <Icon className="size-3" aria-hidden="true" />
      {BACKUP_INTEGRITY_LABEL[status]}
    </Badge>
  );
}

export function BackupOriginBadge({ origin }: { origin: BackupOrigin }) {
  const Icon = origin === "automatic" ? Timer : UserRound;
  return (
    <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground">
      <Icon className="size-3" aria-hidden="true" />
      {BACKUP_ORIGIN_LABEL[origin]}
    </span>
  );
}

const checkTone: Record<IntegrityCheckStatus, string> = {
  pass: "border-success/40 bg-success/10 text-success",
  fail: "border-destructive/40 bg-destructive/10 text-destructive",
  not_run: "border-line bg-muted text-muted-foreground",
};

export function IntegrityStatusBadge({ status }: { status: IntegrityCheckStatus }) {
  const Icon = status === "pass" ? BadgeCheck : status === "fail" ? CircleX : CircleSlash;
  return (
    <Badge variant="outline" className={cn("gap-1 border", checkTone[status])}>
      <Icon className="size-3" aria-hidden="true" />
      {INTEGRITY_CHECK_LABEL[status]}
    </Badge>
  );
}

export function RestoreOutcomeBadge({ outcome }: { outcome: RestoreOutcome["status"] }) {
  const tone =
    outcome === "completed"
      ? "border-success/40 bg-success/10 text-success"
      : outcome === "rolled_back"
        ? "border-warning/40 bg-warning/10 text-warning"
        : "border-destructive/40 bg-destructive/10 text-destructive";
  const Icon =
    outcome === "completed" ? CheckCircle2 : outcome === "rolled_back" ? Undo2 : OctagonAlert;
  return (
    <Badge variant="outline" className={cn("gap-1 border", tone)}>
      <Icon className="size-3" aria-hidden="true" />
      {RESTORE_OUTCOME_LABEL[outcome] ?? outcome}
    </Badge>
  );
}

export function RestoreHistoryOutcomeBadge({ outcome }: { outcome: string }) {
  const textTone =
    outcome === "completed"
      ? "text-success"
      : outcome === "rolled_back"
        ? "text-warning"
        : "text-destructive";
  const Icon =
    outcome === "completed" ? CheckCircle2 : outcome === "rolled_back" ? Undo2 : OctagonAlert;
  return (
    <span className={cn("inline-flex items-center gap-1 text-xs", textTone)}>
      <Icon className="size-3" aria-hidden="true" />
      {RESTORE_OUTCOME_LABEL[outcome] ?? outcome}
    </span>
  );
}
