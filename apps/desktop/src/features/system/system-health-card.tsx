import { AlertCircle, CheckCircle2, RefreshCw } from "lucide-react";
import { Link } from "react-router";

import { ErrorState } from "@/components/error-state";
import { Section, SectionHeader } from "@/components/section";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useDatabaseHealth } from "@/features/system/queries";
import { cn } from "@/lib/utils";

export function SystemHealthCard({ compact = false }: { compact?: boolean }) {
  const health = useDatabaseHealth();
  const ok = health.data?.ok ?? false;

  return (
    <Section>
      <SectionHeader
        title="系统"
        meta={health.data ? (ok ? "SQLite 正常" : "需要关注") : "读取中"}
        actions={
          compact ? (
            <Button asChild variant="ghost" size="sm">
              <Link to="/diagnostics">诊断</Link>
            </Button>
          ) : (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label="刷新系统健康状态"
              onClick={() => void health.refetch()}
              disabled={health.isFetching}
            >
              <RefreshCw
                className={cn("size-3.5", health.isFetching && "animate-spin")}
                aria-hidden="true"
              />
            </Button>
          )
        }
      />

      {health.isPending ? (
        <div className="space-y-2" aria-busy="true">
          <span role="status" className="sr-only">
            正在读取数据库健康状态
          </span>
          <Skeleton className="h-3 w-2/3" />
          <Skeleton className="h-3 w-1/2" />
          <Skeleton className="h-3 w-3/5" />
        </div>
      ) : health.isError ? (
        <ErrorState
          title="无法读取健康状态"
          message={health.error instanceof Error ? health.error.message : "本地数据库暂时不可用"}
          onRetry={() => void health.refetch()}
        />
      ) : (
        <div className="space-y-2">
          <div
            className={cn(
              "flex items-center gap-1.5 text-xs",
              ok ? "text-success" : "text-warning",
            )}
          >
            {ok ? (
              <CheckCircle2 className="size-3.5" aria-hidden="true" />
            ) : (
              <AlertCircle className="size-3.5" aria-hidden="true" />
            )}
            {ok ? "数据库完整性检查通过" : "数据库健康检查需要关注"}
          </div>
          <dl
            className={cn(
              "grid gap-x-5 gap-y-1.5 text-xs",
              compact ? "grid-cols-2" : "sm:grid-cols-2 lg:grid-cols-3",
            )}
          >
            {[
              ["数据库", health.data.databaseFile],
              ["Schema", `v${health.data.schemaVersion}`],
              ["日志模式", health.data.journalMode.toUpperCase()],
              ["外键", health.data.foreignKeysEnabled ? "开启" : "关闭"],
              ["完整性", health.data.integrityOk ? "通过" : "失败"],
              ["持久化探针", `${health.data.recordCount} 条`],
            ]
              .slice(0, compact ? 4 : undefined)
              .map(([label, value]) => (
                <div key={label} className="flex items-center justify-between gap-3">
                  <dt className="text-muted-foreground">{label}</dt>
                  <dd className="truncate font-mono text-[11px]" title={value}>
                    {value}
                  </dd>
                </div>
              ))}
          </dl>
        </div>
      )}
    </Section>
  );
}
