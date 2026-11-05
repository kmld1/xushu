import { describe, expect, it } from "vitest";

import { probeFormSchema } from "./schema";

describe("probeFormSchema", () => {
  it("accepts Chinese text and line breaks", () => {
    const result = probeFormSchema.safeParse({
      label: "跨进程验证",
      content: "第一行\n第二行\t缩进",
    });

    expect(result.success).toBe(true);
  });

  it("trims surrounding whitespace", () => {
    const result = probeFormSchema.parse({
      label: "  标签  ",
      content: "  内容  ",
    });

    expect(result).toEqual({ label: "标签", content: "内容" });
  });

  it("rejects empty, overlong, and control-character input", () => {
    const cases = [
      { label: "   ", content: "内容" },
      { label: "a".repeat(65), content: "内容" },
      { label: "bad\nlabel", content: "内容" },
      { label: "标签", content: "   " },
      { label: "标签", content: "a".repeat(2001) },
      { label: "标签", content: "bad\u0000content" },
    ];

    for (const value of cases) {
      expect(probeFormSchema.safeParse(value).success, JSON.stringify(value)).toBe(false);
    }
  });
});
