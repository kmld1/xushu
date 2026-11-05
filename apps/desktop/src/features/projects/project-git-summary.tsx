import { GitBranch, GitCommitHorizontal, RefreshCw } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useProjectGitStatus } from "@/features/projects/queries";
import { shortHead } from "@/lib/format";
import { cn } from "@/lib/utils";

export function ProjectGitSummary({
  projectId,
  compact = false,
}: {
  projectId: string;
  compact?: boolean;
}) {
  const git = useProjectGitStatus(projectId);

  if (git.isPending) {
    return (
      <div className="flex flex-wrap items-center gap-2" aria-busy="true">
        <Skeleton className="h-5 w-24" />
        <Skeleton className="h-5 w-20" />
        <Skeleton className="h-5 w-16" />
      </div>
    );
  }

  if (git.isError) {
    return (
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="warning">Git 状态不可用</Badge>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => void git.refetch()}
          aria-label="重新检查 Git 状态"
        >
          <RefreshCw aria-hidden="true" />
          重试
        </Button>
      </div>
    );
  }

  return (
    <div className={cn("flex flex-wrap items-center gap-2", compact && "gap-1.5")}>
      <Badge variant="outline" className="gap-1 font-mono text-[11px]">
        <GitBranch className="size-3" aria-hidden="true" />
        {git.data.branch ?? "detached"}
      </Badge>
      <Badge variant="outline" className="gap-1 font-mono text-[11px]">
        <GitCommitHorizontal className="size-3" aria-hidden="true" />
        {shortHead(git.data.head)}
      </Badge>
      {git.data.dirty ? (
        <Badge variant="warning">有未提交改动</Badge>
      ) : (
        <Badge variant="success">工作区干净</Badge>
      )}
      {git.data.unborn ? <Badge variant="warning">尚无提交</Badge> : null}
      {git.data.detached ? <Badge variant="info">detached HEAD</Badge> : null}
      {!compact ? (
        <span className="text-[11px] text-muted-foreground">
          只读检查 · {new Date(git.data.checkedAt).toLocaleTimeString("zh-CN", { hour12: false })}
        </span>
      ) : null}
    </div>
  );
}
