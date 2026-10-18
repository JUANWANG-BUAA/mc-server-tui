use mc_core::{AppError, ErrorCode, TaskSnapshot};

use crate::backup_manager::{BackupCommand, BackupManagerState};

use super::AppState;

impl AppState {
    pub(super) fn open_backup_manager(&mut self) {
        if !self.instance_detail_visible {
            self.status = "请先打开一个实例。".to_owned();
            return;
        }
        let Some(instance_id) = self.instance_detail_id else {
            return;
        };
        let instance_name = self
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .map(|instance| instance.name.clone())
            .or_else(|| {
                self.instance_detail
                    .as_ref()
                    .and_then(|detail| detail.detail.as_ref())
                    .map(|detail| detail.instance.name.clone())
            })
            .unwrap_or_else(|| "实例".to_owned());
        self.backup_manager = Some(BackupManagerState::new(instance_id, instance_name));
        self.backup_manager_visible = true;
        self.queue_backup_list(None);
    }

    pub(super) fn close_backup_manager(&mut self) {
        let Some(manager) = self.backup_manager.as_mut() else {
            self.backup_manager_visible = false;
            return;
        };
        if manager.editing_confirmation {
            manager.cancel_confirmation();
            return;
        }
        if manager.is_reviewing_plan() {
            manager.plan = None;
            manager.notice = Some("已取消恢复预览；没有修改世界。".to_owned());
            manager.error = None;
            return;
        }
        if manager.is_busy() {
            manager.notice = Some("后台任务运行中，完成前请保持本页。".to_owned());
            self.status = manager.notice.clone().unwrap_or_default();
            return;
        }
        self.backup_manager_visible = false;
        self.status = "已返回实例详情；备份目录未修改。".to_owned();
        if let Some(detail) = self.instance_detail.as_mut() {
            detail.notice = Some(self.status.clone());
        }
    }

    pub(super) fn queue_backup_list(&mut self, cursor: Option<mc_core::BackupCursor>) {
        if !self.backup_manager_visible {
            return;
        }
        if self.instance_operation_pending {
            self.status = "上一项实例操作仍在运行。".to_owned();
            return;
        }
        let Some(manager) = self.backup_manager.as_mut() else {
            return;
        };
        if manager.is_busy() {
            self.status = "后台任务尚未完成，请稍候。".to_owned();
            return;
        }
        let instance_id = manager.instance_id;
        manager.begin_list(cursor.clone());
        self.backup_command_requested = Some(BackupCommand::List {
            instance_id,
            cursor,
        });
        self.instance_operation_pending = true;
        self.status = "正在提交备份目录读取任务…".to_owned();
    }

    pub(super) fn queue_backup_more(&mut self) {
        let Some(cursor) = self
            .backup_manager
            .as_ref()
            .and_then(|manager| manager.next_cursor.clone())
        else {
            self.status = "没有更多备份页。".to_owned();
            return;
        };
        self.queue_backup_list(Some(cursor));
    }

    pub(super) fn queue_backup_verify(&mut self) {
        if self.instance_operation_pending {
            self.status = "上一项实例操作仍在运行。".to_owned();
            return;
        }
        let Some(manager) = self.backup_manager.as_mut() else {
            return;
        };
        if manager.is_busy() || manager.plan.is_some() {
            self.status = "请先等待任务完成或退出恢复预览。".to_owned();
            return;
        }
        let Some(backup_id) = manager.selected_backup().map(|backup| backup.id) else {
            self.status = "当前没有可校验的备份。".to_owned();
            return;
        };
        let instance_id = manager.instance_id;
        manager.begin_verify();
        self.backup_command_requested = Some(BackupCommand::Verify {
            instance_id,
            backup_id,
        });
        self.instance_operation_pending = true;
        self.status = "正在提交所选备份的完整性校验任务…".to_owned();
    }

    pub(super) fn queue_backup_restore_plan(&mut self) {
        if self.instance_operation_pending {
            self.status = "上一项实例操作仍在运行。".to_owned();
            return;
        }
        let Some(manager) = self.backup_manager.as_mut() else {
            return;
        };
        if manager.is_busy() || manager.plan.is_some() {
            self.status = "请先等待任务完成或退出恢复预览。".to_owned();
            return;
        }
        let Some(backup_id) = manager.selected_backup().map(|backup| backup.id) else {
            self.status = "当前没有可恢复的备份。".to_owned();
            return;
        };
        let instance_id = manager.instance_id;
        manager.begin_plan();
        self.backup_command_requested = Some(BackupCommand::PlanRestore {
            instance_id,
            backup_id,
        });
        self.instance_operation_pending = true;
        self.status = "正在提交恢复影响预览任务…".to_owned();
    }

