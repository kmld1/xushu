import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import type {
  AcceptanceCriterion,
  ClaudeActivitySnapshot,
  ClaudeHookPreview,
  ClaudeIntegrationStatus,
  DecisionDetail,
  DecisionView,
  MemoryDetail,
  Project,
  ProjectGitStatus,
  ProjectMemoryView,
  RecoveryOverview,
  RecoveryPreference,
  RecoverySnapshot,
  RecoverySnapshotSummary,
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
  version: "1.0.0",
  tauriVersion: "2.12.5",
  phase: "Phase 6",
  platform: "windows",
};

const health = {
  ok: true,
  integrityOk: true,
  foreignKeysEnabled: true,
  foreignKeyViolations: false,
  journalMode: "wal",
  schemaVersion: 7,
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

const memoryView: ProjectMemoryView = {
  memory: {
    id: "88888888-8888-4888-8888-888888888888",
    projectId: project.id,
    category: "constraint",
    title: "必须由用户授权提交",
    content: "任何 Git 写入都必须由用户确认。",
    sourceType: "user",
    confirmationStatus: "confirmed",
    validityStatus: "active",
    linkedTaskId: task.id,
    linkedEvidenceId: null,
    revision: 1,
    createdAt: "2026-11-06T08:00:00.000Z",
    updatedAt: "2026-11-06T08:00:00.000Z",
    confirmedAt: "2026-11-06T08:00:00.000Z",
    archivedAt: null,
  },
  linkedTask: {
    id: task.id,
    title: task.title,
    status: task.status,
    archived: false,
  },
};

const memoryDetail: MemoryDetail = {
  memory: memoryView,
  revisions: [
    {
      id: "99999999-9999-4999-8999-999999999999",
      memoryId: memoryView.memory.id,
      revision: 1,
      category: memoryView.memory.category,
      title: memoryView.memory.title,
      content: memoryView.memory.content,
      sourceType: memoryView.memory.sourceType,
      confirmationStatus: memoryView.memory.confirmationStatus,
      validityStatus: memoryView.memory.validityStatus,
      linkedTaskId: task.id,
      linkedEvidenceId: null,
      changeNote: "创建",
      changedAt: memoryView.memory.createdAt,
    },
  ],
};

const confirmedDecision: DecisionView = {
  decision: {
    id: "66666666-6666-4666-8666-666666666666",
    projectId: project.id,
    taskId: task.id,
    title: "选择 SQLite",
    problem: "需要选择本地持久化方案",
    context: "本地优先，不引入云服务",
    alternatives: ["SQLite", "JSON 文件"],
    decision: "使用 SQLite",
    rationale: "事务与约束更可靠",
    risks: "需要维护迁移",
    status: "confirmed",
    supersedesId: null,
    supersededById: null,
    revision: 2,
    createdAt: "2026-11-06T08:00:00.000Z",
    updatedAt: "2026-11-06T09:00:00.000Z",
    confirmedAt: "2026-11-06T09:00:00.000Z",
    deprecatedAt: null,
  },
  linkedTask: {
    id: task.id,
    title: task.title,
    status: task.status,
    archived: false,
  },
  supersedesTitle: null,
  supersededByTitle: null,
};

const draftDecision: DecisionView = {
  decision: {
    ...confirmedDecision.decision,
    id: "77777777-7777-4777-8777-777777777777",
    title: "草稿决策",
    decision: "尚未决定",
    taskId: null,
    status: "draft",
    revision: 1,
    createdAt: "2026-11-07T08:00:00.000Z",
    updatedAt: "2026-11-07T08:00:00.000Z",
    confirmedAt: null,
  },
  linkedTask: null,
  supersedesTitle: null,
  supersededByTitle: null,
};

const decisionDetail: DecisionDetail = {
  decision: confirmedDecision,
  revisions: [
    {
      id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      decisionId: confirmedDecision.decision.id,
      revision: 1,
      title: confirmedDecision.decision.title,
      problem: confirmedDecision.decision.problem,
      context: confirmedDecision.decision.context,
      alternatives: confirmedDecision.decision.alternatives,
      decision: confirmedDecision.decision.decision,
      rationale: confirmedDecision.decision.rationale,
      risks: confirmedDecision.decision.risks,
      status: "draft",
      supersedesId: null,
      supersededById: null,
      changeNote: "创建",
      changedAt: confirmedDecision.decision.createdAt,
    },
    {
      id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
      decisionId: confirmedDecision.decision.id,
      revision: 2,
      title: confirmedDecision.decision.title,
      problem: confirmedDecision.decision.problem,
      context: confirmedDecision.decision.context,
      alternatives: confirmedDecision.decision.alternatives,
      decision: confirmedDecision.decision.decision,
      rationale: confirmedDecision.decision.rationale,
      risks: confirmedDecision.decision.risks,
      status: "confirmed",
      supersedesId: null,
      supersededById: null,
      changeNote: "确认决策",
      changedAt: confirmedDecision.decision.confirmedAt ?? confirmedDecision.decision.updatedAt,
    },
  ],
};

function makeRecoverySnapshot(): RecoverySnapshot {
  const generatedAt = "2026-11-12T10:00:00.000Z";
  const fingerprint = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
  return {
    id: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
    projectId: project.id,
    generatedAt,
    fingerprint,
    markdown: [
      "# XUSHU 恢复上下文 — 示例项目",
      "",
      "生成时间（UTC）：2026-11-12T10:00:00.000Z",
      "",
      "## 下一步",
      "",
      "- 来源：系统建议（需人工确认）",
      "- 依据：人工验收条件尚未确认：人工确认导入结果",
    ].join("\n"),
    taskCount: 1,
    memoryCount: 1,
    decisionCount: 1,
    package: {
      project: {
        id: project.id,
        name: project.name,
        rootPath: project.rootPath,
        archived: false,
        createdAt: project.createdAt,
        updatedAt: project.updatedAt,
      },
      generatedAt,
      fingerprint,
      git: {
        state: "available",
        checkedAt: generatedAt,
        branch: "main",
        head: gitStatus.head,
        dirty: false,
        detached: false,
        unborn: false,
        workspaceFingerprint:
          "sha256:feedfacefeedfacefeedfacefeedfacefeedfacefeedfacefeedfacefeedface",
        fingerprintComplete: true,
        changedFileCount: 0,
        note: null,
      },
      tasks: {
        inProgress: [],
        ready: [],
        blocked: [],
        awaitingVerification: [
          {
            id: task.id,
            title: task.title,
            status: "awaiting_verification",
            priority: "high",
            revision: 3,
            blockReason: null,
            updatedAt: task.updatedAt,
            requiredCount: 1,
            satisfiedRequiredCount: 0,
            blockers: ["人工验收条件尚未确认：人工确认导入结果"],
          },
        ],
        doneWithValidEvidence: [],
        doneWithInvalidEvidence: [],
        draftCount: 0,
      },
      verification: [],
      claude: {
        hasActivity: false,
        eventCount: 0,
        sessionCount: 0,
        lastEventAt: null,
        lastEventType: null,
        lastSessionRef: null,
        lastSessionStartedAt: null,
        lastSessionLastEventAt: null,
        lastSessionEndedAt: null,
        lastSessionEndReason: null,
        lastSessionFailedToolCount: 0,
        lastSessionRunningToolCount: 0,
        lastSessionUnknownToolCount: 0,
        endConfirmed: false,
        note: "尚无已导入的 Claude Code 事件；未见失败事件不代表成功。",
      },
      memories: [
        {
          id: memoryView.memory.id,
          category: "constraint",
          title: memoryView.memory.title,
          content: memoryView.memory.content,
          sourceType: "user",
          confirmationStatus: "confirmed",
          revision: 1,
          updatedAt: memoryView.memory.updatedAt,
          linkedTaskId: task.id,
          linkedTaskTitle: task.title,
          linkedTaskStatus: task.status,
          linkedTaskArchived: false,
        },
      ],
      unconfirmedMemories: [],
      decisions: [
        {
          id: confirmedDecision.decision.id,
          title: confirmedDecision.decision.title,
          status: "confirmed",
          taskId: task.id,
          taskTitle: task.title,
          problem: confirmedDecision.decision.problem,
          context: confirmedDecision.decision.context,
          alternatives: confirmedDecision.decision.alternatives,
          decision: confirmedDecision.decision.decision,
          rationale: confirmedDecision.decision.rationale,
          risks: confirmedDecision.decision.risks,
          revision: 2,
          updatedAt: confirmedDecision.decision.updatedAt,
          confirmedAt: confirmedDecision.decision.confirmedAt,
          supersedesTitle: null,
          supersededByTitle: null,
        },
      ],
      supersededDecisions: [],
      nextStep: {
        kind: "suggested",
        taskId: task.id,
        taskTitle: task.title,
        taskStatus: "awaiting_verification",
        reason: "任务等待验收：人工验收条件尚未确认：人工确认导入结果",
        requiresConfirmation: true,
      },
      filesToRead: ["README.md"],
      conflicts: [],
      missingInformation: ["尚无独立验证运行记录；任何完成声明都需要证据。"],
      sources: {
        databaseReadAt: generatedAt,
        gitCheckedAt: generatedAt,
        taskCandidateCount: 1,
        activeTaskCount: 1,
        doneTaskCount: 0,
        draftTaskCount: 0,
        memoryConfirmedActiveCount: 1,
        memoryUnconfirmedActiveCount: 0,
        memoryArchivedCount: 0,
        decisionConfirmedCount: 1,
        decisionDraftCount: 0,
        decisionSupersededCount: 0,
        decisionDeprecatedCount: 0,
        verificationRunCount: 0,
        claudeEventCount: 0,
        claudeSessionCount: 0,
        includedLimits: ["任务：最近更新最多 200 条。"],
        exclusions: [
          "未导入的 Claude Code 本地收件箱事件不在恢复包内。",
          "不包含原始 Hook JSON、Prompt、工具输入输出、transcript 或密钥。",
        ],
      },
    },
  };
}

function makeRecoveryOverview(
  state: "fresh" | "stale" | "no_snapshot" = "fresh",
): RecoveryOverview {
  if (state === "no_snapshot") {
    return {
      latest: null,
      freshness: {
        state: "no_snapshot",
        checkedAt: "2026-11-12T10:00:00.000Z",
        changed: [],
      },
    };
  }
  return {
    latest: makeRecoverySnapshot(),
    freshness: {
      state,
      checkedAt: "2026-11-12T11:00:00.000Z",
      changed: state === "stale" ? ["Git 状态或工作区指纹已变化"] : [],
    },
  };
}

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
  platform = "windows",
  withProject = false,
  failHealth = false,
  tasks = [task],
  details = [taskDetail],
  recoveryState = "fresh",
  readRecoveryOverview = () => makeRecoveryOverview(recoveryState),
}: {
  platform?: "macos" | "windows";
  withProject?: boolean;
  failHealth?: boolean;
  tasks?: Task[];
  details?: TaskDetail[];
  recoveryState?: "fresh" | "stale" | "no_snapshot";
  readRecoveryOverview?: () => RecoveryOverview;
} = {}) {
  invokeMock.mockImplementation(async (command, args) => {
    switch (command) {
      case "get_app_info":
        return { ...appInfo, platform };
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
      case "list_project_memories": {
        if (!withProject) {
          return [];
        }
        const filter = (args?.filter ?? {}) as {
          category?: string | null;
          includeArchived?: boolean | null;
          taskId?: string | null;
        };
        if (filter.category && filter.category !== memoryView.memory.category) {
          return [];
        }
        if (filter.taskId && filter.taskId !== memoryView.memory.linkedTaskId) {
          return [];
        }
        return [memoryView];
      }
      case "get_project_memory":
        if (args?.memoryId === memoryView.memory.id) {
          return memoryDetail;
        }
        throw { code: "NOT_FOUND", message: "未找到指定记录" };
      case "create_project_memory": {
        const input = (args?.input ?? {}) as {
          title?: string;
          content?: string;
          category?: ProjectMemoryView["memory"]["category"];
          linkedTaskId?: string | null;
        };
        return {
          memory: {
            ...memoryView.memory,
            id: "new-memory-id",
            title: input.title ?? memoryView.memory.title,
            content: input.content ?? memoryView.memory.content,
            category: input.category ?? memoryView.memory.category,
            linkedTaskId: input.linkedTaskId ?? null,
            revision: 1,
            createdAt: "2026-11-12T10:00:00.000Z",
            updatedAt: "2026-11-12T10:00:00.000Z",
            confirmedAt: "2026-11-12T10:00:00.000Z",
          },
          linkedTask: null,
        };
      }
      case "update_project_memory":
      case "confirm_project_memory":
      case "archive_project_memory":
      case "restore_project_memory":
        return memoryView;
      case "list_project_decisions": {
        if (!withProject) {
          return [];
        }
        const filter = (args?.filter ?? {}) as {
          status?: string | null;
          taskId?: string | null;
        };
        const decisions = [confirmedDecision, draftDecision];
        return decisions.filter(
          (item) =>
            (!filter.status || item.decision.status === filter.status) &&
            (!filter.taskId || item.decision.taskId === filter.taskId),
        );
      }
      case "get_project_decision":
        if (args?.decisionId === confirmedDecision.decision.id) {
          return decisionDetail;
        }
        if (args?.decisionId === draftDecision.decision.id) {
          return { decision: draftDecision, revisions: [] };
        }
        throw { code: "NOT_FOUND", message: "未找到指定记录" };
      case "create_project_decision": {
        const input = (args?.input ?? {}) as { title?: string };
        return {
          ...draftDecision,
          decision: {
            ...draftDecision.decision,
            id: "new-decision-id",
            title: input.title ?? draftDecision.decision.title,
          },
        };
      }
      case "update_project_decision":
        return draftDecision;
      case "confirm_project_decision":
        return confirmedDecision;
      case "deprecate_project_decision":
        return {
          ...confirmedDecision,
          decision: {
            ...confirmedDecision.decision,
            status: "deprecated",
            deprecatedAt: "2026-11-12T10:00:00.000Z",
          },
        };
      case "get_recovery_overview":
        return readRecoveryOverview();
      case "generate_recovery_package":
        return makeRecoverySnapshot();
      case "list_recovery_snapshots": {
        const snapshot = makeRecoverySnapshot();
        return [
          {
            id: snapshot.id,
            projectId: snapshot.projectId,
            generatedAt: snapshot.generatedAt,
            fingerprint: snapshot.fingerprint,
            taskCount: snapshot.taskCount,
            memoryCount: snapshot.memoryCount,
            decisionCount: snapshot.decisionCount,
          },
          {
            id: "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
            projectId: snapshot.projectId,
            generatedAt: "2026-11-11T10:00:00.000Z",
            fingerprint: snapshot.fingerprint,
            taskCount: 1,
            memoryCount: 1,
            decisionCount: 1,
          },
        ] satisfies RecoverySnapshotSummary[];
      }
      case "get_recovery_snapshot": {
        const snapshot = makeRecoverySnapshot();
        return {
          ...snapshot,
          id: String(args?.snapshotId ?? snapshot.id),
          generatedAt:
            args?.snapshotId === "dddddddd-dddd-4ddd-8ddd-dddddddddddd"
              ? "2026-11-11T10:00:00.000Z"
              : snapshot.generatedAt,
        };
      }
      case "set_recovery_next_task": {
        const input = (args?.input ?? {}) as { taskId?: string | null };
        return {
          projectId: project.id,
          nextTaskId: input.taskId ?? null,
          updatedAt: "2026-11-12T10:00:00.000Z",
        } satisfies RecoveryPreference;
      }
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

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it.each(["macos", "windows"] as const)(
    "renders the matching title bar on %s",
    async (platform) => {
      mockBackend({ platform });
      render(<App />);

      expect(await screen.findByText("v1.0.0")).toBeInTheDocument();
      const header = screen.getByRole("banner");
      const controls = within(header).queryByRole("group", { name: "窗口控制" });
      const theme = within(header).getByRole("group", { name: "主题设置" });
      if (platform === "macos") {
        expect(controls).not.toBeInTheDocument();
        expect(header).not.toHaveAttribute("data-tauri-drag-region");

        const light = within(theme).getByRole("button", { name: "浅色" });
        fireEvent.mouseDown(light, { button: 0, detail: 1 });
        fireEvent.doubleClick(light, { button: 0, detail: 2 });
        expect(invokeMock).not.toHaveBeenCalledWith("plugin:window|start_dragging");
        expect(invokeMock).not.toHaveBeenCalledWith("macos_titlebar_double_click");

        fireEvent.mouseDown(header, { button: 0, detail: 1 });
        fireEvent.doubleClick(header, { button: 0, detail: 2 });
        expect(invokeMock).toHaveBeenCalledWith("plugin:window|start_dragging");
        expect(invokeMock).toHaveBeenCalledWith("macos_titlebar_double_click");
      } else {
        expect(controls).toBeInTheDocument();
        expect(header).toHaveAttribute("data-tauri-drag-region", "deep");
      }
      expect(screen.queryByText("Phase 6 · 数据保护")).not.toBeInTheDocument();
    },
  );

  it("renders an honest empty workspace and real health data", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByText("还没有可管理的项目")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "导入第一个项目" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "项目" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "任务" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "数据保护" })).toBeInTheDocument();

    await user.click(screen.getByRole("link", { name: "诊断" }));
    expect(await screen.findByText("系统")).toBeInTheDocument();
    expect(await screen.findByText("v7")).toBeInTheDocument();
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

  it("renders project memory and decisions with explicit confirmation states", async () => {
    mockBackend({ withProject: true });
    const user = userEvent.setup();
    window.location.hash = `#/projects/${project.id}/knowledge`;
    render(<App />);

    expect(await screen.findByRole("heading", { name: "记忆与决策" })).toBeInTheDocument();
    expect(await screen.findByText("必须由用户授权提交")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "必须由用户授权提交" }));
    const memoryRegion = await screen.findByRole("region", { name: "项目记忆详情" });
    expect(
      within(memoryRegion).getAllByText("任何 Git 写入都必须由用户确认。").length,
    ).toBeGreaterThan(0);
    await user.keyboard("{Escape}");
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "项目记忆详情" })).not.toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "开发决策" }));
    expect(await screen.findByText("选择 SQLite")).toBeInTheDocument();
    expect(await screen.findByText("草稿决策")).toBeInTheDocument();
    expect(screen.getByText("草稿不会进入恢复包")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "选择 SQLite" }));
    const decisionRegion = await screen.findByRole("region", { name: "开发决策详情" });
    expect(within(decisionRegion).getByText("使用 SQLite")).toBeInTheDocument();
    expect(within(decisionRegion).getByText("需要选择本地持久化方案")).toBeInTheDocument();
  });

  it("creates a project memory from the knowledge page", async () => {
    mockBackend({ withProject: true });
    const user = userEvent.setup();
    window.location.hash = `#/projects/${project.id}/knowledge`;
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "新建记忆" }));
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    await user.type(screen.getByLabelText("标题"), "新的约束");
    await user.type(screen.getByLabelText("内容"), "约束内容");
    await user.click(screen.getByRole("button", { name: "保存记忆" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "create_project_memory",
        expect.objectContaining({
          input: expect.objectContaining({
            projectId: project.id,
            title: "新的约束",
            content: "约束内容",
            sourceType: "user",
            confirmed: true,
          }),
        }),
      );
    });
  });

  it.each([
    ["memory", "必须由用户授权提交", "编辑", "编辑项目记忆"],
    ["decisions", "草稿决策", "编辑草稿", "编辑决策草稿"],
  ])("restores focus after closing a %s edit dialog", async (tab, title, edit, formTitle) => {
    mockBackend({ withProject: true });
    const user = userEvent.setup();
    window.location.hash = `#/projects/${project.id}/knowledge?tab=${tab}`;
    render(<App />);

    const row = await screen.findByRole("button", { name: title });
    await user.click(row);
    await user.click(await screen.findByRole("button", { name: edit }));
    const form = await screen.findByRole("dialog", { name: formTitle });
    expect(form).toContainElement(document.activeElement as HTMLElement);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(row).toHaveFocus());
  });

  it("creates a decision draft from the knowledge page", async () => {
    mockBackend({ withProject: true });
    const user = userEvent.setup();
    window.location.hash = `#/projects/${project.id}/knowledge?tab=decisions`;
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "新建决策" }));
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    await user.type(screen.getByLabelText("标题"), "新的架构决策");
    await user.type(screen.getByLabelText("要解决的问题"), "需要选择缓存方案");
    await user.type(screen.getByLabelText("背景与约束"), "本地优先");
    await user.type(screen.getByLabelText("备选方案（每行一个）"), "内存缓存\n文件缓存");
    await user.type(screen.getByLabelText("最终采用的方案"), "使用内存缓存");
    await user.type(screen.getByLabelText("采用该方案的原因"), "实现简单");
    await user.type(screen.getByLabelText("已知代价与风险"), "进程重启丢失");
    await user.click(screen.getByRole("button", { name: "保存为草稿" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "create_project_decision",
        expect.objectContaining({
          input: expect.objectContaining({
            projectId: project.id,
            title: "新的架构决策",
            problem: "需要选择缓存方案",
            alternatives: ["内存缓存", "文件缓存"],
            decision: "使用内存缓存",
          }),
        }),
      );
    });
  });

  it("renders recovery summary and copies the exact generated markdown", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
    mockBackend({ withProject: true });
    window.location.hash = "#/recovery";
    render(<App />);

    expect(await screen.findByRole("region", { name: "下一步" })).toBeInTheDocument();
    expect(await screen.findByText("必须由用户授权提交")).toBeInTheDocument();
    expect(screen.getByText("按生成时状态；之后可能过期")).toBeInTheDocument();

    const copyButton = screen.getByRole("button", { name: "复制恢复指令" });
    expect(copyButton).toBeEnabled();
    await user.click(copyButton);
    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(1), { timeout: 2_000 });
    expect(writeText.mock.calls[0][0]).toBe(makeRecoverySnapshot().markdown);
  });

  it("requires confirmation before copying a stale recovery snapshot", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
    mockBackend({ withProject: true, recoveryState: "stale" });
    window.location.hash = "#/recovery";
    render(<App />);

    expect(await screen.findByText("这份快照已经过期")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "复制恢复指令" }));
    expect(await screen.findByText("复制已过期的恢复指令？")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "仍然复制" }));

    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(1), { timeout: 2_000 });
    expect(String(writeText.mock.calls[0][0])).toContain("> 注意：这份恢复上下文生成于");
    expect(String(writeText.mock.calls[0][0])).toContain("Git 状态或工作区指纹已变化");
  });

  it.each(["page", "preview"] as const)(
    "checks for Git changes before copying from the recovery %s",
    async (entry) => {
      const user = userEvent.setup();
      const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
      let gitChanged = false;
      mockBackend({
        withProject: true,
        readRecoveryOverview: () => makeRecoveryOverview(gitChanged ? "stale" : "fresh"),
      });
      window.location.hash = "#/recovery";
      render(<App />);

      await screen.findByRole("region", { name: "下一步" });
      if (entry === "preview") {
        await user.click(screen.getByRole("button", { name: "预览完整指令" }));
      }
      const scope =
        entry === "preview"
          ? within(await screen.findByRole("dialog", { name: "完整恢复指令" }))
          : screen;
      gitChanged = true;
      const copyButton = scope.getByRole("button", { name: "复制恢复指令" });
      await user.click(copyButton);

      expect(await screen.findByText("复制已过期的恢复指令？")).toBeInTheDocument();
      expect(writeText).not.toHaveBeenCalled();
      await user.click(screen.getByRole("button", { name: "仍然复制" }));
      await waitFor(() => expect(writeText).toHaveBeenCalledTimes(1));
      const copied = String(writeText.mock.calls[0][0]);
      expect(copied).toContain("Git 状态或工作区指纹已变化");
      expect(copied.endsWith(makeRecoverySnapshot().markdown)).toBe(true);
      await waitFor(() => expect(copyButton).toHaveFocus());
    },
  );

  it("keeps the previewed snapshot when a newer one is generated before copy", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
    let replaced = false;
    mockBackend({
      withProject: true,
      readRecoveryOverview: () => {
        const overview = makeRecoveryOverview();
        if (replaced && overview.latest) {
          overview.latest = {
            ...overview.latest,
            id: "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
            markdown: "# New snapshot",
          };
        }
        return overview;
      },
    });
    window.location.hash = "#/recovery";
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "预览完整指令" }));
    const preview = within(await screen.findByRole("dialog", { name: "完整恢复指令" }));
    replaced = true;
    await user.click(preview.getByRole("button", { name: "复制恢复指令" }));

    expect(await screen.findByText("复制历史恢复指令？")).toBeInTheDocument();
    expect(writeText).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "仍然复制" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(1));
    const copied = String(writeText.mock.calls[0][0]);
    expect(copied).toContain("这是历史恢复快照");
    expect(copied.endsWith(makeRecoverySnapshot().markdown)).toBe(true);
  });

  it("leaves the clipboard untouched if the current snapshot cannot be checked", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
    let unavailable = false;
    mockBackend({
      withProject: true,
      readRecoveryOverview: () => {
        if (unavailable) throw { code: "RECOVERY_ERROR", message: "检查失败" };
        return makeRecoveryOverview();
      },
    });
    window.location.hash = "#/recovery";
    render(<App />);

    await screen.findByRole("region", { name: "下一步" });
    unavailable = true;
    await user.click(screen.getByRole("button", { name: "复制恢复指令" }));

    expect(await screen.findByText("无法检查恢复快照")).toBeInTheDocument();
    expect(writeText).not.toHaveBeenCalled();
  });

  it("persists the selected next task and regenerates the recovery snapshot", async () => {
    mockBackend({ withProject: true });
    const user = userEvent.setup();
    window.location.hash = "#/recovery";
    render(<App />);

    await screen.findByRole("region", { name: "下一步" });
    const setNextButton = screen.getByRole("button", { name: "设为下一步" });
    await waitFor(() => expect(setNextButton).toBeEnabled());
    await user.click(setNextButton);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "set_recovery_next_task",
        expect.objectContaining({
          input: { projectId: project.id, taskId: task.id },
        }),
      );
      expect(invokeMock).toHaveBeenCalledWith(
        "generate_recovery_package",
        expect.objectContaining({ projectId: project.id }),
      );
    });
  });

  it("opens a historical recovery snapshot with a not-current warning", async () => {
    mockBackend({ withProject: true });
    const user = userEvent.setup();
    window.location.hash = "#/recovery";
    render(<App />);

    const historyButton = await screen.findByRole("button", { name: "历史快照" });
    await user.click(historyButton);
    expect(await screen.findByText("历史恢复快照")).toBeInTheDocument();
    const viewButtons = await screen.findAllByRole("button", { name: "查看" });
    await user.click(viewButtons[1]);

    expect(await screen.findByText("这是历史快照")).toBeInTheDocument();
    expect(await screen.findByText("完整恢复指令")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(historyButton).toHaveFocus());
  });

  it("generates a recovery package when none exists", async () => {
    mockBackend({ withProject: true, recoveryState: "no_snapshot" });
    const user = userEvent.setup();
    window.location.hash = "#/recovery";
    render(<App />);

    expect(await screen.findByText("尚未生成恢复包")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "生成恢复包" }));
    expect(await screen.findByRole("region", { name: "下一步" })).toBeInTheDocument();
  });
});
