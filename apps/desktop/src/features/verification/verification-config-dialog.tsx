import { zodResolver } from "@hookform/resolvers/zod";
import { Save, ShieldAlert, TerminalSquare } from "lucide-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { useSaveVerificationSpec, useVerificationTemplates } from "@/features/verification/queries";
import {
  argsToText,
  parseArgsText,
  verificationSpecSchema,
  type VerificationSpecValues,
} from "@/features/verification/schema";
import type { AcceptanceCriterion, Task, VerificationSpec } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

export function VerificationConfigDialog({
  open,
  onOpenChange,
  task,
  criterion,
  spec,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  task: Task;
  criterion: AcceptanceCriterion;
  spec: VerificationSpec | null;
}) {
  const templates = useVerificationTemplates();
  const saveSpec = useSaveVerificationSpec();
  const [templateId, setTemplateId] = useState("custom");

  const form = useForm<VerificationSpecValues>({
    resolver: zodResolver(verificationSpecSchema),
    defaultValues: {
      executable: spec?.executable ?? "",
      argsText: argsToText(spec?.args ?? []),
      cwd: spec?.cwd ?? "",
      timeoutSeconds: String(spec?.timeoutSeconds ?? 300),
      outputLimitBytes: String(spec?.outputLimitBytes ?? 65_536),
    },
  });

  const applyTemplate = (value: string) => {
    setTemplateId(value);
    const template = templates.data?.find((item) => item.id === value);
    if (!template) {
      return;
    }
    form.setValue("executable", template.executable, { shouldValidate: true });
    form.setValue("argsText", argsToText(template.args), { shouldValidate: true });
    form.setValue("timeoutSeconds", String(template.timeoutSeconds), {
      shouldValidate: true,
    });
  };

  const onSubmit = async (values: VerificationSpecValues) => {
    try {
      await saveSpec.mutateAsync({
        criterionId: criterion.id,
        expectedTaskRevision: task.revision,
        expectedCriterionRevision: criterion.revision,
        expectedSpecRevision: spec?.revision ?? null,
        executable: values.executable.trim(),
        args: parseArgsText(values.argsText),
        cwd: values.cwd.trim() ? values.cwd.trim() : null,
        timeoutSeconds: Number(values.timeoutSeconds),
        outputLimitBytes: Number(values.outputLimitBytes),
      });
      toast.success(spec ? "验证配置已更新" : "验证配置已保存", {
        description: "下一次运行将使用新的配置修订。",
      });
      onOpenChange(false);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>配置自动验收</DialogTitle>
          <DialogDescription>
            {criterion.description}
            <span className="ml-2 font-mono text-xs text-muted-foreground">
              r{criterion.revision}
            </span>
          </DialogDescription>
        </DialogHeader>

        <Alert variant="warning">
          <ShieldAlert className="mt-0.5 size-4" aria-hidden="true" />
          <AlertTitle>命令会以你的系统权限运行</AlertTitle>
          <AlertDescription>
            首版不经过 Shell，且拒绝 cmd、powershell、sh 与 .cmd/.bat/.ps1
            包装器；但测试脚本本身仍可能执行任意代码。请只对可信项目配置验证。
          </AlertDescription>
        </Alert>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <Label htmlFor="verification-template">从模板开始</Label>
            <Select value={templateId} onValueChange={applyTemplate}>
              <SelectTrigger id="verification-template">
                <SelectValue placeholder="选择模板或自定义" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="custom">自定义命令</SelectItem>
                {(templates.data ?? []).map((template) => (
                  <SelectItem key={template.id} value={template.id}>
                    {template.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs leading-5 text-muted-foreground">
              模板只填充字段，不会自动执行；保存后仍需在批准对话框中确认。
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="verification-executable">可执行程序</Label>
            <Input
              id="verification-executable"
              placeholder="例如 cargo、git、node 或绝对路径"
              autoComplete="off"
              spellCheck={false}
              {...form.register("executable")}
            />
            {form.formState.errors.executable ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.executable.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="verification-args">参数（每行一个）</Label>
            <Textarea
              id="verification-args"
              className="min-h-24 font-mono text-xs"
              placeholder={"test\n--workspace"}
              spellCheck={false}
              {...form.register("argsText")}
            />
            {form.formState.errors.argsText ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.argsText.message}
              </p>
            ) : (
              <p className="text-xs leading-5 text-muted-foreground">
                每个非空行会被视为一个独立参数，不会按空格拆分。
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="verification-cwd">工作目录（可选）</Label>
            <Input
              id="verification-cwd"
              placeholder="留空使用项目根目录"
              autoComplete="off"
              spellCheck={false}
              className="font-mono text-xs"
              {...form.register("cwd")}
            />
            {form.formState.errors.cwd ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.cwd.message}
              </p>
            ) : (
              <p className="text-xs leading-5 text-muted-foreground">
                必须是项目根目录或其子目录。
              </p>
            )}
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="verification-timeout">超时（秒）</Label>
              <Input
                id="verification-timeout"
                type="number"
                min={1}
                max={3600}
                {...form.register("timeoutSeconds")}
              />
              {form.formState.errors.timeoutSeconds ? (
                <p role="alert" className="text-xs text-destructive">
                  {form.formState.errors.timeoutSeconds.message}
                </p>
              ) : null}
            </div>
            <div className="space-y-2">
              <Label htmlFor="verification-output-limit">每个输出流上限（字节）</Label>
              <Input
                id="verification-output-limit"
                type="number"
                min={1024}
                max={1_048_576}
                {...form.register("outputLimitBytes")}
              />
              {form.formState.errors.outputLimitBytes ? (
                <p role="alert" className="text-xs text-destructive">
                  {form.formState.errors.outputLimitBytes.message}
                </p>
              ) : null}
            </div>
          </div>

          <div className="rounded-md border bg-surface-raised/40 p-3 text-xs leading-5 text-muted-foreground">
            <p className="flex items-center gap-1.5 font-medium text-foreground">
              <TerminalSquare className="size-3.5" aria-hidden="true" />
              成功规则
            </p>
            <p className="mt-1">
              退出码为 0 判定 PASS；非 0 判定 FAIL；超时判定 FAIL；取消或中断判定无法判定。
            </p>
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              取消
            </Button>
            <Button type="submit" disabled={saveSpec.isPending}>
              <Save aria-hidden="true" />
              {saveSpec.isPending ? "保存中…" : "保存验证配置"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