    pub(super) fn queue_backup_create(&mut self) {
        if self.instance_operation_pending {
            self.status = "上一项实例操作仍在运行。".to_owned();
            return;
        }
        let Some(manager) = self.backup_manager.as_mut() else {
            return;
        };
        if manager.is_busy() || manager.plan.is_some() {
            self.status = "请先等待任务完成或退出恢复预览。".to_owned();
            return;
        }
        let instance_id = manager.instance_id;
        manager.begin_create();
        self.backup_command_requested = Some(BackupCommand::Create { instance_id });
        self.instance_operation_pending = true;
        self.status = "正在提交停机世界备份任务…".to_owned();
    }

    pub(super) fn begin_restore_confirmation(&mut self) {
        if let Some(manager) = self.backup_manager.as_mut() {
            manager.begin_confirmation();
            self.status = manager.notice.clone().unwrap_or_default();
        }
    }

    pub(super) fn insert_restore_confirmation(&mut self, character: char) {
        if let Some(manager) = self.backup_manager.as_mut() {
            manager.insert_confirmation_char(character);
        }
    }

    pub(super) fn backspace_restore_confirmation(&mut self) {
        if let Some(manager) = self.backup_manager.as_mut() {
            manager.backspace_confirmation();
        }
    }

    pub(super) fn cancel_restore_confirmation(&mut self) {
        if let Some(manager) = self.backup_manager.as_mut() {
            manager.cancel_confirmation();
            self.status = manager.notice.clone().unwrap_or_default();
        }
    }

    pub(super) fn submit_restore_confirmation(&mut self) {
        let Some(manager) = self.backup_manager.as_mut() else {
            return;
        };
        if !manager.editing_confirmation {
            return;
        }
        if !manager.confirmation_is_exact() {
            manager.error = Some("确认文本必须完整输入 RESTORE。".to_owned());
            return;
        }
        if self.instance_operation_pending {
            manager.error = Some("上一项实例操作仍在运行。".to_owned());
            return;
        }
        let Some(plan) = manager.plan.clone() else {
            manager.error = Some("恢复预览已失效，请重新生成。".to_owned());
            return;
        };
        let instance_id = manager.instance_id;
        let backup_id = plan.backup_id;
        let confirmation_token = plan.confirmation_token.clone();
        manager.begin_restore();
        self.backup_command_requested = Some(BackupCommand::Restore {
            instance_id,
            backup_id,
            confirmation_token,
        });
        self.instance_operation_pending = true;
        self.status = "恢复确认已提交，正在执行持久恢复事务…".to_owned();
    }

    pub fn apply_backup_command_result(&mut self, result: Result<TaskSnapshot, AppError>) {
        let Some(manager) = self.backup_manager.as_mut() else {
            self.instance_operation_pending = false;
            return;
        };
        match result {
            Ok(task) => {
                let finished = manager.track_task(&task);
                let message = manager
                    .notice
                    .clone()
                    .or_else(|| manager.error.clone())
                    .unwrap_or_else(|| "备份任务已加入后台队列。".to_owned());
                if finished {
                    self.instance_operation_pending = false;
                }
                self.remember_backup_task(task);
                self.status = message;
            }
            Err(error) => {
                let message = error_message(&error);
                manager.mark_submission_error(&error);
                self.instance_operation_pending = false;
                self.status = format!("备份任务提交失败：{message}");
            }
        }
    }

    pub fn apply_backup_task_update(&mut self, result: Result<TaskSnapshot, AppError>) {
        let Some(manager) = self.backup_manager.as_mut() else {
            return;
        };
        match result {
            Ok(task) => {
                if !manager
                    .tracked_task_id()
                    .is_some_and(|tracked| tracked == task.id)
                {
                    return;
                }
                let finished = manager.track_task(&task);
                let message = manager
                    .notice
                    .clone()
                    .or_else(|| manager.error.clone())
                    .unwrap_or_else(|| "后台备份任务仍在运行。".to_owned());
                if finished {
                    self.instance_operation_pending = false;
                }
                self.remember_backup_task(task);
                self.status = message;
            }
            Err(error) => {
                if error.code == ErrorCode::TaskNotFound {
                    manager.fail_without_task(error_message(&error));
                    self.instance_operation_pending = false;
                } else {
                    manager.apply_poll_error(&error);
                }
                self.status = format!("后台任务状态读取失败：{}", error.reason);
            }
        }
    }

    pub fn take_backup_command(&mut self) -> Option<BackupCommand> {
        self.backup_command_requested.take()
    }

    pub fn backup_task_to_poll(&self) -> Option<uuid::Uuid> {
        if !self.backup_manager_visible {
            return None;
        }
        self.backup_manager
            .as_ref()
            .and_then(BackupManagerState::active_task_id)
    }

    fn remember_backup_task(&mut self, task: TaskSnapshot) {
        if let Some(existing) = self
            .tasks
            .iter_mut()
            .find(|existing| existing.id == task.id)
        {
            *existing = task;
        } else {
            self.tasks.insert(0, task);
        }
    }
}

fn error_message(error: &AppError) -> String {
    match &error.next_action {
        Some(action) => format!("{} · 下一步：{}", error.reason, action),
        None => error.reason.clone(),
    }
}
