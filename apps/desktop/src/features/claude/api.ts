import type {
  ClaudeActionInput,
  ClaudeActivitySnapshot,
  ClaudeHookPreview,
  ClaudeImportReport,
  ClaudeIntegrationStatus,
  InstallClaudeHooksInput,
} from "@/lib/domain";
import { invokeCommand } from "@/lib/ipc";

export function getClaudeIntegration(projectId: string): Promise<ClaudeIntegrationStatus> {
  return invokeCommand<ClaudeIntegrationStatus>("get_claude_integration", { projectId });
}

export function getClaudeActivity(projectId: string): Promise<ClaudeActivitySnapshot> {
  return invokeCommand<ClaudeActivitySnapshot>("get_claude_activity", { projectId });
}

export function previewClaudeHooks(projectId: string): Promise<ClaudeHookPreview> {
  return invokeCommand<ClaudeHookPreview>("preview_claude_hooks", { projectId });
}

export function installClaudeHooks(
  input: InstallClaudeHooksInput,
): Promise<ClaudeIntegrationStatus> {
  return invokeCommand<ClaudeIntegrationStatus>("install_claude_hooks", { input });
}

export function disableClaudeHooks(input: ClaudeActionInput): Promise<ClaudeIntegrationStatus> {
  return invokeCommand<ClaudeIntegrationStatus>("disable_claude_hooks", { input });
}

export function uninstallClaudeHooks(input: ClaudeActionInput): Promise<ClaudeIntegrationStatus> {
  return invokeCommand<ClaudeIntegrationStatus>("uninstall_claude_hooks", { input });
}

export function refreshClaudeEvents(projectId: string): Promise<ClaudeImportReport> {
  return invokeCommand<ClaudeImportReport>("refresh_claude_events", { projectId });
}
