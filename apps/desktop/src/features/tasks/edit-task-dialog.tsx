import { zodResolver } from "@hookform/resolvers/zod";
import { Loader2, Save } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { toast } from "sonner";

import { TASK_PRIORITY_META } from "@/lib/task-display";
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
import { useUpdateTask } from "@/features/tasks/queries";
import { editTaskSchema, TASK_PRIORITIES, type EditTaskValues } from "@/features/tasks/schema";
import type { Task } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

export function EditTaskDialog({
  open,
  onOpenChange,
  task,
  onUpdated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  task: Task;
  onUpdated?: () => void;
}) {
  const updateTask = useUpdateTask();
  const form = useForm<EditTaskValues>({
    resolver: zodResolver(editTaskSchema),
    defaultValues: {
      title: task.title,
      description: task.description,
      priority: task.priority,
    },
  });

  const onSubmit = async (values: EditTaskValues) => {
    try {
      await updateTask.mutateAsync({
        taskId: task.id,
        expectedRevision: task.revision,
        title: values.title,
        description: values.description.trim() ? values.description : undefined,
        priority: values.priority,
      });
      toast.success("任务已更新", {
        description: "旧的人工确认会因修订号变化而失效。",
      });
      onOpenChange(false);
      onUpdated?.();
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>编辑任务</DialogTitle>
          <DialogDescription>
            保存后任务修订号增加，已有的人工确认不再适用于新修订。
          </DialogDescription>
        </DialogHeader>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <Label htmlFor="edit-task-title">标题</Label>
            <Input
              id="edit-task-title"
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
            <Label htmlFor="edit-task-description">描述</Label>
            <Textarea
              id="edit-task-description"
              rows={5}
              aria-invalid={Boolean(form.formState.errors.description)}
              {...form.register("description")}
            />
            {form.formState.errors.description ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.description.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-task-priority">优先级</Label>
            <Controller
              control={form.control}
              name="priority"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  <SelectTrigger id="edit-task-priority">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {TASK_PRIORITIES.map((priority) => (
                      <SelectItem key={priority} value={priority}>
                        {TASK_PRIORITY_META[priority].label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            />
          </div>

          {updateTask.error ? (
            <Alert variant="destructive">
              <AlertTitle>无法保存任务</AlertTitle>
              <AlertDescription>{getErrorMessage(updateTask.error)}</AlertDescription>
            </Alert>
          ) : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={updateTask.isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={updateTask.isPending}>
              {updateTask.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <Save aria-hidden="true" />
              )}
              {updateTask.isPending ? "保存中…" : "保存修改"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
