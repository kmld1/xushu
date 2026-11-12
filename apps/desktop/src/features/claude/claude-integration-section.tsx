import { Activity, Loader2, MoreHorizontal, RefreshCw, ShieldCheck, Wrench } from "lucide-react";
import { useRef, useState, type MouseEvent } from "react";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Section, SectionHeader } from "@/components/section";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { Project } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";

import { ClaudeActivitySheet } from "./claude-activity-sheet";
import { ClaudeStateBadge } from "./claude-badges";
import { ClaudeEventRow } from "./claude-event-line";
import { ClaudeInstallDialog } from "./claude-install-dialog";
import {
  useClaudeActivity,
  useClaudeStatus,
  useDisableClaudeHooks,
  useUninstallClaudeHooks,
} from "./queries";

type ConfirmAction = "disable" | "uninstall" | null;

function importErrorLabel(code: string): string {
  return (
    {
      spool_too_large: "收到的事件文件超过大小上限",
      invalid_envelope: "收到的事件格式无效",
      project_mismatch: "事件不属于当前项目或项目路径已变化",
      project_unavailable: "项目目录不可访问，无法校验事件归属",
      spool_unavailable: "本地事件文件不可读",
      DATABASE_ERROR: "本地数据库暂时不可用",
    }[code] ?? "最近一次事件导入未完成"
  );
}

