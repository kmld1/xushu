import { z } from "zod";

import { hasDisallowedControlCharacter } from "@/lib/zod-helpers";

export const MEMORY_CATEGORY_VALUES = [
  "goal",
  "constraint",
  "architecture",
  "convention",
  "fact",
  "risk",
  "follow_up",
  "knowledge",
] as const;

const memoryTitleSchema = z
  .string()
  .trim()
  .min(1, "记忆标题不能为空")
  .max(200, "记忆标题最多 200 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value), "标题包含不支持的控制字符");

const memoryContentSchema = z
  .string()
  .trim()
  .min(1, "记忆内容不能为空")
  .max(20_000, "记忆内容最多 20000 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value, true), "内容包含不支持的控制字符");

const changeNoteSchema = z
  .string()
  .trim()
  .max(1_000, "变更说明最多 1000 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value, true), "变更说明包含不支持的控制字符");

export const memoryFormSchema = z.object({
  category: z.enum(MEMORY_CATEGORY_VALUES),
  title: memoryTitleSchema,
  content: memoryContentSchema,
  linkedTaskId: z.string().optional(),
  changeNote: changeNoteSchema.optional(),
});

export type MemoryFormValues = z.infer<typeof memoryFormSchema>;

const decisionTitleSchema = z
  .string()
  .trim()
  .min(1, "决策标题不能为空")
  .max(200, "决策标题最多 200 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value), "标题包含不支持的控制字符");

const decisionProblemSchema = z
  .string()
  .trim()
  .min(1, "问题不能为空")
  .max(4_000, "问题最多 4000 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value, true), "问题包含不支持的控制字符");

const decisionContextSchema = z
  .string()
  .trim()
  .min(1, "背景与约束不能为空")
  .max(10_000, "背景与约束最多 10000 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value, true), "背景包含不支持的控制字符");

const decisionDecisionSchema = z
  .string()
  .trim()
  .min(1, "最终方案不能为空")
  .max(4_000, "最终方案最多 4000 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value, true), "最终方案包含不支持的控制字符");

const decisionRationaleSchema = z
  .string()
  .trim()
  .min(1, "决策理由不能为空")
  .max(10_000, "决策理由最多 10000 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value, true), "理由包含不支持的控制字符");

const decisionRisksSchema = z
  .string()
  .trim()
  .min(1, "已知代价与风险不能为空")
  .max(10_000, "已知代价与风险最多 10000 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value, true), "风险包含不支持的控制字符");

const alternativesSchema = z
  .string()
  .max(50_000, "备选方案总长度过大")
  .superRefine((value, context) => {
    const alternatives = value
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
    if (alternatives.length > 20) {
      context.addIssue({ code: "custom", message: "备选方案最多 20 个" });
    }
    if (alternatives.some((line) => line.length > 2_000)) {
      context.addIssue({ code: "custom", message: "单个备选方案最多 2000 个字符" });
    }
    if (alternatives.some((line) => hasDisallowedControlCharacter(line, true))) {
      context.addIssue({ code: "custom", message: "备选方案包含不支持的控制字符" });
    }
  });

export const decisionFormSchema = z.object({
  title: decisionTitleSchema,
  taskId: z.string().optional(),
  problem: decisionProblemSchema,
  context: decisionContextSchema,
  alternativesText: alternativesSchema,
  decision: decisionDecisionSchema,
  rationale: decisionRationaleSchema,
  risks: decisionRisksSchema,
  supersedesId: z.string().optional(),
  changeNote: changeNoteSchema.optional(),
});

export type DecisionFormValues = z.infer<typeof decisionFormSchema>;

export function alternativesFromText(value: string): string[] {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

export function alternativesToText(values: string[]): string {
  return values.join("\n");
}
