use std::path::Path;

use xushu_hook_protocol::HookEventType;

use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;
use crate::validation::validate_uuid;

use super::ingest::{
    activity, import_pending, load_integration, load_project, project_counts, IntegrationRow,
    ProjectRow,
};
use super::settings::{
    exact_managed_count, expected_entries, lock_mutation, merged_document, pin_root, preview_token,
    read_snapshot, removed_document, restore_settings, write_document, PlannedEntry,
    SettingsSnapshot, SETTINGS_RELATIVE,
};
use super::{
    ClaudeActionInput, ClaudeActivitySnapshot, ClaudeHookPlannedEntry, ClaudeHookPreview,
    ClaudeImportReport, ClaudeInstallState, ClaudeIntegrationStatus, ClaudeRuntimePaths,
    InstallClaudeHooksInput, EVENT_RETENTION_DAYS, MAX_EVENTS_PER_PROJECT,
};

pub struct ClaudeIntegrationService<'a> {
    database: &'a Database,
    runtime: &'a ClaudeRuntimePaths,
}

impl<'a> ClaudeIntegrationService<'a> {
    pub fn new(database: &'a Database, runtime: &'a ClaudeRuntimePaths) -> Self {
        Self { database, runtime }
    }

    pub fn status(&self, project_id: &str) -> Result<ClaudeIntegrationStatus, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        let (project, integration, counts) = {
            let connection = self.database.lock()?;
            let project = load_project(&connection, &project_id)?.ok_or(CoreError::NotFound)?;
            let integration = load_integration(&connection, &project_id)?;
            let counts = project_counts(&connection, &project_id)?;
            (project, integration, counts)
        };
        Ok(self.build_status(&project, integration, counts))
    }

    pub fn refresh(&self, project_id: &str) -> Result<ClaudeIntegrationStatus, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        self.import_pending(&project_id)?;
        self.status(&project_id)
    }

    pub fn import_pending(&self, project_id: &str) -> Result<ClaudeImportReport, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        import_pending(self.database, self.runtime, &project_id)
    }

    pub fn import_pending_all(&self) -> Vec<ClaudeImportReport> {
        let project_ids = {
            let Ok(connection) = self.database.lock() else {
                return Vec::new();
            };
            let mut statement =
                match connection.prepare("SELECT project_id FROM claude_integrations") {
                    Ok(statement) => statement,
                    Err(_) => return Vec::new(),
                };
            let rows = match statement.query_map([], |row| row.get::<_, String>(0)) {
                Ok(rows) => rows,
                Err(_) => return Vec::new(),
            };
            rows.filter_map(Result::ok).collect::<Vec<_>>()
        };
        project_ids
            .into_iter()
            .filter_map(|project_id| self.import_pending(&project_id).ok())
            .collect()
    }

    pub fn activity(&self, project_id: &str) -> Result<ClaudeActivitySnapshot, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        {
            let connection = self.database.lock()?;
            load_project(&connection, &project_id)?.ok_or(CoreError::NotFound)?;
        }
        self.import_pending(&project_id)?;
        activity(self.database, self.runtime, &project_id)
    }

    pub fn preview(&self, project_id: &str) -> Result<ClaudeHookPreview, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        let (project, integration) = self.load_project_with_integration(&project_id)?;
        let snapshot = project_snapshot(&project, integration.as_ref());
        let generation = current_generation(integration.as_ref(), &snapshot)
            .unwrap_or_else(|| new_generation(integration.as_ref()));
        let mut blocked = Vec::new();
        if project.archived_at.is_some() {
            blocked.push("项目已归档，不能安装或修复 Hooks。".to_owned());
        }
        let hook_binary_available = self.runtime.hook_binary_available();
        let inbox_available = self.runtime.inbox_available();
        if !hook_binary_available {
            blocked.push("未找到 xushu-hook 接收程序，无法安装。".to_owned());
        }
        if !inbox_available {
            blocked.push("本地 Hook 收件箱不可用，无法安装。".to_owned());
        }
        if let Some(error) = &snapshot.error {
            blocked.push(error.message().to_owned());
        }

        let managed_entries = match self.runtime.hook_binary() {
            Some(binary) => expected_entries(
                &project_id,
                &snapshot.project_root,
                binary,
                self.runtime.inbox_dir(),
                generation,
            ),
            None => Vec::new(),
        };
        let token = format!(
            "{generation}:{}",
            preview_token(&snapshot, &managed_entries)
        );
        let mut warnings = vec![
            "只修改该项目的 .claude/settings.local.json，不写入团队共享的 .claude/settings.json。"
                .to_owned(),
            "安装前会备份原设置文件；卸载只删除 XUSHU 自己创建的条目，其他 Hooks 与权限保持原样。"
                .to_owned(),
            "Hooks 只上报事件类型、时间、伪名化会话/工具调用标识、工具名称与类别、成功或失败类别。"
                .to_owned(),
            "不会采集用户 Prompt、AI 回答、命令与参数、文件内容、工具输入输出、凭据或 transcript。"
                .to_owned(),
            "Hook 进程始终安静退出，观察失败不会阻塞 Claude Code 的正常开发。".to_owned(),
        ];
        if snapshot.other_group_count > 0 {
            warnings.push(format!(
                "检测到 {} 个其他 Hook 组，将保持原样并在卸载时保留。",
                snapshot.other_group_count
            ));
        }
        if !snapshot.managed_entries.is_empty() {
            warnings.push(
                "检测到已有的 XUSHU Hook 条目，安装或修复会将其规范化为当前版本。".to_owned(),
            );
        }

        Ok(ClaudeHookPreview {
            project_id,
            project_name: project.name,
            settings_file: SETTINGS_RELATIVE.to_owned(),
            settings_exists: snapshot.exists,
            will_create_file: !snapshot.exists,
            managed_entries: managed_entries.iter().map(planned_entry_view).collect(),
            managed_count_before: snapshot.managed_entries.len(),
            other_hook_group_count: snapshot.other_group_count,
            backup_will_be_created: snapshot.exists,
            receiver_available: hook_binary_available,
            inbox_available,
            preview_token: token,
            warnings,
            blocked_reason: if blocked.is_empty() {
                None
            } else {
                Some(blocked.join(" "))
            },
        })
    }

    pub fn install(
        &self,
        input: InstallClaudeHooksInput,
    ) -> Result<ClaudeIntegrationStatus, CoreError> {
        if !input.approved {
            return Err(CoreError::ApprovalRequired);
        }
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let _mutation = lock_mutation(self.runtime.backup_dir(), &project_id)?;
        let (project, integration) = self.load_project_with_integration(&project_id)?;
        if project.archived_at.is_some() {
            return Err(CoreError::Conflict(
                "项目已归档，不能安装或修复 Hooks".to_owned(),
            ));
        }
        let binary = self
            .runtime
            .hook_binary()
            .ok_or_else(|| CoreError::Claude("未找到 xushu-hook 接收程序".to_owned()))?;
        if !self.runtime.hook_binary_available() {
            return Err(CoreError::Claude(
                "xushu-hook 接收程序当前不可用".to_owned(),
            ));
        }
        if !self.runtime.inbox_available() {
            return Err(CoreError::Claude("本地 Hook 收件箱不可用".to_owned()));
        }

        let snapshot = project_snapshot(&project, integration.as_ref());
        if let Some(error) = &snapshot.error {
            return Err(CoreError::Claude(error.message().to_owned()));
        }
        let (generation, preview_hash) = input
            .preview_token
            .split_once(':')
            .and_then(|(generation, hash)| Some((generation.parse::<i64>().ok()?, hash)))
            .ok_or_else(stale_preview)?;
        match current_generation(integration.as_ref(), &snapshot) {
            Some(current) if generation == current => {}
            None if generation > 0
                && integration
                    .as_ref()
                    .is_none_or(|row| generation != row.capture_generation) => {}
            _ => return Err(stale_preview()),
        }
        let root_identity = snapshot
            .root_identity
            .as_deref()
            .ok_or_else(|| CoreError::Claude("无法确认已授权的项目根目录".to_owned()))?;
        let expected = expected_entries(
            &project_id,
            &snapshot.project_root,
            binary,
            self.runtime.inbox_dir(),
            generation,
        );
        let token = preview_token(&snapshot, &expected);
        if token != preview_hash {
            return Err(stale_preview());
        }

        let _root = pin_root(&snapshot)?;
        let already_exact = exact_managed_count(&snapshot, &expected) == expected.len()
            && snapshot.managed_entries.len() == expected.len();
        let written = if already_exact {
            None
        } else {
            let (document, _changed) = merged_document(&snapshot, &expected, &project_id);
            let written =
                write_document(&snapshot, &document, self.runtime.backup_dir(), &project_id)?;
            let verified = read_snapshot(Path::new(&project.root_path), &project_id)
                .require_root(Some(root_identity));
            let verified_ok = verified.error.is_none()
                && exact_managed_count(&verified, &expected) == expected.len()
                && verified.managed_entries.len() == expected.len();
            if !verified_ok {
                restore_settings(&snapshot, &written, &project_id)?;
                return Err(CoreError::Claude(
                    "写入 Hooks 后校验失败，已恢复原设置文件".to_owned(),
                ));
            }
            Some(written)
        };

        if let Err(error) = self.upsert_install_record(&project_id, generation, root_identity) {
            if let Some(written) = written {
                restore_settings(&snapshot, &written, &project_id)?;
            }
            return Err(error);
        }
        let status = self.status(&project_id)?;
        if status.install_state != ClaudeInstallState::Installed {
            return Err(CoreError::Conflict(
                "安装状态已写入，但项目设置或接收器状态已变化；请检查后重试".to_owned(),
            ));
        }
        Ok(status)
    }

    pub fn disable(&self, input: ClaudeActionInput) -> Result<ClaudeIntegrationStatus, CoreError> {
        self.remove_hooks(input, false)
    }

    pub fn uninstall(
        &self,
        input: ClaudeActionInput,
    ) -> Result<ClaudeIntegrationStatus, CoreError> {
        self.remove_hooks(input, true)
    }

    fn remove_hooks(
        &self,
        input: ClaudeActionInput,
        uninstall: bool,
    ) -> Result<ClaudeIntegrationStatus, CoreError> {
        if !input.approved {
            return Err(CoreError::ApprovalRequired);
        }
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let _mutation = lock_mutation(self.runtime.backup_dir(), &project_id)?;
        let (project, integration) = self.load_project_with_integration(&project_id)?;
        let snapshot = project_snapshot(&project, integration.as_ref());
        if let Some(error) = &snapshot.error {
            return Err(CoreError::Claude(error.message().to_owned()));
        }
        let _root = pin_root(&snapshot)?;
        let (document, removed) = removed_document(&snapshot, &project_id);
        let _written = (removed > 0)
            .then(|| write_document(&snapshot, &document, self.runtime.backup_dir(), &project_id))
            .transpose()?;
        let verified = read_snapshot(Path::new(&project.root_path), &project_id)
            .require_root(snapshot.root_identity.as_deref());
        if verified.error.is_some() || !verified.managed_entries.is_empty() {
            return Err(CoreError::Conflict(
                "无法确认 Hooks 已移除，未更新数据库状态；请检查项目设置后重试".to_owned(),
            ));
        }
        // A failed database write must never re-enable collection in Claude Code.
        self.mark_disabled(&project_id, uninstall).map_err(|error| {
            CoreError::Claude(format!(
                "已确认项目设置没有 XUSHU Hooks，但本地状态保存失败（{}）；请重试停用或卸载以同步状态",
                error.code()
            ))
        })?;
        let status = self.status(&project_id)?;
        if status.install_state != ClaudeInstallState::Disabled {
            return Err(CoreError::Conflict(
                "数据库已关闭采集，但项目设置状态已变化；请检查项目设置后重试".to_owned(),
            ));
        }
        Ok(status)
    }

    fn load_project_with_integration(
        &self,
        project_id: &str,
    ) -> Result<(ProjectRow, Option<IntegrationRow>), CoreError> {
        let connection = self.database.lock()?;
        let project = load_project(&connection, project_id)?.ok_or(CoreError::NotFound)?;
        let integration = load_integration(&connection, project_id)?;
        Ok((project, integration))
    }

    fn upsert_install_record(
        &self,
        project_id: &str,
        generation: i64,
        root_identity: &str,
    ) -> Result<(), CoreError> {
        let now = now_utc_rfc3339();
        let binary_path = self
            .runtime
            .hook_binary()
            .map(|path| path.to_string_lossy().to_string());
        let digest = self.runtime.hook_binary_digest();
        let connection = self.database.lock()?;
        connection.execute(
            "INSERT INTO claude_integrations (
                project_id, desired_enabled, hook_version, hook_binary_path, hook_binary_digest,
                installed_at, uninstalled_at, last_checked_at, created_at, updated_at,
                capture_generation, project_root_identity
             ) VALUES (?1, 1, ?2, ?3, ?4, ?5, NULL, ?5, ?5, ?5, ?6, ?7)
             ON CONFLICT(project_id) DO UPDATE SET
                desired_enabled = 1,
                hook_version = excluded.hook_version,
                hook_binary_path = excluded.hook_binary_path,
                hook_binary_digest = excluded.hook_binary_digest,
                installed_at = excluded.installed_at,
                uninstalled_at = NULL,
                last_checked_at = excluded.last_checked_at,
                last_error_code = NULL,
                last_error_at = NULL,
                capture_generation = excluded.capture_generation,
                project_root_identity = excluded.project_root_identity,
                updated_at = excluded.updated_at",
            rusqlite::params![
                project_id,
                self.runtime.hook_version(),
                binary_path,
                digest,
                now,
                generation,
                root_identity,
            ],
        )?;
        Ok(())
    }

    fn mark_disabled(&self, project_id: &str, uninstall: bool) -> Result<(), CoreError> {
        let now = now_utc_rfc3339();
        let connection = self.database.lock()?;
        if uninstall {
            connection.execute(
                "INSERT INTO claude_integrations (
                    project_id, desired_enabled, uninstalled_at, last_checked_at, created_at, updated_at
                 ) VALUES (?1, 0, ?2, ?2, ?2, ?2)
                 ON CONFLICT(project_id) DO UPDATE SET
                    desired_enabled = 0,
                    uninstalled_at = excluded.uninstalled_at,
                    last_checked_at = excluded.last_checked_at,
                    updated_at = excluded.updated_at",
                rusqlite::params![project_id, now],
            )?;
        } else {
            connection.execute(
                "INSERT INTO claude_integrations (
                    project_id, desired_enabled, uninstalled_at, last_checked_at, created_at, updated_at
                 ) VALUES (?1, 0, NULL, ?2, ?2, ?2)
                 ON CONFLICT(project_id) DO UPDATE SET
                    desired_enabled = 0,
                    uninstalled_at = NULL,
                    last_checked_at = excluded.last_checked_at,
                    updated_at = excluded.updated_at",
                rusqlite::params![project_id, now],
            )?;
        }
        Ok(())
    }

    fn build_status(
        &self,
        project: &ProjectRow,
        integration: Option<IntegrationRow>,
        counts: super::ingest::ProjectCounts,
    ) -> ClaudeIntegrationStatus {
        let project_id = &project.id;
        let expected_count = HookEventType::ALL.len();
        let snapshot = project_snapshot(project, integration.as_ref());
        let managed_hook_count = snapshot.managed_entries.len();
        let hook_binary_available = self.runtime.hook_binary_available();
        let inbox_available = self.runtime.inbox_available();
        let expected = match self.runtime.hook_binary() {
            Some(binary) => expected_entries(
                project_id,
                &snapshot.project_root,
                binary,
                self.runtime.inbox_dir(),
                integration.as_ref().map_or(0, |row| row.capture_generation),
            ),
            None => Vec::new(),
        };
        let exact_count = if expected.is_empty() {
            0
        } else {
            exact_managed_count(&snapshot, &expected)
        };
        let desired_enabled = integration.as_ref().is_some_and(|row| row.desired_enabled);
        let has_record = integration.is_some();
        let uninstalled_at = integration
            .as_ref()
            .and_then(|row| row.uninstalled_at.clone());
        let current_binary_path = self
            .runtime
            .hook_binary()
            .map(|path| path.to_string_lossy().to_string());
        let current_digest = self.runtime.hook_binary_digest();
        let receiver_changed = integration.as_ref().is_some_and(|row| {
            row.installed_at.is_some()
                && (row.hook_binary_path != current_binary_path
                    || row.hook_binary_digest != current_digest)
        });

        let state = if let Some(error) = &snapshot.error {
            match error {
                super::settings::SettingsError::ProjectUnavailable => {
                    ClaudeInstallState::ProjectUnavailable
                }
                _ => ClaudeInstallState::ConfigurationError,
            }
        } else if managed_hook_count == 0 && has_record {
            if desired_enabled {
                ClaudeInstallState::NeedsRepair
            } else {
                ClaudeInstallState::Disabled
            }
        } else if !hook_binary_available || !inbox_available {
            ClaudeInstallState::ReceiverUnavailable
        } else if managed_hook_count == 0 {
            ClaudeInstallState::NotConfigured
        } else if exact_count == expected_count && managed_hook_count == expected_count {
            if desired_enabled {
                if receiver_changed {
                    ClaudeInstallState::NeedsRepair
                } else {
                    ClaudeInstallState::Installed
                }
            } else {
                ClaudeInstallState::Desynchronized
            }
        } else if managed_hook_count < expected_count {
            ClaudeInstallState::Partial
        } else {
            ClaudeInstallState::NeedsRepair
        };

        let mut state_detail = match state {
            ClaudeInstallState::NotConfigured => "尚未安装 Hooks",
            ClaudeInstallState::Disabled => {
                if uninstalled_at.is_some() {
                    "已卸载；历史事件仍保留"
                } else {
                    "事件采集已停用"
                }
            }
            ClaudeInstallState::Installed => "Hooks 已安装并启用",
            ClaudeInstallState::Partial => "仅安装了部分 XUSHU Hooks，需要修复",
            ClaudeInstallState::NeedsRepair => {
                if managed_hook_count == 0 && desired_enabled {
                    "项目中已无 XUSHU Hooks，数据库仍记录启用；可重试停用或卸载以同步状态"
                } else if receiver_changed {
                    "Hook 接收程序已更新或路径变化，需要重新安装 Hooks"
                } else {
                    "Hook 配置与当前版本不一致，需要修复"
                }
            }
            ClaudeInstallState::ConfigurationError => snapshot
                .error
                .as_ref()
                .map(super::settings::SettingsError::message)
                .unwrap_or("项目设置文件异常"),
            ClaudeInstallState::ReceiverUnavailable => {
                "Hook 接收程序或本地收件箱不可用，安装与采集均无法确认"
            }
            ClaudeInstallState::ProjectUnavailable => "项目目录不可访问",
            ClaudeInstallState::Desynchronized => {
                "项目配置中已安装 Hooks，但 XUSHU 记录为停用，需要同步"
            }
        }
        .to_owned();
        if project.archived_at.is_some() {
            state_detail.push_str("；项目已归档");
        }

        ClaudeIntegrationStatus {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            project_archived: project.archived_at.is_some(),
            desired_enabled,
            has_record,
            install_state: state,
            state_detail: Some(state_detail),
            settings_file: SETTINGS_RELATIVE.to_owned(),
            settings_exists: snapshot.exists,
            managed_hook_count,
            expected_hook_count: expected_count,
            other_hook_group_count: snapshot.other_group_count,
            hook_version: integration
                .as_ref()
                .and_then(|row| row.hook_version.clone()),
            installed_at: integration
                .as_ref()
                .and_then(|row| row.installed_at.clone()),
            receiver_changed,
            hook_binary_available,
            inbox_available,
            pending_event_count: self.runtime.pending_count(project_id),
            last_event_at: counts.last_event_at,
            last_event_type: counts.last_event_type,
            last_import_at: integration
                .as_ref()
                .and_then(|row| row.last_import_at.clone()),
            uninstalled_at,
            last_error_code: integration
                .as_ref()
                .and_then(|row| row.last_error_code.clone()),
            last_error_at: integration
                .as_ref()
                .and_then(|row| row.last_error_at.clone()),
            event_count: counts.event_count,
            session_count: counts.session_count,
            running_tool_count: counts.running_tool_count,
            retention_days: EVENT_RETENTION_DAYS,
            max_events: MAX_EVENTS_PER_PROJECT,
        }
    }
}

