import { zodResolver } from "@hookform/resolvers/zod";
import { ListChecks, Loader2 } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
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
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { useCreateCriterion, useUpdateCriterion } from "@/features/tasks/queries";
import { criterionSchema, CRITERION_KINDS, type CriterionValues } from "@/features/tasks/schema";
import type { AcceptanceCriterion, Task } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

const KIND_LABELS: Record<(typeof CRITERION_KINDS)[number], string> = {
  manual: "人工确认",
  auto: "自动验证（Phase 3）",
};

export function CriterionDialog({
  open,
  onOpenChange,
  task,
  criterion,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  task: Task;
  criterion?: AcceptanceCriterion;
}) {
  const createCriterion = useCreateCriterion();
  const updateCriterion = useUpdateCriterion();
  const editing = Boolean(criterion);
  const mutation = editing ? updateCriterion : createCriterion;
  const form = useForm<CriterionValues>({
    resolver: zodResolver(criterionSchema),
    defaultValues: {
      description: criterion?.description ?? "",
      kind: criterion?.kind ?? "manual",
      required: criterion?.required ?? true,
    },
  });
  const kind = form.watch("kind");

  const onSubmit = async (values: CriterionValues) => {
    try {
      if (criterion) {
        await updateCriterion.mutateAsync({
          criterionId: criterion.id,
          expectedTaskRevision: task.revision,
          expectedCriterionRevision: criterion.revision,
          description: values.description,
          kind: values.kind,
          required: values.required,
        });
        toast.success("验收条件已更新", {
          description: "任务修订号增加，旧的人工确认已失效。",
        });
      } else {
        await createCriterion.mutateAsync({
          taskId: task.id,
          expectedTaskRevision: task.revision,
          description: values.description,
          kind: values.kind,
          required: values.required,
        });
        toast.success("验收条件已添加");
      }
      form.reset();
      onOpenChange(false);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{editing ? "编辑验收条件" : "添加验收条件"}</DialogTitle>
          <DialogDescription>
            验收条件与当前任务修订号绑定。修改任务或条件后，旧的人工确认会失效。
          </DialogDescription>
        </DialogHeader>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <Label htmlFor="criterion-description">条件描述</Label>
            <Textarea
              id="criterion-description"
              rows={3}
              placeholder="写清楚什么结果算通过"
              aria-invalid={Boolean(form.formState.errors.description)}
              {...form.register("description")}
            />
            {form.formState.errors.description ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.description.message}
              </p>
            ) : null}
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="criterion-kind">类型</Label>
              <Controller
                control={form.control}
                name="kind"
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger id="criterion-kind">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {CRITERION_KINDS.map((value) => (
                        <SelectItem key={value} value={value}>
                          {KIND_LABELS[value]}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
            </div>

            <label className="flex items-start gap-2 rounded-md border bg-surface-raised/40 p-3 text-sm">
              <input
                type="checkbox"
                className="mt-0.5 size-4 accent-primary"
                aria-label="必需验收条件"
                {...form.register("required")}
              />
              <span>
                <span className="block font-medium">必需条件</span>
                <span className="mt-0.5 block text-xs text-muted-foreground">
                  所有必需条件满足后才能标记完成。
                </span>
              </span>
            </label>
          </div>

          {kind === "auto" ? (
            <Alert variant="warning">
              <ListChecks className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>Phase 2 尚无自动验证引擎</AlertTitle>
              <AlertDescription>
                必需自动条件不会产生 PASS 证据，因此会阻止任务完成。Phase 3 接入验证运行器后再使用。
              </AlertDescription>
            </Alert>
          ) : null}

          {mutation.error ? (
            <Alert variant="destructive">
              <AlertTitle>无法保存验收条件</AlertTitle>
              <AlertDescription>{getErrorMessage(mutation.error)}</AlertDescription>
            </Alert>
          ) : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={mutation.isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={mutation.isPending}>
              {mutation.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <ListChecks aria-hidden="true" />
              )}
              {mutation.isPending ? "保存中…" : editing ? "保存条件" : "创建条件"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
