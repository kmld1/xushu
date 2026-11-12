import { CircleCheck, CircleDot, GitBranch, GitCommitHorizontal, RefreshCw } from "lucide-react";

import { Skeleton } from "@/components/ui/skeleton";
import { useProjectGitStatus } from "@/features/projects/queries";
import { shortHead } from "@/lib/format";
import { cn } from "@/lib/utils";

export function ProjectGitSummary({
  projectId,
  compact = false,
  chrome = false,
}: {
  projectId: string;
  compact?: boolean;
  chrome?: boolean;
}) {
  const git = useProjectGitStatus(projectId);

  if (git.isPending) {
    return <Skeleton className={compact ? "h-4 w-40" : "h-5 w-56"} />;
  }

  if (git.isError) {
    return (
      <div className="flex items-center gap-2 text-[11px] text-warning">
        <span>Git 状态不可用</span>
        <button
          type="button"
          onClick={() => void git.refetch()}
          className="inline-flex items-center gap-1 underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <RefreshCw className="size-3" aria-hidden="true" />
          重试
        </button>
      </div>
    );
  }

  const statusText = git.data.dirty ? "工作区有未提交改动" : "工作区干净";

  if (compact) {
    return (
      <span
        className="inline-flex min-w-0 items-center gap-2 text-xs text-muted-foreground"
        title={`${git.data.branch ?? "detached HEAD"}\nCommit ${git.data.head ?? "尚无提交"}\n${statusText}`}
      >
        <GitBranch className="size-3 shrink-0" aria-hidden="true" />
        <span className="max-w-32 truncate font-mono">{git.data.branch ?? "detached HEAD"}</span>
        <span
          className={cn(
            "inline-flex shrink-0 items-center gap-1.5 font-mono",
            chrome && "max-[1279px]:hidden",
          )}
        >
          <GitCommitHorizontal className="size-3" aria-label="Commit" />
          {shortHead(git.data.head)}
        </span>
        <span
          className={cn(
            "inline-flex shrink-0 items-center gap-1",
            git.data.dirty ? "text-warning" : "text-muted-foreground",
          )}
          aria-label={statusText}
          title={statusText}
        >
          {git.data.dirty ? (
            <CircleDot className="size-3" aria-hidden="true" />
          ) : (
            <CircleCheck className="size-3" aria-hidden="true" />
          )}
          {git.data.dirty ? "有改动" : "干净"}
        </span>
      </span>
    );
  }

  return (
    <div className="space-y-2">
      <dl className="grid gap-x-6 gap-y-2 text-xs sm:grid-cols-2">
        <div className="flex items-center justify-between gap-3 border-b border-line/60 pb-2">
          <dt className="text-muted-foreground">分支</dt>
          <dd className="flex min-w-0 items-center gap-1.5 font-mono">
            <GitBranch className="size-3 shrink-0 text-muted-foreground" aria-hidden="true" />
            <span className="truncate">{git.data.branch ?? "detached HEAD"}</span>
          </dd>
        </div>
        <div className="flex items-center justify-between gap-3 border-b border-line/60 pb-2">
          <dt className="text-muted-foreground">HEAD</dt>
          <dd className="flex items-center gap-1.5 font-mono">
            <GitCommitHorizontal className="size-3 text-muted-foreground" aria-hidden="true" />
            {shortHead(git.data.head)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-3 border-b border-line/60 pb-2">
          <dt className="text-muted-foreground">工作区</dt>
          <dd className="flex items-center gap-1.5">
            <span
              className={cn("size-1.5 rounded-full", git.data.dirty ? "bg-warning" : "bg-success")}
              aria-hidden="true"
            />
            <span className={git.data.dirty ? "text-warning" : "text-success"}>{statusText}</span>
          </dd>
        </div>
        <div className="flex items-center justify-between gap-3 border-b border-line/60 pb-2">
          <dt className="text-muted-foreground">检查</dt>
          <dd className="font-mono text-[11px]">
            {new Date(git.data.checkedAt).toLocaleTimeString("zh-CN", { hour12: false })}
          </dd>
        </div>
      </dl>
      {git.data.unborn || git.data.detached ? (
        <p className="text-[11px] text-warning">
          {git.data.unborn ? "仓库尚无提交" : null}
          {git.data.unborn && git.data.detached ? " · " : null}
          {git.data.detached ? "detached HEAD" : null}
        </p>
      ) : null}
    </div>
  );
}
