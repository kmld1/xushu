import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type {
  BackupRecord,
  BackupVerification,
  DataIntegrityReport,
  DataProtectionOverview,
  RestoreOutcome,
  RestorePreflight,
} from "@/lib/domain";
import { IpcError } from "@/lib/ipc";

import * as api from "@/features/data-protection/api";
import { DataProtectionPage } from "./data-protection-page";

vi.mock("@/features/data-protection/api", () => ({
  getDataProtectionOverview: vi.fn(),
  createBackup: vi.fn(),
  cancelBackup: vi.fn(),
  verifyBackup: vi.fn(),
  deleteBackup: vi.fn(),
  getBackupPolicy: vi.fn(),
  updateBackupPolicy: vi.fn(),
  previewBackupCleanup: vi.fn(),
  applyBackupCleanup: vi.fn(),
  runDueAutomaticBackup: vi.fn(),
  checkDataIntegrity: vi.fn(),
  preflightRestore: vi.fn(),
  performRestore: vi.fn(),
  dismissRestoreNotice: vi.fn(),
}));

const backup: BackupRecord = {
  id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  backupId: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
  directoryPath: "D:\\XUSHU-Backups\\xushu-backup-20261113T100000Z-bbbbbbbb",
  formatVersion: 1,
  schemaVersion: 7,
  appVersion: "0.1.0",
  origin: "manual",
  createdAt: "2026-11-13T10:00:00.000Z",
  registeredAt: "2026-11-13T10:00:01.000Z",
  dataSizeBytes: 4096,
  manifestSizeBytes: 512,
  payloadSha256: "a".repeat(64),
  integrityStatus: "verified",
  verifiedAt: "2026-11-13T10:00:01.000Z",
  lastError: null,
  directoryExists: true,
  manifestExists: true,
};

const counts = {
  projects: 1,
  tasks: 2,
  acceptanceCriteria: 3,
  verificationRuns: 1,
  verificationChecks: 1,
  evidence: 1,
  claudeSessions: 1,
  claudeEvents: 2,
  projectMemories: 1,
  projectMemoryRevisions: 1,
  decisions: 1,
  decisionRevisions: 1,
  recoverySnapshots: 1,
  backupRecords: 1,
};

const integrityReport: DataIntegrityReport = {
  checkedAt: "2026-11-13T10:05:00.000Z",
  ok: true,
  schemaVersion: 7,
  expectedSchemaVersion: 7,
  integrityMessage: "SQLite integrity_check = ok",
  counts,
  checks: [
    {
      id: "sqlite_integrity",
      label: "SQLite 页面完整性",
      status: "pass",
      detail: "SQLite integrity_check = ok",
    },
  ],
};

