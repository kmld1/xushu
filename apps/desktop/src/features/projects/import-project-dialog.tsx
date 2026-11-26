import { zodResolver } from "@hookform/resolvers/zod";
import { FolderGit2, Loader2 } from "lucide-react";
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
import { useImportProject } from "@/features/projects/queries";
import { importProjectSchema, type ImportProjectValues } from "@/features/projects/schema";
import type { Project } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

export function ImportProjectDialog({
  open,
  onOpenChange,
  onImported,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImported?: (project: Project) => void;
}) {
  const importProject = useImportProject();
  const form = useForm<ImportProjectValues>({
    resolver: zodResolver(importProjectSchema),
    defaultValues: { rootPath: "", name: "" },
  });

  const onSubmit = async (values: ImportProjectValues) => {
    try {
      const project = await importProject.mutateAsync({
        rootPath: values.rootPath,
        name: values.name?.trim() ? values.name.trim() : undefined,
      });
      toast.success("项目已导入", { description: project.name });
      form.reset();
      onOpenChange(false);
      onImported?.(project);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>导入 Git 项目</DialogTitle>
          <DialogDescription>
            输入本地 Git 工作树路径。XUSHU 只做只读检查，不会修改仓库文件或执行 Git 写操作。
          </DialogDescription>
        </DialogHeader>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <Label htmlFor="project-root-path">项目路径</Label>
            <Input
              id="project-root-path"
              placeholder="项目根目录绝对路径"
              autoComplete="off"
              spellCheck={false}
              aria-invalid={Boolean(form.formState.errors.rootPath)}
              aria-describedby={
                form.formState.errors.rootPath
                  ? "project-root-path-error"
                  : "project-root-path-hint"
              }
              {...form.register("rootPath")}
            />
            <p id="project-root-path-hint" className="text-xs text-muted-foreground">
              必须是已存在的 Git 工作树；可以是仓库子目录，XUSHU 会解析到 Git 根目录。
            </p>
            {form.formState.errors.rootPath ? (
              <p id="project-root-path-error" role="alert" className="text-xs text-destructive">
                {form.formState.errors.rootPath.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="project-name">项目名称（可选）</Label>
            <Input
              id="project-name"
              placeholder="默认使用目录名"
              autoComplete="off"
              aria-invalid={Boolean(form.formState.errors.name)}
              aria-describedby={form.formState.errors.name ? "project-name-error" : undefined}
              {...form.register("name")}
            />
            {form.formState.errors.name ? (
              <p id="project-name-error" role="alert" className="text-xs text-destructive">
                {form.formState.errors.name.message}
              </p>
            ) : null}
          </div>

          {importProject.error ? (
            <Alert variant="destructive">
              <FolderGit2 className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>无法导入项目</AlertTitle>
              <AlertDescription>{getErrorMessage(importProject.error)}</AlertDescription>
            </Alert>
          ) : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={importProject.isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={importProject.isPending}>
              {importProject.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <FolderGit2 aria-hidden="true" />
              )}
              {importProject.isPending ? "检查并导入…" : "检查并导入"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
