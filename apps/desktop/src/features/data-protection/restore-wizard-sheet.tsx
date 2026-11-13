import {
  AlertTriangle,
  ArrowLeft,
  CheckCircle2,
  FileWarning,
  FolderSearch,
  Loader2,
  RotateCcw,
  ShieldAlert,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useState } from "react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import type {
  IntegrityCheck,
  RestoreDataSummary,
  RestoreOutcome,
  RestorePreflight,
} from "@/lib/domain";
import { formatDateTime, shortId } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";

import { IntegrityCheckList, IntegrityReportSummary } from "./integrity-report";
import { BackupOriginBadge, RestoreOutcomeBadge } from "./data-protection-badges";
import {
  formatSize,
  RESTORE_STEPS,
  restoreResultTone,
  restoreStepIndex,
  restoreStepState,
  type RestoreWizardStage,
} from "./data-protection-display";
import { usePerformRestore, usePreflightRestore } from "./queries";

const CONFIRMATION_TEXT = "恢复";

function StepTrack({
  stage,
  ready,
  preflightFailed,
  resultFailed,
}: {
  stage: RestoreWizardStage;
  ready: boolean;
  preflightFailed: boolean;
  resultFailed: boolean;
}) {
  const current = stage === "result" ? 6 : restoreStepIndex(stage, ready);
  return (
    <ol
      aria-label="恢复步骤"
      className="flex flex-wrap gap-x-3 gap-y-1.5 border-y border-line/70 py-2"
    >
      {RESTORE_STEPS.map((label, index) => {
        let state = restoreStepState(index, current, preflightFailed);
        if (stage === "result") {
          state = index < 6 ? "done" : resultFailed ? "failed" : "done";
        }
        return (
          <li
            key={label}
            aria-current={state === "active" ? "step" : undefined}
            className={cn(
              "flex items-center gap-1.5 text-[11px]",
              state === "done" && "text-success",
              state === "active" && "font-medium text-primary",
              state === "failed" && "text-destructive",
              state === "pending" && "text-muted-foreground",
            )}
          >
            <span
              className={cn(
                "grid size-4 shrink-0 place-items-center rounded-full border font-mono text-[9px]",
                state === "done" && "border-success/50 bg-success/10",
                state === "active" && "border-primary/60 bg-accent",
                state === "failed" && "border-destructive/50 bg-destructive/10",
                state === "pending" && "border-line",
              )}
              aria-hidden="true"
            >
              {state === "done" ? "✓" : index + 1}
            </span>
            {label}
          </li>
        );
      })}
    </ol>
  );
}

