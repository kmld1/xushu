import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import type {
  AcceptanceCriterion,
  ClaudeActivitySnapshot,
  ClaudeHookPreview,
  ClaudeIntegrationStatus,
  Project,
  ProjectGitStatus,
  Task,
  TaskDetail,
  WorkspaceSummary,
} from "@/lib/domain";

import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

type InvokeMock = Mock<(command: string, args?: Record<string, unknown>) => Promise<unknown>>;

const invokeMock = invoke as unknown as InvokeMock;

const appInfo = {
  name: "序枢 XUSHU",
  version: "0.1.0",
  tauriVersion: "2.12.5",
  phase: "Phase 4",
  platform: "windows",
};

const health = {
  ok: true,
  integrityOk: true,
  foreignKeysEnabled: true,
  foreignKeyViolations: false,
  journalMode: "wal",
  schemaVersion: 4,
  recordCount: 0,
  databaseFile: "xushu.db",
};

const project: Project = {
  id: "11111111-1111-4111-8111-111111111111",
  name: "示例项目",
  rootPath: "D:\\Projects\\demo",
  createdAt: "2026-11-05T10:00:00.000Z",
  updatedAt: "2026-11-05T10:00:00.000Z",
  archivedAt: null,
};

const gitStatus: ProjectGitStatus = {
  projectId: project.id,
  rootPath: project.rootPath,
  branch: "main",
  head: "abcdef1234567890abcdef1234567890abcdef12",
  dirty: false,
  detached: false,
  unborn: false,
  checkedAt: "2026-11-06T00:00:00.000Z",
};

const task: Task = {
  id: "22222222-2222-4222-8222-222222222222",
  projectId: project.id,
  title: "实现项目导入",
  description: "导入本地 Git 项目并记录只读状态。",
  priority: "high",
  status: "awaiting_verification",
  revision: 3,
  blockReason: null,
  archivedAt: null,
  createdAt: "2026-11-05T10:00:00.000Z",
  updatedAt: "2026-11-05T11:00:00.000Z",
};

const criterion: AcceptanceCriterion = {
  id: "33333333-3333-4333-8333-333333333333",
  taskId: task.id,
  description: "人工确认导入结果",
  kind: "manual",
  required: true,
  revision: 1,
  createdAt: "2026-11-05T10:00:00.000Z",
  updatedAt: "2026-11-05T10:00:00.000Z",
};

const taskDetail: TaskDetail = {
  task,
  criteria: [{ criterion, satisfied: false, confirmation: null, verification: null }],
  transitions: [
    {
      id: "44444444-4444-4444-8444-444444444444",
      taskId: task.id,
      fromStatus: "running",
      toStatus: "awaiting_verification",
      reason: null,
      taskRevision: 3,
      createdAt: "2026-11-05T11:00:00.000Z",
    },
  ],
  allowedTransitions: ["done", "blocked", "cancelled", "running"],
  completion: {
    canComplete: false,
    requiredCount: 1,
    satisfiedRequiredCount: 0,
    blockers: ["人工验收条件尚未确认：人工确认导入结果"],
  },
};

