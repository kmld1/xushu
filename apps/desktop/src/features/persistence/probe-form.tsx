import { zodResolver } from "@hookform/resolvers/zod";
import { AlertCircle, CheckCircle2, Loader2, Plus } from "lucide-react";
import { useState } from "react";
import { useForm } from "react-hook-form";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { getErrorMessage } from "@/lib/ipc";

import { useCreateProbeRecord } from "./queries";
import { probeFormSchema, type ProbeFormValues } from "./schema";
import type { ProbeRecord } from "./types";

export function ProbeForm() {
  const createProbe = useCreateProbeRecord();
  const [lastCreated, setLastCreated] = useState<ProbeRecord | null>(null);
  const form = useForm<ProbeFormValues>({
    resolver: zodResolver(probeFormSchema),
    defaultValues: {
      label: "",
      content: "",
    },
  });

  const labelError = form.formState.errors.label?.message;
  const contentError = form.formState.errors.content?.message;
  const labelLength = form.watch("label").length;
  const contentLength = form.watch("content").length;

  const onSubmit = async (values: ProbeFormValues) => {
    setLastCreated(null);

    try {
      const record = await createProbe.mutateAsync(values);
      setLastCreated(record);
      form.reset();
    } catch {
      // 失败信息通过 createProbe.error 渲染，避免重复提示。
    }
  };

  const resetForm = () => {
    form.reset();
    setLastCreated(null);
    createProbe.reset();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>写入测试记录</CardTitle>
        <CardDescription>
          记录由 Rust 校验后写入 SQLite。它只用于 Phase 1 持久化验证，不进入 Git 仓库。
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form className="space-y-5" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <div className="flex items-center justify-between gap-2">
              <Label htmlFor="probe-label">标签</Label>
              <span className="text-xs text-muted-foreground">{labelLength}/64</span>
            </div>
            <Input
              id="probe-label"
              maxLength={64}
              placeholder="例如：跨进程验证"
              aria-invalid={Boolean(labelError)}
              aria-describedby={labelError ? "probe-label-error" : "probe-label-hint"}
              {...form.register("label")}
            />
            <p id="probe-label-hint" className="text-xs text-muted-foreground">
              1–64 个字符，不能包含控制字符。
            </p>
            {labelError ? (
              <p id="probe-label-error" role="alert" className="text-xs text-destructive">
                {labelError}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between gap-2">
              <Label htmlFor="probe-content">内容</Label>
              <span className="text-xs text-muted-foreground">{contentLength}/2000</span>
            </div>
            <Textarea
              id="probe-content"
              rows={6}
              maxLength={2000}
              placeholder="写入一条可识别的测试内容"
              aria-invalid={Boolean(contentError)}
              aria-describedby={contentError ? "probe-content-error" : "probe-content-hint"}
              {...form.register("content")}
            />
            <p id="probe-content-hint" className="text-xs text-muted-foreground">
              支持换行；最多 2000 个字符。
            </p>
            {contentError ? (
              <p id="probe-content-error" role="alert" className="text-xs text-destructive">
                {contentError}
              </p>
            ) : null}
          </div>

          {createProbe.error ? (
            <Alert variant="destructive">
              <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>写入失败</AlertTitle>
              <AlertDescription>{getErrorMessage(createProbe.error)}</AlertDescription>
            </Alert>
          ) : null}

          <div aria-live="polite">
            {lastCreated ? (
              <Alert className="border-emerald-200 bg-emerald-50 text-emerald-950 dark:border-emerald-900 dark:bg-emerald-950/50 dark:text-emerald-100">
                <CheckCircle2
                  className="mt-0.5 size-4 text-emerald-600 dark:text-emerald-300"
                  aria-hidden="true"
                />
                <AlertTitle>已写入 SQLite</AlertTitle>
                <AlertDescription>
                  <span className="block break-all font-mono text-xs">{lastCreated.id}</span>
                  <span className="mt-1 block">时间：{lastCreated.createdAt}</span>
                </AlertDescription>
              </Alert>
            ) : null}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button type="submit" disabled={createProbe.isPending}>
              {createProbe.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <Plus aria-hidden="true" />
              )}
              {createProbe.isPending ? "写入中…" : "写入测试记录"}
            </Button>
            <Button
              type="button"
              variant="ghost"
              onClick={resetForm}
              disabled={createProbe.isPending}
            >
              清空
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
