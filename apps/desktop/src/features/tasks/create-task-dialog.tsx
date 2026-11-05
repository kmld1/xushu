import { zodResolver } from "@hookform/resolvers/zod";
import { ListPlus, Loader2 } from "lucide-react";
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
import { useCreateTask } from "@/features/tasks/queries";
import { createTaskSchema, TASK_PRIORITIES, type CreateTaskValues } from "@/features/tasks/schema";
import type { Task } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

export function CreateTaskDialog({
  open,
  onOpenChange,
  projectId,
  projectName,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string | null;
  projectName?: string;
  onCreated?: (task: Task) => void;
}) {
  const createTask = useCreateTask();
  const form = useForm<CreateTaskValues>({
    resolver: zodResolver(createTaskSchema),
    defaultValues: {
      title: "",
      description: "",
      priority: "medium",
    },
  });

  const onSubmit = async (values: CreateTaskValues) => {
    if (!projectId) {
      return;
    }

    try {
      const task = await createTask.mutateAsync({
        projectId,
        title: values.title,
        description: values.description.trim() ? values.description : undefined,
        priority: values.priority,
      });
      toast.success("任务已创建", { description: task.title });
      form.reset();
      onOpenChange(false);
      onCreated?.(task);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>新建任务</DialogTitle>
          <DialogDescription>
            {projectName
              ? `任务将创建在项目「${projectName}」中，初始状态为草稿。`
              : "请先在项目页面导入一个 Git 项目。"}
          </DialogDescription>
        </DialogHeader>

        {!projectId ? (
          <Alert variant="warning">
            <ListPlus className="mt-0.5 size-4" aria-hidden="true" />
            <AlertTitle>没有可用项目</AlertTitle>
            <AlertDescription>导入项目后才能创建任务。</AlertDescription>
          </Alert>
        ) : null}

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <Label htmlFor="new-task-title">标题</Label>
            <Input
              id="new-task-title"
              placeholder="用一句话说明要完成的工作"
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
            <Label htmlFor="new-task-description">描述</Label>
            <Textarea
              id="new-task-description"
              rows={4}
              placeholder="补充目标、范围或背景"
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
            <Label htmlFor="new-task-priority">优先级</Label>
            <Controller
              control={form.control}
              name="priority"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  <SelectTrigger id="new-task-priority">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {TASK_PRIORITIES.map((priority) => {
                      const meta = TASK_PRIORITY_META[priority];
                      return (
                        <SelectItem key={priority} value={priority}>
                          {meta.label}
                        </SelectItem>
                      );
                    })}
                  </SelectContent>
                </Select>
              )}
            />
          </div>

          {createTask.error ? (
            <Alert variant="destructive">
              <ListPlus className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>无法创建任务</AlertTitle>
              <AlertDescription>{getErrorMessage(createTask.error)}</AlertDescription>
            </Alert>
          ) : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={createTask.isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={createTask.isPending || !projectId}>
              {createTask.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <ListPlus aria-hidden="true" />
              )}
              {createTask.isPending ? "创建中…" : "创建任务"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
