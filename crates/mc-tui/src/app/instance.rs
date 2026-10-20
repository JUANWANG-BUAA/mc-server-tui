use mc_core::AppError;

use crate::{
    instance_detail::{
        InstanceCommand, InstanceCommandResult, InstanceDetailQuery, InstanceDetailState,
        InstanceDetailUpdate,
    },
    transcript::TranscriptEntry,
};

use super::{AppState, Panel};

impl AppState {
    pub(super) fn open_selected_instance_detail(&mut self) {
        if self.focused_panel != Panel::Instances {
            self.status = "请先切换到实例面板。".to_owned();
            return;
        }
        let Some(instance) = self.instances.get(self.selected_instance) else {
            self.status = "当前没有可查看的实例。".to_owned();
            return;
        };
        let instance_id = instance.id;
        if self.instance_detail_id != Some(instance_id) {
            if let Some(previous) = self.instance_detail.take() {
                self.instance_cache.insert(previous.instance_id, previous);
            }
            self.instance_detail = Some(
                self.instance_cache
                    .remove(&instance_id)
                    .unwrap_or_else(|| InstanceDetailState::loading(instance_id)),
            );
            self.instance_detail_id = Some(instance_id);
            self.backup_manager = None;
            self.backup_manager_visible = false;
        }
        self.instance_detail_visible = true;
        self.instance_detail_refresh_requested = true;
    }

    pub(super) fn close_instance_detail(&mut self) {
        self.instance_detail_visible = false;
        self.instance_detail_refresh_requested = false;
        self.backup_manager_visible = false;
        self.backup_manager = None;
        if let Some(detail) = &mut self.instance_detail {
            detail.cancel_console_input();
        }
    }

    pub(super) fn request_instance_detail_refresh(&mut self) {
        if self.instance_detail_visible || (self.shell_visible && self.instance_detail_id.is_some())
        {
            self.instance_detail_refresh_requested = true;
        }
    }

    pub(super) fn selected_instance_id(&self) -> Option<uuid::Uuid> {
        self.instance_detail_id
            .filter(|_| self.instance_detail_visible)
            .or_else(|| {
                self.instances
                    .get(self.selected_instance)
                    .map(|instance| instance.id)
            })
            .or(self.instance_detail_id)
    }

    pub(super) fn queue_server_command(&mut self, action: ServerLifecycleAction) {
        let Some(instance_id) = self.selected_instance_id() else {
            self.status = "当前没有选中的实例。".to_owned();
            return;
        };
        self.instance_detail_id = Some(instance_id);
        if self.instance_operation_pending {
            self.status = "上一项服务器操作仍在运行。".to_owned();
            return;
        }
        self.instance_operation_pending = true;
        let (command, status) = match action {
            ServerLifecycleAction::Start => (
                InstanceCommand::Start(instance_id),
                "正在启动服务器…".to_owned(),
            ),
            ServerLifecycleAction::Stop => (
                InstanceCommand::Stop(instance_id),
                "正在请求优雅停服…".to_owned(),
            ),
            ServerLifecycleAction::Restart => (
                InstanceCommand::Restart(instance_id),
                "正在停止服务器，退出后将自动重新启动…".to_owned(),
            ),
        };
        self.instance_command_requested = Some(command);
        self.status = status;
        if let Some(detail) = self.instance_detail.as_mut() {
            detail.notice = Some(self.status.clone());
        }
    }

    pub(super) fn queue_backup(&mut self) {
        if self.backup_manager_visible {
            self.queue_backup_create();
            return;
        }
        let Some(instance_id) = self.selected_instance_id() else {
            self.status = "当前没有选中的实例。".to_owned();
            return;
        };
        self.instance_detail_id = Some(instance_id);
        if self.instance_operation_pending {
            self.status = "上一项实例操作仍在运行。".to_owned();
            return;
        }
        if self.servers.get(&instance_id).is_some_and(|server| {
            matches!(
                server.state,
                mc_core::ServerState::Starting
                    | mc_core::ServerState::Running
                    | mc_core::ServerState::Stopping
            )
        }) {
            self.status = "备份需要先停服：/stop，等待已停止后再 /backup。".to_owned();
            self.push_transcript(TranscriptEntry::warning(self.status.clone()));
            return;
        }
        self.instance_operation_pending = true;
        self.instance_command_requested = Some(InstanceCommand::Backup(instance_id));
        self.status = "正在提交世界备份任务…".to_owned();
        if let Some(detail) = self.instance_detail.as_mut() {
            detail.notice = Some(self.status.clone());
        }
    }

