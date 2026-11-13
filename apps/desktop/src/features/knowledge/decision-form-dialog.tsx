import { zodResolver } from "@hookform/resolvers/zod";
import { Loader2, Scale } from "lucide-react";
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
import type { DecisionView, Task } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

import { useCreateDecision, useUpdateDecision } from "./queries";
import {
  alternativesFromText,
  alternativesToText,
  decisionFormSchema,
  type DecisionFormValues,
} from "./schema";

const NO_TASK = "__none__";
const NO_DECISION = "__none__";

export function DecisionFormDialog({
  open,
  onOpenChange,
  restoreFocus,
  projectId,
  projectName,
  tasks,
  supersededCandidates,
  decision,
  initialSupersedesId,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  restoreFocus: () => void;
  projectId: string;
  projectName: string;
  tasks: Task[];
  supersededCandidates: DecisionView[];
  decision?: DecisionView;
  initialSupersedesId?: string;
  onSaved?: () => void;
}) {
  const createDecision = useCreateDecision();
  const updateDecision = useUpdateDecision();
  const editing = Boolean(decision);
  const pending = createDecision.isPending || updateDecision.isPending;
  const form = useForm<DecisionFormValues>({
    resolver: zodResolver(decisionFormSchema),
    defaultValues: {
      title: "",
      taskId: NO_TASK,
      problem: "",
      context: "",
      alternativesText: "",
      decision: "",
      rationale: "",
      risks: "",
      supersedesId: NO_DECISION,
      changeNote: "",
    },
  });

  const initialValues = useMemo<DecisionFormValues>(() => {
    const value = decision?.decision;
    return {
      title: value?.title ?? "",
      taskId: value?.taskId ?? NO_TASK,
      problem: value?.problem ?? "",
      context: value?.context ?? "",
      alternativesText: alternativesToText(value?.alternatives ?? []),
      decision: value?.decision ?? "",
      rationale: value?.rationale ?? "",
      risks: value?.risks ?? "",
      supersedesId: value?.supersedesId ?? initialSupersedesId ?? NO_DECISION,
      changeNote: "",
    };
  }, [decision, initialSupersedesId]);

  useEffect(() => {
    if (!open) {
      return;
    }
    form.reset(initialValues);
  }, [open, initialValues, form]);

  const onSubmit = async (values: DecisionFormValues) => {
    const taskId = values.taskId && values.taskId !== NO_TASK ? values.taskId : null;
    const supersedesId =
      values.supersedesId && values.supersedesId !== NO_DECISION ? values.supersedesId : null;
    try {
      if (decision) {
        await updateDecision.mutateAsync({
          projectId,
          decisionId: decision.decision.id,
          expectedRevision: decision.decision.revision,
          taskId,
          title: values.title,
          problem: values.problem,
          context: values.context,
          alternatives: alternativesFromText(values.alternativesText),
          decision: values.decision,
          rationale: values.rationale,
          risks: values.risks,
          supersedesId,
          changeNote: values.changeNote?.trim() ? values.changeNote : undefined,
        });
        toast.success("决策草稿已更新", {
          description: "确认后才会进入恢复包。",
        });
      } else {
        await createDecision.mutateAsync({
          projectId,
          taskId,
          title: values.title,
          problem: values.problem,
          context: values.context,
          alternatives: alternativesFromText(values.alternativesText),
          decision: values.decision,
          rationale: values.rationale,
          risks: values.risks,
          supersedesId,
        });
        toast.success("决策已保存为草稿", {
          description: supersedesId
            ? "确认新决策后，旧决策才会被标记为已被取代。"
            : "确认后才会进入恢复包。",
        });
      }
      form.reset();
      onOpenChange(false);
      onSaved?.();
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  const candidates = supersededCandidates.filter(
    (candidate) => candidate.decision.id !== decision?.decision.id,
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-3xl"
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocus();
        }}
      >
        <DialogHeader>
          <DialogTitle>{editing ? "编辑决策草稿" : "新建开发决策"}</DialogTitle>
          <DialogDescription>
            {projectName} ·
            记录为什么采用当前方案、放弃了什么，以及仍然有效的代价。决策不会修改任务状态。
          </DialogDescription>
        </DialogHeader>

        <form className="space-y-4" noValidate onSubmit={form.handleSubmit(onSubmit)}>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="decision-title">标题</Label>
              <Input
                id="decision-title"
                placeholder="例如：使用 SQLite 作为本地持久化"
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
              <Label htmlFor="decision-task">关联任务（可选）</Label>
              <Controller
                control={form.control}
                name="taskId"
                render={({ field }) => (
                  <Select value={field.value ?? NO_TASK} onValueChange={field.onChange}>
                    <SelectTrigger id="decision-task">
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
          </div>

          <div className="space-y-2">
            <Label htmlFor="decision-problem">要解决的问题</Label>
            <Textarea
              id="decision-problem"
              rows={3}
              placeholder="当时必须解决什么问题？"
              aria-invalid={Boolean(form.formState.errors.problem)}
              {...form.register("problem")}
            />
            {form.formState.errors.problem ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.problem.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="decision-context">背景与约束</Label>
            <Textarea
              id="decision-context"
              rows={4}
              placeholder="技术、时间、安全、兼容性或团队约束"
              aria-invalid={Boolean(form.formState.errors.context)}
              {...form.register("context")}
            />
            {form.formState.errors.context ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.context.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="decision-alternatives">备选方案（每行一个）</Label>
            <Textarea
              id="decision-alternatives"
              rows={4}
              placeholder={"例如：\nSQLite\nJSON 文件\n远程数据库"}
              aria-invalid={Boolean(form.formState.errors.alternativesText)}
              {...form.register("alternativesText")}
            />
            {form.formState.errors.alternativesText ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.alternativesText.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="decision-final">最终采用的方案</Label>
            <Textarea
              id="decision-final"
              rows={3}
              placeholder="明确写出最终选择，而不是泛泛描述"
              aria-invalid={Boolean(form.formState.errors.decision)}
              {...form.register("decision")}
            />
            {form.formState.errors.decision ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.decision.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="decision-rationale">采用该方案的原因</Label>
            <Textarea
              id="decision-rationale"
              rows={3}
              placeholder="说明取舍依据，以及为什么没有选择其他方案"
              aria-invalid={Boolean(form.formState.errors.rationale)}
              {...form.register("rationale")}
            />
            {form.formState.errors.rationale ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.rationale.message}
              </p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="decision-risks">已知代价与风险</Label>
            <Textarea
              id="decision-risks"
              rows={3}
              placeholder="哪些限制仍然有效？未来需要关注什么？"
              aria-invalid={Boolean(form.formState.errors.risks)}
              {...form.register("risks")}
            />
            {form.formState.errors.risks ? (
              <p role="alert" className="text-xs text-destructive">
                {form.formState.errors.risks.message}
              </p>
            ) : null}
          </div>

          {candidates.length > 0 ? (
            <div className="space-y-2">
              <Label htmlFor="decision-supersedes">取代已确认决策（可选）</Label>
              <Controller
                control={form.control}
                name="supersedesId"
                render={({ field }) => (
                  <Select value={field.value ?? NO_DECISION} onValueChange={field.onChange}>
                    <SelectTrigger id="decision-supersedes">
                      <SelectValue placeholder="不取代其他决策" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={NO_DECISION}>不取代其他决策</SelectItem>
                      {candidates.map((candidate) => (
                        <SelectItem key={candidate.decision.id} value={candidate.decision.id}>
                          {candidate.decision.title}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
              <p className="text-[11px] leading-5 text-muted-foreground">
                旧决策会保持“已确认”，直到你确认这条新决策；确认后旧决策变为“已被取代”，并双向保留引用。
              </p>
            </div>
          ) : null}

          {editing ? (
            <div className="space-y-2">
              <Label htmlFor="decision-change-note">变更说明（可选）</Label>
              <Textarea
                id="decision-change-note"
                rows={2}
                placeholder="说明为什么修改草稿；会记录在修订历史中。"
                aria-invalid={Boolean(form.formState.errors.changeNote)}
                {...form.register("changeNote")}
              />
              {form.formState.errors.changeNote ? (
                <p role="alert" className="text-xs text-destructive">
                  {form.formState.errors.changeNote.message}
                </p>
              ) : null}
            </div>
          ) : null}

          <Alert variant="info">
            <Scale className="mt-0.5 size-4" aria-hidden="true" />
            <AlertTitle>草稿不会进入恢复包</AlertTitle>
            <AlertDescription>
              确认决策后，它才会作为当前有效约束进入跨会话恢复上下文。已确认决策不能原地改写；需要变化时请创建取代决策或标记废弃。
            </AlertDescription>
          </Alert>

          {createDecision.error || updateDecision.error ? (
            <Alert variant="destructive">
              <Scale className="mt-0.5 size-4" aria-hidden="true" />
              <AlertTitle>无法保存决策</AlertTitle>
              <AlertDescription>
                {getErrorMessage(createDecision.error ?? updateDecision.error)}
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
                <Scale aria-hidden="true" />
              )}
              {pending ? "保存中…" : editing ? "保存草稿" : "保存为草稿"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
