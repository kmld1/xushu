import { BadgeCheck, ShieldAlert } from "lucide-react";

import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import type { BackupVerification } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";

import { IntegrityCheckList, IntegrityReportSummary } from "./integrity-report";

export function BackupVerifySheet({
  open,
  onOpenChange,
  verification,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  verification: BackupVerification | null;
}) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent aria-describedby="backup-verify-description">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            备份完整性校验
            {verification?.verified ? (
              <BadgeCheck className="size-4 text-success" aria-hidden="true" />
            ) : verification ? (
              <ShieldAlert className="size-4 text-destructive" aria-hidden="true" />
            ) : null}
          </SheetTitle>
          <SheetDescription id="backup-verify-description">
            读取清单、数据文件 SHA-256，并在应用数据目录的隔离副本中执行 SQLite 与业务关系检查。
          </SheetDescription>
        </SheetHeader>
        <SheetBody className="space-y-4">
          {!verification ? (
            <p className="text-xs text-muted-foreground" role="status">
              正在校验备份…
            </p>
          ) : (
            <>
              <div className="border-y border-line/70 py-2">
                <p className="text-xs font-medium">
                  {verification.verified ? "校验通过" : "校验未通过"}
                </p>
                <p className="mt-0.5 text-[11px] leading-5 text-muted-foreground">
                  {verification.message}
                </p>
                <p className="mt-1 font-mono text-[11px] text-muted-foreground">
                  备份 {verification.backupId} · 校验时间 {formatDateTime(verification.verifiedAt)}
                </p>
              </div>
              <IntegrityCheckList checks={verification.checks} />
              {verification.integrity ? (
                <IntegrityReportSummary report={verification.integrity} compact />
              ) : null}
              <p className="text-[11px] leading-5 text-muted-foreground">
                校验和与 SQLite
                完整性证明文件可读取且与清单一致，不证明备份来源可信。只恢复自己创建或明确信任的备份。
              </p>
            </>
          )}
        </SheetBody>
        <SheetFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
