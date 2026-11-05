use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;

const MAX_LABEL_CHARS: usize = 64;
const MAX_CONTENT_CHARS: usize = 2000;
const DEFAULT_LIST_LIMIT: u32 = 20;
const MAX_LIST_LIMIT: u32 = 100;

/// 写入一条本地持久化测试记录的输入。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProbeInput {
    pub label: String,
    pub content: String,
}

/// 已通过 Rust 侧校验的输入。
#[derive(Debug, Clone)]
pub(crate) struct ValidatedProbeInput {
    label: String,
    content: String,
}

impl CreateProbeInput {
    pub(crate) fn validate(self) -> Result<ValidatedProbeInput, CoreError> {
        let label = self.label.trim().to_owned();
        let content = self.content.trim().to_owned();

        if label.is_empty() {
            return Err(CoreError::Validation("标签不能为空".to_owned()));
        }
        if label.chars().count() > MAX_LABEL_CHARS {
            return Err(CoreError::Validation(format!(
                "标签最多 {MAX_LABEL_CHARS} 个字符"
            )));
        }
        if label.chars().any(|character| character.is_control()) {
            return Err(CoreError::Validation("标签不能包含控制字符".to_owned()));
        }

        if content.is_empty() {
            return Err(CoreError::Validation("内容不能为空".to_owned()));
        }
        if content.chars().count() > MAX_CONTENT_CHARS {
            return Err(CoreError::Validation(format!(
                "内容最多 {MAX_CONTENT_CHARS} 个字符"
            )));
        }
        if content
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return Err(CoreError::Validation("内容包含不支持的控制字符".to_owned()));
        }

        Ok(ValidatedProbeInput { label, content })
    }
}

/// Phase 1 的本地持久化测试记录。
///
/// 该类型只用于验证 SQLite 写入/读取与跨进程持久化，不是 Phase 2 的任务、
/// 项目或验收实体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeRecord {
    pub id: String,
    pub label: String,
    pub content: String,
    pub created_at: String,
}

impl Database {
    /// 校验并写入一条测试记录。
    pub fn create_probe(&self, input: CreateProbeInput) -> Result<ProbeRecord, CoreError> {
        let input = input.validate()?;
        let record = ProbeRecord {
            id: Uuid::new_v4().to_string(),
            label: input.label,
            content: input.content,
            created_at: now_utc_rfc3339(),
        };

        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO persistence_probes (id, label, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.id, record.label, record.content, record.created_at],
        )?;

        Ok(record)
    }

    /// 按 ID 读取记录；ID 非法时返回 `Validation`，不存在时返回 `None`。
    pub fn get_probe(&self, id: &str) -> Result<Option<ProbeRecord>, CoreError> {
        let id = Uuid::parse_str(id.trim())
            .map_err(|_| CoreError::Validation("记录 ID 格式无效".to_owned()))?
            .to_string();

        let conn = self.lock()?;
        let record = conn
            .query_row(
                "SELECT id, label, content, created_at
                 FROM persistence_probes
                 WHERE id = ?1",
                [id],
                row_to_probe,
            )
            .optional()?;

        Ok(record)
    }

    /// 按创建时间倒序读取最近记录。
    pub fn list_probes(&self, limit: Option<u32>) -> Result<Vec<ProbeRecord>, CoreError> {
        let limit = match limit {
            None => DEFAULT_LIST_LIMIT,
            Some(value) if value == 0 || value > MAX_LIST_LIMIT => {
                return Err(CoreError::Validation(format!(
                    "limit 必须在 1..={MAX_LIST_LIMIT} 之间"
                )));
            }
            Some(value) => value,
        };

        let conn = self.lock()?;
        let mut statement = conn.prepare(
            "SELECT id, label, content, created_at
             FROM persistence_probes
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([i64::from(limit)], row_to_probe)?;
        let records = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }
}

fn row_to_probe(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProbeRecord> {
    Ok(ProbeRecord {
        id: row.get(0)?,
        label: row.get(1)?,
        content: row.get(2)?,
        created_at: row.get(3)?,
    })
}

/// 校验 RFC3339 时间；供测试与调用方判断记录时间是否可解析。
pub fn parse_created_at(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}
