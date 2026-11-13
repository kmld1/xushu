//! 序枢 XUSHU 的核心领域库。
//!
//! Phase 1 提供 SQLite 基础与持久化探针；Phase 2 增加项目、任务、验收条件、
//! 人工确认与统一状态机。所有业务状态转换都必须经过本 crate 的服务层。

pub mod claude;
pub mod code_state;
pub mod db;
pub mod decision;
pub mod error;
pub mod git;
pub mod memory;
pub mod models;
pub mod probe;
pub mod project;
pub mod recovery;
pub mod redaction;
pub mod runner;
pub mod state_machine;
pub mod task;
mod validation;
pub mod verification;

pub use claude::{
    ClaudeActionInput, ClaudeActivityEvent, ClaudeActivitySnapshot, ClaudeHookPlannedEntry,
    ClaudeHookPreview, ClaudeImportReport, ClaudeInstallState, ClaudeIntegrationService,
    ClaudeIntegrationStatus, ClaudeRuntimePaths, ClaudeSessionSummary, ClaudeToolCallSummary,
    InstallClaudeHooksInput,
};
pub use code_state::CodeState;
pub use db::{run_migrations, Database, DatabaseHealth, Migration, MigrationReport, MIGRATIONS};
pub use decision::{
    ConfirmDecisionInput, CreateDecisionInput, Decision, DecisionDetail, DecisionFilter,
    DecisionRevision, DecisionService, DecisionStatus, DecisionView, DeprecateDecisionInput,
    UpdateDecisionInput,
};
pub use error::CoreError;
pub use git::GitStatus;
pub use memory::{
    ArchiveMemoryInput, ConfirmMemoryInput, CreateMemoryInput, LinkedTaskRef, MemoryCategory,
    MemoryConfirmationStatus, MemoryDetail, MemoryFilter, MemoryService, MemorySourceType,
    MemoryValidityStatus, ProjectMemory, ProjectMemoryRevision, ProjectMemoryView,
    UpdateMemoryInput,
};
pub use models::*;
pub use probe::{parse_created_at, CreateProbeInput, ProbeRecord};
pub use project::ProjectService;
pub use recovery::{
    RecoveryClaude, RecoveryConflict, RecoveryDecisionItem, RecoveryDecisionNotice,
    RecoveryFreshness, RecoveryFreshnessState, RecoveryGit, RecoveryGitState, RecoveryMemoryItem,
    RecoveryNextStep, RecoveryNextStepKind, RecoveryOverview, RecoveryPackage, RecoveryPreference,
    RecoveryProject, RecoveryService, RecoverySnapshot, RecoverySnapshotSummary, RecoverySources,
    RecoveryTaskItem, RecoveryTasks, RecoveryVerificationItem, SetNextTaskInput,
};
pub use runner::{ExecutionStatus, RunControl, RunOutcome, RunPlan};
pub use task::TaskService;
pub use verification::{
    AcceptanceReport, CriterionVerification, Evidence, SaveVerificationSpecInput,
    StartVerificationInput, VerificationCheck, VerificationPreview, VerificationQueueItem,
    VerificationResult, VerificationRun, VerificationRunSummary, VerificationService,
    VerificationSpec, VerificationTemplate,
};

/// 产品展示名称。
pub const APP_NAME: &str = "序枢 XUSHU";

/// 返回产品展示名称。
pub fn app_name() -> &'static str {
    APP_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_is_not_empty() {
        assert!(!app_name().is_empty());
    }
}
