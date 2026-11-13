import {
  Database,
  FileArchive,
  KeyRound,
  LockKeyhole,
  RefreshCw,
  ShieldCheck,
  ShieldOff,
  UserRoundCheck,
} from "lucide-react";

import type { DataAsset, DataProtectionOverview } from "@/lib/domain";
import { cn } from "@/lib/utils";

import { formatSize } from "./data-protection-display";

function AssetState({ asset }: { asset: DataAsset }) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px]">
      <span
        className={cn(
          "inline-flex items-center gap-1",
          asset.includedInBackup ? "text-success" : "text-muted-foreground",
        )}
      >
        {asset.includedInBackup ? (
          <ShieldCheck className="size-3" aria-hidden="true" />
        ) : (
          <ShieldOff className="size-3" aria-hidden="true" />
        )}
        {asset.includedInBackup ? "进入备份" : "不在备份"}
      </span>
      {asset.sensitive ? (
        <span className="inline-flex items-center gap-1 text-warning">
          <LockKeyhole className="size-3" aria-hidden="true" />
          可能敏感
        </span>
      ) : null}
      <span className="inline-flex items-center gap-1 text-muted-foreground">
        {asset.authoritative ? (
          <Database className="size-3" aria-hidden="true" />
        ) : (
          <FileArchive className="size-3" aria-hidden="true" />
        )}
        {asset.authoritative ? "权威数据" : "派生或观察"}
      </span>
      <span className="inline-flex items-center gap-1 text-muted-foreground">
        {asset.regenerable ? (
          <RefreshCw className="size-3" aria-hidden="true" />
        ) : (
          <ShieldOff className="size-3" aria-hidden="true" />
        )}
        {asset.regenerable ? "可重建" : "不可重建"}
      </span>
    </div>
  );
}

export function PrivacyInventory({ assets }: { assets: DataAsset[] }) {
  return (
    <ul aria-label="本地数据资产" className="divide-y divide-line/60 border-y border-line/70">
      {assets.map((asset) => (
        <li key={asset.id} className="py-2.5">
          <div className="flex flex-wrap items-start gap-x-3 gap-y-1">
            <div className="min-w-0 flex-1">
              <p className="text-xs font-medium">{asset.label}</p>
              <p className="break-all font-mono text-[10px] text-muted-foreground">
                {asset.location}
              </p>
              <p className="mt-0.5 text-[11px] text-muted-foreground">
                {asset.countLabel}
                {asset.sizeBytes !== null ? ` · ${formatSize(asset.sizeBytes)}` : ""}
              </p>
            </div>
            <AssetState asset={asset} />
          </div>
          <details className="mt-1 text-[11px] leading-5 text-muted-foreground">
            <summary className="cursor-pointer select-none">保留、丢失影响与导出</summary>
            <dl className="mt-1 grid gap-x-4 gap-y-1.5 sm:grid-cols-2">
              <div>
                <dt className="font-medium text-foreground/80">保留</dt>
                <dd>{asset.retention}</dd>
              </div>
              <div>
                <dt className="font-medium text-foreground/80">丢失影响</dt>
                <dd>{asset.impactIfLost}</dd>
              </div>
              <div>
                <dt className="font-medium text-foreground/80">导出</dt>
                <dd>{asset.exportNote}</dd>
              </div>
              <div>
                <dt className="font-medium text-foreground/80">说明</dt>
                <dd>{asset.notes}</dd>
              </div>
            </dl>
          </details>
        </li>
      ))}
    </ul>
  );
}

export function HooksBoundary({ overview }: { overview: DataProtectionOverview }) {
  const { hooks } = overview;
  return (
    <div className="space-y-3">
      <div className="grid gap-x-6 gap-y-3 border-y border-line/70 py-3 md:grid-cols-2">
        <section aria-label="会采集的 Hook 字段">
          <h3 className="flex items-center gap-1.5 text-[11px] font-medium text-success">
            <UserRoundCheck className="size-3.5" aria-hidden="true" />
            会采集（白名单元数据）
          </h3>
          <ul className="mt-1.5 space-y-1 text-[11px] leading-5 text-muted-foreground">
            {hooks.collectedFields.map((field) => (
              <li key={field}>{field}</li>
            ))}
          </ul>
        </section>
        <section aria-label="不会采集的 Hook 字段">
          <h3 className="flex items-center gap-1.5 text-[11px] font-medium text-destructive">
            <ShieldOff className="size-3.5" aria-hidden="true" />
            不会采集
          </h3>
          <ul className="mt-1.5 space-y-1 text-[11px] leading-5 text-muted-foreground">
            {hooks.excludedFields.map((field) => (
              <li key={field}>{field}</li>
            ))}
          </ul>
        </section>
      </div>
      <dl className="grid gap-x-4 gap-y-1.5 text-[11px] sm:grid-cols-2">
        <div className="flex justify-between gap-3">
          <dt className="text-muted-foreground">已导入事件 / 会话</dt>
          <dd className="font-mono">
            {hooks.eventCount} / {hooks.sessionCount}
          </dd>
        </div>
        <div className="flex justify-between gap-3">
          <dt className="text-muted-foreground">待导入 Spool</dt>
          <dd className="font-mono">
            {hooks.pendingSpoolEvents} 条 · {formatSize(hooks.pendingSpoolBytes)}
          </dd>
        </div>
        <div className="flex justify-between gap-3">
          <dt className="flex items-center gap-1 text-muted-foreground">
            <KeyRound className="size-3" aria-hidden="true" />
            伪名化 Salt
          </dt>
          <dd className="font-mono">
            {hooks.saltPresent ? `存在 · ${formatSize(hooks.saltBytes)}` : "不存在"}
          </dd>
        </div>
        <div className="flex justify-between gap-3">
          <dt className="text-muted-foreground">保留策略</dt>
          <dd className="font-mono">
            {hooks.retentionDays} 天 / {hooks.maxEventsPerProject} 条每项目
          </dd>
        </div>
        <div className="flex justify-between gap-3">
          <dt className="text-muted-foreground">期望启用接入的项目</dt>
          <dd className="font-mono">{hooks.desiredEnabledProjectCount}</dd>
        </div>
        <div className="flex justify-between gap-3">
          <dt className="text-muted-foreground">接收器状态</dt>
          <dd>{hooks.inboxAvailable ? "可用" : "不可用"}</dd>
        </div>
      </dl>
      <p className="text-[11px] leading-5 text-muted-foreground">{hooks.note}</p>
    </div>
  );
}
