import type {
  AcceptanceReport,
  Evidence,
  SaveVerificationSpecInput,
  StartVerificationInput,
  VerificationCheck,
  VerificationPreview,
  VerificationQueueItem,
  VerificationRun,
  VerificationRunSummary,
  VerificationSpec,
  VerificationTemplate,
} from "@/lib/domain";
import { invokeCommand } from "@/lib/ipc";

export function listVerificationTemplates(): Promise<VerificationTemplate[]> {
  return invokeCommand<VerificationTemplate[]>("list_verification_templates");
}

export function getVerificationSpec(criterionId: string): Promise<VerificationSpec | null> {
  return invokeCommand<VerificationSpec | null>("get_verification_spec", { criterionId });
}

export function saveVerificationSpec(input: SaveVerificationSpecInput): Promise<VerificationSpec> {
  return invokeCommand<VerificationSpec>("save_verification_spec", { input });
}

export function previewVerification(criterionId: string): Promise<VerificationPreview> {
  return invokeCommand<VerificationPreview>("preview_verification", { criterionId });
}

export function startVerification(input: StartVerificationInput): Promise<VerificationRun> {
  return invokeCommand<VerificationRun>("start_verification", { input });
}

export function cancelVerification(runId: string): Promise<void> {
  return invokeCommand<void>("cancel_verification", { runId });
}

export function getVerificationRun(runId: string): Promise<VerificationRun> {
  return invokeCommand<VerificationRun>("get_verification_run", { runId });
}

export function listVerificationRuns(taskId: string): Promise<VerificationRunSummary[]> {
  return invokeCommand<VerificationRunSummary[]>("list_verification_runs", { taskId });
}

export function getVerificationCheck(runId: string): Promise<VerificationCheck | null> {
  return invokeCommand<VerificationCheck | null>("get_verification_check", { runId });
}

export function getEvidence(evidenceId: string): Promise<Evidence> {
  return invokeCommand<Evidence>("get_evidence", { evidenceId });
}

export function listVerificationQueue(): Promise<VerificationQueueItem[]> {
  return invokeCommand<VerificationQueueItem[]>("list_verification_queue");
}

export function exportAcceptanceReport(taskId: string): Promise<AcceptanceReport> {
  return invokeCommand<AcceptanceReport>("export_acceptance_report", { taskId });
}
