import { z } from "zod";

import { hasDisallowedControlCharacter } from "@/lib/zod-helpers";

export const verificationSpecSchema = z.object({
  executable: z
    .string()
    .trim()
    .min(1, "验证程序不能为空")
    .max(4096, "验证程序最多 4096 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value), "验证程序包含不支持的控制字符"),
  argsText: z
    .string()
    .max(131_072, "参数总长度过大")
    .refine((value) => {
      const lines = value
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean);
      return lines.length <= 128;
    }, "参数最多 128 个")
    .refine((value) => {
      const lines = value
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean);
      return lines.every(
        (line) => line.length <= 4096 && !hasDisallowedControlCharacter(line, true),
      );
    }, "每个参数最多 4096 个字符且不能包含控制字符"),
  cwd: z
    .string()
    .trim()
    .max(4096, "工作目录最多 4096 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value), "工作目录包含不支持的控制字符"),
  timeoutSeconds: z
    .string()
    .trim()
    .refine((value) => /^\d+$/.test(value), "超时时间必须是整数秒")
    .refine((value) => Number(value) >= 1 && Number(value) <= 3600, "超时时间必须在 1–3600 秒"),
  outputLimitBytes: z
    .string()
    .trim()
    .refine((value) => /^\d+$/.test(value), "输出上限必须是字节整数")
    .refine(
      (value) => Number(value) >= 1024 && Number(value) <= 1_048_576,
      "输出上限必须在 1024–1048576 字节",
    ),
});

export type VerificationSpecValues = z.infer<typeof verificationSpecSchema>;

export function parseArgsText(value: string): string[] {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

export function argsToText(args: string[]): string {
  return args.join("\n");
}
