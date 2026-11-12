import { Database, RefreshCw } from "lucide-react";

import { EmptyState } from "@/components/empty-state";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Section, SectionHeader } from "@/components/section";
import { Button } from "@/components/ui/button";
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
    <Section className="min-w-0">
      <SectionHeader
        title="最近写入"
        meta={probes.data ? `${probes.data.length} 条` : "最近 20 条"}
        actions={
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="刷新测试记录"
            onClick={() => void probes.refetch()}
            disabled={probes.isFetching}
          >
            <RefreshCw
              className={cn("size-3.5", probes.isFetching && "animate-spin")}
              aria-hidden="true"
            />
          </Button>
        }
      />

      {probes.isPending ? (
        <LoadingState rows={3} />
      ) : probes.isError ? (
        <ErrorState
          title="无法读取测试记录"
          message={getErrorMessage(probes.error)}
          onRetry={() => void probes.refetch()}
        />
      ) : probes.data.length === 0 ? (
        <EmptyState
          icon={Database}
          title="尚无测试记录"
          description="使用左侧表单写入第一条记录。完全关闭并重新打开 XUSHU 后，它仍应从 SQLite 中读回。"
        />
      ) : (
        <ul className="divide-y divide-line/70 border-y border-line/70">
          {probes.data.map((record) => (
            <li key={record.id} className="py-3">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div className="min-w-0">
                  <p className="break-words text-[13px] font-medium">{record.label}</p>
                  <p className="mt-0.5 text-[10px] text-muted-foreground">
                    ID <code className="break-all font-mono">{record.id}</code>
                  </p>
                </div>
                <time
                  className="shrink-0 font-mono text-[10px] text-muted-foreground"
                  dateTime={record.createdAt}
                  title={record.createdAt}
                >
                  {formatTimestamp(record.createdAt)}
                </time>
              </div>
              <p className="mt-2 whitespace-pre-wrap break-words text-xs leading-5 text-foreground/90">
                {record.content}
              </p>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
