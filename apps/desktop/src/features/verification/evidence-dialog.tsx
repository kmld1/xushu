import { FileSearch, ShieldAlert } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { LoadingState } from "@/components/loading-state";
import { useEvidence } from "@/features/verification/queries";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { describeSensitiveFlag } from "@/lib/verification-display";

export function EvidenceDialog({
  open,
  onOpenChange,
  evidenceId,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  evidenceId: string | null;
}) {
  const evidence = useEvidence(evidenceId, open);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileSearch className="size-4 text-primary" aria-hidden="true" />
            验证证据
          </DialogTitle>
          <DialogDescription>
            证据是运行结果的本地快照，不是对代码正确性的绝对保证。
          </DialogDescription>
        </DialogHeader>

        {evidence.isPending ? (
          <LoadingState rows={4} />
        ) : evidence.isError || !evidence.data ? (
          <Alert variant="destructive">
            <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
            <AlertTitle>无法读取证据</AlertTitle>
            <AlertDescription>
              {evidence.isError ? getErrorMessage(evidence.error) : "证据不存在"}
            </AlertDescription>
          </Alert>
        ) : (
          <div className="space-y-4">
            <dl className="grid gap-3 text-sm sm:grid-cols-2">
              <div className="space-y-1">
                <dt className="text-xs text-muted-foreground">摘要</dt>
                <dd>{evidence.data.summary}</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-xs text-muted-foreground">采集时间</dt>
                <dd>{formatDateTime(evidence.data.createdAt)}</dd>
              </div>
              <div className="space-y-1 sm:col-span-2">
                <dt className="text-xs text-muted-foreground">内容摘要哈希</dt>
                <dd className="break-all font-mono text-xs">{evidence.data.contentDigest}</dd>
              </div>
              <div className="space-y-1 sm:col-span-2">
                <dt className="text-xs text-muted-foreground">敏感性标记</dt>
                <dd className="flex flex-wrap gap-1.5">
                  {evidence.data.sensitiveFlags.length === 0 ? (
                    <span className="text-xs text-muted-foreground">未命中已知敏感模式</span>
                  ) : (
                    evidence.data.sensitiveFlags.map((flag) => (
                      <Badge key={flag} variant="warning">
                        {describeSensitiveFlag(flag)}
                      </Badge>
                    ))
                  )}
                </dd>
              </div>
            </dl>

            {evidence.data.sensitiveFlags.length > 0 ? (
              <Alert variant="warning">
                <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
                <AlertTitle>输出已做模式化脱敏</AlertTitle>
                <AlertDescription>
                  脱敏是尽力而为，不能保证识别所有密钥格式；请勿把证据内容直接粘贴到外部服务。
                </AlertDescription>
              </Alert>
            ) : null}

            <div>
              <p className="mb-2 text-xs font-medium text-muted-foreground">脱敏输出</p>
              {evidence.data.redactedOutput ? (
                <pre className="max-h-80 overflow-auto whitespace-pre-wrap break-all rounded-md border bg-surface-raised/50 p-4 font-mono text-xs leading-6">
                  {evidence.data.redactedOutput}
                </pre>
              ) : (
                <p className="text-sm text-muted-foreground">本次运行没有可展示的输出。</p>
              )}
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
