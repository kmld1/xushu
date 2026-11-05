import {
  Activity,
  AlertCircle,
  CheckCircle2,
  Database,
  Layers3,
  RefreshCw,
  ShieldCheck,
  XCircle,
} from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router";

import { PageHeader } from "@/components/page-header";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { HealthBadge } from "@/features/system/health-badge";
import { useAppInfo, useDatabaseHealth } from "@/features/system/queries";
import type { DatabaseHealth } from "@/features/system/types";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";

const phaseOneScope = [
  {
    title: "SQLite 持久化",
    description: "版本化迁移、事务、外键、WAL 与完整性检查。",
    icon: Database,
  },
  {
    title: "受控 Tauri IPC",
    description: "应用信息、健康状态、写入、列表与按 ID 读取。",
    icon: ShieldCheck,
  },
  {
    title: "React 桌面外壳",
    description: "路由、深浅主题、错误边界、加载与空状态。",
    icon: Layers3,
  },
  {
    title: "跨进程验证",
    description: "关闭应用后重新启动，仍能从 SQLite 读回原始测试记录。",
    icon: Activity,
  },
];

function HealthValue({ ok, children }: { ok: boolean; children: ReactNode }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      {ok ? (
        <CheckCircle2
          className="size-3.5 text-emerald-600 dark:text-emerald-300"
          aria-hidden="true"
        />
      ) : (
        <XCircle className="size-3.5 text-destructive" aria-hidden="true" />
      )}
      {children}
    </span>
  );
}

function HealthItem({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="rounded-lg border bg-background/60 p-3">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="mt-1 text-sm font-medium">{value}</dd>
    </div>
  );
}

function HealthDetails({ health }: { health: DatabaseHealth }) {
  return (
    <div className="space-y-4">
      {health.foreignKeyViolations ? (
        <Alert variant="warning">
          <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>检测到外键约束异常</AlertTitle>
          <AlertDescription>
            外键检查返回了违规记录，请先停止写入并检查本地数据库。
          </AlertDescription>
        </Alert>
      ) : null}

      <dl className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <HealthItem label="数据库文件" value={health.databaseFile} />
        <HealthItem label="Schema 版本" value={`v${health.schemaVersion}`} />
        <HealthItem label="日志模式" value={health.journalMode.toUpperCase()} />
        <HealthItem
          label="外键约束"
          value={
            <HealthValue ok={health.foreignKeysEnabled}>
              {health.foreignKeysEnabled ? "已开启" : "未开启"}
            </HealthValue>
          }
        />
        <HealthItem
          label="完整性检查"
          value={
            <HealthValue ok={health.integrityOk}>
              {health.integrityOk ? "通过" : "失败"}
            </HealthValue>
          }
        />
        <HealthItem label="测试记录" value={`${health.recordCount} 条`} />
      </dl>
    </div>
  );
}

function HealthSkeleton() {
  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3" aria-busy="true">
      <span role="status" className="sr-only">
        正在读取数据库健康状态
      </span>
      {[0, 1, 2, 3, 4, 5].map((item) => (
        <div key={item} className="rounded-lg border p-3">
          <Skeleton className="h-3 w-16" />
          <Skeleton className="mt-2 h-4 w-24" />
        </div>
      ))}
    </div>
  );
}

function AppInfoDetails() {
  const appInfo = useAppInfo();

  if (appInfo.isPending) {
    return (
      <div className="space-y-3" aria-busy="true">
        <span role="status" className="sr-only">
          正在读取应用信息
        </span>
        {[0, 1, 2, 3, 4].map((item) => (
          <Skeleton key={item} className="h-4 w-full" />
        ))}
      </div>
    );
  }

  if (appInfo.isError) {
    return (
      <Alert variant="destructive">
        <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
        <AlertTitle>无法读取应用信息</AlertTitle>
        <AlertDescription>
          <span className="block">{getErrorMessage(appInfo.error)}</span>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="mt-3"
            onClick={() => void appInfo.refetch()}
          >
            重试
          </Button>
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <dl className="space-y-3 text-sm">
      {[
        ["产品", appInfo.data.name],
        ["应用版本", appInfo.data.version],
        ["Tauri", appInfo.data.tauriVersion],
        ["阶段", appInfo.data.phase],
        ["平台", appInfo.data.platform],
      ].map(([label, value]) => (
        <div
          key={label}
          className="flex items-center justify-between gap-4 border-b pb-2 last:border-0"
        >
          <dt className="text-muted-foreground">{label}</dt>
          <dd className="text-right font-medium">{value}</dd>
        </div>
      ))}
    </dl>
  );
}

export function OverviewPage() {
  const health = useDatabaseHealth();

  return (
    <div className="space-y-6">
      <PageHeader
        title="概览"
        description="查看本地数据库、运行环境与 Phase 1 已实际落地的能力。所有状态均来自真实后端查询。"
        actions={
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void health.refetch()}
            disabled={health.isFetching}
          >
            <RefreshCw
              className={cn("size-4", health.isFetching && "animate-spin")}
              aria-hidden="true"
            />
            刷新状态
          </Button>
        }
      />

      <div className="grid gap-6 xl:grid-cols-[1.6fr_1fr]">
        <Card>
          <CardHeader>
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <CardTitle>系统健康</CardTitle>
                <CardDescription>
                  Rust 后端对本地 SQLite 执行的真实查询，不是前端静态占位数据。
                </CardDescription>
              </div>
              {health.data ? <HealthBadge ok={health.data.ok} /> : null}
            </div>
          </CardHeader>
          <CardContent>
            {health.isPending ? (
              <HealthSkeleton />
            ) : health.isError ? (
              <Alert variant="destructive">
                <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
                <AlertTitle>无法读取数据库健康状态</AlertTitle>
                <AlertDescription>
                  <span className="block">{getErrorMessage(health.error)}</span>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="mt-3"
                    onClick={() => void health.refetch()}
                  >
                    重新检查
                  </Button>
                </AlertDescription>
              </Alert>
            ) : (
              <HealthDetails health={health.data} />
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>应用信息</CardTitle>
            <CardDescription>构建时与运行时的真实版本信息。</CardDescription>
          </CardHeader>
          <CardContent>
            <AppInfoDetails />
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Phase 1 已实现范围</CardTitle>
          <CardDescription>只列出已经落地的能力，不代表 Phase 2 业务功能。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <ul className="grid gap-3 sm:grid-cols-2">
            {phaseOneScope.map((item) => {
              const Icon = item.icon;
              return (
                <li key={item.title} className="flex gap-3 rounded-lg border bg-background/60 p-3">
                  <Icon className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden="true" />
                  <div>
                    <p className="text-sm font-medium">{item.title}</p>
                    <p className="mt-0.5 text-xs leading-5 text-muted-foreground">
                      {item.description}
                    </p>
                  </div>
                </li>
              );
            })}
          </ul>
          <Button asChild variant="outline" size="sm">
            <Link to="/persistence">打开本地数据测试</Link>
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
