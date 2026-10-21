//! Management commands for the selected instance; no global hotkeys in the composer.
use crate::app::AppState;
use mc_core::{ServerSnapshot, ServerState};

pub(crate) fn next_actions(app: &AppState, server: Option<&ServerSnapshot>) -> &'static str {
    if app.connected == Some(false) {
        return "/refresh 重连 · /help 帮助";
    }
    if app.instances.is_empty() {
        return if app.has_animated_work() {
            "/tasks 查看建服进度 · /help 帮助"
        } else {
            "/create 创建 Paper · /fabric 创建 Fabric · /help 帮助"
        };
    }
    match server.map(|server| server.state) {
        Some(ServerState::Running) => "/connect 加入方式 · /open 日志 · /backups 备份",
        Some(ServerState::Starting | ServerState::Stopping) => "/open 查看进展 · /logs 最近日志",
        Some(ServerState::Installed | ServerState::Stopped) => {
            "/start 启动 · /config 设置 · /backups 备份"
        }
        Some(ServerState::Failed) => "/open 检查失败原因 · /logs 最近日志",
        _ => "/refresh 刷新状态 · /open 查看详情",
    }
}
