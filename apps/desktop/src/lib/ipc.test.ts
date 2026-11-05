import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import { IpcError, invokeCommand } from "./ipc";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

type InvokeMock = Mock<(command: string, args?: Record<string, unknown>) => Promise<unknown>>;

const invokeMock = invoke as unknown as InvokeMock;

describe("invokeCommand", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("returns the backend result", async () => {
    invokeMock.mockResolvedValueOnce({ ok: true });

    await expect(invokeCommand<{ ok: boolean }>("health")).resolves.toEqual({ ok: true });
    expect(invokeMock).toHaveBeenCalledWith("health", undefined);
  });

  it("maps structured backend errors to IpcError", async () => {
    invokeMock.mockRejectedValueOnce({
      code: "VALIDATION_ERROR",
      message: "标签不能为空",
    });

    const error = await invokeCommand("create_probe_record").catch((caught: unknown) => caught);

    expect(error).toBeInstanceOf(IpcError);
    expect(error).toMatchObject({
      code: "VALIDATION_ERROR",
      message: "标签不能为空",
    });
  });

  it("does not leak unstructured rejection payloads", async () => {
    invokeMock.mockRejectedValueOnce("raw internal failure");

    const error = await invokeCommand("health").catch((caught: unknown) => caught);

    expect(error).toBeInstanceOf(IpcError);
    expect(error).toMatchObject({
      code: "IPC_ERROR",
      message: "无法连接本地桌面服务，请重启应用后重试",
    });
  });
});
