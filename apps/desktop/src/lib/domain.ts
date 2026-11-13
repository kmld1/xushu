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

export type MemoryCategory =
  | "goal"
  | "constraint"
  | "architecture"
  | "convention"
  | "fact"
  | "risk"
  | "follow_up"
  | "knowledge";

export type MemorySourceType = "user" | "system" | "git" | "verification" | "hook" | "inference";

export type MemoryConfirmationStatus = "unconfirmed" | "confirmed";

export type MemoryValidityStatus = "active" | "archived";

export type LinkedTaskRef = {
  id: string;
  title: string;
  status: TaskStatus;
  archived: boolean;
};

export type ProjectMemory = {
  id: string;
  projectId: string;
  category: MemoryCategory;
  title: string;
  content: string;
  sourceType: MemorySourceType;
  confirmationStatus: MemoryConfirmationStatus;
  validityStatus: MemoryValidityStatus;
  linkedTaskId: string | null;
  linkedEvidenceId: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
  confirmedAt: string | null;
  archivedAt: string | null;
};

export type ProjectMemoryView = {
  memory: ProjectMemory;
  linkedTask: LinkedTaskRef | null;
};

export type ProjectMemoryRevision = {
  id: string;
  memoryId: string;
  revision: number;
  category: MemoryCategory;
  title: string;
  content: string;
  sourceType: MemorySourceType;
  confirmationStatus: MemoryConfirmationStatus;
  validityStatus: MemoryValidityStatus;
  linkedTaskId: string | null;
  linkedEvidenceId: string | null;
  changeNote: string | null;
  changedAt: string;
};

export type MemoryDetail = {
  memory: ProjectMemoryView;
  revisions: ProjectMemoryRevision[];
};

export type CreateMemoryInput = {
  projectId: string;
  category: MemoryCategory;
  title: string;
  content: string;
  sourceType?: MemorySourceType | null;
  confirmed?: boolean | null;
  linkedTaskId?: string | null;
  linkedEvidenceId?: string | null;
};

export type UpdateMemoryInput = {
  projectId: string;
  memoryId: string;
  expectedRevision: number;
  category: MemoryCategory;
  title: string;
  content: string;
  linkedTaskId?: string | null;
  linkedEvidenceId?: string | null;
  changeNote?: string | null;
};

export type ConfirmMemoryInput = {
  projectId: string;
  memoryId: string;
  expectedRevision: number;
};

export type ArchiveMemoryInput = {
  projectId: string;
  memoryId: string;
  expectedRevision: number;
  reason?: string | null;
};

export type MemoryFilter = {
  projectId: string;
  category?: MemoryCategory | null;
  includeArchived?: boolean | null;
  taskId?: string | null;
};

export type DecisionStatus = "draft" | "confirmed" | "superseded" | "deprecated";

export type Decision = {
  id: string;
  projectId: string;
  taskId: string | null;
  title: string;
  problem: string;
  context: string;
  alternatives: string[];
  decision: string;
  rationale: string;
  risks: string;
  status: DecisionStatus;
  supersedesId: string | null;
  supersededById: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
  confirmedAt: string | null;
  deprecatedAt: string | null;
};

export type DecisionView = {
  decision: Decision;
  linkedTask: LinkedTaskRef | null;
  supersedesTitle: string | null;
  supersededByTitle: string | null;
};

export type DecisionRevision = {
  id: string;
  decisionId: string;
  revision: number;
  title: string;
  problem: string;
  context: string;
  alternatives: string[];
  decision: string;
  rationale: string;
  risks: string;
  status: DecisionStatus;
  supersedesId: string | null;
  supersededById: string | null;
  changeNote: string | null;
  changedAt: string;
};

export type DecisionDetail = {
  decision: DecisionView;
  revisions: DecisionRevision[];
};

export type CreateDecisionInput = {
  projectId: string;
  taskId?: string | null;
  title: string;
  problem: string;
  context: string;
  alternatives: string[];
  decision: string;
  rationale: string;
  risks: string;
  supersedesId?: string | null;
};

export type UpdateDecisionInput = {
  projectId: string;
  decisionId: string;
  expectedRevision: number;
  taskId?: string | null;
  title: string;
  problem: string;
  context: string;
  alternatives: string[];
  decision: string;
  rationale: string;
  risks: string;
  supersedesId?: string | null;
  changeNote?: string | null;
};

export type ConfirmDecisionInput = {
  projectId: string;
  decisionId: string;
  expectedRevision: number;
};

export type DeprecateDecisionInput = {
  projectId: string;
  decisionId: string;
  expectedRevision: number;
  reason: string;
};

export type DecisionFilter = {
  projectId: string;
  status?: DecisionStatus | null;
  taskId?: string | null;
};

export type RecoveryGitState = "available" | "unknown";

export type RecoveryNextStepKind = "user_selected" | "suggested" | "needs_confirmation";

export type RecoveryFreshnessState = "fresh" | "stale" | "no_snapshot";

export type RecoveryProject = {
  id: string;
  name: string;
  rootPath: string;
  archived: boolean;
  createdAt: string;
  updatedAt: string;
};

export type RecoveryGit = {
  state: RecoveryGitState;
  checkedAt: string | null;
  branch: string | null;
  head: string | null;
  dirty: boolean;
  detached: boolean;
  unborn: boolean;
  workspaceFingerprint: string | null;
  fingerprintComplete: boolean;
  changedFileCount: number;
  note: string | null;
};

