import { Database } from "lucide-react";

import { ProbeForm } from "@/features/persistence/probe-form";
import { ProbeList } from "@/features/persistence/probe-list";
import { SystemHealthCard } from "@/features/system/system-health-card";

export function DiagnosticsPage() {
  return (
    <div className="space-y-5">
      <h1 className="sr-only">诊断</h1>

      <SystemHealthCard />

      <p className="flex items-start gap-2 border-l-2 border-primary bg-accent/25 px-3 py-2 text-[11px] leading-5 text-muted-foreground">
        <Database className="mt-0.5 size-3.5 shrink-0 text-primary" aria-hidden="true" />
        持久化探针不是业务数据。本页只创建带明确标识的测试记录，不包含项目、任务或验收数据；数据库文件不进入
        Git 仓库。
      </p>

      <div className="grid min-w-0 gap-6 border-t border-line/70 pt-5 xl:grid-cols-[minmax(0,380px)_minmax(0,1fr)]">
        <ProbeForm />
        <div className="xl:border-l xl:border-line/70 xl:pl-6">
          <ProbeList />
        </div>
      </div>
    </div>
  );
}