fn project_snapshot(
    project: &ProjectRow,
    integration: Option<&IntegrationRow>,
) -> SettingsSnapshot {
    read_snapshot(Path::new(&project.root_path), &project.id)
        .require_root(integration.and_then(|row| row.project_root_identity.as_deref()))
}

fn current_generation(
    integration: Option<&IntegrationRow>,
    snapshot: &SettingsSnapshot,
) -> Option<i64> {
    let integration = integration?;
    (integration.desired_enabled && snapshot.has_generation(integration.capture_generation))
        .then_some(integration.capture_generation)
}

fn new_generation(integration: Option<&IntegrationRow>) -> i64 {
    loop {
        let generation = (uuid::Uuid::new_v4().as_u128() as u64 & i64::MAX as u64) as i64;
        if generation > 0 && integration.is_none_or(|row| generation != row.capture_generation) {
            return generation;
        }
    }
}

fn stale_preview() -> CoreError {
    CoreError::Conflict("安装预览已过期：项目目录或设置可能已变化，请重新预览".to_owned())
}

fn planned_entry_view(entry: &PlannedEntry) -> ClaudeHookPlannedEntry {
    ClaudeHookPlannedEntry {
        event: entry.event.clone(),
        matcher: entry.matcher.clone(),
        command: entry.command.clone(),
        args: entry.args.clone(),
        timeout_seconds: entry.timeout_seconds,
    }
}
