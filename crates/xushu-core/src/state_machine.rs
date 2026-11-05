use crate::error::CoreError;
use crate::models::TaskStatus;

/// 状态机的标准转换图。
///
/// `cancelled -> draft` 不在此图中；重新开启必须调用独立的受控 `reopen_task`
/// 服务方法，确保留下明确原因与历史。
pub fn allowed_transitions(from: TaskStatus) -> Vec<TaskStatus> {
    match from {
        TaskStatus::Draft => vec![TaskStatus::Ready, TaskStatus::Cancelled],
        TaskStatus::Ready => vec![
            TaskStatus::Running,
            TaskStatus::Blocked,
            TaskStatus::Cancelled,
        ],
        TaskStatus::Running => vec![
            TaskStatus::AwaitingVerification,
            TaskStatus::Blocked,
            TaskStatus::Cancelled,
        ],
        TaskStatus::AwaitingVerification => vec![
            TaskStatus::Running,
            TaskStatus::Done,
            TaskStatus::Blocked,
            TaskStatus::Cancelled,
        ],
        TaskStatus::Blocked => vec![
            TaskStatus::Ready,
            TaskStatus::Running,
            TaskStatus::Cancelled,
        ],
        TaskStatus::Done | TaskStatus::Cancelled => Vec::new(),
    }
}

pub fn validate_transition(from: TaskStatus, to: TaskStatus) -> Result<(), CoreError> {
    if allowed_transitions(from).contains(&to) {
        Ok(())
    } else {
        Err(CoreError::InvalidTransition {
            from: from.as_str().to_owned(),
            to: to.as_str().to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected(from: TaskStatus) -> Vec<TaskStatus> {
        match from {
            TaskStatus::Draft => vec![TaskStatus::Ready, TaskStatus::Cancelled],
            TaskStatus::Ready => vec![
                TaskStatus::Running,
                TaskStatus::Blocked,
                TaskStatus::Cancelled,
            ],
            TaskStatus::Running => vec![
                TaskStatus::AwaitingVerification,
                TaskStatus::Blocked,
                TaskStatus::Cancelled,
            ],
            TaskStatus::AwaitingVerification => vec![
                TaskStatus::Running,
                TaskStatus::Done,
                TaskStatus::Blocked,
                TaskStatus::Cancelled,
            ],
            TaskStatus::Blocked => vec![
                TaskStatus::Ready,
                TaskStatus::Running,
                TaskStatus::Cancelled,
            ],
            TaskStatus::Done | TaskStatus::Cancelled => Vec::new(),
        }
    }

    #[test]
    fn every_status_pair_matches_the_declared_transition_graph() {
        for from in TaskStatus::ALL {
            let allowed = expected(from);
            for to in TaskStatus::ALL {
                let result = validate_transition(from, to);
                if allowed.contains(&to) {
                    assert!(result.is_ok(), "{from:?} -> {to:?} should be legal");
                } else {
                    assert!(result.is_err(), "{from:?} -> {to:?} should be illegal");
                }
            }
        }
    }

    #[test]
    fn terminal_statuses_have_no_transitions() {
        assert!(allowed_transitions(TaskStatus::Done).is_empty());
        assert!(allowed_transitions(TaskStatus::Cancelled).is_empty());
    }

    #[test]
    fn done_is_only_reachable_from_awaiting_verification() {
        for from in TaskStatus::ALL {
            if from == TaskStatus::AwaitingVerification {
                assert!(validate_transition(from, TaskStatus::Done).is_ok());
            } else {
                assert!(validate_transition(from, TaskStatus::Done).is_err());
            }
        }
    }
}
