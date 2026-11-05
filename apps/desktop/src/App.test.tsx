import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import type {
  AcceptanceCriterion,
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
  phase: "Phase 3",
  platform: "windows",
};

const health = {
  ok: true,
  integrityOk: true,
  foreignKeysEnabled: true,
  foreignKeyViolations: false,
  journalMode: "wal",
  schemaVersion: 3,
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
}: {
  withProject?: boolean;
  failHealth?: boolean;
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
        return [task];
      }
      case "get_task":
        return taskDetail;
      case "list_probe_records":
        return [];
      case "list_verification_queue":
        return [];
      case "list_verification_runs":
        return [];
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

  it("renders an honest empty workspace with real health data", async () => {
    render(<App />);

    expect(await screen.findByText("还没有可管理的项目")).toBeInTheDocument();
    expect(await screen.findByText("系统健康")).toBeInTheDocument();
    expect(await screen.findByText("v3")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "项目" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "任务" })).toBeInTheDocument();
  });

  it("shows real workspace counts and the current project", async () => {
    mockBackend({ withProject: true });
    render(<App />);

    expect(await screen.findAllByText("当前项目")).not.toHaveLength(0);
    expect(await screen.findAllByText("示例项目")).not.toHaveLength(0);
    expect(await screen.findAllByText("实现项目导入")).not.toHaveLength(0);
    expect(await screen.findByText("需要关注")).toBeInTheDocument();
    expect(await screen.findByText("没有阻塞任务。")).toBeInTheDocument();
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
    expect(await screen.findByText("Git 状态")).toBeInTheDocument();
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
    expect(await screen.findByRole("heading", { name: "本地数据与系统健康" })).toBeInTheDocument();
    expect(await screen.findByText("持久化探针不是业务数据")).toBeInTheDocument();
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
    render(<App />);

    expect(
      await screen.findByText("无法读取健康状态", undefined, { timeout: 3_000 }),
    ).toBeInTheDocument();
    expect(screen.getByText("本地数据库尚未就绪")).toBeInTheDocument();
  });
});
