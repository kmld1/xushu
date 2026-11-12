import {
  AlertCircle,
  Database,
  Monitor,
  Moon,
  ShieldCheck,
  Sun,
  type LucideIcon,
} from "lucide-react";

import { Section, SectionHeader } from "@/components/section";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
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

function DefinitionRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-4 border-b border-line/60 py-2 text-xs last:border-0">
      <dt className="shrink-0 text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-all text-right font-mono text-[11px]">{value}</dd>
    </div>
  );
}

function AboutSection() {
  const appInfo = useAppInfo();

  return (
    <Section className="border-t border-line/70 pt-5">
      <SectionHeader title="关于" />
      {appInfo.isPending ? (
        <p className="text-xs text-muted-foreground" role="status">
          正在读取应用信息…
        </p>
      ) : appInfo.isError ? (
        <Alert variant="destructive">
          <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>无法读取应用信息</AlertTitle>
          <AlertDescription>{getErrorMessage(appInfo.error)}</AlertDescription>
        </Alert>
      ) : (
        <dl className="border-t border-line/70">
          <DefinitionRow label="产品" value={appInfo.data.name} />
          <DefinitionRow label="应用版本" value={appInfo.data.version} />
          <DefinitionRow label="Tauri" value={appInfo.data.tauriVersion} />
          <DefinitionRow label="阶段" value={appInfo.data.phase} />
          <DefinitionRow label="平台" value={appInfo.data.platform} />
        </dl>
      )}
    </Section>
  );
}

function DataPrivacySection() {
  const health = useDatabaseHealth();

  return (
    <Section className="border-t border-line/70 pt-5">
      <SectionHeader title="本地数据与隐私" meta="最小权限" />
      <dl className="border-t border-line/70">
        <DefinitionRow label="数据库" value={health.data?.databaseFile ?? "读取中…"} />
        <DefinitionRow label="Schema" value={health.data ? `v${health.data.schemaVersion}` : "—"} />
        <DefinitionRow label="存储位置" value="Windows 应用数据目录" />
        <DefinitionRow label="网络" value="无云同步" />
        <DefinitionRow label="前端权限" value="固定 Tauri Commands" />
      </dl>
      <p className="flex items-start gap-2 pt-1 text-[11px] leading-5 text-muted-foreground">
        <ShieldCheck className="mt-0.5 size-3.5 shrink-0 text-primary" aria-hidden="true" />
        前端没有任意 SQL、Shell 或通用文件系统权限；Rust
        领域层负责校验、状态机与事务。本页不展示绝对路径、API Key 或 token。
      </p>
    </Section>
  );
}

export function SettingsPage() {
  const mode = useThemeStore((state) => state.mode);
  const setMode = useThemeStore((state) => state.setMode);

  return (
    <div className="max-w-3xl space-y-5">
      <h1 className="sr-only">设置</h1>

      <Section>
        <SectionHeader title="外观" meta="立即生效" />
        <fieldset>
          <legend className="sr-only">选择主题</legend>
          <div className="border-t border-line/70">
            {themeOptions.map((option) => {
              const Icon = option.icon;
              const active = mode === option.mode;

              return (
                <label
                  key={option.mode}
                  className={cn(
                    "flex cursor-pointer items-center gap-3 border-b border-line/60 px-1 py-2.5 transition-colors last:border-0",
                    "focus-within:ring-2 focus-within:ring-ring",
                    active ? "bg-accent/25" : "hover:bg-hover/60",
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
                  <span
                    className={cn(
                      "grid size-7 shrink-0 place-items-center rounded-md border",
                      active
                        ? "border-primary/50 bg-accent text-primary"
                        : "border-line text-muted-foreground",
                    )}
                    aria-hidden="true"
                  >
                    <Icon className="size-3.5" />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block text-xs font-medium">{option.label}</span>
                    <span className="block text-[11px] text-muted-foreground">
                      {option.description}
                    </span>
                  </span>
                  <span
                    className={cn(
                      "size-3 shrink-0 rounded-full border",
                      active ? "border-primary bg-primary" : "border-line-strong",
                    )}
                    aria-hidden="true"
                  />
                </label>
              );
            })}
          </div>
        </fieldset>
      </Section>

      <AboutSection />
      <DataPrivacySection />

      <div className="flex items-center gap-2 border-t border-line/70 pt-4 text-[11px] text-muted-foreground">
        <Database className="size-3.5" aria-hidden="true" />
        所有设置保存在本机；不修改项目仓库或全局开发工具配置。
      </div>
    </div>
  );
}
