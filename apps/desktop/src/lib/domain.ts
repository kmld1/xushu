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

export type VerificationStatus = "pass" | "fail" | "not_run" | "inconclusive" | "stale";

export type VerificationExecutionStatus =
  "running" | "completed" | "cancelled" | "timed_out" | "start_failed" | "interrupted";

export type CodeState = {
  head: string | null;
  branch: string | null;
  detached: boolean;
  unborn: boolean;
  dirty: boolean;
  fingerprint: string | null;
  fingerprintComplete: boolean;
  changedFileCount: number;
  note: string | null;
};

export type VerificationSpec = {
  id: string;
  criterionId: string;
  executable: string;
  args: string[];
  cwd: string;
  timeoutSeconds: number;
  outputLimitBytes: number;
  successRule: string;
  revision: number;
  createdAt: string;
  updatedAt: string;
};

export type VerificationCheck = {
  id: string;
  runId: string;
  taskId: string;
  criterionId: string | null;
  result: VerificationStatus;
  reason: string;
  evidenceId: string | null;
  createdAt: string;
};

export type VerificationRunSummary = {
  id: string;
  taskId: string;
  criterionId: string | null;
  criterionDescription: string;
  executable: string;
  args: string[];
  cwd: string;
  executionStatus: VerificationExecutionStatus;
  result: VerificationStatus | null;
  reason: string | null;
  evidenceId: string | null;
  specRevision: number;
  taskRevision: number;
  criterionRevision: number;
  startedAt: string;
  finishedAt: string | null;
  exitCode: number | null;
  durationMs: number | null;
  stdoutTruncated: boolean;
  stderrTruncated: boolean;
  sensitiveFlags: string[];
  createdAt: string;
};

export type VerificationRun = {
  id: string;
  taskId: string;
  criterionId: string | null;
  criterionDescription: string;
  specId: string | null;
  specRevision: number;
  taskRevision: number;
  criterionRevision: number;
  executable: string;
  resolvedExecutable: string | null;
  args: string[];
  cwd: string;
  timeoutSeconds: number;
  outputLimitBytes: number;
  successRule: string;
  approvedAt: string;
  startedAt: string;
  finishedAt: string | null;
  executionStatus: VerificationExecutionStatus;
  exitCode: number | null;
  durationMs: number | null;
  startCodeState: CodeState | null;
  endCodeState: CodeState | null;
  stdoutSummary: string | null;
  stderrSummary: string | null;
  stdoutBytes: number | null;
  stderrBytes: number | null;
  stdoutTruncated: boolean;
  stderrTruncated: boolean;
  sensitiveFlags: string[];
  outputDigest: string | null;
  errorCode: string | null;
  errorMessage: string | null;
  cancelReason: string | null;
  createdAt: string;
};

export type CriterionVerification = {
  status: VerificationStatus;
  spec: VerificationSpec | null;
  latestRun: VerificationRunSummary | null;
  latestCheck: VerificationCheck | null;
  validPassRunId: string | null;
  staleReason: string | null;
  detail: string | null;
};

export type VerificationPreview = {
  criterionId: string;
  criterionDescription: string;
  taskId: string;
  taskTitle: string;
  executable: string;
  resolvedExecutable: string | null;
  args: string[];
  cwd: string;
  timeoutSeconds: number;
  outputLimitBytes: number;
  successRule: string;
  specRevision: number;
  taskRevision: number;
  criterionRevision: number;
  warnings: string[];
};

export type VerificationTemplate = {
  id: string;
  name: string;
  description: string;
  executable: string;
  args: string[];
  timeoutSeconds: number;
};

export type Evidence = {
  id: string;
  sourceType: string;
  taskId: string;
  runId: string | null;
  summary: string;
  redactedOutput: string | null;
  contentDigest: string;
  sensitiveFlags: string[];
  createdAt: string;
};

export type VerificationQueueItem = {
  task: Task;
  projectName: string;
  completion: CompletionStatus;
  autoTotal: number;
  autoSatisfied: number;
  manualPending: number;
  runningCount: number;
};

export type SaveVerificationSpecInput = {
  criterionId: string;
  expectedTaskRevision: number;
  expectedCriterionRevision: number;
  expectedSpecRevision: number | null;
  executable: string;
  args: string[];
  cwd?: string | null;
  timeoutSeconds: number;
  outputLimitBytes: number;
};

