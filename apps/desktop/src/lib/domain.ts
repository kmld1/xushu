export type TaskStatus =
  "draft" | "ready" | "running" | "awaiting_verification" | "done" | "blocked" | "cancelled";

export type TaskPriority = "low" | "medium" | "high" | "urgent";

export type CriterionKind = "auto" | "manual";

export type Project = {
  id: string;
  name: string;
  rootPath: string;
  createdAt: string;
  updatedAt: string;
  archivedAt: string | null;
};

export type ProjectGitStatus = {
  projectId: string;
  rootPath: string;
  branch: string | null;
  head: string | null;
  dirty: boolean;
  detached: boolean;
  unborn: boolean;
  checkedAt: string;
};

export type Task = {
  id: string;
  projectId: string;
  title: string;
  description: string;
  priority: TaskPriority;
  status: TaskStatus;
  revision: number;
  blockReason: string | null;
  archivedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type AcceptanceCriterion = {
  id: string;
  taskId: string;
  description: string;
  kind: CriterionKind;
  required: boolean;
  revision: number;
  createdAt: string;
  updatedAt: string;
};

export type TaskTransition = {
  id: string;
  taskId: string;
  fromStatus: TaskStatus | null;
  toStatus: TaskStatus;
  reason: string | null;
  taskRevision: number;
  createdAt: string;
};

export type ManualConfirmation = {
  id: string;
  criterionId: string;
  criterionRevision: number;
  taskRevision: number;
  confirmedAt: string;
  note: string | null;
};

export type CriterionView = {
  criterion: AcceptanceCriterion;
  satisfied: boolean;
  confirmation: ManualConfirmation | null;
};

export type CompletionStatus = {
  canComplete: boolean;
  requiredCount: number;
  satisfiedRequiredCount: number;
  blockers: string[];
};

export type TaskDetail = {
  task: Task;
  criteria: CriterionView[];
  transitions: TaskTransition[];
  allowedTransitions: TaskStatus[];
  completion: CompletionStatus;
};

export type TaskCounts = {
  total: number;
  draft: number;
  ready: number;
  running: number;
  awaitingVerification: number;
  done: number;
  blocked: number;
  cancelled: number;
};

export type WorkspaceSummary = {
  projectCount: number;
  archivedProjectCount: number;
  taskCounts: TaskCounts;
  recentTasks: Task[];
  blockedTasks: Task[];
  awaitingTasks: Task[];
};

export type TaskFilter = {
  projectId?: string | null;
  status?: TaskStatus | null;
  search?: string | null;
  includeArchived?: boolean | null;
};

export type ImportProjectInput = {
  rootPath: string;
  name?: string | null;
};

export type UpdateProjectInput = {
  projectId: string;
  name: string;
};

export type CreateTaskInput = {
  projectId: string;
  title: string;
  description?: string | null;
  priority: TaskPriority;
};

export type UpdateTaskInput = {
  taskId: string;
  expectedRevision: number;
  title: string;
  description?: string | null;
  priority: TaskPriority;
};

export type ArchiveTaskInput = {
  taskId: string;
  expectedRevision: number;
};

export type TransitionTaskInput = {
  taskId: string;
  expectedRevision: number;
  toStatus: TaskStatus;
  reason?: string | null;
};

export type ReopenTaskInput = {
  taskId: string;
  expectedRevision: number;
  reason: string;
};

export type CreateCriterionInput = {
  taskId: string;
  expectedTaskRevision: number;
  description: string;
  kind: CriterionKind;
  required: boolean;
};

export type UpdateCriterionInput = {
  criterionId: string;
  expectedTaskRevision: number;
  expectedCriterionRevision: number;
  description: string;
  kind: CriterionKind;
  required: boolean;
};

export type RemoveCriterionInput = {
  criterionId: string;
  expectedTaskRevision: number;
  expectedCriterionRevision: number;
};

export type ConfirmManualInput = {
  criterionId: string;
  expectedTaskRevision: number;
  expectedCriterionRevision: number;
  note?: string | null;
};
