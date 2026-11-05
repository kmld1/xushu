//! 序枢 XUSHU 的核心领域库。
//!
//! Phase 1 提供 SQLite 基础与持久化探针；Phase 2 增加项目、任务、验收条件、
//! 人工确认与统一状态机。所有业务状态转换都必须经过本 crate 的服务层。

pub mod db;
pub mod error;
pub mod git;
pub mod models;
pub mod probe;
pub mod project;
pub mod state_machine;
pub mod task;
mod validation;

pub use db::{run_migrations, Database, DatabaseHealth, Migration, MigrationReport, MIGRATIONS};
pub use error::CoreError;
pub use git::GitStatus;
pub use models::*;
pub use probe::{parse_created_at, CreateProbeInput, ProbeRecord};
pub use project::ProjectService;
pub use task::TaskService;

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
