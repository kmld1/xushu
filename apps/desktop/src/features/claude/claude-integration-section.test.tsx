import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type {
  ClaudeActivitySnapshot,
  ClaudeHookPreview,
  ClaudeIntegrationStatus,
  Project,
} from "@/lib/domain";

import { ClaudeIntegrationSection } from "./claude-integration-section";
import {
  getClaudeActivity,
  getClaudeIntegration,
  installClaudeHooks,
  previewClaudeHooks,
  disableClaudeHooks,
} from "./api";

vi.mock("./api", () => ({
  getClaudeIntegration: vi.fn(),
  getClaudeActivity: vi.fn(),
  previewClaudeHooks: vi.fn(),
  installClaudeHooks: vi.fn(),
  disableClaudeHooks: vi.fn(),
  uninstallClaudeHooks: vi.fn(),
  refreshClaudeEvents: vi.fn(),
}));

const project: Project = {
  id: "11111111-1111-4111-8111-111111111111",
  name: "示例项目",
  rootPath: "D:\\Projects\\demo",
  createdAt: "2026-11-05T10:00:00.000Z",
  updatedAt: "2026-11-05T10:00:00.000Z",
  archivedAt: null,
};

const notConfigured: ClaudeIntegrationStatus = {
  projectId: project.id,
  projectName: project.name,
  projectArchived: false,
  desiredEnabled: false,
  hasRecord: false,
  installState: "not_configured",
  stateDetail: "尚未安装 Hooks",
  settingsFile: ".claude/settings.local.json",
  settingsExists: false,
  managedHookCount: 0,
  expectedHookCount: 6,
  otherHookGroupCount: 0,
  hookVersion: null,
  installedAt: null,
  receiverChanged: false,
  hookBinaryAvailable: true,
  inboxAvailable: true,
  pendingEventCount: 0,
  lastEventAt: null,
  lastEventType: null,
  lastImportAt: null,
  uninstalledAt: null,
  lastErrorCode: null,
  lastErrorAt: null,
  eventCount: 0,
  sessionCount: 0,
  runningToolCount: 0,
  retentionDays: 30,
  maxEvents: 5000,
};

const installed: ClaudeIntegrationStatus = {
  ...notConfigured,
  desiredEnabled: true,
  hasRecord: true,
  installState: "installed",
  stateDetail: "Hooks 已安装并启用",
  settingsExists: true,
  managedHookCount: 6,
  hookVersion: "0.1.0",
  installedAt: "2026-11-06T08:00:00.000Z",
  lastEventAt: "2026-11-06T09:30:00.000Z",
  lastImportAt: "2026-11-06T09:30:01.000Z",
  eventCount: 6,
  sessionCount: 1,
};

const configurationError: ClaudeIntegrationStatus = {
  ...notConfigured,
  hasRecord: true,
  installState: "configuration_error",
  stateDetail: "settings.local.json 不是有效 JSON，未做任何修改",
  settingsExists: true,
};

const emptyActivity: ClaudeActivitySnapshot = {
  projectId: project.id,
  generatedAt: "2026-11-06T09:30:00.000Z",
  pendingEventCount: 0,
  sessions: [],
  events: [],
  toolCalls: [],
};

const session = {
  id: "session-row-1",
  sessionRef: "9f2c1a7b3d4e",
  startedAt: "2026-11-06T09:00:00.000Z",
  lastEventAt: "2026-11-06T09:30:00.000Z",
  endedAt: "2026-11-06T09:30:00.000Z",
  startSource: "startup",
  endReason: "other",
  startObserved: true,
  activityState: "ended" as const,
  eventCount: 6,
  toolCallCount: 3,
  runningToolCount: 1,
  failedToolCount: 1,
  unknownToolCount: 1,
};

