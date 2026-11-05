import { describe, expect, it } from "vitest";

import { confirmationNoteSchema, createTaskSchema, criterionSchema, reasonSchema } from "./schema";

describe("task schemas", () => {
  it("accepts valid task input and multiline descriptions", () => {
    expect(
      createTaskSchema.safeParse({
        title: "实现项目导入",
        description: "第一行\n第二行",
        priority: "high",
      }).success,
    ).toBe(true);
  });

  it("rejects empty, overlong, and control-character titles", () => {
    expect(
      createTaskSchema.safeParse({
        title: "   ",
        description: "",
        priority: "medium",
      }).success,
    ).toBe(false);
    expect(
      createTaskSchema.safeParse({
        title: "a".repeat(201),
        description: "",
        priority: "medium",
      }).success,
    ).toBe(false);
    expect(
      createTaskSchema.safeParse({
        title: "bad\u0000title",
        description: "",
        priority: "medium",
      }).success,
    ).toBe(false);
  });

  it("validates criterion kind and required flag", () => {
    expect(
      criterionSchema.safeParse({
        description: "人工确认结果",
        kind: "manual",
        required: true,
      }).success,
    ).toBe(true);
    expect(
      criterionSchema.safeParse({
        description: "自动测试通过",
        kind: "unknown",
        required: true,
      }).success,
    ).toBe(false);
    expect(
      criterionSchema.safeParse({
        description: "   ",
        kind: "auto",
        required: false,
      }).success,
    ).toBe(false);
  });

  it("requires a non-empty reason and allows an optional confirmation note", () => {
    expect(reasonSchema.safeParse({ reason: "等待接口" }).success).toBe(true);
    expect(reasonSchema.safeParse({ reason: " " }).success).toBe(false);
    expect(confirmationNoteSchema.safeParse({ note: "" }).success).toBe(true);
    expect(confirmationNoteSchema.safeParse({ note: "已验证" }).success).toBe(true);
  });
});
