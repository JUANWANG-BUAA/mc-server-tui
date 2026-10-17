use mc_core::AppError;

use crate::instance_detail::{
    InstanceCommand, InstanceCommandResult, InstanceDetailQuery, InstanceDetailState,
    InstanceDetailUpdate,
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
            self.instance_detail = Some(InstanceDetailState::loading(instance_id));
            self.instance_detail_id = Some(instance_id);
        }
        self.instance_detail_visible = true;
        self.instance_detail_refresh_requested = true;
    }

    pub(super) fn close_instance_detail(&mut self) {
        self.instance_detail_visible = false;
        self.instance_detail_refresh_requested = false;
        if let Some(detail) = &mut self.instance_detail {
            detail.cancel_console_input();
        }
    }

    pub(super) fn request_instance_detail_refresh(&mut self) {
        if self.instance_detail_visible {
            self.instance_detail_refresh_requested = true;
        }
    }

    pub(super) fn queue_server_command(&mut self, stop: bool) {
        let Some(instance_id) = self
            .instance_detail_id
            .filter(|_| self.instance_detail_visible)
        else {
            return;
        };
        if self.instance_operation_pending {
            self.status = "上一项服务器操作仍在运行。".to_owned();
            return;
        }
        self.instance_operation_pending = true;
        self.instance_command_requested = Some(if stop {
            InstanceCommand::Stop(instance_id)
        } else {
            InstanceCommand::Start(instance_id)
        });
        self.status = if stop {
            "正在请求优雅停服…".to_owned()
        } else {
            "正在启动服务器…".to_owned()
        };
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

    pub fn apply_instance_detail_result(&mut self, result: Result<InstanceDetailUpdate, AppError>) {
        let Some(instance_id) = self.instance_detail_id else {
            return;
        };
        let detail = self
            .instance_detail
            .get_or_insert_with(|| InstanceDetailState::loading(instance_id));
        detail.apply_update(result);
        if let Some(error) = &detail.error {
            self.status = format!("实例详情刷新失败：{error}");
        } else if let Some(server) = detail.detail.as_ref().map(|detail| &detail.server) {
            self.status = format!("{} · {}", server.address, server.message);
        }
    }

    pub fn apply_instance_command_result(&mut self, result: InstanceCommandResult) {
        self.instance_operation_pending = false;
        match result {
            InstanceCommandResult::Status(result) => {
                if let Some(detail) = self.instance_detail.as_mut() {
                    detail.apply_status(result);
                    if let Some(error) = &detail.error {
                        self.status = format!("生命周期操作失败：{error}");
                        detail.notice = Some(error.clone());
                    } else if let Some(server) = detail.detail.as_ref().map(|data| &data.server) {
                        self.status = format!("{} · {}", server.address, server.message);
                        detail.notice = Some(server.message.clone());
                    }
                }
            }
            InstanceCommandResult::Console(result) => match result {
                Ok(()) => {
                    self.status = "控制台命令已发送。".to_owned();
                    if let Some(detail) = self.instance_detail.as_mut() {
                        detail.notice = Some(self.status.clone());
                    }
                    self.instance_detail_refresh_requested = true;
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
                }
            },
        }
        self.instance_detail_refresh_requested = true;
    }

    pub fn take_instance_detail_query(&mut self) -> Option<InstanceDetailQuery> {
        if !self.instance_detail_visible
            || !std::mem::take(&mut self.instance_detail_refresh_requested)
        {
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
