import { zodResolver } from "@hookform/resolvers/zod";
import { CheckCircle2, Loader2 } from "lucide-react";
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
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { useConfirmManualCriterion } from "@/features/tasks/queries";
import { confirmationNoteSchema, type ConfirmationNoteValues } from "@/features/tasks/schema";
import type { CriterionView, Task } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

export function ConfirmManualDialog({
  open,
  onOpenChange,
  task,
  criterionView,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  task: Task;
  criterionView: CriterionView;
}) {
  const confirmManual = useConfirmManualCriterion();
  const form = useForm<ConfirmationNoteValues>({
    resolver: zodResolver(confirmationNoteSchema),
    defaultValues: { note: "" },
  });

  const onSubmit = async (values: ConfirmationNoteValues) => {
    try {
      await confirmManual.mutateAsync({
        criterionId: criterionView.criterion.id,
        expectedTaskRevision: task.revision,
        expectedCriterionRevision: criterionView.criterion.revision,
        note: values.note?.trim() ? values.note : undefined,
      });
      toast.success("人工验收已确认", {
        description: criterionView.criterion.description,
      });
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
          <DialogTitle>确认人工验收</DialogTitle>
          <DialogDescription>
            确认会绑定当前任务修订 {task.revision} 与条件修订 {criterionView.criterion.revision}。
            之后编辑任务或条件，本次确认将失效。
          </DialogDescription>
        </DialogHeader>

        <div className="rounded-md border bg-surface-raised/40 p-3 text-sm">
          {criterionView.criterion.description}
        </div>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <Label htmlFor="confirm-note">备注（可选）</Label>
            <Textarea
              id="confirm-note"
              rows={3}
              placeholder="记录你依据什么确认通过"
              {...form.register("note")}
            />
            {form.formState.errors.note ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.note.message}
              </p>
            ) : null}
          </div>

          {confirmManual.error ? (
            <Alert variant="destructive">
              <AlertTitle>无法确认</AlertTitle>
              <AlertDescription>{getErrorMessage(confirmManual.error)}</AlertDescription>
            </Alert>
          ) : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={confirmManual.isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={confirmManual.isPending}>
              {confirmManual.isPending ? (
                <Loader2 className="animate-spin" aria-hidden="true" />
              ) : (
                <CheckCircle2 aria-hidden="true" />
              )}
              {confirmManual.isPending ? "确认中…" : "确认通过"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
