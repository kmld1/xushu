import { AlertCircle, Database } from "lucide-react";

import { ErrorState } from "@/components/error-state";
import { HealthBadge } from "@/features/system/health-badge";
import { useDatabaseHealth } from "@/features/system/queries";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";

export function SystemHealthCard({ compact = false }: { compact?: boolean }) {
  const health = useDatabaseHealth();

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Database className="size-4 text-primary" aria-hidden="true" />
              系统健康
            </CardTitle>
            <CardDescription>来自 Rust 后端的真实 SQLite 查询。</CardDescription>
          </div>
          {health.data ? <HealthBadge ok={health.data.ok} /> : null}
        </div>
      </CardHeader>
      <CardContent>
        {health.isPending ? (
          <div className="grid gap-3 sm:grid-cols-3" aria-busy="true">
            <span role="status" className="sr-only">
              正在读取数据库健康状态
            </span>
            {[0, 1, 2, 3, 4, 5].map((item) => (
              <div key={item} className="rounded-md border p-3">
                <Skeleton className="h-3 w-16" />
                <Skeleton className="mt-2 h-4 w-20" />
              </div>
            ))}
          </div>
        ) : health.isError ? (
          <ErrorState
            title="无法读取健康状态"
            message={health.error instanceof Error ? health.error.message : "本地数据库暂时不可用"}
            onRetry={() => void health.refetch()}
          />
        ) : (
          <div className={compact ? "grid gap-2 sm:grid-cols-3" : "grid gap-3 sm:grid-cols-3"}>
            {[
              ["数据库", health.data.databaseFile],
              ["Schema", `v${health.data.schemaVersion}`],
              ["日志模式", health.data.journalMode.toUpperCase()],
              ["外键约束", health.data.foreignKeysEnabled ? "已开启" : "未开启"],
              ["完整性", health.data.integrityOk ? "通过" : "失败"],
              ["持久化探针", `${health.data.recordCount} 条`],
            ].map(([label, value]) => (
              <div key={label} className="rounded-md border bg-surface-raised/40 p-3">
                <p className="text-xs text-muted-foreground">{label}</p>
                <p className="mt-1 truncate text-sm font-medium" title={value}>
                  {value}
                </p>
              </div>
            ))}
          </div>
        )}
        {health.isError ? (
          <div className="mt-3">
            <Button type="button" variant="ghost" size="sm" onClick={() => void health.refetch()}>
              <AlertCircle aria-hidden="true" />
              重试
            </Button>
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}