const activity: ClaudeActivitySnapshot = {
  projectId: project.id,
  generatedAt: "2026-11-06T09:30:00.000Z",
  pendingEventCount: 0,
  sessions: [session],
  events: [
    {
      id: "event-running",
      sessionId: session.id,
      sessionRef: session.sessionRef,
      eventType: "pre_tool_use",
      occurredAt: "2026-11-06T09:29:00.000Z",
      toolName: "Bash",
      toolCategory: "shell",
      status: "running",
      failureKind: null,
      durationMs: null,
      source: null,
      endReason: null,
    },
    {
      id: "event-succeeded",
      sessionId: session.id,
      sessionRef: session.sessionRef,
      eventType: "post_tool_use",
      occurredAt: "2026-11-06T09:28:00.000Z",
      toolName: "Read",
      toolCategory: "file_read",
      status: "succeeded",
      failureKind: null,
      durationMs: 42,
      source: null,
      endReason: null,
    },
    {
      id: "event-failed",
      sessionId: session.id,
      sessionRef: session.sessionRef,
      eventType: "post_tool_use_failure",
      occurredAt: "2026-11-06T09:27:00.000Z",
      toolName: "Bash",
      toolCategory: "shell",
      status: "failed",
      failureKind: "exit_code_nonzero",
      durationMs: 1200,
      source: null,
      endReason: null,
    },
    {
      id: "event-unknown",
      sessionId: session.id,
      sessionRef: session.sessionRef,
      eventType: "pre_tool_use",
      occurredAt: "2026-11-06T09:26:00.000Z",
      toolName: "Glob",
      toolCategory: "search",
      status: "unknown",
      failureKind: null,
      durationMs: null,
      source: null,
      endReason: null,
    },
    {
      id: "event-start",
      sessionId: session.id,
      sessionRef: session.sessionRef,
      eventType: "session_start",
      occurredAt: "2026-11-06T09:00:00.000Z",
      toolName: null,
      toolCategory: null,
      status: "observed",
      failureKind: null,
      durationMs: null,
      source: "startup",
      endReason: null,
    },
    {
      id: "event-end",
      sessionId: session.id,
      sessionRef: session.sessionRef,
      eventType: "session_end",
      occurredAt: "2026-11-06T09:30:00.000Z",
      toolName: null,
      toolCategory: null,
      status: "ended",
      failureKind: null,
      durationMs: null,
      source: null,
      endReason: "other",
    },
  ],
  toolCalls: [
    {
      id: "tool-running",
      sessionId: session.id,
      sessionRef: session.sessionRef,
      toolName: "Bash",
      toolCategory: "shell",
      status: "running",
      failureKind: null,
      startedAt: "2026-11-06T09:29:00.000Z",
      finishedAt: null,
      durationMs: null,
      startObserved: true,
      finishObserved: false,
      conflict: false,
    },
    {
      id: "tool-unknown",
      sessionId: session.id,
      sessionRef: session.sessionRef,
      toolName: "Glob",
      toolCategory: "search",
      status: "unknown",
      failureKind: null,
      startedAt: null,
      finishedAt: null,
      durationMs: null,
      startObserved: false,
      finishObserved: false,
      conflict: false,
    },
  ],
};

const preview: ClaudeHookPreview = {
  projectId: project.id,
  projectName: project.name,
  settingsFile: ".claude/settings.local.json",
  settingsExists: false,
  willCreateFile: true,
  managedEntries: [
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "Stop",
    "SessionEnd",
  ].map((event) => ({
    event,
    matcher: event === "Stop" ? null : "*",
    command: "C:\\Users\\demo\\AppData\\Roaming\\dev.xushu.desktop\\xushu-hook.exe",
    args: ["--xushu-managed", project.id],
    timeoutSeconds: 5,
  })),
  managedCountBefore: 0,
  otherHookGroupCount: 0,
  backupWillBeCreated: false,
  receiverAvailable: true,
  inboxAvailable: true,
  previewToken: "preview-token",
  warnings: ["只修改该项目的 .claude/settings.local.json。"],
  blockedReason: null,
};

function renderSection() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ClaudeIntegrationSection project={project} archived={false} />
    </QueryClientProvider>,
  );
}

