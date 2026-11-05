import { AlertCircle, Database, RefreshCw } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";

import { useProbeRecords } from "./queries";

const timestampFormatter = new Intl.DateTimeFormat("zh-CN", {
  dateStyle: "medium",
  timeStyle: "medium",
  hour12: false,
});

function formatTimestamp(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : timestampFormatter.format(date);
}

export function ProbeList() {
  const probes = useProbeRecords();

  return (
    <Card className="min-w-0">
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <CardTitle>最近写入</CardTitle>
            <CardDescription>
              最多读取最近 20 条；时间来自 SQLite 中的 UTC RFC3339 字段。
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            {probes.data ? (
              <span className="text-xs text-muted-foreground">{probes.data.length} 条</span>
            ) : null}
            <Button
              type="button"
              variant="outline"
              size="icon"
              aria-label="刷新测试记录"
              onClick={() => void probes.refetch()}
              disabled={probes.isFetching}
            >
              <RefreshCw
                className={cn("size-4", probes.isFetching && "animate-spin")}
                aria-hidden="true"
              />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {probes.isPending ? (
          <div className="space-y-3" aria-busy="true">
            <span role="status" className="sr-only">
              正在加载测试记录
            </span>
            {[0, 1, 2].map((item) => (
              <div key={item} className="rounded-lg border p-4">
                <Skeleton className="h-4 w-1/3" />
                <Skeleton className="mt-3 h-3 w-2/3" />
                <Skeleton className="mt-2 h-3 w-1/2" />
              </div>
            ))}
          </div>
        ) : probes.isError ? (
          <Alert variant="destructive">
            <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
            <AlertTitle>无法读取测试记录</AlertTitle>
            <AlertDescription>
              <span className="block">{getErrorMessage(probes.error)}</span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="mt-3"
                onClick={() => void probes.refetch()}
              >
                重新加载
              </Button>
            </AlertDescription>
          </Alert>
        ) : probes.data.length === 0 ? (
          <div className="flex flex-col items-center justify-center rounded-lg border border-dashed px-6 py-12 text-center">
            <Database className="size-8 text-muted-foreground" aria-hidden="true" />
            <p className="mt-3 text-sm font-medium">尚无测试记录</p>
            <p className="mt-1 max-w-sm text-sm text-muted-foreground">
              使用左侧表单写入第一条记录。完全关闭并重新打开 XUSHU 后，它仍应从 SQLite 中读回。
            </p>
          </div>
        ) : (
          <ul className="space-y-3">
            {probes.data.map((record) => (
              <li key={record.id} className="rounded-lg border bg-background/60 p-4">
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div className="min-w-0">
                    <p className="break-words font-medium">{record.label}</p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      ID <code className="break-all font-mono">{record.id}</code>
                    </p>
                  </div>
                  <time
                    className="shrink-0 text-xs text-muted-foreground"
                    dateTime={record.createdAt}
                    title={record.createdAt}
                  >
                    {formatTimestamp(record.createdAt)}
                  </time>
                </div>
                <p className="mt-3 whitespace-pre-wrap break-words text-sm leading-6 text-foreground/90">
                  {record.content}
                </p>
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}