    pub(super) fn begin_console_input(&mut self) {
        if !self.instance_operation_pending {
            if let Some(detail) = self.instance_detail.as_mut() {
                detail.begin_console_input();
            }
        }
    }

    pub(super) fn insert_console_character(&mut self, character: char) {
        if let Some(detail) = self.instance_detail.as_mut() {
            detail.insert_console_char(character);
        }
    }

    pub(super) fn backspace_console(&mut self) {
        if let Some(detail) = self.instance_detail.as_mut() {
            detail.backspace_console();
        }
    }

    pub(super) fn cancel_console_input(&mut self) {
        if let Some(detail) = self.instance_detail.as_mut() {
            detail.cancel_console_input();
        }
    }

    pub(super) fn submit_console_command(&mut self) {
        if let Some(detail) = self.instance_detail.as_mut() {
            if detail.editing_filter {
                detail.editing_filter = false;
                return;
            }
        }
        if self.instance_operation_pending {
            return;
        }
        let Some(instance_id) = self
            .instance_detail_id
            .filter(|_| self.instance_detail_visible)
        else {
            return;
        };
        let Some(command) = self
            .instance_detail
            .as_mut()
            .and_then(InstanceDetailState::take_console_command)
        else {
            return;
        };
        self.instance_operation_pending = true;
        self.instance_command_requested = Some(InstanceCommand::Console {
            instance_id,
            command,
        });
        self.status = "正在发送控制台命令…".to_owned();
        if let Some(detail) = self.instance_detail.as_mut() {
            detail.notice = Some(self.status.clone());
        }
    }

    pub(super) fn queue_console_command(&mut self, command: String) {
        if self.instance_operation_pending {
            self.status = "上一项实例操作仍在运行。".to_owned();
            return;
        }
        let Some(instance_id) = self.selected_instance_id() else {
            self.status = "当前没有选中的实例。".to_owned();
            return;
        };
        self.instance_detail_id = Some(instance_id);
        self.instance_operation_pending = true;
        self.instance_command_requested = Some(InstanceCommand::Console {
            instance_id,
            command,
        });
        self.status = "正在发送控制台命令…".to_owned();
    }

    pub fn apply_instance_detail_result(&mut self, result: Result<InstanceDetailUpdate, AppError>) {
        let Some(instance_id) = self.instance_detail_id else {
            return;
        };
        if result
            .as_ref()
            .is_ok_and(|update| update.detail.instance.id != instance_id)
        {
            return;
        }
        let detail = self
            .instance_detail
            .get_or_insert_with(|| InstanceDetailState::loading(instance_id));
        let previous_alerts = detail.alerts();
        detail.apply_update(result);
        let alerts = detail.alerts();
        let new_alerts = alerts
            .into_iter()
            .filter(|alert| !previous_alerts.contains(alert))
            .collect::<Vec<_>>();
        if let Some(error) = &detail.error {
            self.status = format!("实例详情刷新失败：{error}");
        } else if let Some(server) = detail.detail.as_ref().map(|detail| &detail.server) {
            self.status = crate::ui::server_message(&server.message);
        }
        for alert in new_alerts {
            self.push_transcript(TranscriptEntry::warning(format!("告警：{alert}")));
        }
    }

