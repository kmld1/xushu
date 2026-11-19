import {
  Database,
  DatabaseBackup,
  FolderGit2,
  History,
  LayoutDashboard,
  ListChecks,
  Settings,
  ShieldCheck,
} from "lucide-react";
import { NavLink, Outlet } from "react-router";

import { WorkspaceChrome } from "@/app/workspace-chrome";
import { useAppInfo } from "@/features/system/queries";
import { useWorkspaceSummary } from "@/features/tasks/queries";
import { cn } from "@/lib/utils";

const navigation = [
  { to: "/", label: "工作台", icon: LayoutDashboard, end: true, badge: null },
  { to: "/projects", label: "项目", icon: FolderGit2, end: false, badge: null },
  { to: "/tasks", label: "任务", icon: ListChecks, end: false, badge: "blocked" },
  {
    to: "/verification",
    label: "验收中心",
    icon: ShieldCheck,
    end: false,
    badge: "awaiting",
  },
  { to: "/recovery", label: "恢复", icon: History, end: false, badge: null },
  { to: "/data-protection", label: "数据保护", icon: DatabaseBackup, end: false, badge: null },
  { to: "/diagnostics", label: "诊断", icon: Database, end: false, badge: null },
  { to: "/settings", label: "设置", icon: Settings, end: false, badge: null },
] as const;

const SIDEBAR_WIDTH = "w-[240px] max-[759px]:w-[52px]";

export function AppLayout() {
  const appInfo = useAppInfo();
  const summary = useWorkspaceSummary();

  const blockedCount = summary.data?.taskCounts.blocked ?? 0;
  const awaitingCount = summary.data?.taskCounts.awaitingVerification ?? 0;

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
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

      <WorkspaceChrome platform={appInfo.data?.platform} />

      <div className="flex min-h-0 flex-1">
        <aside
          className={`${SIDEBAR_WIDTH} flex h-full shrink-0 flex-col border-r border-line bg-sidebar`}
        >
          <nav aria-label="主导航" className="min-h-0 flex-1 overflow-y-auto px-2 py-2.5">
            <div className="space-y-0.5">
              {navigation.map((item) => {
                const Icon = item.icon;
                const badge =
                  item.badge === "blocked"
                    ? blockedCount
                    : item.badge === "awaiting"
                      ? awaitingCount
                      : 0;

                return (
                  <NavLink
                    key={item.to}
                    to={item.to}
                    end={item.end}
                    className={({ isActive }) =>
                      cn(
                        "group relative flex h-8 items-center gap-2.5 rounded-md border-l-2 px-2.5 text-[13px] font-medium transition-colors",
                        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                        "max-[759px]:justify-center max-[759px]:px-0",
                        isActive
                          ? "border-primary bg-workspace text-foreground"
                          : "border-transparent text-muted-foreground hover:bg-workspace/60 hover:text-foreground",
                      )
                    }
                  >
                    {({ isActive }) => (
                      <>
                        <Icon
                          className={cn(
                            "size-4 shrink-0",
                            isActive
                              ? "text-primary"
                              : "text-muted-foreground group-hover:text-foreground",
                          )}
                          aria-hidden="true"
                        />
                        <span className="flex-1 truncate max-[759px]:sr-only">{item.label}</span>
                        {badge > 0 ? (
                          <>
                            <span
                              className={cn(
                                "font-mono text-[10px] tabular-nums max-[759px]:hidden",
                                item.badge === "blocked" ? "text-destructive" : "text-warning",
                              )}
                              aria-label={`${badge} 项`}
                            >
                              {badge}
                            </span>
                            <span
                              className={cn(
                                "absolute right-1.5 top-1.5 hidden size-1.5 rounded-full max-[759px]:block",
                                item.badge === "blocked" ? "bg-destructive" : "bg-warning",
                              )}
                              aria-label={`${badge} 项`}
                            />
                          </>
                        ) : null}
                      </>
                    )}
                  </NavLink>
                );
              })}
            </div>
          </nav>

          <div className="shrink-0 border-t border-line/70 px-3 py-2.5 max-[759px]:hidden">
            <div className="flex items-center justify-between gap-2 text-[10px] text-muted-foreground">
              <span>本地 · 无云同步</span>
              <span className="font-mono">
                {appInfo.isPending ? "…" : appInfo.data ? `v${appInfo.data.version}` : "—"}
              </span>
            </div>
          </div>
        </aside>

        <div className="flex min-w-0 flex-1 flex-col bg-workspace">
          <main
            id="main-content"
            tabIndex={-1}
            className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden outline-none [scrollbar-gutter:stable]"
          >
            <div className="mx-auto w-full max-w-[1280px] px-5 pb-8 pt-4 max-[599px]:px-3 lg:px-8">
              <Outlet />
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
