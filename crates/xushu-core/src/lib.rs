//! 序枢 XUSHU 的核心领域库。
//!
//! Phase 0 仅保留 workspace 骨架；领域模型、状态机与 SQLite 仓储将在后续阶段实现。

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