function SummaryComparison({
  current,
  incoming,
}: {
  current: RestoreDataSummary;
  incoming: RestoreDataSummary;
}) {
  const rows: Array<[string, number, number]> = [
    ["项目", current.projects, incoming.projects],
    ["任务", current.tasks, incoming.tasks],
    ["已完成任务", current.doneTasks, incoming.doneTasks],
    ["验收条件", current.criteria, incoming.criteria],
    ["验证运行", current.verificationRuns, incoming.verificationRuns],
    ["证据", current.evidence, incoming.evidence],
    ["Claude 事件", current.claudeEvents, incoming.claudeEvents],
    ["长期记忆", current.memories, incoming.memories],
    ["开发决策", current.decisions, incoming.decisions],
    ["恢复快照", current.recoverySnapshots, incoming.recoverySnapshots],
  ];

  return (
    <div className="space-y-2">
      <div className="overflow-x-auto rounded-md border border-line/70">
        <table className="w-full text-[11px]">
          <thead className="border-b border-line/70 text-left text-muted-foreground">
            <tr>
              <th className="px-2 py-1.5 font-medium">数据</th>
              <th className="px-2 py-1.5 text-right font-medium">当前数据库</th>
              <th className="px-2 py-1.5 text-right font-medium">备份中</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-line/60">
            {rows.map(([label, left, right]) => (
              <tr key={label}>
                <th scope="row" className="px-2 py-1 text-left font-normal text-muted-foreground">
                  {label}
                </th>
                <td className="px-2 py-1 text-right font-mono tabular-nums">{left}</td>
                <td className="px-2 py-1 text-right font-mono tabular-nums">{right}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="grid gap-1 text-[11px] leading-5 text-muted-foreground sm:grid-cols-2">
        <p>当前数据量：{formatSize(current.databaseSizeBytes)}</p>
        <p>备份数据量：{formatSize(incoming.databaseSizeBytes)}</p>
        <p className="break-words">
          备份项目：{incoming.projectNames.length > 0 ? incoming.projectNames.join("、") : "无"}
        </p>
        <p>
          缺失本地路径：{incoming.missingProjectRootCount} 个 · 未结束验证：
          {incoming.runningVerificationCount} 个
        </p>
      </div>
    </div>
  );
}

function ChecksForStep({ checks, prefix }: { checks: IntegrityCheck[]; prefix?: string }) {
  const filtered = prefix
    ? checks.filter((check) => check.id.startsWith(prefix))
    : checks.filter((check) => !check.id.startsWith("isolated_"));
  return <IntegrityCheckList checks={filtered} />;
}

function PreflightDetails({
  preflight,
  onContinue,
  onRetry,
  onBack,
}: {
  preflight: RestorePreflight;
  onContinue: () => void;
  onRetry: () => void;
  onBack: () => void;
}) {
  const warningList = preflight.warnings;
  const effectList = preflight.effects;

  return (
    <div className="space-y-4">
      <div className="grid gap-x-4 gap-y-1 border-y border-line/70 py-2 text-[11px] sm:grid-cols-2">
        <p className="text-muted-foreground">
          备份 <span className="font-mono">{shortId(preflight.backupId, 12)}</span>
        </p>
        <p className="text-muted-foreground">
          创建时间 <span className="font-mono">{formatDateTime(preflight.createdAt)}</span>
        </p>
        <p className="text-muted-foreground">
          Schema v{preflight.backupSchemaVersion} ·{" "}
          {preflight.schemaAction === "same"
            ? "与当前一致"
            : preflight.schemaAction === "upgrade"
              ? "需要升级"
              : "不兼容"}
        </p>
        <p className="text-muted-foreground">
          格式 v{preflight.formatVersion} · {formatSize(preflight.payloadSizeBytes)}
        </p>
        <p className="flex items-center gap-1.5 text-muted-foreground">
          来源 <BackupOriginBadge origin={preflight.origin} />
        </p>
        <p className="break-all font-mono text-muted-foreground">{preflight.backupDirectory}</p>
        <p className="text-muted-foreground sm:col-span-2">
          预检有效期至 <span className="font-mono">{formatDateTime(preflight.expiresAt)}</span>
        </p>
      </div>

      {!preflight.ready ? (
        <Alert variant="destructive">
          <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>不能进入正式恢复</AlertTitle>
          <AlertDescription>{preflight.blockingReason ?? "备份未通过预检。"}</AlertDescription>
        </Alert>
      ) : null}

      <section className="space-y-2" aria-label="兼容性与清单检查">
        <h3 className="text-xs font-semibold">兼容性与清单检查</h3>
        <ChecksForStep checks={preflight.checks} />
      </section>

      {preflight.incoming ? (
        <>
          <section className="space-y-2" aria-label="影响范围">
            <h3 className="text-xs font-semibold">影响范围</h3>
            <SummaryComparison current={preflight.current} incoming={preflight.incoming} />
            <div className="rounded-md border border-warning/40 bg-warning/5 p-3 text-[11px]">
              <p className="font-medium text-warning">备份之后当前数据库的变化</p>
              {preflight.changesSinceBackup.length > 0 ? (
                <>
                  <p className="mt-1 leading-5 text-muted-foreground">
                    这些变更不在备份中；恢复后仅保留在回退副本，不会出现在恢复后的数据库。
                  </p>
                  <ul className="mt-1 grid gap-x-4 gap-y-0.5 sm:grid-cols-2">
                    {preflight.changesSinceBackup.map((change) => (
                      <li key={change.label}>
                        {change.label}：<span className="font-mono">{change.count}</span>
                      </li>
                    ))}
                  </ul>
                </>
              ) : (
                <p className="mt-1 leading-5 text-muted-foreground">
                  按时间戳未检测到备份之后的变更；仍请确认当前数据库状态。
                </p>
              )}
            </div>
            <ul className="list-disc space-y-1 pl-4 text-[11px] leading-5 text-muted-foreground">
              {effectList.map((effect) => (
                <li key={effect}>{effect}</li>
              ))}
            </ul>
          </section>

          <section className="space-y-2" aria-label="隔离验证">
            <h3 className="text-xs font-semibold">隔离恢复验证</h3>
            <ChecksForStep checks={preflight.checks} prefix="isolated_" />
          </section>
        </>
      ) : null}

      {warningList.length > 0 ? (
        <Alert variant="warning">
          <FileWarning className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>恢复前必须了解的边界</AlertTitle>
          <AlertDescription>
            <ul className="list-disc space-y-1 pl-4">
              {warningList.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </AlertDescription>
        </Alert>
      ) : null}

      <Alert variant="info">
        <ShieldCheck className="mt-0.5 size-4" aria-hidden="true" />
        <AlertTitle>Hooks 与本地配置不会被自动修改</AlertTitle>
        <AlertDescription>
          {preflight.hooksReview.note} 当前有 {preflight.hooksReview.projectCount} 个项目期望启用
          Hooks；恢复后请重新检查。
        </AlertDescription>
      </Alert>

      <div className="flex flex-wrap items-center gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeft aria-hidden="true" />
          返回选择
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={onRetry}>
          <RotateCcw aria-hidden="true" />
          重新预检
        </Button>
        {preflight.ready && preflight.token ? (
          <Button type="button" size="sm" onClick={onContinue}>
            继续确认
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function RestoreResult({
  result,
  error,
  onRestart,
  onClose,
}: {
  result: RestoreOutcome | null;
  error: string | null;
  onRestart: () => void;
  onClose: () => void;
}) {
  if (error || !result) {
    return (
      <div className="space-y-4">
        <Alert variant="destructive">
          <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>恢复未执行</AlertTitle>
          <AlertDescription>{error ?? "恢复未返回结果。"}</AlertDescription>
        </Alert>
        <p className="text-[11px] leading-5 text-muted-foreground">
          若错误发生在预检或确认阶段，当前数据库未被替换。请重新预检后再试。
        </p>
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="outline" size="sm" onClick={onRestart}>
            重新开始
          </Button>
          <Button type="button" size="sm" onClick={onClose}>
            关闭
          </Button>
        </div>
      </div>
    );
  }

  const tone = restoreResultTone(result);
  const variant = tone === "success" ? "success" : tone === "warning" ? "warning" : "destructive";
  const Icon = tone === "success" ? CheckCircle2 : tone === "warning" ? RotateCcw : ShieldAlert;

  return (
    <div className="space-y-4">
      <Alert variant={variant}>
        <Icon className="mt-0.5 size-4" aria-hidden="true" />
        <AlertTitle className="flex flex-wrap items-center gap-2">
          <RestoreOutcomeBadge outcome={result.status} />
          {result.message}
        </AlertTitle>
        <AlertDescription>
          <p>
            操作 {shortId(result.operationId, 12)} · {formatDateTime(result.completedAt)}
          </p>
          <p>
            切换后最终校验：{result.switchVerified ? "通过" : "未通过"} · 回退状态：
            {result.rollback.verified ? "已验证" : "未验证"}
          </p>
          {result.rollback.directory ? (
            <p className="mt-1 break-all font-mono text-[10px]">{result.rollback.directory}</p>
          ) : null}
          <p className="mt-1">{result.rollback.message}</p>
        </AlertDescription>
      </Alert>

      {result.finalReport ? (
        <section className="space-y-2" aria-label="恢复后的数据完整性">
          <h3 className="text-xs font-semibold">恢复后的数据完整性</h3>
          <IntegrityReportSummary report={result.finalReport} compact />
        </section>
      ) : (
        <p className="text-[11px] leading-5 text-muted-foreground">
          未获得恢复后的最终完整性报告；不能把本次恢复视为已验证。
        </p>
      )}

      {result.hooksReview ? (
        <Alert variant="warning">
          <FileWarning className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>需要重新检查 Claude Code Hooks</AlertTitle>
          <AlertDescription>
            {result.hooksReview.note} 受影响项目：{result.hooksReview.projectCount} 个。项目 .claude
            配置未被修改。
          </AlertDescription>
        </Alert>
      ) : null}

      {result.warnings.length > 0 ? (
        <section className="space-y-1" aria-label="恢复警告">
          <h3 className="text-xs font-semibold">注意事项</h3>
          <ul className="list-disc space-y-1 pl-4 text-[11px] leading-5 text-muted-foreground">
            {result.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </section>
      ) : null}

      <div className="flex flex-wrap gap-2">
        <Button type="button" variant="outline" size="sm" onClick={onRestart}>
          重新开始
        </Button>
        <Button type="button" size="sm" onClick={onClose}>
          关闭
        </Button>
      </div>
    </div>
  );
}

export function RestoreWizardSheet({
  open,
  onOpenChange,
  initialDirectory,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialDirectory: string;
}) {
  const [stage, setStage] = useState<RestoreWizardStage>("choose");
  const [directory, setDirectory] = useState(initialDirectory);
  const [preflight, setPreflight] = useState<RestorePreflight | null>(null);
  const [preflightError, setPreflightError] = useState<string | null>(null);
  const [confirmationText, setConfirmationText] = useState("");
  const [acknowledged, setAcknowledged] = useState(false);
  const [result, setResult] = useState<RestoreOutcome | null>(null);
  const [performError, setPerformError] = useState<string | null>(null);
  const preflightMutation = usePreflightRestore();
  const performMutation = usePerformRestore();

  useEffect(() => {
    if (!open) {
      return;
    }
    setStage("choose");
    setDirectory(initialDirectory);
    setPreflight(null);
    setPreflightError(null);
    setConfirmationText("");
    setAcknowledged(false);
    setResult(null);
    setPerformError(null);
  }, [open, initialDirectory]);

  const ready = Boolean(preflight?.ready && preflight.token);
  const canExecute =
    Boolean(preflight?.token) &&
    confirmationText.trim() === CONFIRMATION_TEXT &&
    acknowledged &&
    !performMutation.isPending;

  function startPreflight() {
    const trimmed = directory.trim();
    if (!trimmed) {
      setPreflightError("请输入备份目录");
      return;
    }
    setStage("preflight");
    setPreflight(null);
    setPreflightError(null);
    setResult(null);
    preflightMutation.mutate(trimmed, {
      onSuccess: (value) => setPreflight(value),
      onError: (error) => setPreflightError(getErrorMessage(error)),
    });
  }

  function execute() {
    if (!preflight?.token || !canExecute) {
      return;
    }
    setStage("executing");
    setPerformError(null);
    performMutation.mutate(
      {
        token: preflight.token,
        confirmationText,
        acknowledged,
      },
      {
        onSuccess: (outcome) => {
          setResult(outcome);
          setStage("result");
        },
        onError: (error) => {
          setPerformError(getErrorMessage(error));
          setStage("result");
        },
      },
    );
  }

  function restart() {
    setStage("choose");
    setPreflight(null);
    setPreflightError(null);
    setConfirmationText("");
    setAcknowledged(false);
    setResult(null);
    setPerformError(null);
  }

  const stageFailed = stage === "preflight" && !ready && !preflightMutation.isPending;
  const resultFailed =
    stage === "result" && (Boolean(performError) || !result || result.status !== "completed");
  const title =
    stage === "choose"
      ? "恢复数据库"
      : stage === "preflight"
        ? "恢复预检"
        : stage === "confirm"
          ? "确认恢复"
          : stage === "executing"
            ? "正在恢复"
            : "恢复结果";

  return (
    <Sheet
      open={open}
      onOpenChange={(next) => {
        if (!performMutation.isPending) {
          onOpenChange(next);
        }
      }}
    >
      <SheetContent aria-describedby="restore-wizard-description">
        <SheetHeader>
          <SheetTitle>{title}</SheetTitle>
          <SheetDescription id="restore-wizard-description">
            选择备份、只读预检、隔离验证、明确确认、执行切换并验证结果。任何未通过最终校验的恢复都不会被标记为成功。
          </SheetDescription>
        </SheetHeader>
        <SheetBody className="space-y-4">
          <StepTrack
            stage={stage}
            ready={ready}
            preflightFailed={stageFailed}
            resultFailed={resultFailed}
          />

          {stage === "choose" ? (
            <div className="space-y-4">
              <Alert variant="destructive">
                <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
                <AlertTitle>恢复会替换当前数据库</AlertTitle>
                <AlertDescription>
                  未包含在备份中的当前变更将丢失。切换前会创建当前数据库的一致性回退副本；只有通过隔离验证后才能继续。
                </AlertDescription>
              </Alert>
              <div className="space-y-1.5">
                <Label htmlFor="restore-directory">备份目录</Label>
                <Input
                  id="restore-directory"
                  value={directory}
                  onChange={(event) => setDirectory(event.target.value)}
                  placeholder="输入包含 manifest.json 与 xushu.db 的备份目录"
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      startPreflight();
                    }
                  }}
                />
                <p className="text-[11px] leading-5 text-muted-foreground">
                  可直接使用备份历史中的“恢复”入口。预检只读取备份并在隔离目录中验证，不修改当前数据库。
                </p>
              </div>
              {preflightError ? (
                <Alert variant="destructive" role="alert">
                  <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
                  <AlertTitle>预检未开始</AlertTitle>
                  <AlertDescription>{preflightError}</AlertDescription>
                </Alert>
              ) : null}
              <Button type="button" onClick={startPreflight}>
                <FolderSearch aria-hidden="true" />
                开始预检
              </Button>
            </div>
          ) : null}

          {stage === "preflight" ? (
            <div className="space-y-4">
              {preflightMutation.isPending ? (
                <Alert variant="info">
                  <Loader2 className="mt-0.5 size-4 animate-spin" aria-hidden="true" />
                  <AlertTitle>正在执行只读预检与隔离恢复</AlertTitle>
                  <AlertDescription>
                    正在校验清单与 SHA-256，并复制到隔离目录执行迁移、SQLite
                    完整性与业务关系检查。不会修改当前数据库。
                  </AlertDescription>
                </Alert>
              ) : preflightError ? (
                <Alert variant="destructive">
                  <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
                  <AlertTitle>预检失败</AlertTitle>
                  <AlertDescription>{preflightError}</AlertDescription>
                </Alert>
              ) : preflight ? (
                <PreflightDetails
                  preflight={preflight}
                  onBack={restart}
                  onRetry={startPreflight}
                  onContinue={() => setStage("confirm")}
                />
              ) : null}
              {stageFailed ? (
                <Button type="button" variant="outline" size="sm" onClick={startPreflight}>
                  <RotateCcw aria-hidden="true" />
                  重试预检
                </Button>
              ) : null}
            </div>
          ) : null}

          {stage === "confirm" && preflight ? (
            <div className="space-y-4">
              <Alert variant="destructive">
                <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
                <AlertTitle>最后确认</AlertTitle>
                <AlertDescription>
                  当前数据库将被备份替换。XUSHU
                  会先创建回退副本，但恢复仍可能因磁盘、文件占用或数据关系问题失败。失败时会优先回退。
                </AlertDescription>
              </Alert>
              {preflight.incoming ? (
                <SummaryComparison current={preflight.current} incoming={preflight.incoming} />
              ) : null}
              {preflight.changesSinceBackup.length > 0 ? (
                <Alert variant="warning">
                  <FileWarning className="mt-0.5 size-4" aria-hidden="true" />
                  <AlertTitle>备份之后存在当前变更</AlertTitle>
                  <AlertDescription>
                    {preflight.changesSinceBackup
                      .map((change) => `${change.label} ${change.count}`)
                      .join(" · ")}
                    。这些变更在恢复后会从当前数据库消失，只保留在回退副本中。
                  </AlertDescription>
                </Alert>
              ) : null}
              <div className="space-y-1.5">
                <Label htmlFor="restore-confirmation">输入“{CONFIRMATION_TEXT}”以确认</Label>
                <Input
                  id="restore-confirmation"
                  value={confirmationText}
                  onChange={(event) => setConfirmationText(event.target.value)}
                  autoComplete="off"
                  aria-describedby="restore-confirmation-hint"
                />
                <p id="restore-confirmation-hint" className="text-[11px] text-muted-foreground">
                  必须逐字输入中文“恢复”；该确认只在本次预检中有效。
                </p>
              </div>
              <label className="flex cursor-pointer items-start gap-2 text-[11px] leading-5">
                <input
                  type="checkbox"
                  checked={acknowledged}
                  onChange={(event) => setAcknowledged(event.target.checked)}
                  className="mt-0.5 size-4 accent-[var(--destructive)]"
                />
                我已阅读影响范围与风险，确认用该备份替换当前数据库，并理解未包含在备份中的变更会丢失。
              </label>
              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setStage("preflight")}
                >
                  返回预检
                </Button>
                <Button
                  type="button"
                  variant="destructive"
                  size="sm"
                  onClick={execute}
                  disabled={!canExecute}
                >
                  <ShieldAlert aria-hidden="true" />
                  执行恢复
                </Button>
              </div>
            </div>
          ) : null}

          {stage === "executing" ? (
            <Alert variant="info">
              <Loader2 className="mt-0.5 size-4 animate-spin" aria-hidden="true" />
              <AlertTitle>正在创建回退副本、切换数据库并执行最终校验</AlertTitle>
              <AlertDescription>
                请勿关闭应用或移动备份/应用数据目录。恢复命令不可中途取消；若切换或最终校验失败，将自动尝试回退。
              </AlertDescription>
            </Alert>
          ) : null}

          {stage === "result" ? (
            <RestoreResult
              result={result}
              error={performError}
              onRestart={restart}
              onClose={() => onOpenChange(false)}
            />
          ) : null}
        </SheetBody>
        <SheetFooter>
          {stage === "choose" ? (
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              取消
            </Button>
          ) : null}
          {stage === "executing" ? (
            <Button type="button" variant="outline" disabled>
              <Loader2 className="animate-spin" aria-hidden="true" />
              执行中
            </Button>
          ) : null}
          {stage === "result" ? (
            <Button type="button" onClick={() => onOpenChange(false)}>
              关闭
            </Button>
          ) : null}
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
