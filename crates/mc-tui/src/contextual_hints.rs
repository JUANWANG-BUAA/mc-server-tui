// 上下文感知的引导提示系统
use mc_core::{ServerState, TaskStatus};
use ratatui::{
    style::Modifier,
    text::{Line, Span},
};

use crate::{app::AppState, theme};

/// 上下文提示类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintType {
    /// 首次使用
    FirstTime,
    /// Java 未找到
    NoJava,
    /// 服务器已停止
    ServerStopped,
    /// 服务器运行中
    ServerRunning,
    /// 服务器启动中
    ServerStarting,
    /// 服务器启动失败
    ServerFailed,
    /// 任务进行中
    TaskRunning,
    /// 任务失败
    TaskFailed,
    /// 常规操作
    Normal,
}

/// 上下文提示
#[derive(Debug, Clone)]
pub struct ContextualHint {
    pub hint_type: HintType,
    pub primary_message: String,
    pub actions: Vec<(String, String)>, // (快捷键, 说明)
}

impl ContextualHint {
    /// 根据应用状态生成上下文提示
    pub fn from_app_state(app: &AppState) -> Self {
        // 首次使用：没有实例也没有任务
        if app.instances.is_empty() && app.tasks.is_empty() {
            return Self {
                hint_type: HintType::FirstTime,
                primary_message: "开始创建你的第一个 Minecraft 服务器".to_owned(),
                actions: vec![
                    ("n".to_owned(), "创建 Paper".to_owned()),
                    ("i".to_owned(), "导入整合包".to_owned()),
                    ("/".to_owned(), "命令面板".to_owned()),
                ],
            };
        }

        // 有任务在运行
        let running_tasks = app
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Running | TaskStatus::Queued))
            .count();
        
        if running_tasks > 0 {
            return Self {
                hint_type: HintType::TaskRunning,
                primary_message: format!("后台任务进行中 ({running_tasks})"),
                actions: vec![
                    ("Tab".to_owned(), "切换到任务".to_owned()),
                    ("Enter".to_owned(), "查看详情".to_owned()),
                    ("r".to_owned(), "刷新".to_owned()),
                ],
            };
        }

        // 检查选中的服务器状态
        if let Some(instance) = app.instances.get(app.selected_instance) {
            if let Some(server) = app.servers.get(&instance.id) {
                return match server.state {
                    ServerState::Stopped => Self {
                        hint_type: HintType::ServerStopped,
                        primary_message: format!("{} 已停止", instance.name),
                        actions: vec![
                            ("Enter".to_owned(), "查看详情".to_owned()),
                            ("s".to_owned(), "启动服务器".to_owned()),
                            ("b".to_owned(), "备份管理".to_owned()),
                        ],
                    },
                    ServerState::Running => Self {
                        hint_type: HintType::ServerRunning,
                        primary_message: format!("{} 运行中", instance.name),
                        actions: vec![
                            ("Enter".to_owned(), "查看详情".to_owned()),
                            ("l".to_owned(), "查看日志".to_owned()),
                            ("c".to_owned(), "打开控制台".to_owned()),
                        ],
                    },
                    ServerState::Starting => Self {
                        hint_type: HintType::ServerStarting,
                        primary_message: format!("{} 启动中...", instance.name),
                        actions: vec![
                            ("Enter".to_owned(), "查看详情".to_owned()),
                            ("l".to_owned(), "查看日志".to_owned()),
                            ("r".to_owned(), "刷新状态".to_owned()),
                        ],
                    },
                    ServerState::Failed => Self {
                        hint_type: HintType::ServerFailed,
                        primary_message: format!("{} 启动失败", instance.name),
                        actions: vec![
                            ("Enter".to_owned(), "查看详情".to_owned()),
                            ("l".to_owned(), "查看错误日志".to_owned()),
                            ("j".to_owned(), "检查 Java".to_owned()),
                        ],
                    },
                    _ => Self::normal(),
                };
            }
        }

        Self::normal()
    }

    /// 默认常规提示
    fn normal() -> Self {
        Self {
            hint_type: HintType::Normal,
            primary_message: "使用快捷键快速操作".to_owned(),
            actions: vec![
                ("↑/↓".to_owned(), "选择".to_owned()),
                ("Enter".to_owned(), "打开".to_owned()),
                ("?".to_owned(), "帮助".to_owned()),
            ],
        }
    }

    /// 渲染为状态栏行
    pub fn render_as_status_line(&self) -> Line<'static> {
        let mut spans = Vec::new();

        // 图标
        let (icon, icon_style) = match self.hint_type {
            HintType::FirstTime => ("◆ ", theme::accent()),
            HintType::NoJava => ("⚠ ", theme::warning()),
            HintType::ServerStopped => ("○ ", theme::muted()),
            HintType::ServerRunning => ("● ", theme::success()),
            HintType::ServerStarting => ("◐ ", theme::warning()),
            HintType::ServerFailed => ("× ", theme::danger()),
            HintType::TaskRunning => ("⟳ ", theme::warning()),
            HintType::TaskFailed => ("× ", theme::danger()),
            HintType::Normal => ("▪ ", theme::muted()),
        };

        spans.push(Span::styled(icon, icon_style));
        spans.push(Span::styled(
            self.primary_message.clone(),
            match self.hint_type {
                HintType::ServerFailed | HintType::TaskFailed | HintType::NoJava => {
                    theme::danger()
                }
                HintType::ServerStarting | HintType::TaskRunning => theme::warning(),
                HintType::ServerRunning => theme::success(),
                HintType::FirstTime => theme::accent().add_modifier(Modifier::BOLD),
                _ => theme::text(),
            },
        ));

        Line::from(spans)
    }

    /// 渲染快捷键提示行
    pub fn render_actions_line(&self) -> Line<'static> {
        let mut spans = Vec::new();

        for (i, (key, desc)) in self.actions.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ·  ", theme::dim()));
            }
            spans.push(Span::styled(
                key.clone(),
                theme::text().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(format!(" {desc}"), theme::muted()));
        }

        Line::from(spans)
    }
}

