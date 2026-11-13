import { Brain, History, Scale } from "lucide-react";
import { Link } from "react-router";

import { Section, SectionHeader } from "@/components/section";
import { Button } from "@/components/ui/button";
import { DecisionStatusBadge, MemoryConfirmationBadge } from "./knowledge-badges";
import { categoryLabel } from "./knowledge-display";
import { useDecisions, useMemories } from "./queries";

export function TaskKnowledgeSection({ projectId, taskId }: { projectId: string; taskId: string }) {
  const memories = useMemories({
    projectId,
    taskId,
    includeArchived: false,
    category: null,
  });
  const decisions = useDecisions({ projectId, taskId, status: null });

  const memoryRows = memories.data ?? [];
  const decisionRows = decisions.data ?? [];
  const loading = memories.isPending || decisions.isPending;
  const failed = memories.isError || decisions.isError;

  return (
    <Section aria-label="相关决策与记忆">
      <SectionHeader
        title="相关决策与记忆"
        meta="由项目上下文关联"
        actions={
          <Button asChild variant="ghost" size="sm">
            <Link to={`/projects/${projectId}/knowledge?task=${taskId}&tab=decisions`}>
              <History aria-hidden="true" />
              查看项目上下文
            </Link>
          </Button>
        }
      />

      {loading ? (
        <p className="text-xs text-muted-foreground">正在读取相关记忆与决策…</p>
      ) : failed ? (
        <p className="text-xs text-destructive">相关记忆或决策暂时无法读取。</p>
      ) : memoryRows.length === 0 && decisionRows.length === 0 ? (
        <p className="text-xs leading-5 text-muted-foreground">
          尚未有记忆或决策关联到此任务。可在
          <Link
            to={`/projects/${projectId}/knowledge?task=${taskId}&tab=memory`}
            className="mx-1 text-foreground underline-offset-4 hover:text-primary hover:underline"
          >
            项目记忆与决策
          </Link>
          中建立关联；它们不会改变任务状态或验收门禁。
        </p>
      ) : (
        <div className="divide-y divide-line/60 border-y border-line/70">
          {memoryRows.slice(0, 3).map((row) => (
            <div key={row.memory.id} className="flex min-w-0 flex-wrap items-center gap-2 py-2">
              <Brain className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
              <span className="shrink-0 text-[11px] text-muted-foreground">
                {categoryLabel(row.memory.category)}
              </span>
              <span
                className="min-w-0 flex-1 truncate text-xs font-medium"
                title={row.memory.title}
              >
                {row.memory.title}
              </span>
              <MemoryConfirmationBadge status={row.memory.confirmationStatus} />
            </div>
          ))}
          {decisionRows.slice(0, 3).map((row) => (
            <div key={row.decision.id} className="flex min-w-0 flex-wrap items-center gap-2 py-2">
              <Scale className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
              <span
                className="min-w-0 flex-1 truncate text-xs font-medium"
                title={row.decision.title}
              >
                {row.decision.title}
              </span>
              <DecisionStatusBadge status={row.decision.status} />
            </div>
          ))}
          {(memoryRows.length > 3 || decisionRows.length > 3) && (
            <p className="py-2 text-[11px] text-muted-foreground">
              其余关联内容请在项目上下文中查看。
            </p>
          )}
        </div>
      )}
    </Section>
  );
}
