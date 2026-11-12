import { Check, FilePenLine, Loader2, OctagonAlert, ShieldCheck, X } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { Project } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

import { settingsEventLabel } from "./claude-display";
import { useClaudePreview, useInstallClaudeHooks } from "./queries";

const COLLECTED = [
  "事件类型与接收时间",
  "伪名化的会话与工具调用标识",
  "工具名称与类别",
  "成功、失败或未知类别",
  "工具执行耗时（若有）",
];

const NOT_COLLECTED = [
  "用户 Prompt 与 AI 回答正文",
  "完整命令及命令参数",
  "文件内容与文件路径",
  "工具输入与工具输出正文",
  "API Key、Token、环境变量值",
  "完整对话 transcript",
];

export function ClaudeInstallDialog({
  project,
  open,
  onOpenChange,
  restoreFocus,
  mode = "install",
}: {
  project: Project;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  restoreFocus: () => void;
  mode?: "install" | "repair";
}) {
  const preview = useClaudePreview(project.id, open);
  const install = useInstallClaudeHooks(project.id);
  const [acknowledged, setAcknowledged] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setAcknowledged(false);
      setSubmitError(null);
    }
  }, [open]);

  const blocked = preview.data?.blockedReason ?? null;
  const canInstall = Boolean(preview.data && !blocked && acknowledged && !install.isPending);

  async function handleInstall() {
    if (!preview.data || blocked) {
      return;
    }
    setSubmitError(null);
    try {
      await install.mutateAsync({
        projectId: project.id,
        previewToken: preview.data.previewToken,
        approved: true,
      });
      toast.success(mode === "repair" ? "Claude Code 接入已修复" : "Claude Code 接入已启用", {
        description: "Hooks 只写入项目本地设置；停用或卸载不会删除历史事件。",
      });
      onOpenChange(false);
    } catch (error) {
      setSubmitError(getErrorMessage(error));
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="flex max-w-2xl flex-col overflow-hidden"
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocus();
        }}
      >
        <DialogHeader className="shrink-0">
          <DialogTitle>
            {mode === "repair" ? "修复 Claude Code 接入" : "启用 Claude Code 接入"}
          </DialogTitle>
          <DialogDescription>
            项目：{project.name}。XUSHU 只接收最小事件元数据，不会把 Hook 事件当作任务完成证据。
          </DialogDescription>
        </DialogHeader>

        {preview.isPending ? (
          <LoadingState rows={4} label="正在生成安装预览" />
        ) : preview.isError ? (
          <ErrorState
            title="无法生成安装预览"
            message={getErrorMessage(preview.error)}
            onRetry={() => void preview.refetch()}
          />
        ) : preview.data ? (
          <div
            role="region"
            aria-label="安装预览内容"
            tabIndex={0}
            className="min-h-0 space-y-5 overflow-y-auto pr-1 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
          >
            {blocked ? (
              <Alert variant="destructive">
                <OctagonAlert className="mt-0.5 size-4" aria-hidden="true" />
                <AlertTitle>当前不能安装</AlertTitle>
                <AlertDescription>{blocked}</AlertDescription>
              </Alert>
            ) : null}

            <section className="space-y-2">
              <h3 className="text-[13px] font-semibold">写入位置</h3>
              <div className="flex flex-wrap items-center gap-2 text-xs">
                <code className="rounded-sm bg-surface-raised px-2 py-1 font-mono text-[11px] text-foreground">
                  {preview.data.settingsFile}
                </code>
                <Badge variant="outline">项目本地</Badge>
                <span className="text-muted-foreground">
                  {preview.data.willCreateFile
                    ? "文件尚不存在，将新建。"
                    : "将结构化合并，保留其他配置。"}
                </span>
              </div>
              <p className="text-[11px] leading-5 text-muted-foreground">
                不会写入团队共享的 <code className="font-mono">.claude/settings.json</code>
                ，也不会修改用户级或系统级 Claude Code 设置。
              </p>
            </section>

            <section className="space-y-2">
              <h3 className="text-[13px] font-semibold">将要安装的 Hooks</h3>
              <div className="overflow-hidden rounded-md border border-line/70">
                <table className="w-full text-left text-xs">
                  <thead className="border-b border-line/70 bg-surface-raised/50 text-[11px] text-muted-foreground">
                    <tr>
                      <th scope="col" className="px-3 py-2 font-medium">
                        事件
                      </th>
                      <th scope="col" className="px-3 py-2 font-medium">
                        Matcher
                      </th>
                      <th scope="col" className="px-3 py-2 font-medium">
                        超时
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-line/60">
                    {preview.data.managedEntries.map((entry) => (
                      <tr key={entry.event}>
                        <td className="px-3 py-2">
                          <span className="font-medium">{settingsEventLabel(entry.event)}</span>
                          <span className="ml-2 font-mono text-[10px] text-muted-foreground">
                            {entry.event}
                          </span>
                        </td>
                        <td className="px-3 py-2 font-mono text-[11px] text-muted-foreground">
                          {entry.matcher ? "全部" : "—"}
                        </td>
                        <td className="px-3 py-2 font-mono text-[11px] tabular-nums text-muted-foreground">
                          {entry.timeoutSeconds}s
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {preview.data.managedEntries.length > 0 ? (
                <details className="rounded-md border border-line/70 bg-surface-raised/30">
                  <summary className="cursor-pointer px-3 py-2 text-xs font-medium">
                    查看接收命令
                  </summary>
                  <div className="border-t border-line/60 bg-code-bg px-3 py-2 font-mono text-[10px] leading-5 text-code-fg">
                    <p className="break-all text-code-muted">
                      {preview.data.managedEntries[0].command}
                    </p>
                    <p className="break-all">{preview.data.managedEntries[0].args.join(" ")}</p>
                  </div>
                </details>
              ) : null}
              {preview.data.otherHookGroupCount > 0 ? (
                <p className="text-[11px] text-muted-foreground">
                  检测到 {preview.data.otherHookGroupCount} 个其他 Hook 组，安装与卸载都会保持原样。
                </p>
              ) : null}
            </section>

            <section className="space-y-2">
              <h3 className="text-[13px] font-semibold">数据边界</h3>
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="rounded-md border border-line/70 p-3">
                  <p className="mb-2 flex items-center gap-1.5 text-xs font-medium text-success">
                    <Check className="size-3.5" aria-hidden="true" />
                    会采集
                  </p>
                  <ul className="space-y-1 text-[11px] leading-5 text-muted-foreground">
                    {COLLECTED.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </div>
                <div className="rounded-md border border-line/70 p-3">
                  <p className="mb-2 flex items-center gap-1.5 text-xs font-medium text-destructive">
                    <X className="size-3.5" aria-hidden="true" />
                    不会采集
                  </p>
                  <ul className="space-y-1 text-[11px] leading-5 text-muted-foreground">
                    {NOT_COLLECTED.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </div>
              </div>
            </section>

            <section className="space-y-2">
              <h3 className="text-[13px] font-semibold">安装前确认</h3>
              <ul className="space-y-1.5 text-[11px] leading-5 text-muted-foreground">
                {preview.data.warnings.map((warning) => (
                  <li key={warning} className="flex gap-2">
                    <ShieldCheck className="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
                    <span>{warning}</span>
                  </li>
                ))}
              </ul>
              <p className="flex items-center gap-2 text-[11px] text-muted-foreground">
                <FilePenLine className="size-3.5 shrink-0" aria-hidden="true" />
                {preview.data.backupWillBeCreated
                  ? "写入前会备份现有设置文件。"
                  : "当前没有设置文件，无需备份。"}
              </p>
            </section>

            <label className="flex cursor-pointer items-start gap-2.5 rounded-md border border-line/70 bg-surface-raised/40 p-3 text-xs leading-5">
              <input
                type="checkbox"
                className="mt-0.5 size-3.5 shrink-0 accent-primary"
                checked={acknowledged}
                onChange={(event) => setAcknowledged(event.target.checked)}
              />
              <span>
                我了解 XUSHU 只会写入该项目的本地设置文件，并且可以在之后停用或卸载。Hook
                事件只表示观察到某次操作，不代表验收通过。
              </span>
            </label>

            {submitError ? (
              <Alert variant="destructive">
                <OctagonAlert className="mt-0.5 size-4" aria-hidden="true" />
                <AlertTitle>安装未完成</AlertTitle>
                <AlertDescription>
                  <span className="block">{submitError}</span>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="mt-2"
                    onClick={() => void preview.refetch()}
                  >
                    重新生成预览
                  </Button>
                </AlertDescription>
              </Alert>
            ) : null}
          </div>
        ) : null}

        <DialogFooter className="shrink-0 border-t border-line/70 pt-4">
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button
            type="button"
            onClick={() => void handleInstall()}
            disabled={!canInstall}
            aria-busy={install.isPending}
          >
            {install.isPending ? <Loader2 className="animate-spin" aria-hidden="true" /> : null}
            {mode === "repair" ? "修复并启用" : "安装并启用"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