const overview: DataProtectionOverview = {
  generatedAt: "2026-11-13T10:05:00.000Z",
  appVersion: "0.1.0",
  schemaVersion: 7,
  dataDirectory: "C:\\Users\\tester\\AppData\\Roaming\\dev.xushu.desktop",
  database: {
    ok: true,
    integrityOk: true,
    foreignKeysEnabled: true,
    foreignKeyViolations: false,
    journalMode: "wal",
    schemaVersion: 7,
    recordCount: 0,
    databaseFile: "xushu.db",
  },
  databaseSizeBytes: 4096,
  walSizeBytes: 8192,
  shmSizeBytes: 32768,
  counts,
  backups: [backup],
  lastBackup: backup,
  lastVerifiedBackup: backup,
  backupTotalSizeBytes: 4608,
  policy: {
    automaticEnabled: false,
    targetDirectory: "D:\\XUSHU-Backups",
    intervalHours: 24,
    retentionCount: 5,
    lastAutomaticAt: null,
    lastAutomaticErrorCode: null,
    lastAutomaticErrorAt: null,
    updatedAt: "2026-11-13T10:00:00.000Z",
  },
  policyTargetFreeBytes: 123456789,
  automaticDue: false,
  restoreOperations: [],
  startupNotice: null,
  inventory: [
    {
      id: "projects_tasks",
      label: "项目与任务",
      location: "应用数据目录 / xushu.db",
      authoritative: true,
      regenerable: false,
      sensitive: true,
      includedInBackup: true,
      exportAvailable: true,
      exportNote: "通过一致性备份导出",
      specialRecovery: false,
      retention: "保留至归档",
      impactIfLost: "项目与任务上下文丢失",
      countLabel: "1 项目 · 2 任务",
      sizeBytes: 4096,
      notes: "包含本地路径",
    },
    {
      id: "hook_spool",
      label: "Hook 待导入 Spool",
      location: "应用数据目录 / hook-inbox / pending",
      authoritative: false,
      regenerable: false,
      sensitive: true,
      includedInBackup: false,
      exportAvailable: false,
      exportNote: "不导出",
      specialRecovery: true,
      retention: "导入后移除",
      impactIfLost: "未导入事件丢失",
      countLabel: "0 条待导入",
      sizeBytes: 0,
      notes: "不属于数据库备份",
    },
  ],
  hooks: {
    eventCount: 2,
    sessionCount: 1,
    integrationCount: 1,
    desiredEnabledProjectCount: 1,
    pendingSpoolEvents: 0,
    pendingSpoolBytes: 0,
    saltPresent: true,
    saltBytes: 32,
    inboxAvailable: true,
    retentionDays: 30,
    maxEventsPerProject: 5000,
    collectedFields: ["事件类型与时间"],
    excludedFields: ["用户原始 Prompt"],
    note: "Hook 事件不是验收证据。",
  },
  retention: {
    hookEventDays: 30,
    hookMaxEventsPerProject: 5000,
    hookMaxSessionsPerProject: 500,
    recoverySnapshotsPerProject: 20,
    backupRetentionCount: 5,
    restoreWorkspaces: "启动时清理",
    rollbackArchives: "不自动删除",
    backupFiles: "只清理自动备份",
  },
  encryption: {
    encrypted: false,
    supported: false,
    note: "当前备份不加密。",
  },
  warnings: ["Phase 6 备份不加密。"],
};

const dataSummary = {
  projects: 1,
  archivedProjects: 0,
  tasks: 2,
  doneTasks: 1,
  criteria: 3,
  verificationRuns: 1,
  evidence: 1,
  claudeEvents: 2,
  memories: 1,
  decisions: 1,
  recoverySnapshots: 1,
  projectNames: ["示例项目"],
  missingProjectRootCount: 0,
  runningVerificationCount: 0,
  desiredHookProjectCount: 1,
  databaseSizeBytes: 4096,
  lastUpdatedAt: "2026-11-13T09:00:00.000Z",
};

const preflight: RestorePreflight = {
  token: "restore-token",
  ready: true,
  blockingReason: null,
  backupId: backup.backupId,
  backupDirectory: backup.directoryPath,
  createdAt: backup.createdAt,
  appVersion: "0.1.0",
  origin: "manual",
  formatVersion: 1,
  backupSchemaVersion: 7,
  currentSchemaVersion: 7,
  schemaAction: "same",
  manifestSha256: "b".repeat(64),
  payloadSha256: backup.payloadSha256,
  payloadSizeBytes: 4096,
  current: dataSummary,
  incoming: dataSummary,
  changesSinceBackup: [{ label: "任务更新", count: 2 }],
  checks: [
    { id: "manifest", label: "备份清单格式", status: "pass", detail: "通过" },
    { id: "payload_hash", label: "数据文件校验和", status: "pass", detail: "一致" },
    {
      id: "isolated_sqlite_integrity",
      label: "隔离：SQLite 页面完整性",
      status: "pass",
      detail: "integrity_check = ok",
    },
  ],
  warnings: ["备份未加密。"],
  effects: ["当前数据库将被替换。"],
  hooksReview: {
    projectCount: 1,
    settingsFilesModified: false,
    pendingSpoolIncluded: false,
    saltIncluded: false,
    note: "需要重新检查 Hooks。",
  },
  expiresAt: "2026-11-13T10:20:00.000Z",
};

