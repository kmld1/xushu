//! 序枢 XUSHU 的核心领域库。
//!
//! Phase 1 提供 SQLite 打开/配置、幂等迁移、事务、完整性检查与本地测试记录
//! 的持久化。任务、项目与验收状态机仍属于后续阶段。

pub mod db;
pub mod error;
pub mod probe;

pub use db::{run_migrations, Database, DatabaseHealth, Migration, MigrationReport, MIGRATIONS};
pub use error::CoreError;
pub use probe::{parse_created_at, CreateProbeInput, ProbeRecord};

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
