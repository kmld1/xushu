import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import type { ProbeRecord } from "@/features/persistence/types";
import type { AppInfo, DatabaseHealth } from "@/features/system/types";

import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

type InvokeMock = Mock<(command: string, args?: Record<string, unknown>) => Promise<unknown>>;

const invokeMock = invoke as unknown as InvokeMock;

const appInfo: AppInfo = {
  name: "序枢 XUSHU",
  version: "0.1.0",
  tauriVersion: "2.12.5",
  phase: "Phase 1",
  platform: "windows",
};

const healthyDatabase: DatabaseHealth = {
  ok: true,
  integrityOk: true,
  foreignKeysEnabled: true,
  foreignKeyViolations: false,
  journalMode: "wal",
  schemaVersion: 1,
  recordCount: 0,
  databaseFile: "xushu.db",
};

function mockBackend(initialRecords: ProbeRecord[] = []) {
  let records = [...initialRecords];

  invokeMock.mockImplementation(async (command, args) => {
    switch (command) {
      case "get_app_info":
        return appInfo;
      case "get_database_health":
        return { ...healthyDatabase, recordCount: records.length };
      case "list_probe_records":
        return records;
      case "create_probe_record": {
        const input = (args?.input ?? {}) as { label?: string; content?: string };
        const record: ProbeRecord = {
          id: "11111111-1111-4111-8111-111111111111",
          label: input.label ?? "",
          content: input.content ?? "",
          createdAt: "2026-11-05T15:00:00.000Z",
        };
        records = [record, ...records];
        return record;
      }
      case "get_probe_record": {
        const record = records.find((item) => item.id === args?.id);
        if (!record) {
          throw { code: "NOT_FOUND", message: "未找到指定记录" };
        }
        return record;
      }
      default:
        throw { code: "UNKNOWN_COMMAND", message: `未知命令: ${command}` };
    }
  });

  return {
    getRecords: () => records,
  };
}

describe("XUSHU desktop shell", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.localStorage.clear();
    window.location.hash = "#/";
  });

  it("renders real backend health and application information", async () => {
    mockBackend();
    render(<App />);

    expect(await screen.findByRole("heading", { name: "概览" })).toBeInTheDocument();
    expect(await screen.findByText("系统健康")).toBeInTheDocument();
    expect(await screen.findByText("WAL")).toBeInTheDocument();
    expect(await screen.findByText("v1")).toBeInTheDocument();
    expect(await screen.findByText("已开启")).toBeInTheDocument();
    expect(await screen.findByText("2.12.5")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("get_database_health", undefined);
  });

  it("navigates to the persistence page and shows an empty state", async () => {
    mockBackend();
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "本地数据" }));

    expect(await screen.findByRole("heading", { name: "本地数据" })).toBeInTheDocument();
    expect(await screen.findByText("尚无测试记录")).toBeInTheDocument();
  });

  it("validates the form before invoking the backend", async () => {
    mockBackend();
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "本地数据" }));
    await screen.findByText("尚无测试记录");

    await user.click(screen.getByRole("button", { name: "写入测试记录" }));

    expect(await screen.findByText("标签不能为空")).toBeInTheDocument();
    expect(await screen.findByText("内容不能为空")).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith("create_probe_record", expect.anything());
  });

  it("writes a record through IPC and displays the returned row", async () => {
    mockBackend();
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("link", { name: "本地数据" }));
    await screen.findByText("尚无测试记录");

    await user.type(screen.getByLabelText("标签"), "跨进程验证");
    await user.type(screen.getByLabelText("内容"), "phase-1-persistence");
    await user.click(screen.getByRole("button", { name: "写入测试记录" }));

    expect(await screen.findByText("已写入 SQLite")).toBeInTheDocument();
    expect(await screen.findByText("跨进程验证")).toBeInTheDocument();
    expect(await screen.findByText("phase-1-persistence")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith(
      "create_probe_record",
      expect.objectContaining({
        input: { label: "跨进程验证", content: "phase-1-persistence" },
      }),
    );
  });

  it("switches between dark and light themes", async () => {
    mockBackend();
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

  it("shows a readable error when the backend health query fails", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "get_app_info") {
        return appInfo;
      }
      throw { code: "DATABASE_UNAVAILABLE", message: "本地数据库尚未就绪" };
    });

    render(<App />);

    expect(
      await screen.findByText("无法读取数据库健康状态", undefined, { timeout: 3_000 }),
    ).toBeInTheDocument();
    expect(screen.getByText("本地数据库尚未就绪")).toBeInTheDocument();
  });
});
