import { Brain, History, Scale } from "lucide-react";
import { Link } from "react-router";

import { Section, SectionHeader } from "@/components/section";
import { Button } from "@/components/ui/button";
import type { Project } from "@/lib/domain";

import { useDecisions, useMemories } from "./queries";

export function ProjectKnowledgeSection({
  project,
  archived,
}: {
  project: Project;
  archived: boolean;
}) {
  const memories = useMemories({
    projectId: project.id,
    includeArchived: false,
    category: null,
    taskId: null,
  });
  const decisions = useDecisions({
    projectId: project.id,
    status: null,
    taskId: null,
  });

  const allMemories = memories.data ?? [];
  const allDecisions = decisions.data ?? [];
  const confirmedMemories = allMemories.filter(
    (memory) => memory.memory.confirmationStatus === "confirmed",
  );
  const unconfirmedMemories = allMemories.length - confirmedMemories.length;
  const confirmedDecisions = allDecisions.filter(
    (decision) => decision.decision.status === "confirmed",
  );
  const draftDecisions = allDecisions.filter((decision) => decision.decision.status === "draft");
  const recentMemory = confirmedMemories[0]?.memory;
  const recentDecision = confirmedDecisions[0]?.decision;
  const loading = memories.isPending || decisions.isPending;
  const failed = memories.isError || decisions.isError;

  return (
    <Section aria-label="项目记忆与决策" className="min-w-0 border-y border-line/70 py-3">
      <SectionHeader
        title="项目记忆与决策"
        meta="本地上下文 · 不改变任务门禁"
        actions={
          <Button asChild variant="ghost" size="sm">
            <Link to={`/projects/${project.id}/knowledge`}>
              <History aria-hidden="true" />
              打开记忆与决策
            </Link>
          </Button>
        }
      />

      {loading ? (
        <p className="text-xs text-muted-foreground">正在读取项目记忆与决策…</p>
      ) : failed ? (
        <p className="text-xs text-destructive">项目记忆或决策暂时无法读取。</p>
      ) : allMemories.length === 0 && allDecisions.length === 0 ? (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1.5">
            <Brain className="size-3.5" aria-hidden="true" />
            尚无项目记忆
          </span>
          <span className="inline-flex items-center gap-1.5">
            <Scale className="size-3.5" aria-hidden="true" />
            尚无开发决策
          </span>
          <span>
            {archived
              ? "项目已归档，记忆与决策只读。"
              : "记录目标、约束与关键取舍后，恢复包会自动纳入已确认内容。"}
          </span>
        </div>
      ) : (
        <div className="space-y-1.5 text-xs">
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-muted-foreground">
            <span className="inline-flex items-center gap-1.5">
              <Brain className="size-3.5" aria-hidden="true" />
              已确认记忆 {confirmedMemories.length}
              {unconfirmedMemories > 0 ? ` · 未确认 ${unconfirmedMemories}` : ""}
            </span>
            <span className="inline-flex items-center gap-1.5">
              <Scale className="size-3.5" aria-hidden="true" />
              已确认决策 {confirmedDecisions.length}
              {draftDecisions.length > 0 ? ` · 草稿 ${draftDecisions.length}` : ""}
            </span>
            {archived ? <span className="text-warning">项目已归档，只读</span> : null}
          </div>
          {recentMemory ? (
            <p className="min-w-0 truncate text-muted-foreground" title={recentMemory.content}>
              最近记忆：<span className="text-foreground">{recentMemory.title}</span>
            </p>
          ) : null}
          {recentDecision ? (
            <p className="min-w-0 truncate text-muted-foreground" title={recentDecision.decision}>
              最近决策：<span className="text-foreground">{recentDecision.title}</span>
            </p>
          ) : null}
        </div>
      )}
    </Section>
  );
}