const claudeStatus: ClaudeIntegrationStatus = {
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

const claudeActivity: ClaudeActivitySnapshot = {
  projectId: project.id,
  generatedAt: "2026-11-06T00:00:00.000Z",
  pendingEventCount: 0,
  sessions: [],
  events: [],
  toolCalls: [],
};

const claudePreview: ClaudeHookPreview = {
  projectId: project.id,
  projectName: project.name,
  settingsFile: ".claude/settings.local.json",
  settingsExists: false,
  willCreateFile: true,
  managedEntries: [],
  managedCountBefore: 0,
  otherHookGroupCount: 0,
  backupWillBeCreated: false,
  receiverAvailable: true,
  inboxAvailable: true,
  previewToken: "preview-token",
  warnings: [],
  blockedReason: null,
};

const emptySummary: WorkspaceSummary = {
  projectCount: 0,
  archivedProjectCount: 0,
  taskCounts: {
    total: 0,
    draft: 0,
    ready: 0,
    running: 0,
    awaitingVerification: 0,
    done: 0,
    blocked: 0,
    cancelled: 0,
  },
  recentTasks: [],
  blockedTasks: [],
  awaitingTasks: [],
};

const populatedSummary: WorkspaceSummary = {
  ...emptySummary,
  projectCount: 1,
  taskCounts: {
    ...emptySummary.taskCounts,
    total: 1,
    awaitingVerification: 1,
  },
  recentTasks: [task],
  awaitingTasks: [task],
};

function mockBackend({
  withProject = false,
  failHealth = false,
  tasks = [task],
  details = [taskDetail],
}: {
  withProject?: boolean;
  failHealth?: boolean;
  tasks?: Task[];
  details?: TaskDetail[];
} = {}) {
  invokeMock.mockImplementation(async (command, args) => {
    switch (command) {
      case "get_app_info":
        return appInfo;
      case "get_database_health":
        if (failHealth) {
          throw { code: "DATABASE_UNAVAILABLE", message: "本地数据库尚未就绪" };
        }
        return health;
      case "list_projects":
        return withProject ? [project] : [];
      case "get_project":
        return project;
      case "get_project_git_status":
        return gitStatus;
      case "get_workspace_summary":
        return withProject ? populatedSummary : emptySummary;
      case "list_tasks": {
        const filter = (args?.filter ?? {}) as { projectId?: string };
        if (!withProject) {
          return [];
        }
        if (filter.projectId && filter.projectId !== project.id) {
          return [];
        }
        return tasks;
      }
      case "get_task":
        return details.find((detail) => detail.task.id === args?.id) ?? taskDetail;
      case "list_probe_records":
        return [];
      case "list_verification_queue":
        return [];
      case "list_verification_runs":
        return [];
      case "get_claude_integration":
        return claudeStatus;
      case "get_claude_activity":
        return claudeActivity;
      case "preview_claude_hooks":
        return claudePreview;
      case "refresh_claude_events":
        return {
          projectId: project.id,
          imported: 0,
          duplicates: 0,
          rejected: 0,
          ignored: 0,
          remaining: 0,
          lastErrorCode: null,
        };
      case "get_probe_record":
        throw { code: "NOT_FOUND", message: "未找到指定记录" };
      case "create_probe_record":
        throw { code: "UNKNOWN_COMMAND", message: "not needed" };
      case "create_task":
        throw { code: "UNKNOWN_COMMAND", message: "not needed" };
      default:
        throw { code: "UNKNOWN_COMMAND", message: `未知命令: ${command}` };
    }
  });
}

describe("XUSHU desktop shell", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.localStorage.clear();
    window.location.hash = "#/";
    mockBackend();
  });

  it("renders an honest empty workspace and real health data", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByText("还没有可管理的项目")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "导入第一个项目" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "项目" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "任务" })).toBeInTheDocument();

    await user.click(screen.getByRole("link", { name: "诊断" }));
    expect(await screen.findByText("系统")).toBeInTheDocument();
    expect(await screen.findByText("v4")).toBeInTheDocument();
  });

  it("shows real workspace counts and the current project", async () => {
    mockBackend({ withProject: true });
    render(<App />);

    expect(await screen.findAllByText("当前项目")).not.toHaveLength(0);
    expect(await screen.findAllByText("示例项目")).not.toHaveLength(0);
    expect(await screen.findAllByText("实现项目导入")).not.toHaveLength(0);
    expect(await screen.findByText("需要处理")).toBeInTheDocument();
    expect(await screen.findByText("最近验证")).toBeInTheDocument();
    expect(await screen.findByText("当前项目最近任务还没有验证运行记录。")).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "窗口控制" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "最大化窗口" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "关闭窗口" })).toBeInTheDocument();
  });

  it("opens the projects page and validates the import dialog", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "项目" }));
    expect(await screen.findByRole("heading", { name: "项目" })).toBeInTheDocument();

    const importButtons = screen.getAllByRole("button", { name: "导入项目" });
    await user.click(importButtons[0]);
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "检查并导入" }));

    expect(await screen.findByText("请输入 Git 项目路径")).toBeInTheDocument();
  });

  it("keeps the focus task actionable without repeating it in the attention list", async () => {
    mockBackend({ withProject: true });
    render(<App />);

    const focus = await screen.findByRole("region", { name: "下一步" });
    expect(within(focus).getByText(task.title)).toBeInTheDocument();
    expect(await within(focus).findByText(taskDetail.completion.blockers[0])).toBeInTheDocument();
    expect(within(focus).getByRole("link", { name: "进入验收" })).toHaveAttribute(
      "href",
      `#/verification?task=${task.id}`,
    );
    const attention = screen.getByRole("region", { name: "需要处理" });
    expect(within(attention).queryByText(task.title)).not.toBeInTheDocument();
    expect(within(attention).getByText("待处理任务已列在「下一步」。")).toBeInTheDocument();
    expect(within(attention).queryByText("没有阻塞或待验收任务。")).not.toBeInTheDocument();
  });

  it("preserves other attention tasks and their full blocking reasons", async () => {
    const blocked: Task = {
      ...task,
      id: "55555555-5555-4555-8555-555555555555",
      title: "等待构建环境恢复",
      status: "blocked",
      blockReason: "缺少本机编译工具，需要安装后重新验证。",
    };
    mockBackend({ withProject: true, tasks: [task, blocked] });
    render(<App />);

    const focus = await screen.findByRole("region", { name: "下一步" });
    expect(within(focus).getByText(blocked.blockReason!)).toBeInTheDocument();
    const attention = screen.getByRole("region", { name: "需要处理" });
    expect(within(attention).getByRole("link", { name: task.title })).toHaveAttribute(
      "href",
      `#/tasks/${task.id}`,
    );
    expect(within(attention).queryByText(blocked.title)).not.toBeInTheDocument();
    expect(within(attention).getByText("另 1 项 · 焦点任务见上方")).toBeInTheDocument();
  });

  it("shows verification result, exit code and duration with the corresponding task entry", async () => {
    const details: TaskDetail[] = [
      {
        ...taskDetail,
        criteria: [
          {
            ...taskDetail.criteria[0],
            criterion: { ...criterion, kind: "auto" },
            verification: {
              status: "fail",
              spec: null,
              latestCheck: null,
              validPassRunId: null,
              staleReason: null,
              detail: "退出码不为 0",
              latestRun: {
                id: "run-failed",
                taskId: task.id,
                criterionId: criterion.id,
                criterionDescription: "检查项目导入",
                executable: "cargo",
                args: ["test"],
                cwd: project.rootPath,
                executionStatus: "completed",
                result: "fail",
                reason: "退出码不为 0",
                evidenceId: "evidence-failed",
                specRevision: 1,
                taskRevision: 3,
                criterionRevision: 1,
                startedAt: task.updatedAt,
                finishedAt: task.updatedAt,
                exitCode: 1,
                durationMs: 1250,
                stdoutTruncated: false,
                stderrTruncated: false,
                sensitiveFlags: [],
                createdAt: task.updatedAt,
              },
            },
          },
        ],
      },
    ];
    mockBackend({ withProject: true, details });
    render(<App />);

    const activity = await screen.findByRole("region", { name: "最近验证" });
    expect(await within(activity).findByText("失败")).toBeInTheDocument();
    expect(within(activity).getByText("退出码 1")).toBeInTheDocument();
    expect(within(activity).getByText("1.3 s")).toBeInTheDocument();
    expect(within(activity).getByRole("link", { name: criterion.description })).toHaveAttribute(
      "href",
      `#/tasks/${task.id}`,
    );
  });

  it("renders task detail with state track and a blocked completion gate", async () => {
    mockBackend({ withProject: true });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "任务" }));
    await user.click(await screen.findByRole("link", { name: "实现项目导入" }));

    expect(await screen.findByRole("heading", { name: "实现项目导入" })).toBeInTheDocument();
    expect(await screen.findByText("完成门禁")).toBeInTheDocument();
    expect(await screen.findByText("尚不能完成")).toBeInTheDocument();
    expect(await screen.findAllByText("人工验收条件尚未确认：人工确认导入结果")).not.toHaveLength(
      0,
    );
    expect(screen.getByRole("button", { name: "标记完成" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "确认人工验收" })).toBeInTheDocument();
    expect(screen.getByText("状态历史")).toBeInTheDocument();
  });

  it("renders project detail with git status and tasks", async () => {
    mockBackend({ withProject: true });
    window.location.hash = `#/projects/${project.id}`;
    render(<App />);

    expect(await screen.findByRole("heading", { name: "示例项目" })).toBeInTheDocument();
    expect(await screen.findByText("仓库")).toBeInTheDocument();
    expect(await screen.findByText("Claude Code 接入")).toBeInTheDocument();
    expect(await screen.findByText("未配置")).toBeInTheDocument();
    expect(await screen.findByText("项目任务")).toBeInTheDocument();
    expect(await screen.findByText("实现项目导入")).toBeInTheDocument();
  });

  it("navigates to settings and exposes appearance and data boundaries", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "设置" }));

    expect(await screen.findByRole("heading", { name: "设置" })).toBeInTheDocument();
    expect(await screen.findByText("外观")).toBeInTheDocument();
    expect(await screen.findByText("本地数据与隐私")).toBeInTheDocument();
  });

  it("navigates to the verification center and shows an honest empty queue", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "验收中心" }));
    expect(await screen.findByRole("heading", { name: "验收中心" })).toBeInTheDocument();
    expect(await screen.findByText("当前没有待验收任务")).toBeInTheDocument();
  });

  it("navigates to diagnostics and keeps the Phase 1 probe surface", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "诊断" }));
    expect(await screen.findByRole("heading", { name: "诊断" })).toBeInTheDocument();
    expect(await screen.findByText(/持久化探针不是业务数据/)).toBeInTheDocument();
    expect(await screen.findByText("尚无测试记录")).toBeInTheDocument();
  });

  it("switches between dark and light themes", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "深色" }));
    await waitFor(() => {
      expect(document.documentElement).toHaveClass("dark");
    });

    await user.click(screen.getByRole("button", { name: "浅色" }));
    await waitFor(() => {
      expect(document.documentElement).not.toHaveClass("dark");
    });
  });

  it("shows a readable health error without fake data", async () => {
    mockBackend({ failHealth: true });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "诊断" }));
    expect(
      await screen.findByText("无法读取健康状态", undefined, { timeout: 3_000 }),
    ).toBeInTheDocument();
    expect(screen.getByText("本地数据库尚未就绪")).toBeInTheDocument();
  });
});
