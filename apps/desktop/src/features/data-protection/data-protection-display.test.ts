import { describe, expect, it } from "vitest";

import {
  formatSize,
  RESTORE_STEPS,
  restoreResultTone,
  restoreStepIndex,
  restoreStepState,
  shortHash,
} from "./data-protection-display";

describe("data protection display helpers", () => {
  it("formats backup sizes across KB, MB and GB", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(2048)).toBe("2.0 KB");
    expect(formatSize(5 * 1024 * 1024)).toBe("5.0 MB");
    expect(formatSize(3 * 1024 * 1024 * 1024)).toBe("3.00 GB");
    expect(formatSize(null)).toBe("—");
  });

  it("shortens long hashes without hiding that they are hashes", () => {
    expect(shortHash(null)).toBe("—");
    expect(shortHash("a".repeat(64))).toBe(`${"a".repeat(16)}…`);
    expect(shortHash("abcd")).toBe("abcd");
  });

  it("models the seven-step restore sequence", () => {
    expect(RESTORE_STEPS).toHaveLength(7);
    expect(restoreStepIndex("choose", false)).toBe(0);
    expect(restoreStepIndex("preflight", false)).toBe(1);
    expect(restoreStepIndex("preflight", true)).toBe(3);
    expect(restoreStepIndex("confirm", true)).toBe(4);
    expect(restoreStepIndex("executing", true)).toBe(5);
    expect(restoreStepIndex("result", true)).toBe(6);
    expect(restoreStepState(2, 3, false)).toBe("done");
    expect(restoreStepState(3, 3, false)).toBe("active");
    expect(restoreStepState(1, 1, true)).toBe("failed");
  });

  it("does not treat rollback as a successful restore", () => {
    expect(restoreResultTone(null)).toBe("warning");
    expect(restoreResultTone({ status: "completed" } as never)).toBe("success");
    expect(restoreResultTone({ status: "rolled_back" } as never)).toBe("warning");
    expect(restoreResultTone({ status: "rollback_failed" } as never)).toBe("destructive");
  });
});