export function ClaudeIntegrationSection({
  project,
  archived,
}: {
  project: Project;
  archived: boolean;
}) {
  const status = useClaudeStatus(project.id);
  const activity = useClaudeActivity(
    project.id,
    Boolean(
      status.data &&
      (status.data.hasRecord ||
        status.data.eventCount > 0 ||
        status.data.installState !== "not_configured"),
    ),
  );
  const disable = useDisableClaudeHooks(project.id);
  const uninstall = useUninstallClaudeHooks(project.id);
  const [installOpen, setInstallOpen] = useState(false);
  const [installMode, setInstallMode] = useState<"install" | "repair">("install");
  const [activityOpen, setActivityOpen] = useState(false);
  const [confirmAction, setConfirmAction] = useState<ConfirmAction>(null);
  const installButton = useRef<HTMLButtonElement>(null);
  const activityButton = useRef<HTMLButtonElement>(null);
  const activityOpener = useRef<HTMLButtonElement>(null);
  const moreActionsButton = useRef<HTMLButtonElement>(null);

  const data = status.data;
  const state = data?.installState ?? "not_configured";
  const showInstall =
    !archived &&
    data !== undefined &&
    state !== "installed" &&
    (state === "not_configured" ||
      state === "disabled" ||
      state === "partial" ||
      state === "needs_repair" ||
      state === "configuration_error" ||
      state === "receiver_unavailable" ||
      state === "project_unavailable" ||
      state === "desynchronized");
  const showActivity = Boolean(data && (data.hasRecord || data.eventCount > 0));
  const showMore = Boolean(
    data && (data.hasRecord || data.managedHookCount > 0 || data.desiredEnabled),
  );
  const needsAttention =
    data !== undefined &&
    (state === "partial" ||
      state === "needs_repair" ||
      state === "configuration_error" ||
      state === "receiver_unavailable" ||
      state === "project_unavailable" ||
      state === "desynchronized" ||
      Boolean(data.lastErrorCode));

  function openInstall(mode: "install" | "repair") {
    setInstallMode(mode);
    setInstallOpen(true);
  }

  function openActivity(event: MouseEvent<HTMLButtonElement>) {
    activityOpener.current = event.currentTarget;
    setActivityOpen(true);
  }

  function restoreMoreActionsFocus(event: Event) {
    event.preventDefault();
    moreActionsButton.current?.focus();
  }

  function installLabel(): string {
    if (state === "disabled") {
      return "重新启用";
    }
    if (state === "not_configured") {
      return "查看并安装";
    }
    return "修复接入";
  }

  async function confirmActionRun() {
    if (!confirmAction) {
      return;
    }
    try {
      if (confirmAction === "disable") {
        await disable.mutateAsync({ projectId: project.id, approved: true });
        toast.success("Claude Code 事件采集已停用", {
          description: "XUSHU 创建的 Hooks 已移除，其他配置与历史事件保留。",
        });
      } else {
        await uninstall.mutateAsync({ projectId: project.id, approved: true });
        toast.success("Claude Code 接入已卸载", {
          description: "XUSHU 创建的 Hooks 已移除；历史事件仍保留。",
        });
      }
      setConfirmAction(null);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  }

  return (
    <Section aria-label="Claude Code 接入" className="min-w-0 border-y border-line/70 py-3">
      <SectionHeader
        title="Claude Code 接入"
        meta="可选 · 项目本地 Hooks"
        actions={
          data ? (
            <>
              {showActivity ? (
                <Button
                  ref={activityButton}
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={openActivity}
                >
                  <Activity aria-hidden="true" />
                  查看活动
                </Button>
              ) : null}
              {showInstall ? (
                <Button
                  ref={installButton}
                  type="button"
                  size="sm"
                  onClick={() =>
                    openInstall(
                      state === "not_configured" || state === "disabled" ? "install" : "repair",
                    )
                  }
                >
                  {state === "partial" ||
                  state === "needs_repair" ||
                  state === "configuration_error" ||
                  state === "receiver_unavailable" ||
                  state === "project_unavailable" ||
                  state === "desynchronized" ? (
                    <Wrench aria-hidden="true" />
                  ) : null}
                  {installLabel()}
                </Button>
              ) : null}
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label="刷新 Claude Code 接入状态"
                onClick={() => void status.refetch()}
                disabled={status.isFetching}
              >
                {status.isFetching ? (
                  <Loader2 className="animate-spin" aria-hidden="true" />
                ) : (
                  <RefreshCw aria-hidden="true" />
                )}
              </Button>
              {showMore ? (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      ref={moreActionsButton}
                      type="button"
                      variant="ghost"
                      size="icon"
                      aria-label="更多 Claude Code 接入操作"
                    >
                      <MoreHorizontal aria-hidden="true" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      onSelect={() => setConfirmAction("disable")}
                      disabled={disable.isPending || uninstall.isPending}
                    >
                      停用事件采集
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      destructive
                      onSelect={() => setConfirmAction("uninstall")}
                      disabled={disable.isPending || uninstall.isPending}
                    >
                      卸载接入
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              ) : null}
            </>
          ) : null
        }
      />

      {status.isPending ? (
        <LoadingState rows={2} label="正在检查 Claude Code 接入状态" />
      ) : status.isError ? (
        <ErrorState
          title="无法读取 Claude Code 接入状态"
          message={getErrorMessage(status.error)}
          onRetry={() => void status.refetch()}
        />
      ) : data ? (
        <>
          <div className="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-2 text-xs">
            <ClaudeStateBadge state={data.installState} />
            <span className="min-w-0 flex-1 text-muted-foreground">{data.stateDetail}</span>
            <span className="shrink-0 text-[11px] text-muted-foreground">
              {data.lastEventAt ? `最近接收 ${formatDateTime(data.lastEventAt)}` : "尚未收到事件"}
            </span>
          </div>

          <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px] text-muted-foreground">
            <span>
              {data.managedHookCount}/{data.expectedHookCount} 个 Hook
            </span>
            {data.otherHookGroupCount > 0 ? (
              <span>保留其他 Hook 组 {data.otherHookGroupCount} 个</span>
            ) : null}
            {data.pendingEventCount > 0 ? (
              <span className="text-warning">待导入 {data.pendingEventCount} 条</span>
            ) : null}
            {data.runningToolCount > 0 ? (
              <span className="inline-flex items-center gap-1 text-primary">
                <Loader2 className="size-3 animate-spin" aria-hidden="true" />
                {data.runningToolCount} 个工具调用未结束
              </span>
            ) : null}
            {data.receiverChanged ? (
              <span className="text-warning">接收程序已更新，需要重新安装</span>
            ) : null}
            <span className="font-mono">{data.settingsFile}</span>
          </div>

          {needsAttention ? (
            <Alert variant={state === "configuration_error" ? "destructive" : "warning"}>
              <Wrench className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>
                {state === "configuration_error"
                  ? "项目设置文件异常，XUSHU 不会覆盖它"
                  : state === "receiver_unavailable"
                    ? "Hook 接收器不可用"
                    : "接入需要处理"}
              </AlertTitle>
              <AlertDescription>
                <span className="block">{data.stateDetail}</span>
                {data.lastErrorCode ? (
                  <span className="mt-1 block">
                    最近导入异常：{importErrorLabel(data.lastErrorCode)}
                    {data.lastErrorAt ? `（${formatDateTime(data.lastErrorAt)}）` : ""}
                  </span>
                ) : null}
                {data.lastErrorCode ? (
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="mt-2"
                    onClick={() => void status.refetch()}
                  >
                    重新检查
                  </Button>
                ) : null}
              </AlertDescription>
            </Alert>
          ) : null}

          {archived ? (
            <p className="text-[11px] text-muted-foreground">
              项目已归档，不能安装或修复接入；仍可停用或卸载已有 Hooks。
            </p>
          ) : null}

          {showActivity ? (
            <div className="space-y-1">
              {activity.isPending ? (
                <LoadingState rows={2} label="正在读取最近活动" />
              ) : activity.isError ? (
                <ErrorState
                  title="无法读取最近活动"
                  message={getErrorMessage(activity.error)}
                  onRetry={() => void activity.refetch()}
                />
              ) : activity.data && activity.data.events.length > 0 ? (
                <>
                  <div className="divide-y divide-line/60 border-y border-line/70">
                    {activity.data.events.slice(0, 4).map((event) => (
                      <ClaudeEventRow key={event.id} event={event} compact />
                    ))}
                  </div>
                  <div className="flex items-center justify-between gap-2 pt-1">
                    <span className="text-[11px] text-muted-foreground">
                      {data.eventCount} 条事件 · 保留 {data.retentionDays} 天
                    </span>
                    <Button type="button" variant="ghost" size="sm" onClick={openActivity}>
                      查看全部活动
                    </Button>
                  </div>
                </>
              ) : (
                <EmptyState
                  icon={Activity}
                  title="尚未收到 Claude Code 事件"
                  description="启用后，在项目目录运行 Claude Code。事件会先写入本地收件箱，再在应用打开时导入。"
                  className="py-6"
                />
              )}
            </div>
          ) : state === "not_configured" ? (
            <div className="space-y-1">
              <p className="text-xs leading-5 text-muted-foreground">
                启用后，XUSHU 观察 Claude Code
                的会话、工具调用与结束事件，形成可追溯的开发活动时间线。它不会替代验收，也不会修改任务状态。
              </p>
              <p className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                <ShieldCheck className="size-3.5 shrink-0" aria-hidden="true" />
                仅接收事件类型、时间、伪名化标识与工具元数据；不读取
                Prompt、命令、文件内容或输出正文。
              </p>
            </div>
          ) : state === "disabled" ? (
            <p className="text-xs leading-5 text-muted-foreground">
              事件采集已停用。历史事件仍可查看，重新启用不会删除已有记录。
            </p>
          ) : null}

          <div className="flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
            {data.hookVersion ? <span className="font-mono">v{data.hookVersion}</span> : null}
            {data.installedAt ? <span>安装于 {formatDateTime(data.installedAt)}</span> : null}
            {data.lastImportAt ? <span>最近导入 {formatDateTime(data.lastImportAt)}</span> : null}
            {data.uninstalledAt ? <span>卸载于 {formatDateTime(data.uninstalledAt)}</span> : null}
            <span className={cn("ml-auto", data.inboxAvailable ? "" : "text-warning")}>
              本地收件箱{data.inboxAvailable ? "可用" : "不可用"}
            </span>
          </div>
        </>
      ) : null}

      {installOpen ? (
        <ClaudeInstallDialog
          project={project}
          open
          onOpenChange={setInstallOpen}
          mode={installMode}
          restoreFocus={() => (installButton.current ?? activityButton.current)?.focus()}
        />
      ) : null}

      {activityOpen ? (
        <ClaudeActivitySheet
          project={project}
          open
          onOpenChange={setActivityOpen}
          restoreFocus={() => {
            const opener = activityOpener.current;
            (opener?.isConnected ? opener : activityButton.current)?.focus();
          }}
        />
      ) : null}

      <ConfirmationDialog
        open={confirmAction === "disable"}
        onOpenChange={(open) => {
          if (!open) {
            setConfirmAction(null);
          }
        }}
        title="停用 Claude Code 事件采集？"
        description="将删除该项目 .claude/settings.local.json 中 XUSHU 创建的 Hooks；其他 Hooks、权限与历史事件保留。"
        confirmLabel="停用采集"
        pending={disable.isPending}
        onCloseAutoFocus={restoreMoreActionsFocus}
        onConfirm={() => void confirmActionRun()}
      />
      <ConfirmationDialog
        open={confirmAction === "uninstall"}
        onOpenChange={(open) => {
          if (!open) {
            setConfirmAction(null);
          }
        }}
        title="卸载 Claude Code 接入？"
        description="将删除 XUSHU 创建的 Hooks，并标记接入为已卸载。历史事件保留，不会删除项目文件。"
        confirmLabel="卸载接入"
        destructive
        pending={uninstall.isPending}
        onCloseAutoFocus={restoreMoreActionsFocus}
        onConfirm={() => void confirmActionRun()}
      />
    </Section>
  );
}
