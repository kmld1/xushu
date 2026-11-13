import { zodResolver } from "@hookform/resolvers/zod";
import { BookOpen, Loader2 } from "lucide-react";
import { useEffect, useMemo } from "react";
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
import type { ProjectMemoryView, Task } from "@/lib/domain";
import { formatDateTime } from "@/lib/format";
import { getErrorMessage } from "@/lib/ipc";

import { MEMORY_CATEGORY_META } from "./knowledge-display";
import { useCreateMemory, useUpdateMemory } from "./queries";
import { MEMORY_CATEGORY_VALUES, memoryFormSchema, type MemoryFormValues } from "./schema";

const NO_TASK = "__none__";

export function MemoryFormDialog({
  open,
  onOpenChange,
  restoreFocus,
  projectId,
  projectName,
  tasks,
  memory,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  restoreFocus: () => void;
  projectId: string;
  projectName: string;
  tasks: Task[];
  memory?: ProjectMemoryView;
  onSaved?: () => void;
}) {
  const createMemory = useCreateMemory();
  const updateMemory = useUpdateMemory();
  const editing = Boolean(memory);
  const pending = createMemory.isPending || updateMemory.isPending;
  const form = useForm<MemoryFormValues>({
    resolver: zodResolver(memoryFormSchema),
    defaultValues: {
      category: "constraint",
      title: "",
      content: "",
      linkedTaskId: NO_TASK,
      changeNote: "",
    },
  });

  const initialValues = useMemo<MemoryFormValues>(() => {
    const value = memory?.memory;
    return {
      category: value?.category ?? "constraint",
      title: value?.title ?? "",
      content: value?.content ?? "",
      linkedTaskId: value?.linkedTaskId ?? NO_TASK,
      changeNote: "",
    };
  }, [memory]);

  useEffect(() => {
    if (!open) {
      return;
    }
    form.reset(initialValues);
  }, [open, initialValues, form]);

  const onSubmit = async (values: MemoryFormValues) => {
    const linkedTaskId =
      values.linkedTaskId && values.linkedTaskId !== NO_TASK ? values.linkedTaskId : null;
    try {
      if (memory) {
        await updateMemory.mutateAsync({
          projectId,
          memoryId: memory.memory.id,
          expectedRevision: memory.memory.revision,
          category: values.category,
          title: values.title,
          content: values.content,
          linkedTaskId,
          linkedEvidenceId: memory.memory.linkedEvidenceId,
          changeNote: values.changeNote?.trim() ? values.changeNote : undefined,
        });
        toast.success("记忆已更新", {
          description: "旧版本保留在修订历史中。",
        });
      } else {
        await createMemory.mutateAsync({
          projectId,
          category: values.category,
          title: values.title,
          content: values.content,
          linkedTaskId,
          sourceType: "user",
          confirmed: true,
        });
        toast.success("记忆已保存", {
          description: "该记忆已由你确认，可进入恢复包。",
        });
      }
      form.reset();
      onOpenChange(false);
      onSaved?.();
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-2xl"
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocus();
        }}
      >
        <DialogHeader>
          <DialogTitle>{editing ? "编辑项目记忆" : "新建项目记忆"}</DialogTitle>
          <DialogDescription>
            {projectName} · 记忆是开发上下文，不会改变任务状态、验收条件或完成门禁。
          </DialogDescription>
        </DialogHeader>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_220px]">
            <div className="space-y-2">
              <Label htmlFor="memory-title">标题</Label>
              <Input
                id="memory-title"
                placeholder="一句话概括这条记忆"
                aria-invalid={Boolean(form.formState.errors.title)}
                {...form.register("title")}
              />
              {form.formState.errors.title ? (
                <p role="alert" className="text-xs text-destructive">
                  {form.formState.errors.title.message}
                </p>
              ) : null}
            </div>
            <div className="space-y-2">
              <Label htmlFor="memory-category">类别</Label>
              <Controller
                control={form.control}
                name="category"
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger id="memory-category">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {MEMORY_CATEGORY_VALUES.map((category) => (
                        <SelectItem key={category} value={category}>
                          {MEMORY_CATEGORY_META[category].label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="memory-content">内容</Label>
            <Textarea
              id="memory-content"
              rows={8}
              placeholder="记录目标、约束、事实或风险；尽量写清适用范围。"
              aria-invalid={Boolean(form.formState.errors.content)}
              {...form.register("content")}
            />
            {form.formState.errors.content ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.content.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="memory-task">关联任务（可选）</Label>
            <Controller
              control={form.control}
              name="linkedTaskId"
              render={({ field }) => (
                <Select value={field.value ?? NO_TASK} onValueChange={field.onChange}>
                  <SelectTrigger id="memory-task">
                    <SelectValue placeholder="不关联任务" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={NO_TASK}>不关联任务</SelectItem>
                    {tasks.map((task) => (
                      <SelectItem key={task.id} value={task.id}>
                        {task.archivedAt ? `${task.title}（已归档）` : task.title}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            />
          </div>

          {editing ? (
            <div className="space-y-2">
              <Label htmlFor="memory-change-note">变更说明（可选）</Label>
              <Textarea
                id="memory-change-note"
                rows={2}
                placeholder="说明为什么修改这条记忆；会记录在修订历史中。"
                aria-invalid={Boolean(form.formState.errors.changeNote)}
                {...form.register("changeNote")}
              />
              {form.formState.errors.changeNote ? (
                <p role="alert" className="text-xs text-destructive">
                  {form.formState.errors.changeNote.message}
                </p>
              ) : null}
              <p className="text-[11px] leading-5 text-muted-foreground">
                {memory?.memory.confirmationStatus === "confirmed"
                  ? `这条记忆已确认于 ${formatDateTime(memory.memory.confirmedAt)}。保存后以本次人工编辑作为新的确认时间，旧版本仍可追溯。`
                  : "这条记忆尚未确认；保存修改不会自动确认。"}
              </p>
            </div>
          ) : (
            <p className="text-[11px] leading-5 text-muted-foreground">
              新建记忆默认标记为“已确认”，因为内容由你本人录入。AI 推断必须保持未确认并人工复核。
            </p>
          )}

          {createMemory.error || updateMemory.error ? (
            <Alert variant="destructive">
              <BookOpen className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>无法保存记忆</AlertTitle>
              <AlertDescription>
                {getErrorMessage(createMemory.error ?? updateMemory.error)}
              </AlertDescription>
            </Alert>
          ) : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={pending}
            >
              取消
            </Button>
            <Button type="submit" disabled={pending}>
              {pending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <BookOpen aria-hidden="true" />
              )}
              {pending ? "保存中…" : editing ? "保存修改" : "保存记忆"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