const outcome: RestoreOutcome = {
  operationId: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
  status: "completed",
  switchVerified: true,
  finalReport: integrityReport,
  rollback: {
    status: "available",
    directory: "C:\\Users\\tester\\AppData\\Roaming\\dev.xushu.desktop\\restore-rollback\\op",
    verified: true,
    message: "回退副本已保留",
  },
  hooksReview: {
    projectCount: 1,
    settingsFilesModified: false,
    pendingSpoolIncluded: false,
    saltIncluded: false,
    note: "需要重新检查 Hooks。",
  },
  message: "恢复完成并通过最终完整性检查",
  completedAt: "2026-11-13T10:06:00.000Z",
  warnings: ["备份未加密。"],
};

const verification: BackupVerification = {
  backupId: backup.backupId,
  verified: true,
  verifiedAt: "2026-11-13T10:07:00.000Z",
  checks: [
    { id: "manifest", label: "备份清单", status: "pass", detail: "格式通过" },
    { id: "payload_hash", label: "数据文件校验和", status: "pass", detail: "一致" },
  ],
  integrity: integrityReport,
  message: "备份完整性验证通过",
};

function mockDefaults() {
  vi.mocked(api.getDataProtectionOverview).mockResolvedValue(overview);
  vi.mocked(api.createBackup).mockResolvedValue(backup);
  vi.mocked(api.verifyBackup).mockResolvedValue(verification);
  vi.mocked(api.deleteBackup).mockResolvedValue(undefined);
  vi.mocked(api.runDueAutomaticBackup).mockResolvedValue({
    status: "not_due",
    backup: null,
    removedCount: 0,
    message: null,
  });
  vi.mocked(api.checkDataIntegrity).mockResolvedValue(integrityReport);
  vi.mocked(api.preflightRestore).mockResolvedValue(preflight);
  vi.mocked(api.performRestore).mockResolvedValue(outcome);
  vi.mocked(api.dismissRestoreNotice).mockResolvedValue(undefined);
  vi.mocked(api.cancelBackup).mockResolvedValue(undefined);
  vi.mocked(api.updateBackupPolicy).mockResolvedValue(overview.policy);
  vi.mocked(api.previewBackupCleanup).mockResolvedValue([]);
  vi.mocked(api.applyBackupCleanup).mockResolvedValue({
    removedBackupIds: [],
    skippedCount: 0,
    failures: [],
  });
  vi.mocked(api.getBackupPolicy).mockResolvedValue(overview.policy);
}

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <DataProtectionPage />
    </QueryClientProvider>,
  );
}