/// 为实例详情页面生成上下文提示
pub fn hints_for_instance_detail(state: ServerState, has_logs: bool) -> ContextualHint {
    match state {
        ServerState::Stopped => ContextualHint {
            hint_type: HintType::ServerStopped,
            primary_message: "服务器已停止".to_owned(),
            actions: vec![
                ("s".to_owned(), "启动服务器".to_owned()),
                ("b".to_owned(), "备份管理".to_owned()),
                ("p".to_owned(), "编辑配置".to_owned()),
                ("Esc".to_owned(), "返回".to_owned()),
            ],
        },
        ServerState::Running => ContextualHint {
            hint_type: HintType::ServerRunning,
            primary_message: "服务器运行中 · 玩家可以加入".to_owned(),
            actions: vec![
                ("c".to_owned(), "打开控制台".to_owned()),
                ("l".to_owned(), "查看日志".to_owned()),
                ("x".to_owned(), "优雅停止".to_owned()),
                ("Esc".to_owned(), "返回".to_owned()),
            ],
        },
        ServerState::Starting => ContextualHint {
            hint_type: HintType::ServerStarting,
            primary_message: "服务器启动中 · 正在加载世界...".to_owned(),
            actions: vec![
                ("l".to_owned(), "查看启动日志".to_owned()),
                ("r".to_owned(), "刷新状态".to_owned()),
                ("Esc".to_owned(), "返回".to_owned()),
            ],
        },
        ServerState::Failed => {
            let message = if has_logs {
                "启动失败 · 查看日志了解原因"
            } else {
                "启动失败 · 刷新加载日志"
            };
            ContextualHint {
                hint_type: HintType::ServerFailed,
                primary_message: message.to_owned(),
                actions: vec![
                    ("l".to_owned(), "查看错误日志".to_owned()),
                    ("j".to_owned(), "检查 Java".to_owned()),
                    ("p".to_owned(), "检查配置".to_owned()),
                    ("Esc".to_owned(), "返回".to_owned()),
                ],
            }
        }
        _ => ContextualHint {
            hint_type: HintType::Normal,
            primary_message: "查看服务器状态".to_owned(),
            actions: vec![
                ("r".to_owned(), "刷新".to_owned()),
                ("Esc".to_owned(), "返回".to_owned()),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hint_type_classification() {
        // 测试提示类型正确分类
        let hint = ContextualHint {
            hint_type: HintType::FirstTime,
            primary_message: "测试".to_owned(),
            actions: vec![],
        };
        assert_eq!(hint.hint_type, HintType::FirstTime);
    }

    #[test]
    fn test_normal_hint() {
        let hint = ContextualHint::normal();
        assert_eq!(hint.hint_type, HintType::Normal);
        assert!(!hint.actions.is_empty());
    }
}
