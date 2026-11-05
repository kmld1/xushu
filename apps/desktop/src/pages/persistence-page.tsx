import { Database } from "lucide-react";

import { PageHeader } from "@/components/page-header";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { ProbeForm } from "@/features/persistence/probe-form";
import { ProbeList } from "@/features/persistence/probe-list";

export function PersistencePage() {
  return (
    <div className="space-y-6">
      <PageHeader
        title="本地数据"
        description="写入并读取 Phase 1 的持久化测试记录，用于验证 Rust、SQLite 与 Tauri IPC 的完整链路。"
      />

      <Alert>
        <Database className="mt-0.5 size-4" aria-hidden="true" />
        <AlertTitle>这不是业务数据</AlertTitle>
        <AlertDescription>
          本页只创建带明确标识的测试记录，不包含项目、任务或验收数据。数据库文件位于系统应用数据目录，
          不进入 Git 仓库，界面也不展示绝对路径。
        </AlertDescription>
      </Alert>

      <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,420px)_1fr]">
        <ProbeForm />
        <ProbeList />
      </div>
    </div>
  );
}