describe("data protection page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockDefaults();
  });

  it("renders real backup state, privacy boundaries and restore history", async () => {
    renderPage();

    expect(await screen.findByText("数据库备份")).toBeInTheDocument();
    expect(screen.getByText("已验证")).toBeInTheDocument();
    expect(screen.getByText(backup.directoryPath)).toBeInTheDocument();
    expect(screen.getByText("Hook 事件不是验收证据。")).toBeInTheDocument();
    expect(screen.getByText("不在备份")).toBeInTheDocument();
    expect(screen.getByText("尚无恢复操作记录")).toBeInTheDocument();
    expect(screen.getByText("当前备份不加密。")).toBeInTheDocument();
  });

  it("creates a backup from the dialog and shows the verified result", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText("数据库备份");
    await user.click(screen.getByRole("button", { name: "创建备份" }));
    const dialog = await screen.findByRole("dialog", { name: "创建数据库备份" });
    const target = within(dialog).getByLabelText("备份目标目录");
    await user.clear(target);
    await user.type(target, "D:\\Backups");
    await user.click(within(dialog).getByRole("button", { name: "创建备份" }));

    await waitFor(() => expect(api.createBackup).toHaveBeenCalledWith("D:\\Backups"));
    expect(await within(dialog).findByText("备份已创建")).toBeInTheDocument();
  });

  it("opens an unregistered backup for preflight with an empty registry", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getDataProtectionOverview).mockResolvedValue({
      ...overview,
      backups: [],
      lastBackup: null,
      lastVerifiedBackup: null,
      backupTotalSizeBytes: 0,
    });
    renderPage();

    expect(await screen.findByText("当前数据库没有备份登记")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "打开外部备份" }));
    const directory = await screen.findByLabelText("备份目录");
    expect(directory).toHaveValue("");
    await user.type(directory, backup.directoryPath);
    await user.click(screen.getByRole("button", { name: "开始预检" }));

    await waitFor(() => expect(api.preflightRestore).toHaveBeenCalledWith(backup.directoryPath));
    expect(api.performRestore).not.toHaveBeenCalled();
    expect(api.createBackup).not.toHaveBeenCalled();
  });

  it("verifies a backup and shows the integrity checks", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText("数据库备份");
    await user.click(screen.getByRole("button", { name: "校验" }));

    expect(await screen.findByText("备份完整性校验")).toBeInTheDocument();
    expect(await screen.findByText("校验通过")).toBeInTheDocument();
    await waitFor(() => expect(api.verifyBackup).toHaveBeenCalledWith(backup.backupId));
  });

  it("runs an explicit integrity check", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText("数据完整性");
    await user.click(screen.getByRole("button", { name: "立即检查" }));
    expect(await screen.findByText("结构与业务关系检查通过")).toBeInTheDocument();
    expect(api.checkDataIntegrity).toHaveBeenCalledTimes(1);
  });

  it("requires typed confirmation and acknowledgement before restoring", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText("数据库备份");
    await user.click(screen.getByRole("button", { name: "恢复" }));

    const sheet = await screen.findByRole("dialog", { name: "恢复数据库" });
    for (const label of [
      "选择备份",
      "兼容性",
      "影响范围",
      "隔离验证",
      "明确确认",
      "执行恢复",
      "验证结果",
    ]) {
      expect(within(sheet).getByText(label)).toBeInTheDocument();
    }
    expect(within(sheet).getByLabelText("备份目录")).toHaveValue(backup.directoryPath);
    await user.click(within(sheet).getByRole("button", { name: "开始预检" }));

    expect(await within(sheet).findByText("继续确认")).toBeInTheDocument();
    expect(within(sheet).getByText("备份之后当前数据库的变化")).toBeInTheDocument();
    expect(within(sheet).getByText(/任务更新/)).toBeInTheDocument();
    await user.click(within(sheet).getByRole("button", { name: "继续确认" }));

    const execute = within(sheet).getByRole("button", { name: "执行恢复" });
    expect(execute).toBeDisabled();
    await user.type(within(sheet).getByLabelText("输入“恢复”以确认"), "恢复");
    expect(execute).toBeDisabled();
    await user.click(
      within(sheet).getByRole("checkbox", {
        name: /我已阅读影响范围与风险/,
      }),
    );
    expect(execute).toBeEnabled();
    await user.click(execute);

    await waitFor(() =>
      expect(api.performRestore).toHaveBeenCalledWith({
        token: "restore-token",
        confirmationText: "恢复",
        acknowledged: true,
      }),
    );
    expect(await within(sheet).findByText("恢复完成")).toBeInTheDocument();
  });

  it("shows a preflight failure without enabling restore", async () => {
    vi.mocked(api.preflightRestore).mockRejectedValue(
      new IpcError("UNSAFE_PATH", "所选路径不是可安全使用的本地目录"),
    );
    const user = userEvent.setup();
    renderPage();

    await screen.findByText("数据库备份");
    await user.click(screen.getByRole("button", { name: "恢复" }));
    const sheet = await screen.findByRole("dialog", { name: "恢复数据库" });
    await user.click(within(sheet).getByRole("button", { name: "开始预检" }));

    expect(await within(sheet).findByText("所选路径不是可安全使用的本地目录")).toBeInTheDocument();
    expect(within(sheet).queryByRole("button", { name: "继续确认" })).not.toBeInTheDocument();
  });

  it("shows and dismisses a startup restore notice", async () => {
    vi.mocked(api.getDataProtectionOverview).mockResolvedValue({
      ...overview,
      startupNotice: {
        outcome: "rolled_back",
        message: "检测到未完成的恢复切换；已自动回退到原数据库。",
        journalFile: "C:\\data\\restore-journal.json",
        rollbackDirectory: "C:\\data\\restore-rollback\\op",
        checkedAt: "2026-11-13T10:00:00.000Z",
      },
    });
    const user = userEvent.setup();
    renderPage();

    expect(
      await screen.findByText("检测到未完成的恢复切换；已自动回退到原数据库。"),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "知道了" }));
    await waitFor(() => expect(api.dismissRestoreNotice).toHaveBeenCalledTimes(1));
  });
});
