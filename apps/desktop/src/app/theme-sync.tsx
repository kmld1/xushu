import { invoke } from "@tauri-apps/api/core";
import { useEffect } from "react";

import { useThemeStore } from "@/stores/theme-store";

function syncWindowTheme(theme: "light" | "dark" | null) {
  if (!("__TAURI_INTERNALS__" in window)) {
    return;
  }

  // 与 Window.setTheme 使用同一窗口级命令；只同步唯一的 main 窗口。
  // 需要 capabilities 中的 core:window:allow-set-theme。
  void invoke("plugin:window|set_theme", { label: "main", value: theme }).catch(() => undefined);
}

export function ThemeSync() {
  const mode = useThemeStore((state) => state.mode);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");

    const applyTheme = () => {
      const isDark = mode === "dark" || (mode === "system" && media.matches);
      document.documentElement.classList.toggle("dark", isDark);
      document.documentElement.style.colorScheme = isDark ? "dark" : "light";

      // 失败只影响原生标题栏配色，不回滚页面主题。
      syncWindowTheme(mode === "system" ? null : isDark ? "dark" : "light");
    };

    applyTheme();
    media.addEventListener("change", applyTheme);

    return () => {
      media.removeEventListener("change", applyTheme);
    };
  }, [mode]);

  return null;
}
