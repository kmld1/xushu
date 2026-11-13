import { ExternalLink, FolderGit2 } from "lucide-react";
import { Link, useLocation } from "react-router";

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
import { WindowControls } from "@/components/window-controls";
import { ProjectGitSummary } from "@/features/projects/project-git-summary";
import { useCurrentProject } from "@/features/workspace/use-current-project";
import { useWorkspaceStore } from "@/stores/workspace-store";

const SIDEBAR_WIDTH = "w-[240px] max-[759px]:w-[52px]";

function pageTitle(pathname: string) {
  if (pathname === "/") {
    return "工作台";
  }
  if (pathname.startsWith("/projects/") && pathname.endsWith("/knowledge")) {
    return "记忆与决策";
  }
  if (pathname === "/projects" || pathname.startsWith("/projects/")) {
    return "项目";
  }
  if (pathname === "/tasks" || pathname.startsWith("/tasks/")) {
    return "任务";
  }
  if (pathname === "/verification" || pathname.startsWith("/verification/")) {
    return "验收中心";
  }
  if (pathname === "/recovery" || pathname.startsWith("/recovery/")) {
    return "恢复";
  }
  if (pathname === "/data-protection" || pathname.startsWith("/data-protection/")) {
    return "数据保护";
  }
  if (pathname === "/diagnostics" || pathname.startsWith("/diagnostics/")) {
    return "诊断";
  }
  if (pathname === "/settings" || pathname.startsWith("/settings/")) {
    return "设置";
  }
  return "未找到";
}

function ProjectContext() {
  const { activeProjects, currentProject, isPending, isError } = useCurrentProject();
  const setSelectedProjectId = useWorkspaceStore((state) => state.setSelectedProjectId);

  return (
    <div className="flex min-w-0 max-w-[520px] flex-1 items-center gap-2 min-[1100px]:gap-3">
      <span className="sr-only">当前项目</span>

      {isPending ? (
        <Skeleton className="h-5 w-36" />
      ) : isError ? (
        <span className="text-[11px] text-destructive">项目列表不可用</span>
      ) : activeProjects.length === 0 ? (
        <Button asChild variant="ghost" size="sm" className="h-7 px-1.5">
          <Link to="/projects">
            <FolderGit2 aria-hidden="true" />
            导入项目
          </Link>
        </Button>
      ) : (
        <>
          <Select
            value={currentProject?.id ?? ""}
            onValueChange={(value) => setSelectedProjectId(value)}
          >
            <SelectTrigger
              className="h-7 min-w-0 max-w-[220px] border-0 bg-transparent px-1.5 text-sm font-medium shadow-none transition-colors duration-120 hover:bg-hover focus-visible:ring-1 max-[759px]:max-w-[160px]"
              aria-label="切换当前项目"
              title={
                currentProject ? `${currentProject.name}\n${currentProject.rootPath}` : "选择项目"
              }
            >
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

          {currentProject ? (
            <>
              <div className="hidden min-w-0 shrink-0 min-[1100px]:flex">
                <ProjectGitSummary projectId={currentProject.id} compact chrome />
              </div>
              <Link
                to={`/projects/${currentProject.id}`}
                aria-label="打开当前项目详情"
                title="打开项目详情"
                className="grid size-7 shrink-0 place-items-center rounded-sm text-muted-foreground transition-colors duration-120 hover:bg-hover hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring max-[879px]:hidden"
              >
                <ExternalLink className="size-3" aria-hidden="true" />
              </Link>
            </>
          ) : null}
        </>
      )}
    </div>
  );
}

export function WorkspaceChrome() {
  const { pathname } = useLocation();
  const title = pageTitle(pathname);

  return (
    <header data-tauri-drag-region="deep" className="flex h-11 shrink-0 select-none bg-workspace">
      <Link
        to="/"
        aria-label="序枢 XUSHU 工作台"
        className={`${SIDEBAR_WIDTH} flex shrink-0 items-center gap-2.5 border-r border-line bg-sidebar px-3.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring max-[759px]:justify-center max-[759px]:px-0`}
      >
        <span
          className="grid size-7 shrink-0 place-items-center rounded-[5px] bg-primary text-[13px] font-semibold text-primary-foreground"
          aria-hidden="true"
        >
          序
        </span>
        <span className="truncate text-[13px] font-semibold tracking-[-0.01em] max-[759px]:sr-only">
          序枢 XUSHU
        </span>
      </Link>

      <div className="flex min-w-0 flex-1 items-center gap-3 border-b border-line/60 pl-3.5 pr-0 max-[759px]:gap-2 max-[759px]:pl-2.5">
        <ProjectContext />
        <span className="h-3.5 w-px shrink-0 bg-line" aria-hidden="true" />
        <p className="shrink-0 text-sm font-semibold tracking-[-0.01em]">{title}</p>
        <div className="ml-auto flex shrink-0 items-center">
          <ThemeControl />
        </div>
        <WindowControls />
      </div>
    </header>
  );
}