describe("Claude Code integration section", () => {
  beforeEach(() => {
    vi.mocked(getClaudeIntegration).mockReset();
    vi.mocked(getClaudeActivity).mockReset();
    vi.mocked(previewClaudeHooks).mockReset();
    vi.mocked(installClaudeHooks).mockReset();
    vi.mocked(disableClaudeHooks).mockReset();
    vi.mocked(getClaudeActivity).mockResolvedValue(emptyActivity);
  });

  it("requires explicit acknowledgement before installing hooks", async () => {
    const user = userEvent.setup();
    let isInstalled = false;
    vi.mocked(getClaudeIntegration).mockImplementation(async () =>
      isInstalled ? installed : notConfigured,
    );
    vi.mocked(previewClaudeHooks).mockResolvedValue(preview);
    vi.mocked(installClaudeHooks).mockImplementation(async () => {
      isInstalled = true;
      return installed;
    });

    renderSection();
    expect(await screen.findByText("未配置")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "查看并安装" }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("将要安装的 Hooks")).toBeInTheDocument();
    expect(within(dialog).getByText("数据边界")).toBeInTheDocument();
    expect(within(dialog).getByText("会话开始")).toBeInTheDocument();

    const installButton = within(dialog).getByRole("button", { name: "安装并启用" });
    expect(installButton).toBeDisabled();
    await user.click(
      within(dialog).getByRole("checkbox", {
        name: /我了解 XUSHU 只会写入该项目的本地设置文件/,
      }),
    );
    expect(installButton).toBeEnabled();
    installButton.focus();
    await user.keyboard("{Enter}");

    await waitFor(() => {
      expect(installClaudeHooks).toHaveBeenCalledWith({
        projectId: project.id,
        previewToken: "preview-token",
        approved: true,
      });
    });
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
  });

  it("returns keyboard focus to the install preview button on Escape", async () => {
    const user = userEvent.setup();
    vi.mocked(getClaudeIntegration).mockResolvedValue(notConfigured);
    vi.mocked(previewClaudeHooks).mockResolvedValue(preview);

    renderSection();
    const trigger = await screen.findByRole("button", { name: "查看并安装" });
    trigger.focus();
    await user.keyboard("{Enter}");
    await within(await screen.findByRole("dialog")).findByText("将要安装的 Hooks");
    await user.keyboard("{Escape}");

    await waitFor(() => expect(trigger).toHaveFocus());
    expect(installClaudeHooks).not.toHaveBeenCalled();
  });

  it.each(["查看活动", "查看全部活动"])(
    "returns focus to %s after closing the Sheet",
    async (name) => {
      const user = userEvent.setup();
      vi.mocked(getClaudeIntegration).mockResolvedValue(installed);
      vi.mocked(getClaudeActivity).mockResolvedValue(activity);

      renderSection();
      const trigger = await screen.findByRole("button", { name });
      await user.click(trigger);
      const sheet = await screen.findByRole("dialog");
      const [close] = within(sheet).getAllByRole("button", { name: "关闭" });
      await user.click(close!);

      await waitFor(() => expect(trigger).toHaveFocus());
    },
  );

  it("shows running, successful, failed and unknown activity without raw content", async () => {
    const user = userEvent.setup();
    vi.mocked(getClaudeIntegration).mockResolvedValue(installed);
    vi.mocked(getClaudeActivity).mockResolvedValue(activity);

    renderSection();
    expect(await screen.findByText("已启用")).toBeInTheDocument();
    const section = screen.getByRole("region", { name: "Claude Code 接入" });
    expect(await within(section).findByText("执行中")).toBeInTheDocument();
    expect(within(section).getByText("成功")).toBeInTheDocument();
    expect(within(section).getByText("失败")).toBeInTheDocument();
    expect(within(section).getByText("状态未知")).toBeInTheDocument();
    expect(within(section).queryByText(/SECRET|curl|sk-live/)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "查看活动" }));
    const sheet = await screen.findByRole("dialog");
    expect(within(sheet).getByText("事件时间线")).toBeInTheDocument();
    expect(within(sheet).getByText("未结束或未确认")).toBeInTheDocument();
    expect(within(sheet).getByText("会话结束")).toBeInTheDocument();
  });

  it("shows an honest empty activity state instead of sample events", async () => {
    const user = userEvent.setup();
    vi.mocked(getClaudeIntegration).mockResolvedValue(installed);
    vi.mocked(getClaudeActivity).mockResolvedValue(emptyActivity);

    renderSection();
    expect(await screen.findByText("已启用")).toBeInTheDocument();
    expect(await screen.findByText("尚未收到 Claude Code 事件")).toBeInTheDocument();
    expect(screen.queryByText("执行中")).not.toBeInTheDocument();
    expect(screen.queryByText("成功")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "查看活动" }));
    const sheet = await screen.findByRole("dialog", { name: "Claude Code 活动" });
    expect(within(sheet).getByText("尚未收到 Claude Code 事件")).toBeInTheDocument();
    expect(within(sheet).queryByText("事件时间线")).not.toBeInTheDocument();
  });

  it("shows an activity load error in the Sheet and recovers on retry", async () => {
    const user = userEvent.setup();
    vi.mocked(getClaudeIntegration).mockResolvedValue(installed);
    vi.mocked(getClaudeActivity).mockRejectedValue(new Error("测试读取失败"));

    renderSection();
    await user.click(await screen.findByRole("button", { name: "查看活动" }));
    const sheet = within(await screen.findByRole("dialog", { name: "Claude Code 活动" }));
    expect(await sheet.findByText("无法读取 Claude Code 活动")).toBeInTheDocument();
    expect(sheet.getByText("测试读取失败")).toBeInTheDocument();

    vi.mocked(getClaudeActivity).mockResolvedValue(activity);
    await user.click(sheet.getByRole("button", { name: "重试" }));
    expect(await sheet.findByText("事件时间线")).toBeInTheDocument();
  });

  it("shows a configuration error and blocks installation", async () => {
    const user = userEvent.setup();
    vi.mocked(getClaudeIntegration).mockResolvedValue(configurationError);
    vi.mocked(previewClaudeHooks).mockResolvedValue({
      ...preview,
      blockedReason: "settings.local.json 不是有效 JSON，未做任何修改",
    });

    renderSection();
    expect(await screen.findByText("配置异常")).toBeInTheDocument();
    expect(
      (await screen.findAllByText(/settings\.local\.json 不是有效 JSON/)).length,
    ).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "修复接入" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("当前不能安装")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "修复并启用" })).toBeDisabled();
  });

  it("confirms disabling collection", async () => {
    const user = userEvent.setup();
    vi.mocked(getClaudeIntegration).mockResolvedValue(installed);
    vi.mocked(disableClaudeHooks).mockResolvedValue({
      ...installed,
      desiredEnabled: false,
      installState: "disabled",
      managedHookCount: 0,
      stateDetail: "事件采集已停用",
    });

    renderSection();
    await screen.findByText("已启用");
    await user.click(screen.getByRole("button", { name: "更多 Claude Code 接入操作" }));
    await user.click(await screen.findByRole("menuitem", { name: "停用事件采集" }));
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "停用采集" }));

    await waitFor(() => {
      expect(disableClaudeHooks).toHaveBeenCalledWith({
        projectId: project.id,
        approved: true,
      });
    });
  });

  it.each(["停用事件采集", "卸载接入"])(
    "returns focus to more actions after cancelling %s",
    async (name) => {
      const user = userEvent.setup();
      vi.mocked(getClaudeIntegration).mockResolvedValue(installed);

      renderSection();
      const trigger = await screen.findByRole("button", { name: "更多 Claude Code 接入操作" });
      await user.click(trigger);
      await user.click(await screen.findByRole("menuitem", { name }));
      await screen.findByRole("dialog");
      await user.keyboard("{Escape}");

      await waitFor(() => expect(trigger).toHaveFocus());
    },
  );
});
