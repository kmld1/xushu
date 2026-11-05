import { describe, expect, it } from "vitest";

import { importProjectSchema, renameProjectSchema } from "./schema";

describe("project schemas", () => {
  it("accepts a local path and optional name", () => {
    const result = importProjectSchema.safeParse({
      rootPath: "D:\\Projects\\demo",
      name: "示例项目",
    });
    expect(result.success).toBe(true);
  });

  it("rejects empty, overlong, and control-character paths", () => {
    const cases = [
      { rootPath: "   ", name: "" },
      { rootPath: `D:\\${"a".repeat(4100)}`, name: "" },
      { rootPath: "D:\\bad\u0000path", name: "" },
    ];

    for (const value of cases) {
      expect(importProjectSchema.safeParse(value).success).toBe(false);
    }
  });

  it("rejects empty or overlong rename values", () => {
    expect(renameProjectSchema.safeParse({ name: "  " }).success).toBe(false);
    expect(renameProjectSchema.safeParse({ name: "a".repeat(121) }).success).toBe(false);
    expect(renameProjectSchema.safeParse({ name: "有效名称" }).success).toBe(true);
  });
});
