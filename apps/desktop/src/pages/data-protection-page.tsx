import {
  ArchiveRestore,
  BadgeCheck,
  Clock3,
  Copy,
  Database,
  DatabaseBackup,
  FileWarning,
  HardDrive,
  History,
  Loader2,
  LockKeyhole,
  RefreshCw,
  ShieldAlert,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Section, SectionHeader } from "@/components/section";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import type {
  BackupRecord,
  BackupVerification,
  DataProtectionOverview,
  RestoreStartupNotice,
} from "@/lib/domain";
import { formatDateTime, shortId } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";

import { copyTextToClipboard } from "@/features/recovery/copy-recovery";
import { BackupPolicyDialog } from "@/features/data-protection/backup-policy-dialog";
import { BackupVerifySheet } from "@/features/data-protection/backup-verify-sheet";
import { CreateBackupDialog } from "@/features/data-protection/create-backup-dialog";
import {
  BackupIntegrityBadge,
  BackupOriginBadge,
  RestoreHistoryOutcomeBadge,
} from "@/features/data-protection/data-protection-badges";
import { formatSize, shortHash } from "@/features/data-protection/data-protection-display";
import { IntegrityReportSummary } from "@/features/data-protection/integrity-report";
import { HooksBoundary, PrivacyInventory } from "@/features/data-protection/privacy-inventory";
import {
  useCheckDataIntegrity,
  useDataProtectionOverview,
  useDeleteBackup,
  useDismissRestoreNotice,
  useRunDueAutomaticBackup,
  useVerifyBackup,
} from "@/features/data-protection/queries";
import { RestoreWizardSheet } from "@/features/data-protection/restore-wizard-sheet";

function StatusFact({
  icon: Icon,
  label,
  value,
  detail,
  tone = "default",
}: {
  icon: typeof Database;
  label: string;
  value: string;
  detail?: string;
  tone?: "default" | "success" | "warning" | "destructive";
}) {
  return (
    <div className="min-w-0">
      <p className="flex items-center gap-1.5 text-[10px] text-muted-foreground">
        <Icon
          className={cn(
            "size-3.5",
            tone === "success" && "text-success",
            tone === "warning" && "text-warning",
            tone === "destructive" && "text-destructive",
            tone === "default" && "text-muted-foreground",
          )}
          aria-hidden="true"
        />
        {label}
      </p>
      <p className="mt-1 truncate text-xs font-medium" title={value}>
        {value}
      </p>
      {detail ? (
        <p className="mt-0.5 break-words text-[10px] leading-4 text-muted-foreground">{detail}</p>
      ) : null}
    </div>
  );
}

function StartupNoticeAlert({
  notice,
  onDismiss,
}: {
  notice: RestoreStartupNotice;
  onDismiss: () => void;
}) {
  const variant =
    notice.outcome === "completed"
      ? "success"
      : notice.outcome === "needs_manual"
        ? "destructive"
        : "warning";
  const Icon =
    notice.outcome === "completed"
      ? ShieldCheck
      : notice.outcome === "needs_manual"
        ? ShieldAlert
        : History;
  return (
    <Alert variant={variant}>
      <Icon className="mt-0.5 size-4" aria-hidden="true" />
      <AlertTitle>
        启动恢复检查：{notice.outcome === "needs_manual" ? "需要人工处理" : "已处理"}
      </AlertTitle>
      <AlertDescription>
        <p>{notice.message}</p>
        <p className="mt-1 break-all font-mono text-[10px]">日志 {notice.journalFile}</p>
        {notice.rollbackDirectory ? (
          <p className="break-all font-mono text-[10px]">回退目录 {notice.rollbackDirectory}</p>
        ) : null}
        <div className="mt-2">
          <Button type="button" variant="outline" size="sm" onClick={onDismiss}>
            知道了
          </Button>
        </div>
        <p className="mt-1 text-[10px] opacity-80">
          自动回退与恢复日志不会被删除；本提示只从当前界面隐藏。
        </p>
      </AlertDescription>
    </Alert>
  );
}

