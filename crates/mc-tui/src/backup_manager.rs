use mc_core::{
    AppError, BackupCursor, BackupRestorePlan, BackupSummary, TaskResult, TaskSnapshot, TaskStatus,
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BackupCommand {
    List {
        instance_id: Uuid,
        cursor: Option<BackupCursor>,
    },
    Create {
        instance_id: Uuid,
    },
    Verify {
        instance_id: Uuid,
        backup_id: Uuid,
    },
    PlanRestore {
        instance_id: Uuid,
        backup_id: Uuid,
    },
    Restore {
        instance_id: Uuid,
        backup_id: Uuid,
        confirmation_token: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackupOperation {
    LoadingPage,
    LoadingMore,
    Creating,
    Verifying,
    Planning,
    Restoring,
}

impl BackupOperation {
    fn label(self) -> &'static str {
        match self {
            Self::LoadingPage => "正在读取并校验备份页…",
            Self::LoadingMore => "正在加载更多备份…",
            Self::Creating => "正在创建停机世界备份…",
            Self::Verifying => "正在重新哈希所选备份…",
            Self::Planning => "正在校验目标并生成恢复影响预览…",
            Self::Restoring => "正在替换世界目录并执行恢复事务…",
        }
    }
}

impl BackupOperation {
    pub(crate) fn is_restoring(self) -> bool {
        self == Self::Restoring
    }
}

#[derive(Debug)]
pub(crate) struct BackupManagerState {
    pub instance_id: Uuid,
    pub instance_name: String,
    pub backups: Vec<BackupSummary>,
    pub selected: usize,
    pub next_cursor: Option<BackupCursor>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub plan: Option<BackupRestorePlan>,
    pub verified_backup_id: Option<Uuid>,
    pub restored_backup_id: Option<Uuid>,
    pub confirmation_input: String,
    pub editing_confirmation: bool,
    operation: Option<BackupOperation>,
    tracked_task: Option<TaskSnapshot>,
}

impl BackupManagerState {
    pub fn new(instance_id: Uuid, instance_name: String) -> Self {
        Self {
            instance_id,
            instance_name,
            backups: Vec::new(),
            selected: 0,
            next_cursor: None,
            error: None,
            notice: Some("正在读取备份清单并校验本页内容…".to_owned()),
            plan: None,
            verified_backup_id: None,
            restored_backup_id: None,
            confirmation_input: String::new(),
            editing_confirmation: false,
            operation: None,
            tracked_task: None,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.operation.is_some()
            || self
                .tracked_task
                .as_ref()
                .is_some_and(|task| is_active_task(task.status))
    }

    pub fn operation(&self) -> Option<BackupOperation> {
        self.operation
    }

    pub fn is_restoring(&self) -> bool {
        self.operation.is_some_and(BackupOperation::is_restoring)
    }

    pub fn is_reviewing_plan(&self) -> bool {
        self.plan.is_some() && !self.is_restoring()
    }

    pub fn has_more(&self) -> bool {
        self.next_cursor.is_some()
    }

    pub fn selected_backup(&self) -> Option<&BackupSummary> {
        self.backups.get(self.selected)
    }
}

impl BackupManagerState {
    pub fn select_previous(&mut self) {
        if !self.is_busy() && self.plan.is_none() {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub fn select_next(&mut self) {
        if !self.is_busy() && self.plan.is_none() && !self.backups.is_empty() {
            self.selected = (self.selected + 1).min(self.backups.len() - 1);
        }
    }

    pub fn begin_list(&mut self, cursor: Option<BackupCursor>) {
        self.error = None;
        if cursor.is_none() {
            self.verified_backup_id = None;
        }
        self.plan = None;
        self.restored_backup_id = None;
        self.editing_confirmation = false;
        self.confirmation_input.clear();
        self.operation = Some(if cursor.is_some() {
            BackupOperation::LoadingMore
        } else {
            BackupOperation::LoadingPage
        });
        self.notice = Some(self.operation().expect("operation set").label().to_owned());
        self.tracked_task = None;
    }

    pub fn begin_create(&mut self) {
        self.error = None;
        self.restored_backup_id = None;
        self.operation = Some(BackupOperation::Creating);
        self.notice = Some(BackupOperation::Creating.label().to_owned());
        self.tracked_task = None;
    }

    pub fn begin_verify(&mut self) {
        self.error = None;
        self.verified_backup_id = None;
        self.restored_backup_id = None;
        self.operation = Some(BackupOperation::Verifying);
        self.notice = Some(BackupOperation::Verifying.label().to_owned());
        self.tracked_task = None;
    }

    pub fn begin_plan(&mut self) {
        self.error = None;
        self.plan = None;
        self.restored_backup_id = None;
        self.editing_confirmation = false;
        self.confirmation_input.clear();
        self.operation = Some(BackupOperation::Planning);
        self.notice = Some(BackupOperation::Planning.label().to_owned());
        self.tracked_task = None;
    }
}

impl BackupManagerState {
    pub fn begin_restore(&mut self) {
        self.error = None;
        self.restored_backup_id = None;
        self.editing_confirmation = false;
        self.confirmation_input.clear();
        self.operation = Some(BackupOperation::Restoring);
        self.notice = Some(BackupOperation::Restoring.label().to_owned());
        self.tracked_task = None;
    }

    pub fn begin_confirmation(&mut self) {
        if self.plan.is_some() && !self.is_busy() && !self.is_restoring() {
            self.editing_confirmation = true;
            self.confirmation_input.clear();
            self.error = None;
            self.notice = Some("输入 RESTORE 后按 Enter；Esc 取消确认。".to_owned());
        }
    }

    pub fn cancel_confirmation(&mut self) {
        self.editing_confirmation = false;
        self.confirmation_input.clear();
        self.notice = Some("恢复尚未执行；请核对预览后再次确认。".to_owned());
        self.error = None;
    }

    pub fn insert_confirmation_char(&mut self, character: char) {
        if self.editing_confirmation
            && character.is_ascii_alphabetic()
            && self.confirmation_input.len() < 16
        {
            self.confirmation_input.push(character.to_ascii_uppercase());
        }
    }

    pub fn backspace_confirmation(&mut self) {
        if self.editing_confirmation {
            self.confirmation_input.pop();
        }
    }

    pub fn confirmation_is_exact(&self) -> bool {
        self.confirmation_input == "RESTORE"
    }

    pub fn mark_submission_error(&mut self, error: &AppError) {
        self.operation = None;
        self.tracked_task = None;
        self.error = Some(error_message(error));
        self.notice = None;
    }
}

impl BackupManagerState {
    pub fn track_task(&mut self, task: &TaskSnapshot) -> bool {
        if task.instance_id != Some(self.instance_id) {
            self.error = Some("后台返回了其他实例的备份任务。".to_owned());
            self.operation = None;
            self.tracked_task = None;
            return true;
        }
        self.tracked_task = Some(task.clone());
        if is_active_task(task.status) {
            self.error = None;
            self.notice = Some(format!(
                "{} {}",
                task_progress_symbol(task.status),
                task.message
            ));
            false
        } else {
            self.finish_task(task);
            true
        }
    }

    pub fn apply_poll_error(&mut self, error: &AppError) {
        self.error = Some(format!("{} · 正在重试读取后台任务。", error_message(error)));
    }

    pub fn fail_without_task(&mut self, message: impl Into<String>) {
        self.operation = None;
        self.tracked_task = None;
        self.error = Some(message.into());
        self.notice = None;
    }

    pub fn tracked_task_id(&self) -> Option<Uuid> {
        self.tracked_task.as_ref().map(|task| task.id)
    }

    pub fn active_task_id(&self) -> Option<Uuid> {
        self.operation?;
        self.tracked_task
            .as_ref()
            .filter(|task| is_active_task(task.status))
            .map(|task| task.id)
    }
}

impl BackupManagerState {
    fn finish_task(&mut self, task: &TaskSnapshot) {
        if task.status != TaskStatus::Succeeded {
            if self.operation.is_some_and(BackupOperation::is_restoring) {
                self.plan = None;
            }
            let detail = task
                .error
                .as_ref()
                .map(error_message)
                .unwrap_or_else(|| task.message.clone());
            self.operation = None;
            self.error = Some(detail);
            self.notice = None;
            return;
        }

        let operation = self.operation;
        let result = task.result.as_ref();
        match operation {
            Some(BackupOperation::LoadingPage) | Some(BackupOperation::LoadingMore) => {
                if let Some(TaskResult::WorldBackupsListed { instance_id, page }) = result {
                    if *instance_id != self.instance_id {
                        self.fail_without_task("后台返回了其他实例的备份清单。");
                        return;
                    }
                    if matches!(operation, Some(BackupOperation::LoadingPage)) {
                        self.backups.clear();
                        self.selected = 0;
                    }
                    for backup in &page.backups {
                        if !self.backups.iter().any(|existing| existing.id == backup.id) {
                            self.backups.push(backup.clone());
                        }
                    }
                    self.next_cursor = page.next_cursor.clone();
                    self.selected = self.selected.min(self.backups.len().saturating_sub(1));
                    self.notice = Some(if self.backups.is_empty() {
                        "没有可恢复的世界备份；先停止实例并创建备份。".to_owned()
                    } else {
                        format!(
                            "已载入 {} 个通过完整性校验的备份{}。",
                            self.backups.len(),
                            if self.next_cursor.is_some() {
                                "，可加载更多"
                            } else {
                                ""
                            }
                        )
                    });
                } else {
                    self.fail_without_task("备份列表任务没有返回有效分页。");
                    return;
                }
            }
            Some(BackupOperation::Creating) => {
                if let Some(TaskResult::WorldBackupCreated {
                    instance_id,
                    backup_id,
                }) = result
                {
                    if *instance_id != self.instance_id {
                        self.fail_without_task("后台返回了其他实例的备份结果。");
                        return;
                    }
                    self.notice = Some(format!(
                        "停机备份 {} 已创建并通过校验；按 r 刷新列表。",
                        short_backup_id(*backup_id)
                    ));
                } else {
                    self.fail_without_task("备份任务没有返回有效备份 ID。");
                    return;
                }
            }
            Some(BackupOperation::Verifying) => {
                if let Some(TaskResult::WorldBackupVerified {
                    instance_id,
                    backup,
                }) = result
                {
                    if *instance_id != self.instance_id {
                        self.fail_without_task("后台返回了其他实例的校验结果。");
                        return;
                    }
                    self.verified_backup_id = Some(backup.id);
                    self.notice = Some(format!(
                        "完整性已验证：{} · {} 个文件 · {}。",
                        short_backup_id(backup.id),
                        backup.file_count,
                        format_bytes(backup.total_bytes)
                    ));
                } else {
                    self.fail_without_task("校验任务没有返回有效备份摘要。");
                    return;
                }
            }
            Some(BackupOperation::Planning) => {
                if let Some(TaskResult::WorldBackupRestorePlanned { instance_id, plan }) = result {
                    if *instance_id != self.instance_id {
                        self.fail_without_task("后台返回了其他实例的恢复计划。");
                        return;
                    }
                    self.plan = Some(plan.clone());
                    self.notice =
                        Some("恢复预览已生成；尚未修改世界。按 c 输入 RESTORE 确认。".to_owned());
                } else {
                    self.fail_without_task("恢复任务没有返回有效影响预览。");
                    return;
                }
            }
            Some(BackupOperation::Restoring) => {
                if let Some(TaskResult::WorldBackupRestored {
                    instance_id,
                    backup_id,
                }) = result
                {
                    if *instance_id != self.instance_id {
                        self.fail_without_task("后台返回了其他实例的恢复结果。");
                        return;
                    }
                    self.plan = None;
                    self.restored_backup_id = Some(*backup_id);
                    self.notice = Some(format!(
                        "世界已从备份 {} 恢复并完成校验。",
                        short_backup_id(*backup_id)
                    ));
                } else {
                    self.fail_without_task("恢复任务没有返回有效备份 ID。");
                    return;
                }
            }
            None => {
                self.error = Some("收到未预期的任务完成结果。".to_owned());
                return;
            }
        }
        self.error = None;
        self.operation = None;
    }
}

pub(crate) fn short_backup_id(id: Uuid) -> String {
    id.to_string()[..8].to_owned()
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub(crate) fn is_active_task(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Queued | TaskStatus::Running)
}

fn task_progress_symbol(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Queued => "◌",
        TaskStatus::Running => "◐",
        TaskStatus::Succeeded => "✓",
        TaskStatus::Failed => "×",
        TaskStatus::Interrupted => "•",
        TaskStatus::Cancelled => "−",
    }
}

fn error_message(error: &AppError) -> String {
    match &error.next_action {
        Some(action) => format!("{} · 下一步：{}", error.reason, action),
        None => error.reason.clone(),
    }
}

#[cfg(test)]
mod tests;
