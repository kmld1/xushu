import { afterEach, describe, expect, it, vi } from "vitest";

import type { RecoveryFreshness, RecoverySnapshot } from "@/lib/domain";

import { buildClipboardText, copyTextToClipboard } from "./copy-recovery";

const snapshot = {
  generatedAt: "2026-11-12T10:00:00.000Z",
  markdown: "# XUSHU 恢复上下文 — 示例项目\n\n真实内容。",
} as unknown as RecoverySnapshot;

const fresh: RecoveryFreshness = {
  state: "fresh",
  checkedAt: "2026-11-12T10:00:00.000Z",
  changed: [],
};

const stale: RecoveryFreshness = {
  state: "stale",
  checkedAt: "2026-11-12T11:00:00.000Z",
  changed: ["Git 状态或工作区指纹已变化", "任务状态或修订已变化"],
};

describe("recovery clipboard text", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("copies the generated markdown unchanged for a fresh snapshot", () => {
    expect(buildClipboardText(snapshot, fresh)).toBe(snapshot.markdown);
  });

  it("prefixes a stale snapshot with an explicit warning and changed areas", () => {
    const text = buildClipboardText(snapshot, stale);
    expect(text.startsWith("> 注意：这份恢复上下文生成于")).toBe(true);
    expect(text).toContain("Git 状态或工作区指纹已变化");
    expect(text).toContain("任务状态或修订已变化");
    expect(text).toContain("不代表当前事实");
    expect(text.endsWith(snapshot.markdown)).toBe(true);
  });

  it("labels historical snapshots as not current", () => {
    const text = buildClipboardText(snapshot, fresh, { historical: true });
    expect(text.startsWith("> 注意：这是历史恢复快照")).toBe(true);
    expect(text).toContain("不代表当前项目状态");
    expect(text.endsWith(snapshot.markdown)).toBe(true);
  });

  it("writes the exact prepared text to the clipboard", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    await copyTextToClipboard("恢复指令内容");

    expect(writeText).toHaveBeenCalledWith("恢复指令内容");
  });
});