function BackupRow({
  record,
  verifying,
  onVerify,
  onRestore,
  onDelete,
}: {
  record: BackupRecord;
  verifying: boolean;
  onVerify: () => void;
  onRestore: () => void;
  onDelete: () => void;
}) {
  return (
    <li className="py-3">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        <BackupOriginBadge origin={record.origin} />
        <span className="text-xs font-medium">{formatDateTime(record.createdAt)}</span>
        <BackupIntegrityBadge status={record.integrityStatus} />
        <span className="font-mono text-[11px] text-muted-foreground">
          Schema v{record.schemaVersion} · 格式 v{record.formatVersion} ·{" "}
          {formatSize(record.dataSizeBytes)}
        </span>
        <span className="font-mono text-[10px] text-muted-foreground">
          {shortId(record.backupId, 12)}
        </span>
      </div>
      <p className="mt-1 break-all font-mono text-[10px] text-muted-foreground">
        {record.directoryPath}
      </p>
      <p className="mt-0.5 break-all font-mono text-[10px] text-muted-foreground">
        SHA-256 {shortHash(record.payloadSha256)}
        {record.verifiedAt ? ` · 校验于 ${formatDateTime(record.verifiedAt)}` : ""}
      </p>
      {record.lastError ? (
        <p className="mt-1 text-[10px] text-destructive">最近错误：{record.lastError}</p>
      ) : null}
      <div className="mt-1.5 flex flex-wrap items-center gap-1">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onVerify}
          disabled={!record.directoryExists || verifying}
        >
          {verifying ? <Loader2 className="animate-spin" aria-hidden="true" /> : null}
          校验
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onRestore}
          disabled={!record.directoryExists}
        >
          <ArchiveRestore aria-hidden="true" />
          恢复
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="text-destructive hover:text-destructive"
          onClick={onDelete}
          disabled={!record.directoryExists}
        >
          <Trash2 aria-hidden="true" />
          删除
        </Button>
        {!record.directoryExists ? (
          <span className="text-[10px] text-destructive">备份目录不可用，不能校验或恢复</span>
        ) : null}
      </div>
    </li>
  );
}

function BackupSection({
  overview,
  onCreate,
  onPolicy,
  onVerify,
  onRestore,
  onDelete,
  verifyingId,
}: {
  overview: DataProtectionOverview;
  onCreate: () => void;
  onPolicy: () => void;
  onVerify: (record: BackupRecord) => void;
  onRestore: (record: BackupRecord) => void;
  onDelete: (record: BackupRecord) => void;
  verifyingId: string | null;
}) {
  const policy = overview.policy;
  return (
    <Section aria-label="数据库备份">
      <SectionHeader
        title="数据库备份"
        meta="一致性快照 · SHA-256 · 未加密"
        actions={
          <>
            <Button type="button" variant="outline" size="sm" onClick={onPolicy}>
              <Clock3 aria-hidden="true" />
              备份策略
            </Button>
            <Button type="button" size="sm" onClick={onCreate}>
              <DatabaseBackup aria-hidden="true" />
              创建备份
            </Button>
          </>
        }
      />

      <div className="border-y border-line/70 py-2 text-[11px] leading-5 text-muted-foreground">
        <p>
          自动备份：{policy.automaticEnabled ? "已启用" : "关闭"} · 间隔 {policy.intervalHours} 小时
          · 保留 {policy.retentionCount} 份。应用关闭后不会运行后台备份。
        </p>
        <p className="break-all">
          目标目录：{policy.targetDirectory ?? "未配置"} · 目标可用空间：
          {overview.policyTargetFreeBytes !== null
            ? formatSize(overview.policyTargetFreeBytes)
            : "无法读取"}{" "}
          · 最近自动备份：{formatDateTime(policy.lastAutomaticAt)}
        </p>
        {policy.lastAutomaticErrorCode ? (
          <p className="text-destructive">
            最近自动备份失败：{policy.lastAutomaticErrorCode} ·{" "}
            {formatDateTime(policy.lastAutomaticErrorAt)}
          </p>
        ) : null}
        <p>
          备份总数 {overview.backups.length} 份 · 占用 {formatSize(overview.backupTotalSizeBytes)}
        </p>
      </div>

      {overview.backups.length === 0 ? (
        <EmptyState
          icon={DatabaseBackup}
          title="尚未创建数据库备份"
          description="创建一致性快照后，XUSHU 会校验清单、SHA-256、SQLite 完整性与关键业务关系。备份文件存在本身不等于可恢复。"
          action={
            <Button type="button" onClick={onCreate}>
              创建第一份备份
            </Button>
          }
        />
      ) : (
        <ul className="divide-y divide-line/60 border-b border-line/70">
          {overview.backups.map((record) => (
            <BackupRow
              key={record.backupId}
              record={record}
              verifying={verifyingId === record.backupId}
              onVerify={() => onVerify(record)}
              onRestore={() => onRestore(record)}
              onDelete={() => onDelete(record)}
            />
          ))}
        </ul>
      )}
    </Section>
  );
}

