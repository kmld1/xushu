import {
  Database,
  FolderGit2,
  LayoutDashboard,
  ListChecks,
  Settings,
  ShieldCheck,
} from "lucide-react";
import { Link, NavLink, Outlet } from "react-router";

import { ThemeControl } from "@/components/theme-control";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { ProjectGitSummary } from "@/features/projects/project-git-summary";
import { useAppInfo } from "@/features/system/queries";
import { useCurrentProject } from "@/features/workspace/use-current-project";
import { cn } from "@/lib/utils";
import { useWorkspaceStore } from "@/stores/workspace-store";

const navigation = [
  { to: "/", label: "工作台", icon: LayoutDashboard, end: true },
  { to: "/projects", label: "项目", icon: FolderGit2, end: false },
  { to: "/tasks", label: "任务", icon: ListChecks, end: false },
  { to: "/verification", label: "验收中心", icon: ShieldCheck, end: false },
  { to: "/diagnostics", label: "诊断", icon: Database, end: false },
  { to: "/settings", label: "设置", icon: Settings, end: false },
];

export function AppLayout() {
  const appInfo = useAppInfo();
  const { activeProjects, currentProject, isPending, isError } = useCurrentProject();
  const setSelectedProjectId = useWorkspaceStore((state) => state.setSelectedProjectId);

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

      <aside className="flex w-[252px] shrink-0 flex-col border-r bg-card">
        <div className="flex h-14 items-center gap-2.5 border-b px-4">
          <div
            className="grid size-8 place-items-center rounded-md bg-primary text-sm font-semibold text-primary-foreground"
            aria-hidden="true"
          >
            序
          </div>
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold leading-none">序枢 XUSHU</p>
            <p className="mt-1 truncate text-[11px] text-muted-foreground">Phase 3 · 独立验收</p>
          </div>
        </div>

        <div className="border-b px-3 py-3">
          <p className="px-2 pb-1.5 text-[11px] text-muted-foreground">当前项目</p>
          {isPending ? (
            <Skeleton className="h-8 w-full" />
          ) : isError ? (
            <p className="px-2 text-xs text-destructive">项目列表不可用</p>
          ) : activeProjects.length === 0 ? (
            <Button asChild variant="outline" size="sm" className="w-full justify-start">
              <Link to="/projects">
                <FolderGit2 aria-hidden="true" />
                导入 Git 项目
              </Link>
            </Button>
          ) : (
            <Select
              value={currentProject?.id ?? ""}
              onValueChange={(value) => setSelectedProjectId(value)}
            >
              <SelectTrigger className="h-8" aria-label="切换当前项目">
                <SelectValue placeholder="选择项目" />
              </SelectTrigger>
              <SelectContent>
                {activeProjects.map((project) => (
                  <SelectItem key={project.id} value={project.id}>
                    {project.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </div>

        <nav aria-label="主导航" className="flex-1 space-y-0.5 p-2">
          {navigation.map((item) => {
            const Icon = item.icon;

            return (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) =>
                  cn(
                    "flex items-center gap-2.5 rounded-md border-l-2 px-3 py-2 text-sm font-medium transition-colors",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    isActive
                      ? "border-primary bg-accent text-accent-foreground"
                      : "border-transparent text-muted-foreground hover:bg-surface-raised hover:text-foreground",
                  )
                }
              >
                <Icon className="size-4" aria-hidden="true" />
                {item.label}
              </NavLink>
            );
          })}
        </nav>

        <div className="border-t px-4 py-3 text-[11px] leading-5 text-muted-foreground">
          <p>本地优先 · 无云同步</p>
          <p>
            {appInfo.isPending
              ? "版本读取中…"
              : appInfo.data
                ? `v${appInfo.data.version}`
                : "版本不可用"}
          </p>
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center justify-between gap-4 border-b px-5 lg:px-6">
          <div className="flex min-w-0 items-center gap-3">
            <p className="truncate text-sm font-medium">{currentProject?.name ?? "尚未选择项目"}</p>
            {currentProject ? <ProjectGitSummary projectId={currentProject.id} compact /> : null}
          </div>
          <ThemeControl />
        </header>

        <main id="main-content" tabIndex={-1} className="flex-1 p-5 lg:p-8">
          <div className="mx-auto w-full max-w-[1240px]">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