    pub fn apply_instance_command_result(&mut self, result: InstanceCommandResult) {
        self.instance_operation_pending = false;
        match result {
            InstanceCommandResult::Management(result) => match result.map(|response| *response) {
                Ok(mc_core::ApiResponse::ManagementReport(report)) => {
                    self.push_transcript(TranscriptEntry::assistant(report));
                    self.status = "操作完成。".to_owned();
                }
                Ok(mc_core::ApiResponse::TaskQueued(task)) => {
                    self.status = "任务已排队；/tasks 查看进度，完成结果会写入会话。".to_owned();
                    self.push_transcript(TranscriptEntry::assistant(self.status.clone()));
                    self.tasks.insert(0, task);
                    self.refresh_requested = true;
                }
                Err(error) => {
                    self.status = format!("操作失败：{}", error.reason);
                    self.push_transcript(TranscriptEntry::error(format!(
                        "{}\n{}",
                        self.status,
                        error.next_action.unwrap_or_default()
                    )));
                }
                Ok(_) => self.push_transcript(TranscriptEntry::error("后台返回了意外响应。")),
            },
            InstanceCommandResult::Status(result) => {
                let mut transcript = None;
                if let Some(detail) = self.instance_detail.as_mut() {
                    detail.apply_status(result);
                    if let Some(error) = &detail.error {
                        self.status = format!("生命周期操作失败：{error}");
                        detail.notice = Some(error.clone());
                        transcript = Some(TranscriptEntry::error(self.status.clone()));
                    } else if let Some(server) = detail.detail.as_ref().map(|data| &data.server) {
                        let message = crate::ui::server_message(&server.message);
                        self.status = format!("{} · {message}", server.address);
                        detail.notice = Some(message.clone());
                        let text = format!("服务器状态：{} · {message}", server.address);
                        // Only a settled state earns the success marker; a start
                        // that is still waiting for readiness is progress, not success.
                        transcript = Some(match server.state {
                            mc_core::ServerState::Running | mc_core::ServerState::Stopped => {
                                TranscriptEntry::success(text)
                            }
                            mc_core::ServerState::Failed => TranscriptEntry::error(text),
                            _ => TranscriptEntry::assistant(text),
                        });
                    }
                }
                if let Some(entry) = transcript {
                    self.push_transcript(entry);
                }
            }
            InstanceCommandResult::Console(result) => match result {
                Ok(()) => {
                    self.status = "控制台命令已发送。".to_owned();
                    if let Some(detail) = self.instance_detail.as_mut() {
                        detail.notice = Some(self.status.clone());
                    }
                    self.instance_detail_refresh_requested = true;
                    self.push_transcript(TranscriptEntry::success("控制台命令已发送。"));
                }
                Err(error) => {
                    let message = match error.next_action {
                        Some(action) => format!("{} · 下一步：{action}", error.reason),
                        None => error.reason,
                    };
                    if let Some(detail) = self.instance_detail.as_mut() {
                        detail.error = Some(message.clone());
                        detail.notice = Some(message.clone());
                    }
                    self.status = format!("控制台命令失败：{message}");
                    self.push_transcript(TranscriptEntry::error(format!(
                        "控制台命令失败：{message}"
                    )));
                }
            },
            InstanceCommandResult::Task(result) => match result {
                Ok(task) => {
                    let message = match task.kind {
                        mc_core::TaskKind::CreateWorldBackup => "世界备份任务已加入后台队列。",
                        mc_core::TaskKind::ListWorldBackups => "备份列表任务已加入后台队列。",
                        mc_core::TaskKind::VerifyWorldBackup => "备份校验任务已加入后台队列。",
                        mc_core::TaskKind::PlanWorldBackupRestore => "恢复预览任务已加入后台队列。",
                        mc_core::TaskKind::RestoreWorldBackup => "世界恢复任务已加入后台队列。",
                        mc_core::TaskKind::CreatePaperInstance
                        | mc_core::TaskKind::ManageInstance => "后台任务已加入队列。",
                    };
                    self.tasks.insert(0, *task);
                    self.selected_task = 0;
                    self.status = message.to_owned();
                    if let Some(detail) = self.instance_detail.as_mut() {
                        detail.notice = Some(self.status.clone());
                    }
                    self.refresh_requested = true;
                    self.push_transcript(TranscriptEntry::assistant(message));
                }
                Err(error) => {
                    let message = match error.next_action {
                        Some(action) => format!("{} · 下一步：{action}", error.reason),
                        None => error.reason,
                    };
                    if let Some(detail) = self.instance_detail.as_mut() {
                        detail.error = Some(message.clone());
                        detail.notice = Some(message.clone());
                    }
                    self.status = format!("实例后台任务提交失败：{message}");
                    self.push_transcript(TranscriptEntry::error(format!(
                        "后台任务提交失败：{message}"
                    )));
                }
            },
        }
        self.instance_detail_refresh_requested = true;
    }

    pub fn take_instance_detail_query(&mut self) -> Option<InstanceDetailQuery> {
        if !std::mem::take(&mut self.instance_detail_refresh_requested) {
            return None;
        }
        self.instance_detail
            .as_ref()
            .map(InstanceDetailState::query)
            .or_else(|| {
                self.instance_detail_id
                    .map(|instance_id| InstanceDetailQuery {
                        instance_id,
                        after_offset: 0,
                    })
            })
    }

    pub fn take_instance_command(&mut self) -> Option<InstanceCommand> {
        self.instance_command_requested.take()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ServerLifecycleAction {
    Start,
    Stop,
    Restart,
}
