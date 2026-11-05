use uuid::Uuid;

use crate::error::CoreError;

pub(crate) fn validate_uuid(value: &str, field: &str) -> Result<String, CoreError> {
    Uuid::parse_str(value.trim())
        .map(|id| id.to_string())
        .map_err(|_| CoreError::Validation(format!("{field}格式无效")))
}

pub(crate) fn validate_revision(value: i64, field: &str) -> Result<(), CoreError> {
    if value < 1 {
        return Err(CoreError::Validation(format!("{field}必须从 1 开始")));
    }
    Ok(())
}

pub(crate) fn required_text(
    value: &str,
    field: &str,
    max_chars: usize,
    allow_line_breaks: bool,
) -> Result<String, CoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CoreError::Validation(format!("{field}不能为空")));
    }
    if value.chars().count() > max_chars {
        return Err(CoreError::Validation(format!(
            "{field}最多 {max_chars} 个字符"
        )));
    }
    if contains_disallowed_control(value, allow_line_breaks) {
        return Err(CoreError::Validation(format!(
            "{field}包含不支持的控制字符"
        )));
    }
    Ok(value.to_owned())
}

pub(crate) fn optional_text(
    value: Option<&str>,
    field: &str,
    max_chars: usize,
) -> Result<String, CoreError> {
    let value = value.unwrap_or("").trim();
    if value.chars().count() > max_chars {
        return Err(CoreError::Validation(format!(
            "{field}最多 {max_chars} 个字符"
        )));
    }
    if contains_disallowed_control(value, true) {
        return Err(CoreError::Validation(format!(
            "{field}包含不支持的控制字符"
        )));
    }
    Ok(value.to_owned())
}

pub(crate) fn optional_nonempty_text(
    value: Option<&str>,
    field: &str,
    max_chars: usize,
) -> Result<Option<String>, CoreError> {
    let value = value.unwrap_or("").trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_chars {
        return Err(CoreError::Validation(format!(
            "{field}最多 {max_chars} 个字符"
        )));
    }
    if contains_disallowed_control(value, true) {
        return Err(CoreError::Validation(format!(
            "{field}包含不支持的控制字符"
        )));
    }
    Ok(Some(value.to_owned()))
}

pub(crate) fn project_path(value: &str) -> Result<String, CoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CoreError::Validation("项目路径不能为空".to_owned()));
    }
    if value.chars().count() > 4096 {
        return Err(CoreError::Validation("项目路径最多 4096 个字符".to_owned()));
    }
    if contains_disallowed_control(value, false) {
        return Err(CoreError::Validation(
            "项目路径包含不支持的控制字符".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

pub(crate) fn search_text(value: Option<&str>) -> Result<Option<String>, CoreError> {
    let value = value.unwrap_or("").trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 200 {
        return Err(CoreError::Validation(
            "搜索关键词最多 200 个字符".to_owned(),
        ));
    }
    if contains_disallowed_control(value, false) {
        return Err(CoreError::Validation(
            "搜索关键词包含不支持的控制字符".to_owned(),
        ));
    }
    Ok(Some(value.to_owned()))
}

fn contains_disallowed_control(value: &str, allow_line_breaks: bool) -> bool {
    value.chars().any(|character| {
        if !character.is_control() {
            return false;
        }
        !(allow_line_breaks && matches!(character, '\n' | '\r' | '\t'))
    })
}
