import { invoke } from "@tauri-apps/api/core";

export class IpcError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "IpcError";
    this.code = code;
  }
}

type ErrorShape = {
  code?: unknown;
  message?: unknown;
};

function isErrorShape(value: unknown): value is { code: string; message: string } {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as ErrorShape;
  return typeof candidate.code === "string" && typeof candidate.message === "string";
}

export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isErrorShape(error)) {
      throw new IpcError(error.code, error.message);
    }

    throw new IpcError("IPC_ERROR", "无法连接本地桌面服务，请重启应用后重试");
  }
}

export function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.length > 0) {
    return error.message;
  }
  return "发生未知错误";
}
