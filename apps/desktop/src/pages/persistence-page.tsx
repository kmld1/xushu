import { Database } from "lucide-react";

import { PageHeader } from "@/components/page-header";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { ProbeForm } from "@/features/persistence/probe-form";
import { ProbeList } from "@/features/persistence/probe-list";
import { SystemHealthCard } from "@/features/system/system-health-card";

export function DiagnosticsPage() {
  return (
    <div className="space-y-6">
      <PageHeader
        title="本地数据与系统健康"
        description="检查 SQLite、迁移、WAL 与外键状态，并验证 Phase 1 的持久化探针。"
      />

      <SystemHealthCard />

      <Alert>
        <Database className="mt-0.5 size-4" aria-hidden="true" />
        <AlertTitle>持久化探针不是业务数据</AlertTitle>
        <AlertDescription>
          本页只创建带明确标识的测试记录，不包含项目、任务或验收数据。数据库文件位于系统应用数据目录，
          不进入 Git 仓库，界面也不展示绝对路径。
        </AlertDescription>
      </Alert>

      <div className="grid items-start gap-5 xl:grid-cols-[minmax(0,420px)_1fr]">
        <ProbeForm />
        <ProbeList />
      </div>
    </div>
  );
}
