import type { RecoveryFreshness, RecoverySnapshot } from "@/lib/domain";

/**
 * 复制到剪贴板的完整文本。
 *
 * 当前快照直接使用生成时的 Markdown；过期快照在开头明确标注“生成后数据已变化”，
 * 防止新会话把旧状态当成当前事实。
 */
export function buildClipboardText(
  snapshot: RecoverySnapshot,
  freshness: RecoveryFreshness,
  options?: { historical?: boolean },
): string {
  if (options?.historical) {
    return [
      `> 注意：这是历史恢复快照，生成于 ${snapshot.generatedAt}（UTC）。`,
      "> 它不代表当前项目状态；请以最新快照或重新生成结果为准。",
      "",
      snapshot.markdown,
    ].join("\n");
  }

  if (freshness.state !== "stale") {
    return snapshot.markdown;
  }

  const changed =
    freshness.changed.length > 0
      ? `已检测到变化：${freshness.changed.join("；")}。`
      : "已检测到项目数据变化。";

  return [
    `> 注意：这份恢复上下文生成于 ${snapshot.generatedAt}（UTC）。`,
    `> ${changed}以下内容反映生成时状态，不代表当前事实。`,
    "> 建议先在 XUSHU 中重新生成恢复包，再复制给新会话。",
    "",
    snapshot.markdown,
  ].join("\n");
}

export async function copyTextToClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  textarea.style.top = "0";
  document.body.appendChild(textarea);
  textarea.select();

  try {
    const copied = document.execCommand("copy");
    if (!copied) {
      throw new Error("浏览器拒绝了复制操作");
    }
  } finally {
    textarea.remove();
  }
}
