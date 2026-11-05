import { z } from "zod";

import { hasDisallowedControlCharacter } from "@/lib/zod-helpers";

export const importProjectSchema = z.object({
  rootPath: z
    .string()
    .trim()
    .min(1, "请输入 Git 项目路径")
    .max(4096, "项目路径最多 4096 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value), "路径包含不支持的控制字符"),
  name: z
    .string()
    .trim()
    .max(120, "项目名称最多 120 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value), "名称包含不支持的控制字符")
    .optional(),
});

export const renameProjectSchema = z.object({
  name: z
    .string()
    .trim()
    .min(1, "项目名称不能为空")
    .max(120, "项目名称最多 120 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value), "名称包含不支持的控制字符"),
});

export type ImportProjectValues = z.infer<typeof importProjectSchema>;
export type RenameProjectValues = z.infer<typeof renameProjectSchema>;