export type StartVerificationInput = {
  criterionId: string;
  expectedTaskRevision: number;
  expectedCriterionRevision: number;
  expectedSpecRevision: number;
  approved: boolean;
};

export type AcceptanceReport = {
  fileName: string;
  markdown: string;
};

export type CriterionView = {
  criterion: AcceptanceCriterion;
  satisfied: boolean;
  confirmation: ManualConfirmation | null;
  verification: CriterionVerification | null;
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

export type ClaudeInstallState =
  | "not_configured"
  | "disabled"
  | "installed"
  | "partial"
  | "needs_repair"
  | "configuration_error"
  | "receiver_unavailable"
  | "project_unavailable"
  | "desynchronized";

export type ClaudeIntegrationStatus = {
  projectId: string;
  projectName: string;
  projectArchived: boolean;
  desiredEnabled: boolean;
  hasRecord: boolean;
  installState: ClaudeInstallState;
  stateDetail: string | null;
  settingsFile: string;
  settingsExists: boolean;
  managedHookCount: number;
  expectedHookCount: number;
  otherHookGroupCount: number;
  hookVersion: string | null;
  installedAt: string | null;
  receiverChanged: boolean;
  hookBinaryAvailable: boolean;
  inboxAvailable: boolean;
  pendingEventCount: number;
  lastEventAt: string | null;
  lastEventType: string | null;
  lastImportAt: string | null;
  uninstalledAt: string | null;
  lastErrorCode: string | null;
  lastErrorAt: string | null;
  eventCount: number;
  sessionCount: number;
  runningToolCount: number;
  retentionDays: number;
  maxEvents: number;
};

export type ClaudeHookPlannedEntry = {
  event: string;
  matcher: string | null;
  command: string;
  args: string[];
  timeoutSeconds: number;
};

export type ClaudeHookPreview = {
  projectId: string;
  projectName: string;
  settingsFile: string;
  settingsExists: boolean;
  willCreateFile: boolean;
  managedEntries: ClaudeHookPlannedEntry[];
  managedCountBefore: number;
  otherHookGroupCount: number;
  backupWillBeCreated: boolean;
  receiverAvailable: boolean;
  inboxAvailable: boolean;
  previewToken: string;
  warnings: string[];
  blockedReason: string | null;
};

export type ClaudeSessionSummary = {
  id: string;
  sessionRef: string;
  startedAt: string;
  lastEventAt: string;
  endedAt: string | null;
  startSource: string;
  endReason: string | null;
  startObserved: boolean;
  activityState: "active" | "ended" | "unknown";
  eventCount: number;
  toolCallCount: number;
  runningToolCount: number;
  failedToolCount: number;
  unknownToolCount: number;
};

export type ClaudeActivityEvent = {
  id: string;
  sessionId: string;
  sessionRef: string;
  eventType: string;
  occurredAt: string;
  toolName: string | null;
  toolCategory: string | null;
  status: string;
  failureKind: string | null;
  durationMs: number | null;
  source: string | null;
  endReason: string | null;
};

export type ClaudeToolCallSummary = {
  id: string;
  sessionId: string;
  sessionRef: string;
  toolName: string;
  toolCategory: string;
  status: "running" | "succeeded" | "failed" | "unknown";
  failureKind: string | null;
  startedAt: string | null;
  finishedAt: string | null;
  durationMs: number | null;
  startObserved: boolean;
  finishObserved: boolean;
  conflict: boolean;
};

export type ClaudeActivitySnapshot = {
  projectId: string;
  generatedAt: string;
  pendingEventCount: number;
  sessions: ClaudeSessionSummary[];
  events: ClaudeActivityEvent[];
  toolCalls: ClaudeToolCallSummary[];
};

export type ClaudeImportReport = {
  projectId: string;
  imported: number;
  duplicates: number;
  rejected: number;
  ignored: number;
  remaining: number;
  lastErrorCode: string | null;
};

export type InstallClaudeHooksInput = {
  projectId: string;
  previewToken: string;
  approved: boolean;
};

export type ClaudeActionInput = {
  projectId: string;
  approved: boolean;
};
