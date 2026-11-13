import { AlertTriangle, Clock3, FolderOpen, Loader2, ShieldCheck, Trash2 } from "lucide-react";
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
import type { BackupPolicy, BackupRecord, CleanupOutcome } from "@/lib/domain";
import { formatDateTime, shortId } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

import { BackupOriginBadge } from "./data-protection-badges";
import { formatSize } from "./data-protection-display";
import { useApplyBackupCleanup, usePreviewBackupCleanup, useUpdateBackupPolicy } from "./queries";

export function BackupPolicyDialog({
  open,
  onOpenChange,
  policy,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  policy: BackupPolicy;
}) {
  const [automaticEnabled, setAutomaticEnabled] = useState(policy.automaticEnabled);
  const [target, setTarget] = useState(policy.targetDirectory ?? "");
  const [intervalHours, setIntervalHours] = useState(String(policy.intervalHours));
  const [retentionCount, setRetentionCount] = useState(String(policy.retentionCount));
  const [error, setError] = useState<string | null>(null);
  const [cleanupPreview, setCleanupPreview] = useState<BackupRecord[] | null>(null);
  const [cleanupApproved, setCleanupApproved] = useState(false);
  const [cleanupResult, setCleanupResult] = useState<CleanupOutcome | null>(null);
  const updatePolicy = useUpdateBackupPolicy();
  const previewCleanup = usePreviewBackupCleanup();
  const applyCleanup = useApplyBackupCleanup();

  useEffect(() => {
    if (!open) {
      return;
    }
    setAutomaticEnabled(policy.automaticEnabled);
    setTarget(policy.targetDirectory ?? "");
    setIntervalHours(String(policy.intervalHours));
    setRetentionCount(String(policy.retentionCount));
    setError(null);
    setCleanupPreview(null);
    setCleanupApproved(false);
    setCleanupResult(null);
  }, [open, policy]);

  function save() {
    const hours = Number(intervalHours);
    const retention = Number(retentionCount);
    if (!Number.isInteger(hours) || hours < 1 || hours > 720) {
      setError("自动备份间隔必须是 1 到 720 之间的整数小时");
      return;
    }
    if (!Number.isInteger(retention) || retention < 1 || retention > 50) {
      setError("保留数量必须是 1 到 50 之间的整数");
      return;
    }
    if (automaticEnabled && !target.trim()) {
      setError("启用自动备份前必须填写目标目录");
      return;
    }
    setError(null);
    updatePolicy.mutate(
      {
        automaticEnabled,
        targetDirectory: target.trim() || null,
        intervalHours: hours,
        retentionCount: retention,
      },
      {
        onSuccess: () => toast.success("备份策略已保存"),
        onError: (mutationError) => setError(getErrorMessage(mutationError)),
      },
    );
  }

  function preview() {
    setError(null);
    setCleanupResult(null);
    setCleanupApproved(false);
    previewCleanup.mutate(undefined, {
      onSuccess: (records) => setCleanupPreview(records),
      onError: (mutationError) => setError(getErrorMessage(mutationError)),
    });
  }

  function cleanup() {
    if (!cleanupApproved) {
      return;
    }
    setError(null);
    applyCleanup.mutate(undefined, {
      onSuccess: (outcome) => {
        setCleanupResult(outcome);
        setCleanupPreview(null);
        setCleanupApproved(false);
        toast.success(`已清理 ${outcome.removedBackupIds.length} 份自动备份`);
      },
      onError: (mutationError) => setError(getErrorMessage(mutationError)),
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl" aria-describedby="backup-policy-description">
        <DialogHeader>
          <DialogTitle>备份策略</DialogTitle>
          <DialogDescription id="backup-policy-description">
            自动备份默认关闭。只有在此处主动启用后，XUSHU 才会在应用运行期间检查并创建自动备份。
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <label className="flex cursor-pointer items-start gap-2.5 border-y border-line/70 py-2.5">
            <input
              type="checkbox"
              checked={automaticEnabled}
              onChange={(event) => setAutomaticEnabled(event.target.checked)}
              className="mt-0.5 size-4 accent-[var(--primary)]"
            />
            <span className="min-w-0">
              <span className="block text-xs font-medium">启用自动备份</span>
              <span className="mt-0.5 block text-[11px] leading-5 text-muted-foreground">
                应用启动时与打开数据保护页时检查是否到期。应用关闭后不会运行；没有后台常驻服务。
              </span>
            </span>
          </label>

          <div className="space-y-1.5">
            <Label htmlFor="policy-target">自动备份目标目录</Label>
            <Input
              id="policy-target"
              value={target}
              onChange={(event) => setTarget(event.target.value)}
              placeholder="例如 D:\XUSHU-Backups"
              disabled={!automaticEnabled}
            />
            <p className="text-[11px] leading-5 text-muted-foreground">
              启用时目录必须通过本机安全路径检查；UNC 网络路径、符号链接与 Junction 会被拒绝。
            </p>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="policy-interval">间隔（小时）</Label>
              <Input
                id="policy-interval"
                type="number"
                min={1}
                max={720}
                value={intervalHours}
                onChange={(event) => setIntervalHours(event.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="policy-retention">保留自动备份数量</Label>
              <Input
                id="policy-retention"
                type="number"
                min={1}
                max={50}
                value={retentionCount}
                onChange={(event) => setRetentionCount(event.target.value)}
              />
            </div>
          </div>

          <dl className="divide-y divide-line/60 border-y border-line/70 text-[11px]">
            <div className="flex flex-wrap items-center justify-between gap-2 py-2">
              <dt className="flex items-center gap-1.5 text-muted-foreground">
                <Clock3 className="size-3.5" aria-hidden="true" />
                最近自动备份
              </dt>
              <dd className="font-mono">{formatDateTime(policy.lastAutomaticAt)}</dd>
            </div>
            <div className="flex flex-wrap items-center justify-between gap-2 py-2">
              <dt className="flex items-center gap-1.5 text-muted-foreground">
                <FolderOpen className="size-3.5" aria-hidden="true" />
                当前目标
              </dt>
              <dd className="max-w-full break-all font-mono">
                {policy.targetDirectory ?? "未配置"}
              </dd>
            </div>
            {policy.lastAutomaticErrorCode ? (
              <div className="flex flex-wrap items-center justify-between gap-2 py-2">
                <dt className="flex items-center gap-1.5 text-destructive">
                  <AlertTriangle className="size-3.5" aria-hidden="true" />
                  最近错误
                </dt>
                <dd className="font-mono text-destructive">
                  {policy.lastAutomaticErrorCode} · {formatDateTime(policy.lastAutomaticErrorAt)}
                </dd>
              </div>
            ) : null}
          </dl>

          <div className="space-y-2">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <p className="text-xs font-medium">自动备份清理</p>
                <p className="text-[11px] leading-5 text-muted-foreground">
                  只清理超过保留数量的 automatic 备份；手动备份永不自动删除。
                </p>
              </div>
              <Button type="button" variant="outline" size="sm" onClick={preview}>
                {previewCleanup.isPending ? (
                  <Loader2 className="animate-spin" aria-hidden="true" />
                ) : null}
                查看可清理备份
              </Button>
            </div>

            {cleanupPreview ? (
              cleanupPreview.length === 0 ? (
                <p className="border-y border-line/70 py-2 text-[11px] text-muted-foreground">
                  没有超过保留数量的自动备份。
                </p>
              ) : (
                <div className="space-y-2 border-y border-line/70 py-2">
                  <ul className="divide-y divide-line/60">
                    {cleanupPreview.map((record) => (
                      <li
                        key={record.backupId}
                        className="flex flex-wrap items-center gap-x-3 gap-y-1 py-1.5 text-[11px]"
                      >
                        <BackupOriginBadge origin={record.origin} />
                        <span className="font-mono">{shortId(record.backupId, 12)}</span>
                        <span className="text-muted-foreground">
                          {formatDateTime(record.createdAt)} · {formatSize(record.dataSizeBytes)}
                        </span>
                        <span className="min-w-0 flex-1 break-all font-mono text-muted-foreground">
                          {record.directoryPath}
                        </span>
                      </li>
                    ))}
                  </ul>
                  <label className="flex cursor-pointer items-start gap-2 text-[11px] leading-5">
                    <input
                      type="checkbox"
                      checked={cleanupApproved}
                      onChange={(event) => setCleanupApproved(event.target.checked)}
                      className="mt-0.5 size-4 accent-[var(--destructive)]"
                    />
                    我确认删除这些自动备份；删除前会重新校验清单与数据文件。
                  </label>
                  <Button
                    type="button"
                    variant="destructive"
                    size="sm"
                    onClick={cleanup}
                    disabled={!cleanupApproved || applyCleanup.isPending}
                  >
                    {applyCleanup.isPending ? (
                      <Loader2 className="animate-spin" aria-hidden="true" />
                    ) : (
                      <Trash2 aria-hidden="true" />
                    )}
                    清理 {cleanupPreview.length} 份自动备份
                  </Button>
                </div>
              )
            ) : null}

            {cleanupResult ? (
              <Alert variant="success">
                <ShieldCheck className="mt-0.5 size-4" aria-hidden="true" />
                <AlertTitle>清理完成</AlertTitle>
                <AlertDescription>
                  已删除 {cleanupResult.removedBackupIds.length} 份自动备份
                  {cleanupResult.skippedCount > 0
                    ? `；跳过 ${cleanupResult.skippedCount} 份未通过安全校验的备份`
                    : ""}
                  。{cleanupResult.failures.join("；")}
                </AlertDescription>
              </Alert>
            ) : null}
          </div>

          {error ? (
            <Alert variant="destructive" role="alert">
              <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>策略未保存</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          ) : null}
        </div>

        <DialogFooter className="flex-wrap">
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
          <Button type="button" onClick={save} disabled={updatePolicy.isPending}>
            {updatePolicy.isPending ? (
              <Loader2 className="animate-spin" aria-hidden="true" />
            ) : null}
            保存策略
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
