import { zodResolver } from "@hookform/resolvers/zod";
import { Loader2 } from "lucide-react";
import { useForm } from "react-hook-form";

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
import { reasonSchema, type ReasonValues } from "@/features/tasks/schema";
import { getErrorMessage } from "@/lib/ipc";

export function ReasonDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  pending,
  error,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirmLabel: string;
  pending: boolean;
  error: unknown;
  onConfirm: (reason: string) => Promise<void>;
}) {
  const form = useForm<ReasonValues>({
    resolver: zodResolver(reasonSchema),
    defaultValues: { reason: "" },
  });

  const onSubmit = async (values: ReasonValues) => {
    try {
      await onConfirm(values.reason);
      form.reset();
      onOpenChange(false);
    } catch {
      // 错误由调用方的 mutation.error 展示。
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="space-y-2">
            <Label htmlFor="reason-value">原因</Label>
            <Textarea
              id="reason-value"
              rows={3}
              placeholder="说明为什么进行这次状态变更"
              aria-invalid={Boolean(form.formState.errors.reason)}
              {...form.register("reason")}
            />
            {form.formState.errors.reason ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.reason.message}
              </p>
            ) : null}
          </div>

          {error ? (
            <Alert variant="destructive">
              <AlertTitle>操作失败</AlertTitle>
              <AlertDescription>{getErrorMessage(error)}</AlertDescription>
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
              {pending ? <Loader2 className="animate-spin" aria-hidden="true" /> : null}
              {pending ? "处理中…" : confirmLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
