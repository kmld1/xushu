import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function runWindowAction(action: "minimize" | "toggleMaximize" | "close") {
  if (!isTauriRuntime()) {
    return;
  }

  const window = getCurrentWindow();

  try {
    if (action === "minimize") {
      await window.minimize();
    } else if (action === "close") {
      await window.close();
    } else {
      await window.toggleMaximize();
    }
  } catch {
    // 窗口控制失败不应影响页面本身；用户仍可用系统快捷键或任务栏操作。
  }
}

function MinimizeIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M0 5.5h10" stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

function MaximizeIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" />
    </svg>
  );
}

function RestoreIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M2.5 2.5V0.5h7v7h-2" fill="none" stroke="currentColor" />
      <rect x="0.5" y="2.5" width="7" height="7" fill="none" stroke="currentColor" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

export function WindowControls() {
  const [maximized, setMaximized] = useState(false);
  const [snapHover, setSnapHover] = useState(false);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let cancelled = false;
    const cleanups: Array<() => void> = [];
    const window = getCurrentWindow();

    const syncMaximized = () => {
      void window
        .isMaximized()
        .then((value) => {
          if (!cancelled) {
            setMaximized(value);
          }
        })
        .catch(() => undefined);
    };

    syncMaximized();

    void window
      .onResized(syncMaximized)
      .then((unlisten) => {
        if (cancelled) {
          unlisten();
        } else {
          cleanups.push(unlisten);
        }
      })
      .catch(() => undefined);

    const subscriptions = [
      window.listen("xushu://chrome/snap-enter", () => setSnapHover(true)),
      window.listen("xushu://chrome/snap-leave", () => setSnapHover(false)),
    ];

    void Promise.all(subscriptions)
      .then((unlisteners) => {
        for (const unlisten of unlisteners) {
          if (cancelled) {
            unlisten();
          } else {
            cleanups.push(unlisten);
          }
        }
      })
      .catch((error) => {
        // 悬停高亮失败不影响窗口控制；生产环境不输出噪音。
        if (import.meta.env.DEV) {
          console.warn("无法监听窗口 Snap 事件", error);
        }
      });

    return () => {
      cancelled = true;
      for (const cleanup of cleanups) {
        cleanup();
      }
    };
  }, []);

  return (
    <div role="group" aria-label="窗口控制" className="flex h-11 shrink-0 items-stretch">
      <button
        type="button"
        aria-label="最小化窗口"
        title="最小化"
        onClick={() => void runWindowAction("minimize")}
        className="grid w-[46px] place-items-center text-muted-foreground transition-colors hover:bg-window-control-hover hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
      >
        <MinimizeIcon />
      </button>
      <button
        type="button"
        aria-label={maximized ? "还原窗口" : "最大化窗口"}
        title={maximized ? "还原" : "最大化"}
        onClick={() => void runWindowAction("toggleMaximize")}
        className={cn(
          "grid w-[46px] place-items-center text-muted-foreground transition-colors hover:bg-window-control-hover hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring",
          snapHover && "bg-window-control-hover text-foreground",
        )}
      >
        {maximized ? <RestoreIcon /> : <MaximizeIcon />}
      </button>
      <button
        type="button"
        aria-label="关闭窗口"
        title="关闭"
        onClick={() => void runWindowAction("close")}
        className="grid w-[46px] place-items-center text-muted-foreground transition-colors hover:bg-window-close hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
      >
        <CloseIcon />
      </button>
    </div>
  );
}
