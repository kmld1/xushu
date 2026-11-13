import { ClipboardCopy, Loader2, ShieldAlert } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetBody,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import type { RecoveryFreshness, RecoverySnapshot } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";

import { buildClipboardText } from "./copy-recovery";
import { RecoveryFreshnessBadge } from "./recovery-badges";
import { shortFingerprint } from "./recovery-display";

export function RecoveryInstructionsSheet({
  snapshot,
  freshness,
  open,
  onOpenChange,
  restoreFocus,
  historical = false,
  copying,
  onCopy,
}: {
  snapshot: RecoverySnapshot;
  freshness: RecoveryFreshness;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  restoreFocus: () => void;
  historical?: boolean;
  copying: boolean;
  onCopy: (opener: HTMLButtonElement) => void;
}) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocus();
        }}
      >
        <SheetHeader>
          <SheetTitle>完整恢复指令</SheetTitle>
          <SheetDescription>
            生成于 {formatDateTime(snapshot.generatedAt)} · 指纹{" "}
            <span className="font-mono">{shortFingerprint(snapshot.fingerprint)}</span>
            {!historical ? (
              <RecoveryFreshnessBadge state={freshness.state} className="ml-2 align-middle" />
            ) : null}
          </SheetDescription>
        </SheetHeader>
        <SheetBody
          role="region"
          aria-label="恢复指令预览"
          tabIndex={0}
          className="space-y-4 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
        >
          {historical ? (
            <Alert variant="warning">
              <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>这是历史快照</AlertTitle>
              <AlertDescription>
                它不代表当前项目状态。复制文本会在开头加入历史警告；请优先使用最新快照或重新生成。
              </AlertDescription>
            </Alert>
          ) : freshness.state === "stale" ? (
            <Alert variant="warning">
              <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>这份快照已经过期</AlertTitle>
              <AlertDescription>
                {freshness.changed.length > 0
                  ? `生成后发生变化：${freshness.changed.join("；")}。`
                  : "生成后项目数据已变化。"}
                复制文本会在开头加入过期警告；建议先重新生成。
              </AlertDescription>
            </Alert>
          ) : null}

          <Alert variant="info">
            <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
            <AlertTitle>复制前请确认内容边界</AlertTitle>
            <AlertDescription>
              恢复指令包含本地项目路径、任务与验证状态、已确认记忆和决策。它不会上传到任何外部服务；
              但分享或粘贴到其他工具前，请检查是否包含你不希望外传的信息。XUSHU 不会自动把它注入
              Claude Code 会话。
            </AlertDescription>
          </Alert>

          <pre className="max-h-[58vh] overflow-auto whitespace-pre-wrap break-words rounded-md border border-code-border bg-code-bg p-4 font-mono text-[11px] leading-5 text-code-fg">
            {buildClipboardText(snapshot, freshness, { historical })}
          </pre>
        </SheetBody>
        <SheetFooter>
          <SheetClose asChild>
            <Button type="button" variant="outline" size="sm">
              关闭
            </Button>
          </SheetClose>
          <Button
            type="button"
            size="sm"
            onClick={(event) => onCopy(event.currentTarget)}
            disabled={copying}
          >
            {copying ? (
              <Loader2 className="animate-spin" aria-hidden="true" />
            ) : (
              <ClipboardCopy aria-hidden="true" />
            )}
            {copying ? "检查中…" : "复制恢复指令"}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
