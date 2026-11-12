import { FileSearch, ShieldAlert } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { LoadingState } from "@/components/loading-state";
import { useEvidence } from "@/features/verification/queries";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { describeSensitiveFlag } from "@/lib/verification-display";

export function EvidenceSheet({
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
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            <FileSearch className="size-4 text-primary" aria-hidden="true" />
            验证证据
          </SheetTitle>
          <SheetDescription>
            证据是运行结果的本地快照，不是对代码正确性的绝对保证。
          </SheetDescription>
        </SheetHeader>

        <SheetBody>
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
            <div className="space-y-5">
              <dl className="grid gap-x-6 gap-y-3 text-xs sm:grid-cols-2">
                <div className="space-y-0.5">
                  <dt className="text-muted-foreground">摘要</dt>
                  <dd className="text-[13px]">{evidence.data.summary}</dd>
                </div>
                <div className="space-y-0.5">
                  <dt className="text-muted-foreground">采集时间</dt>
                  <dd className="font-mono text-[11px]">
                    {formatDateTime(evidence.data.createdAt)}
                  </dd>
                </div>
                <div className="space-y-0.5 sm:col-span-2">
                  <dt className="text-muted-foreground">内容摘要哈希</dt>
                  <dd className="break-all font-mono text-[11px]">{evidence.data.contentDigest}</dd>
                </div>
                <div className="space-y-0.5 sm:col-span-2">
                  <dt className="text-muted-foreground">敏感性标记</dt>
                  <dd className="flex flex-wrap gap-1.5">
                    {evidence.data.sensitiveFlags.length === 0 ? (
                      <span className="text-[11px] text-muted-foreground">未命中已知敏感模式</span>
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
                <p className="mb-2 text-[11px] font-medium text-muted-foreground">脱敏输出</p>
                {evidence.data.redactedOutput ? (
                  <pre className="max-h-[58vh] overflow-auto whitespace-pre-wrap break-all rounded-md border border-code-border bg-code p-4 font-mono text-[11px] leading-5 text-code-foreground">
                    {evidence.data.redactedOutput}
                  </pre>
                ) : (
                  <p className="text-xs text-muted-foreground">本次运行没有可展示的输出。</p>
                )}
              </div>
            </div>
          )}
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}