export function DataProtectionPage() {
  const overviewQuery = useDataProtectionOverview();
  const verifyBackup = useVerifyBackup();
  const deleteBackup = useDeleteBackup();
  const checkIntegrity = useCheckDataIntegrity();
  const dismissNotice = useDismissRestoreNotice();
  const runAutomatic = useRunDueAutomaticBackup();
  const attemptedDue = useRef(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [policyOpen, setPolicyOpen] = useState(false);
  const [verifyOpen, setVerifyOpen] = useState(false);
  const [verification, setVerification] = useState<BackupVerification | null>(null);
  const [verifyingId, setVerifyingId] = useState<string | null>(null);
  const [restoreOpen, setRestoreOpen] = useState(false);
  const [restoreDirectory, setRestoreDirectory] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<BackupRecord | null>(null);

  const overview = overviewQuery.data;

  useEffect(() => {
    if (!overview || attemptedDue.current) {
      return;
    }
    if (overview.policy.automaticEnabled && overview.automaticDue) {
      attemptedDue.current = true;
      runAutomatic.mutate(undefined, {
        onSuccess: (outcome) => {
          if (outcome.status === "created") {
            toast.success("自动备份已创建并通过校验");
          } else if (outcome.status === "failed") {
            toast.error(outcome.message ?? "自动备份失败");
          }
        },
        onError: (error) => toast.error(getErrorMessage(error)),
      });
    }
  }, [overview, runAutomatic]);

  function requestVerify(record: BackupRecord) {
    setVerifyingId(record.backupId);
    setVerification(null);
    verifyBackup.mutate(record.backupId, {
      onSuccess: (result) => {
        setVerification(result);
        setVerifyOpen(true);
        setVerifyingId(null);
      },
      onError: (error) => {
        setVerifyingId(null);
        toast.error(getErrorMessage(error));
      },
    });
  }

  function requestRestore(record: BackupRecord) {
    setRestoreDirectory(record.directoryPath);
    setRestoreOpen(true);
  }

  function confirmDelete() {
    if (!deleteTarget) {
      return;
    }
    const backupId = deleteTarget.backupId;
    deleteBackup.mutate(backupId, {
      onSuccess: () => {
        toast.success("备份已删除");
        setDeleteTarget(null);
      },
      onError: (error) => toast.error(getErrorMessage(error)),
    });
  }

  function copyDataDirectory() {
    if (!overview) {
      return;
    }
    void copyTextToClipboard(overview.dataDirectory)
      .then(() => toast.success("已复制应用数据目录路径"))
      .catch(() => toast.error("复制失败，请手动选择路径"));
  }

  if (overviewQuery.isPending) {
    return <LoadingState label="正在读取数据保护状态" rows={6} />;
  }

  if (overviewQuery.isError || !overview) {
    return (
      <ErrorState
        title="无法读取数据保护状态"
        message={getErrorMessage(overviewQuery.error)}
        onRetry={() => overviewQuery.refetch()}
      />
    );
  }

  const databaseTone = overview.database.ok ? "success" : "destructive";
  const lastBackup = overview.lastVerifiedBackup ?? overview.lastBackup;

  return (
    <div className="space-y-5">
      <h1 className="sr-only">数据保护</h1>

      {overview.startupNotice ? (
        <StartupNoticeAlert
          notice={overview.startupNotice}
          onDismiss={() => dismissNotice.mutate()}
        />
      ) : null}

      <Section aria-label="数据保护状态">
        <div className="grid grid-cols-1 gap-x-5 gap-y-3 border-y border-line/70 py-3 sm:grid-cols-2 xl:grid-cols-4">
          <StatusFact
            icon={Database}
            label="本地数据库"
            value={`v${overview.database.schemaVersion} · ${overview.database.journalMode.toUpperCase()}`}
            detail={
              overview.database.ok
                ? `完整性正常 · 主库 ${formatSize(overview.databaseSizeBytes)} · WAL ${formatSize(overview.walSizeBytes)}`
                : "完整性或外键检查未通过"
            }
            tone={databaseTone}
          />
          <StatusFact
            icon={lastBackup?.integrityStatus === "verified" ? BadgeCheck : ShieldAlert}
            label={lastBackup?.integrityStatus === "verified" ? "最近验证备份" : "最近备份"}
            value={lastBackup ? formatDateTime(lastBackup.createdAt) : "尚无已验证备份"}
            detail={
              lastBackup
                ? `${formatSize(lastBackup.dataSizeBytes)} · ${shortId(lastBackup.backupId, 10)}`
                : "请创建一致性备份并完成校验"
            }
            tone={lastBackup?.integrityStatus === "verified" ? "success" : "warning"}
          />
          <StatusFact
            icon={Clock3}
            label="备份策略"
            value={
              overview.policy.automaticEnabled
                ? `自动 · 每 ${overview.policy.intervalHours} 小时`
                : "手动备份"
            }
            detail={`保留 ${overview.policy.retentionCount} 份自动备份 · 手动备份不自动删除`}
          />
          <StatusFact
            icon={LockKeyhole}
            label="备份隐私"
            value={overview.encryption.encrypted ? "已加密" : "未加密"}
            detail="SHA-256 校验不提供加密；请保存在受控位置"
            tone="warning"
          />
        </div>
      </Section>

      {overview.warnings.length > 0 ? (
        <Alert variant="warning">
          <FileWarning className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>当前数据保护注意事项</AlertTitle>
          <AlertDescription>
            <ul className="list-disc space-y-1 pl-4">
              {overview.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </AlertDescription>
        </Alert>
      ) : null}

      <BackupSection
        overview={overview}
        onCreate={() => setCreateOpen(true)}
        onPolicy={() => setPolicyOpen(true)}
        onVerify={requestVerify}
        onRestore={requestRestore}
        onDelete={setDeleteTarget}
        verifyingId={verifyingId}
      />

      <Section aria-label="恢复历史">
        <SectionHeader title="恢复历史" meta="最近 10 次实际操作" />
        {overview.restoreOperations.length === 0 ? (
          <EmptyState
            icon={History}
            title="尚无恢复操作记录"
            description="只有通过预检、隔离验证和明确确认后才会执行正式恢复。恢复操作不会自动修改 Claude Code Hooks 配置。"
          />
        ) : (
          <ul className="divide-y divide-line/60 border-y border-line/70">
            {overview.restoreOperations.map((operation) => (
              <li key={operation.id} className="flex flex-wrap items-start gap-x-3 gap-y-1 py-2.5">
                <RestoreHistoryOutcomeBadge outcome={operation.outcome} />
                <span className="text-[11px] text-muted-foreground">
                  {formatDateTime(operation.startedAt)} · 备份 {shortId(operation.backupId, 10)}
                </span>
                <span className="text-[11px] text-muted-foreground">
                  切换校验：{operation.switchVerified ? "通过" : "未通过"} · 回退：
                  {operation.rollbackStatus}
                </span>
                <span className="min-w-0 flex-1 break-all text-right font-mono text-[10px] text-muted-foreground">
                  {operation.backupDirectory}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Section>

      <Section aria-label="数据完整性">
        <SectionHeader
          title="数据完整性"
          meta="SQLite 结构、外键与业务关系"
          actions={
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => checkIntegrity.mutate()}
              disabled={checkIntegrity.isPending}
            >
              {checkIntegrity.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <RefreshCw aria-hidden="true" />
              )}
              立即检查
            </Button>
          }
        />
        <div className="border-y border-line/70 py-2 text-[11px] leading-5 text-muted-foreground">
          当前健康状态：{overview.database.ok ? "通过" : "需要处理"} · Schema v
          {overview.database.schemaVersion} · 外键
          {overview.database.foreignKeysEnabled ? "已启用" : "未启用"} · 主库{" "}
          {formatSize(overview.databaseSizeBytes)}
        </div>
        {checkIntegrity.isPending ? (
          <LoadingState label="正在检查数据库完整性与业务关系" rows={3} />
        ) : null}
        {checkIntegrity.isError ? (
          <ErrorState title="完整性检查失败" message={getErrorMessage(checkIntegrity.error)} />
        ) : null}
        {checkIntegrity.data ? (
          <div className="pt-2">
            <IntegrityReportSummary report={checkIntegrity.data} />
          </div>
        ) : (
          <p className="text-[11px] leading-5 text-muted-foreground">
            尚未在本页执行检查。检查不会修改任务状态、验收证据或完成门禁。
          </p>
        )}
      </Section>

      <Section aria-label="隐私与本地数据">
        <SectionHeader title="隐私与本地数据" meta="本地优先 · 无云同步" />
        <div className="space-y-3">
          <p className="text-[11px] leading-5 text-muted-foreground">
            XUSHU 不提供通用 SQL、Shell
            或任意文件系统权限。下列清单来自当前数据库与运行时探测；不含数据正文。
            备份是完整导出路径，复制或上传前请先阅读敏感性说明。
          </p>
          <HooksBoundary overview={overview} />
          <PrivacyInventory assets={overview.inventory} />
          <div className="rounded-md border border-line/80 p-3">
            <p className="flex items-center gap-1.5 text-[11px] font-medium">
              <LockKeyhole className="size-3.5 text-warning" aria-hidden="true" />
              备份加密状态：未加密（当前版本不支持）
            </p>
            <p className="mt-1 text-[11px] leading-5 text-muted-foreground">
              {overview.encryption.note}
            </p>
            <p className="mt-1 text-[11px] leading-5 text-muted-foreground">
              保留策略：Hook 事件 {overview.retention.hookEventDays} 天 /{" "}
              {overview.retention.hookMaxEventsPerProject} 条每项目；恢复快照{" "}
              {overview.retention.recoverySnapshotsPerProject} 份每项目；自动备份保留{" "}
              {overview.retention.backupRetentionCount} 份。
            </p>
            <p className="mt-1 text-[11px] leading-5 text-muted-foreground">
              回退副本：{overview.retention.rollbackArchives} 自动清理：
              {overview.retention.backupFiles}
            </p>
          </div>
          <div className="border-y border-line/70 py-2">
            <p className="text-[11px] font-medium">应用数据目录</p>
            <p className="mt-1 break-all font-mono text-[10px] text-muted-foreground">
              {overview.dataDirectory}
            </p>
            <div className="mt-1.5 flex flex-wrap items-center gap-2">
              <Button type="button" variant="outline" size="sm" onClick={copyDataDirectory}>
                <Copy aria-hidden="true" />
                复制路径
              </Button>
              <span className="text-[10px] text-muted-foreground">仅在本机显示，不会上传。</span>
            </div>
          </div>
          <p className="flex items-start gap-1.5 text-[11px] leading-5 text-muted-foreground">
            <HardDrive className="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
            数据目录位于 Windows
            应用数据目录；备份目标由用户选择，可选择其他磁盘。请确认目标磁盘有足够空间。
          </p>
        </div>
      </Section>

      <div className="flex items-center gap-2 border-t border-line/70 pt-3 text-[11px] text-muted-foreground">
        <ShieldCheck className="size-3.5" aria-hidden="true" />
        自动验证结果、人工确认与备份状态分别记录；备份或恢复成功不会自动改变任务完成门禁。
      </div>

      <CreateBackupDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        defaultTarget={overview.policy.targetDirectory ?? ""}
        estimatedBytes={overview.databaseSizeBytes + overview.walSizeBytes}
      />
      <BackupPolicyDialog open={policyOpen} onOpenChange={setPolicyOpen} policy={overview.policy} />
      <BackupVerifySheet
        open={verifyOpen}
        onOpenChange={(next) => {
          setVerifyOpen(next);
          if (!next) {
            setVerification(null);
          }
        }}
        verification={verification}
      />
      <RestoreWizardSheet
        open={restoreOpen}
        onOpenChange={setRestoreOpen}
        initialDirectory={restoreDirectory}
      />
      <ConfirmationDialog
        open={Boolean(deleteTarget)}
        onOpenChange={(next) => {
          if (!next) {
            setDeleteTarget(null);
          }
        }}
        title="删除这份备份？"
        description={
          deleteTarget
            ? `将永久删除 ${deleteTarget.directoryPath} 中的 manifest.json 与 xushu.db。删除前会重新校验清单与数据文件；目录中存在其他文件时会拒绝删除。`
            : ""
        }
        confirmLabel="删除备份"
        destructive
        pending={deleteBackup.isPending}
        onConfirm={confirmDelete}
      />
      {runAutomatic.isPending ? (
        <p className="sr-only" role="status">
          正在执行到期的自动备份
        </p>
      ) : null}
    </div>
  );
}