export type RecoveryTaskItem = {
  id: string;
  title: string;
  status: TaskStatus;
  priority: TaskPriority;
  revision: number;
  blockReason: string | null;
  updatedAt: string;
  requiredCount: number;
  satisfiedRequiredCount: number;
  blockers: string[];
};

export type RecoveryTasks = {
  inProgress: RecoveryTaskItem[];
  ready: RecoveryTaskItem[];
  blocked: RecoveryTaskItem[];
  awaitingVerification: RecoveryTaskItem[];
  doneWithValidEvidence: RecoveryTaskItem[];
  doneWithInvalidEvidence: RecoveryTaskItem[];
  draftCount: number;
};

export type RecoveryVerificationItem = {
  taskId: string;
  taskTitle: string;
  criterionId: string;
  criterionDescription: string;
  status: VerificationStatus;
  executionStatus: string | null;
  runId: string | null;
  evidenceId: string | null;
  startedAt: string | null;
  finishedAt: string | null;
  exitCode: number | null;
  staleReason: string | null;
  detail: string | null;
};

export type RecoveryClaude = {
  hasActivity: boolean;
  eventCount: number;
  sessionCount: number;
  lastEventAt: string | null;
  lastEventType: string | null;
  lastSessionRef: string | null;
  lastSessionStartedAt: string | null;
  lastSessionLastEventAt: string | null;
  lastSessionEndedAt: string | null;
  lastSessionEndReason: string | null;
  lastSessionFailedToolCount: number;
  lastSessionRunningToolCount: number;
  lastSessionUnknownToolCount: number;
  endConfirmed: boolean;
  note: string | null;
};

export type RecoveryMemoryItem = {
  id: string;
  category: MemoryCategory;
  title: string;
  content: string;
  sourceType: MemorySourceType;
  confirmationStatus: MemoryConfirmationStatus;
  revision: number;
  updatedAt: string;
  linkedTaskId: string | null;
  linkedTaskTitle: string | null;
  linkedTaskStatus: TaskStatus | null;
  linkedTaskArchived: boolean;
};

export type RecoveryDecisionItem = {
  id: string;
  title: string;
  status: DecisionStatus;
  taskId: string | null;
  taskTitle: string | null;
  problem: string;
  context: string;
  alternatives: string[];
  decision: string;
  rationale: string;
  risks: string;
  revision: number;
  updatedAt: string;
  confirmedAt: string | null;
  supersedesTitle: string | null;
  supersededByTitle: string | null;
};

export type RecoveryDecisionNotice = {
  id: string;
  title: string;
  status: DecisionStatus;
  revision: number;
  taskId: string | null;
  taskTitle: string | null;
  supersededByTitle: string | null;
  updatedAt: string;
  note: string;
};

export type RecoveryNextStep = {
  kind: RecoveryNextStepKind;
  taskId: string | null;
  taskTitle: string | null;
  taskStatus: TaskStatus | null;
  draftTaskRevision?: number;
  reason: string;
  requiresConfirmation: boolean;
};

export type RecoveryConflict = {
  kind: string;
  subject: string;
  detail: string;
};

export type RecoverySources = {
  databaseReadAt: string;
  gitCheckedAt: string | null;
  taskCandidateCount: number;
  activeTaskCount: number;
  doneTaskCount: number;
  draftTaskCount: number;
  memoryConfirmedActiveCount: number;
  memoryUnconfirmedActiveCount: number;
  memoryArchivedCount: number;
  decisionConfirmedCount: number;
  decisionDraftCount: number;
  decisionSupersededCount: number;
  decisionDeprecatedCount: number;
  verificationRunCount: number;
  claudeEventCount: number;
  claudeSessionCount: number;
  includedLimits: string[];
  exclusions: string[];
};

export type RecoveryPackage = {
  project: RecoveryProject;
  generatedAt: string;
  fingerprint: string;
  git: RecoveryGit;
  tasks: RecoveryTasks;
  verification: RecoveryVerificationItem[];
  claude: RecoveryClaude;
  memories: RecoveryMemoryItem[];
  unconfirmedMemories: RecoveryMemoryItem[];
  decisions: RecoveryDecisionItem[];
  supersededDecisions: RecoveryDecisionNotice[];
  nextStep: RecoveryNextStep;
  filesToRead: string[];
  conflicts: RecoveryConflict[];
  missingInformation: string[];
  sources: RecoverySources;
};

export type RecoverySnapshot = {
  id: string;
  projectId: string;
  generatedAt: string;
  fingerprint: string;
  markdown: string;
  package: RecoveryPackage;
  taskCount: number;
  memoryCount: number;
  decisionCount: number;
};

export type RecoverySnapshotSummary = {
  id: string;
  projectId: string;
  generatedAt: string;
  fingerprint: string;
  taskCount: number;
  memoryCount: number;
  decisionCount: number;
};

export type RecoveryFreshness = {
  state: RecoveryFreshnessState;
  checkedAt: string;
  changed: string[];
};

export type RecoveryOverview = {
  latest: RecoverySnapshot | null;
  freshness: RecoveryFreshness;
};

export type RecoveryPreference = {
  projectId: string;
  nextTaskId: string | null;
  updatedAt: string;
};

export type SetNextTaskInput = {
  projectId: string;
  taskId: string | null;
};
