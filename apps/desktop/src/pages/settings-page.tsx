import {
  AlertCircle,
  Database,
  Monitor,
  Moon,
  ShieldCheck,
  Sun,
  type LucideIcon,
} from "lucide-react";

import { PageHeader } from "@/components/page-header";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useAppInfo, useDatabaseHealth } from "@/features/system/queries";
import { getErrorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useThemeStore, type ThemeMode } from "@/stores/theme-store";

const themeOptions: Array<{
  mode: ThemeMode;
  label: string;
  description: string;
  icon: LucideIcon;
}> = [
  { mode: "light", label: "浅色", description: "始终使用浅色界面", icon: Sun },
  { mode: "dark", label: "深色", description: "始终使用深色界面", icon: Moon },
  { mode: "system", label: "跟随系统", description: "跟随 Windows 主题切换", icon: Monitor },
];

function AppInfoCard() {
  const appInfo = useAppInfo();

  return (
    <Card>
      <CardHeader>
        <CardTitle>关于</CardTitle>
        <CardDescription>当前构建的真实版本信息。</CardDescription>
      </CardHeader>
      <CardContent>
        {appInfo.isPending ? (
          <p className="text-sm text-muted-foreground" role="status">
            正在读取应用信息…
          </p>
        ) : appInfo.isError ? (
          <Alert variant="destructive">
            <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
            <AlertTitle>无法读取应用信息</AlertTitle>
            <AlertDescription>{getErrorMessage(appInfo.error)}</AlertDescription>
          </Alert>
        ) : (
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
        )}
      </CardContent>
    </Card>
  );
}

function DataPrivacyCard() {
  const health = useDatabaseHealth();

  return (
    <Card>
      <CardHeader>
        <CardTitle>本地数据与隐私</CardTitle>
        <CardDescription>Phase 1 的数据库边界。</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 text-sm leading-6 text-muted-foreground">
        <div className="flex gap-3">
          <Database className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden="true" />
          <p>
            数据库文件：{" "}
            <span className="font-medium text-foreground">
              {health.data?.databaseFile ?? "读取中…"}
            </span>
            <br />
            存储位置：Windows 系统应用数据目录（不在项目仓库内）。
          </p>
        </div>
        <div className="flex gap-3">
          <ShieldCheck className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden="true" />
          <p>
            前端没有任意 SQL、Shell 命令或通用文件系统权限。所有数据库写入都经过 Rust
            侧输入校验和固定命令白名单。
          </p>
        </div>
        <p className="text-xs">本页不展示绝对路径、API Key、token 或其他凭据。V1 不包含云同步。</p>
      </CardContent>
    </Card>
  );
}

export function SettingsPage() {
  const mode = useThemeStore((state) => state.mode);
  const setMode = useThemeStore((state) => state.setMode);

  return (
    <div className="space-y-6">
      <PageHeader title="设置" description="管理界面主题并查看本地应用边界。当前设置保存在本机。" />

      <div className="grid items-start gap-6 xl:grid-cols-[1.2fr_1fr]">
        <Card>
          <CardHeader>
            <CardTitle>外观</CardTitle>
            <CardDescription>主题切换立即生效，并保存在本机浏览器存储中。</CardDescription>
          </CardHeader>
          <CardContent>
            <fieldset>
              <legend className="sr-only">选择主题</legend>
              <div className="grid gap-3 sm:grid-cols-3">
                {themeOptions.map((option) => {
                  const Icon = option.icon;
                  const active = mode === option.mode;

                  return (
                    <label
                      key={option.mode}
                      className={cn(
                        "relative flex cursor-pointer flex-col gap-2 rounded-lg border p-4 transition-colors",
                        "focus-within:ring-2 focus-within:ring-ring",
                        active ? "border-primary bg-accent/60" : "hover:bg-accent/40",
                      )}
                    >
                      <input
                        type="radio"
                        name="theme"
                        value={option.mode}
                        checked={active}
                        onChange={() => setMode(option.mode)}
                        className="sr-only"
                      />
                      <Icon className="size-5" aria-hidden="true" />
                      <span className="text-sm font-medium">{option.label}</span>
                      <span className="text-xs leading-5 text-muted-foreground">
                        {option.description}
                      </span>
                    </label>
                  );
                })}
              </div>
            </fieldset>
          </CardContent>
        </Card>

        <AppInfoCard />
      </div>

      <DataPrivacyCard />

      <div className="flex justify-end">
        <Button type="button" variant="outline" onClick={() => setMode("system")}>
          恢复跟随系统
        </Button>
      </div>
    </div>
  );
}
