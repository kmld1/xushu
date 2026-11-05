import { zodResolver } from "@hookform/resolvers/zod";
import { Loader2, Save } from "lucide-react";
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
import { useUpdateProject } from "@/features/projects/queries";
import { renameProjectSchema, type RenameProjectValues } from "@/features/projects/schema";
import type { Project } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

export function RenameProjectDialog({
  open,
  onOpenChange,
  project,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  project: Project;
}) {
  const updateProject = useUpdateProject();
  const form = useForm<RenameProjectValues>({
    resolver: zodResolver(renameProjectSchema),
    defaultValues: { name: project.name },
  });

  const onSubmit = async (values: RenameProjectValues) => {
    try {
      await updateProject.mutateAsync({
        projectId: project.id,
        name: values.name,
      });
      toast.success("项目已重命名", { description: values.name });
      onOpenChange(false);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>重命名项目</DialogTitle>
          <DialogDescription>
            只修改 XUSHU 中的项目显示名称，不修改磁盘目录或 Git 配置。
          </DialogDescription>
        </DialogHeader>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <Label htmlFor="rename-project-name">项目名称</Label>
            <Input
              id="rename-project-name"
              aria-invalid={Boolean(form.formState.errors.name)}
              {...form.register("name")}
            />
            {form.formState.errors.name ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.name.message}
              </p>
            ) : null}
          </div>

          {updateProject.error ? (
            <Alert variant="destructive">
              <AlertTitle>无法重命名项目</AlertTitle>
              <AlertDescription>{getErrorMessage(updateProject.error)}</AlertDescription>
            </Alert>
          ) : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={updateProject.isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={updateProject.isPending}>
              {updateProject.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <Save aria-hidden="true" />
              )}
              {updateProject.isPending ? "保存中…" : "保存名称"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
