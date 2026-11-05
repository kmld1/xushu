import { Database, LayoutDashboard, Settings } from "lucide-react";
import { NavLink, Outlet } from "react-router";

import { ThemeControl } from "@/components/theme-control";
import { useAppInfo } from "@/features/system/queries";
import { cn } from "@/lib/utils";

const navigation = [
  { to: "/", label: "概览", icon: LayoutDashboard, end: true },
  { to: "/persistence", label: "本地数据", icon: Database, end: false },
  { to: "/settings", label: "设置", icon: Settings, end: false },
];

export function AppLayout() {
  const appInfo = useAppInfo();

  return (
    <div className="flex min-h-screen bg-background">
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-primary focus:px-4 focus:py-2 focus:text-primary-foreground"
        onClick={(event) => {
          event.preventDefault();
          document.getElementById("main-content")?.focus();
        }}
      >
        跳到主要内容
      </a>

      <aside className="flex w-64 shrink-0 flex-col border-r bg-card/60">
        <div className="flex h-16 items-center gap-3 border-b px-5">
          <div
            className="grid size-9 place-items-center rounded-lg bg-primary text-sm font-semibold text-primary-foreground"
            aria-hidden="true"
          >
            序
          </div>
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold leading-none">序枢 XUSHU</p>
            <p className="mt-1 truncate text-xs text-muted-foreground">Phase 1 · 本地持久化</p>
          </div>
        </div>

        <nav aria-label="主导航" className="flex-1 space-y-1 p-3">
          {navigation.map((item) => {
            const Icon = item.icon;

            return (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) =>
                  cn(
                    "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    isActive
                      ? "bg-accent text-accent-foreground"
                      : "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
                  )
                }
              >
                <Icon className="size-4" aria-hidden="true" />
                {item.label}
              </NavLink>
            );
          })}
        </nav>

        <div className="border-t p-4 text-xs text-muted-foreground">
          <p>本地优先 · 无云同步</p>
          <p className="mt-1">
            {appInfo.isPending
              ? "版本读取中…"
              : appInfo.data
                ? `v${appInfo.data.version}`
                : "版本不可用"}
          </p>
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-16 shrink-0 items-center justify-between border-b px-6">
          <p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
            Local-first work control
          </p>
          <ThemeControl />
        </header>

        <main id="main-content" tabIndex={-1} className="flex-1 p-6 lg:p-8">
          <div className="mx-auto w-full max-w-6xl">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
