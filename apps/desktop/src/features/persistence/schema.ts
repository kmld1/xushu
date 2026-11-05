import { z } from "zod";

function hasDisallowedControlCharacter(value: string, allowLineBreaks: boolean) {
  for (const character of value) {
    const codePoint = character.codePointAt(0);
    if (codePoint === undefined) {
      continue;
    }

    const isControl = codePoint <= 0x1f || codePoint === 0x7f;
    if (!isControl) {
      continue;
    }

    if (allowLineBreaks && (codePoint === 0x09 || codePoint === 0x0a || codePoint === 0x0d)) {
      continue;
    }

    return true;
  }

  return false;
}

export const probeFormSchema = z.object({
  label: z
    .string()
    .trim()
    .min(1, "标签不能为空")
    .max(64, "标签最多 64 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value, false), "标签不能包含控制字符"),
  content: z
    .string()
    .trim()
    .min(1, "内容不能为空")
    .max(2000, "内容最多 2000 个字符")
    .refine((value) => !hasDisallowedControlCharacter(value, true), "内容包含不支持的控制字符"),
});

export type ProbeFormValues = z.infer<typeof probeFormSchema>;
