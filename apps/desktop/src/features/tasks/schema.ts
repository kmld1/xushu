import { z } from "zod";

import { hasDisallowedControlCharacter } from "@/lib/zod-helpers";

export const TASK_PRIORITIES = ["low", "medium", "high", "urgent"] as const;
export const CRITERION_KINDS = ["manual", "auto"] as const;

const titleSchema = z
  .string()
  .trim()
  .min(1, "任务标题不能为空")
  .max(200, "任务标题最多 200 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value), "标题包含不支持的控制字符");

const descriptionSchema = z
  .string()
  .trim()
  .max(10_000, "任务描述最多 10000 个字符")
  .refine((value) => !hasDisallowedControlCharacter(value, true), "描述包含不支持的控制字符");

export const createTaskSchema = z.object({
  title: titleSchema,
  description: descriptionSchema,
  priority: z.enum(TASK_PRIORITIES),
});

export const editTaskSchema = createTaskSchema;

export const criterionSchema = z.object({
  description: z
    .string()
    .trim()
    .min(1, "验收条件不能为空")
    .max(1_000, "验收条件最多 1000 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value, true), "验收条件包含不支持的控制字符"),
  kind: z.enum(CRITERION_KINDS),
  required: z.boolean(),
});

export const reasonSchema = z.object({
  reason: z
    .string()
    .trim()
    .min(1, "请填写原因")
    .max(1_000, "原因最多 1000 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value, true), "原因包含不支持的控制字符"),
});

export const confirmationNoteSchema = z.object({
  note: z
    .string()
    .trim()
    .max(1_000, "备注最多 1000 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value, true), "备注包含不支持的控制字符")
    .optional(),
});

export type CreateTaskValues = z.infer<typeof createTaskSchema>;
export type EditTaskValues = z.infer<typeof editTaskSchema>;
export type CriterionValues = z.infer<typeof criterionSchema>;
export type ReasonValues = z.infer<typeof reasonSchema>;
export type ConfirmationNoteValues = z.infer<typeof confirmationNoteSchema>;
