import { AlertTriangle, Database, Loader2, LockKeyhole } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { BackupRecord } from "@/lib/domain";
import { formatDateTime, shortId } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

import { BackupIntegrityBadge } from "./data-protection-badges";
import { formatSize, shortHash } from "./data-protection-display";
import { useCancelBackup, useCreateBackup } from "./queries";

export function CreateBackupDialog({
  open,
  onOpenChange,
  defaultTarget,
  estimatedBytes,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  defaultTarget: string;
  estimatedBytes: number;
  onCreated?: () => void;
}) {
  const [target, setTarget] = useState(defaultTarget);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BackupRecord | null>(null);
  const createBackup = useCreateBackup();
  const cancelBackup = useCancelBackup();

  useEffect(() => {
    if (open) {
      setTarget(defaultTarget);
      setError(null);
      setResult(null);
    }
  }, [open, defaultTarget]);

  const pending = createBackup.isPending;

  function submit() {
    const trimmed = target.trim();
    if (!trimmed) {
      setError("请输入本机备份目标目录");
      return;
    }
    setError(null);
    setResult(null);
    createBackup.mutate(trimmed, {
      onSuccess: (record) => {
        setResult(record);
        onCreated?.();
        toast.success("备份已创建并通过完整性校验");
      },
      onError: (mutationError) => {
        setError(getErrorMessage(mutationError));
      },
    });
  }

  function requestCancel() {
    cancelBackup.mutate(undefined, {
      onSuccess: () => toast.info("已请求取消备份；若已完成则保留已校验的备份"),
      onError: (mutationError) => toast.error(getErrorMessage(mutationError)),
    });
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="max-w-2xl" aria-describedby="create-backup-description">
        <DialogHeader>
          <DialogTitle>创建数据库备份</DialogTitle>
          <DialogDescription id="create-backup-description">
            使用 SQLite Online Backup API 生成一致性快照；完成后立即执行完整性与业务关系校验。
          </DialogDescription>
        </DialogHeader>

        {result ? (
          <div className="space-y-3">
            <Alert>
              <Database className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle className="flex flex-wrap items-center gap-2">
                备份已创建
                <BackupIntegrityBadge status={result.integrityStatus} />
              </AlertTitle>
              <AlertDescription>
                <span className="block break-all font-mono text-[11px]">
                  {result.directoryPath}
                </span>
                <span className="mt-1 block text-[11px]">
                  {formatDateTime(result.createdAt)} · {formatSize(result.dataSizeBytes)} · Schema v
                  {result.schemaVersion} · 备份 {shortId(result.backupId, 12)}
                </span>
                <span className="mt-1 block break-all font-mono text-[10px] text-muted-foreground">
                  SHA-256 {shortHash(result.payloadSha256)}
                </span>
              </AlertDescription>
            </Alert>
            <p className="text-[11px] leading-5 text-muted-foreground">
              备份未加密，包含本机项目路径与用户录入内容。校验和只证明文件内容与清单一致，不代表来源可信。
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="backup-target">备份目标目录</Label>
              <Input
                id="backup-target"
                value={target}
                onChange={(event) => setTarget(event.target.value)}
                placeholder="备份目标目录绝对路径"
                disabled={pending}
                aria-invalid={Boolean(error)}
                aria-describedby="backup-target-hint"
              />
              <p id="backup-target-hint" className="text-[11px] leading-5 text-muted-foreground">
                输入本机绝对路径。父目录必须存在；最后一级目录不存在时由 XUSHU 创建。当前版本不接受
                UNC 网络路径、符号链接或 Junction。
              </p>
            </div>

            <div className="border-y border-line/70 py-2 text-[11px] leading-5 text-muted-foreground">
              <p>
                备份范围：SQLite 数据库（项目、任务、验收、验证记录与证据摘要、Claude
                元数据、记忆、决策、恢复快照）。
              </p>
              <p className="mt-1">
                不包含：Hook Spool、伪名化 Salt、项目 .claude 配置、导出报告、WebView 本地偏好。
              </p>
              <p className="mt-1">
                预计数据量：约 {formatSize(estimatedBytes)}，另加小型清单文件与磁盘余量。
              </p>
            </div>

            <Alert variant="warning">
              <LockKeyhole className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>备份不加密</AlertTitle>
              <AlertDescription>
                备份可能包含本地路径、任务描述、记忆和脱敏后的测试输出。请保存在受控磁盘，不要上传到不受信任的位置。
              </AlertDescription>
            </Alert>

            {pending ? (
              <Alert>
                <Loader2 className="mt-0.5 size-4 animate-spin" aria-hidden="true" />
                <AlertTitle>正在创建一致性快照并校验</AlertTitle>
                <AlertDescription>
                  完成前不会登记为有效备份。可以请求取消；取消或失败不会留下可误认成有效备份的文件。
                </AlertDescription>
              </Alert>
            ) : null}

            {error ? (
              <Alert variant="destructive" role="alert">
                <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
                <AlertTitle>备份未完成</AlertTitle>
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            ) : null}
          </div>
        )}

        <DialogFooter className="flex-wrap">
          {pending ? (
            <Button
              type="button"
              variant="outline"
              onClick={requestCancel}
              disabled={cancelBackup.isPending}
            >
              {cancelBackup.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : null}
              请求取消
            </Button>
          ) : result ? (
            <Button type="button" onClick={() => onOpenChange(false)}>
              关闭
            </Button>
          ) : (
            <>
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                取消
              </Button>
              <Button type="button" onClick={submit} disabled={!target.trim()}>
                <Database aria-hidden="true" />
                创建备份
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
