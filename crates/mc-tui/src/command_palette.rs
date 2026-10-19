use crate::app::{AppState, Panel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandId {
    OpenCreate,
    ImportModpack,
    OpenSelectedInstance,
    OpenSelectedTask,
    FocusInstances,
    FocusTasks,
    Refresh,
    TogglePanel,
    OpenBackupManager,
    StartServer,
    StopServer,
    BeginConsoleInput,
    CreateBackup,
    VerifyBackup,
    PreviewRestore,
    LoadMoreBackups,
    CancelTask,
    AnalyzeModpack,
    RefreshModpack,
    WizardAdvance,
    WizardBack,
    CancelWizard,
    GoDashboard,
    ShowShell,
    ShowClassic,
    Theme,
    Help,
    Back,
    Quit,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandEntry {
    pub id: CommandId,
    pub group: &'static str,
    pub title: String,
    pub detail: String,
    pub shortcut: Option<&'static str>,
    search: String,
}

impl CommandEntry {
    fn new(
        id: CommandId,
        group: &'static str,
        title: impl Into<String>,
        detail: impl Into<String>,
        shortcut: Option<&'static str>,
        aliases: &[&str],
    ) -> Self {
        let title = title.into();
        let detail = detail.into();
        let search = title.to_lowercase()
            + " "
            + &detail.to_lowercase()
            + " "
            + &aliases.join(" ").to_lowercase();
        Self {
            id,
            group,
            title,
            detail,
            shortcut,
            search,
        }
    }
}

#[derive(Debug)]
pub(crate) struct CommandPaletteState {
    pub query: String,
    pub context: &'static str,
    pub entries: Vec<CommandEntry>,
    filtered: Vec<usize>,
    pub selected: usize,
}

impl CommandPaletteState {
    pub fn for_state(state: &AppState) -> Self {
        let context = if state.wizard.is_some() {
            "新建服务器向导"
        } else if state.modpack_import_visible {
            "整合包导入"
        } else if state.backup_manager_visible {
            "世界与备份"
        } else if state.task_detail_visible {
            "任务详情"
        } else if state.instance_detail_visible {
            "实例详情"
        } else if state.shell_visible {
            "会话终端"
        } else if state.focused_panel == Panel::Tasks {
            "后台任务"
        } else {
            "实例列表"
        };
        let mut entries = Vec::new();
        if state.shell_visible
            && state.wizard.is_none()
            && !state.modpack_import_visible
            && !state.backup_manager_visible
            && !state.task_detail_visible
            && !state.instance_detail_visible
        {
            entries.push(CommandEntry::new(
                CommandId::OpenCreate,
                "创建",
                "新建 Paper 服务器",
                "打开结构化向导；版本与 Java 自动发现",
                Some("/create"),
                &["new", "create", "paper", "新建", "创建"],
            ));
            entries.push(CommandEntry::new(
                CommandId::ImportModpack,
                "创建",
                "导入整合包",
                "扫描 .mrpack 并执行只读兼容性检查",
                Some("/import"),
                &["import", "modpack", "mrpack", "整合包"],
            ));
            entries.push(CommandEntry::new(
                CommandId::OpenSelectedInstance,
                "导航",
                "打开所选实例",
                "进入实例详情：日志、启停、备份和命令输入",
                Some("/open"),
                &["open", "instance", "打开", "实例"],
            ));
            entries.push(CommandEntry::new(
                CommandId::OpenSelectedTask,
                "导航",
                "打开所选后台任务",
                "查看字节进度、事件游标和取消入口",
                Some("/task"),
                &["open", "task", "任务"],
            ));
            entries.push(CommandEntry::new(
                CommandId::Refresh,
                "实例",
                "刷新实例与任务",
                "立即重新读取仪表盘和选中实例状态",
                Some("/refresh"),
                &["refresh", "刷新"],
            ));
            entries.push(CommandEntry::new(
                CommandId::ShowClassic,
                "导航",
                "切换到经典双面板",
                "保留会话记录，切回实例/任务双面板布局",
                Some("/classic"),
                &["classic", "dashboard", "经典", "面板"],
            ));
        } else if state.wizard.is_some() {
            entries.push(CommandEntry::new(
                CommandId::WizardAdvance,
                "向导",
                "下一步",
                "校验当前字段并进入下一个安装步骤",
                Some("Enter"),
                &["next", "advance", "下一步"],
            ));
            entries.push(CommandEntry::new(
                CommandId::WizardBack,
                "向导",
                "返回上一步",
                "保留已填写内容回到上一个安装步骤",
                Some("Esc"),
                &["back", "previous", "上一步", "返回"],
            ));
            entries.push(CommandEntry::new(
                CommandId::CancelWizard,
                "向导",
                "取消创建向导",
                "不创建实例，立即回到仪表盘",
                None,
                &["cancel", "close", "取消", "关闭"],
            ));
        } else if state.modpack_import_visible {
            entries.push(CommandEntry::new(
                CommandId::AnalyzeModpack,
                "整合包",
                "检查所选整合包",
                "只读分析 .mrpack 的版本、加载器、Mod 与文件风险",
                Some("Enter"),
                &["analyze", "inspect", "分析", "检查"],
            ));
            entries.push(CommandEntry::new(
                CommandId::RefreshModpack,
                "整合包",
                "重新扫描本地整合包",
                "刷新 Downloads、桌面和当前目录中的候选文件",
                Some("r"),
                &["scan", "refresh", "扫描", "刷新"],
            ));
        } else if state.backup_manager_visible {
            entries.push(CommandEntry::new(
                CommandId::Refresh,
                "世界与备份",
                "刷新备份目录",
                "重新读取并校验第一页备份快照",
                Some("r"),
                &["backup", "list", "刷新"],
            ));
            entries.push(CommandEntry::new(
                CommandId::CreateBackup,
                "世界与备份",
                "创建停机世界备份",
                "停止状态校验通过后复制并校验世界目录",
                Some("b"),
                &["backup", "snapshot", "备份"],
            ));
            entries.push(CommandEntry::new(
                CommandId::VerifyBackup,
                "世界与备份",
                "校验所选备份",
                "重新哈希选中快照并与清单比对",
                Some("v"),
                &["verify", "hash", "校验"],
            ));
            entries.push(CommandEntry::new(
                CommandId::PreviewRestore,
                "世界与备份",
                "预览恢复影响",
                "生成目录替换、空间和安全事务预览",
                Some("Enter"),
                &["restore", "recovery", "恢复"],
            ));
            if state
                .backup_manager
                .as_ref()
                .is_some_and(|manager| manager.has_more())
            {
                entries.push(CommandEntry::new(
                    CommandId::LoadMoreBackups,
                    "世界与备份",
                    "加载更多备份",
                    "使用稳定 cursor 读取下一页",
                    Some("m"),
                    &["more", "next", "更多"],
                ));
            }
        } else if state.task_detail_visible {
            entries.push(CommandEntry::new(
                CommandId::Refresh,
                "任务",
                "刷新任务与事件",
                "立即读取最新阶段、进度和事件游标",
                Some("r"),
                &["refresh", "event", "刷新"],
            ));
            if state
                .task_detail
                .as_ref()
                .is_some_and(|detail| detail.can_cancel())
            {
                entries.push(CommandEntry::new(
                    CommandId::CancelTask,
                    "任务",
                    "取消下载任务",
                    "保留已完成的临时文件，等待安全中断",
                    Some("x"),
                    &["cancel", "pause", "取消", "暂停"],
                ));
            }
        } else if state.instance_detail_visible {
            entries.push(CommandEntry::new(
                CommandId::StartServer,
                "实例",
                "启动服务器",
                "执行 Java、端口和就绪预检后启动",
                Some("s"),
                &["start", "run", "启动"],
            ));
            entries.push(CommandEntry::new(
                CommandId::StopServer,
                "实例",
                "优雅停止服务器",
                "发送 stop 并等待进程确认退出",
                Some("x"),
                &["stop", "shutdown", "停止"],
            ));
            entries.push(CommandEntry::new(
                CommandId::BeginConsoleInput,
                "控制台",
                "发送控制台命令",
                "打开命令输入行，向运行中的服务端发送指令",
                Some(":"),
                &["console", "command", "控制台", "命令"],
            ));
            entries.push(CommandEntry::new(
                CommandId::CreateBackup,
                "世界与备份",
                "创建世界备份",
                "提交停机一致性备份任务",
                Some("b"),
                &["backup", "snapshot", "备份"],
            ));
            entries.push(CommandEntry::new(
                CommandId::OpenBackupManager,
                "世界与备份",
                "打开世界与备份",
                "浏览、校验并恢复已验证快照",
                Some("l"),
                &["backup", "restore", "恢复"],
            ));
            entries.push(CommandEntry::new(
                CommandId::Refresh,
                "实例",
                "刷新实例状态",
                "读取服务器状态和增量控制台日志",
                Some("r"),
                &["refresh", "logs", "刷新"],
            ));
        } else {
            entries.push(CommandEntry::new(
                CommandId::OpenCreate,
                "创建",
                "新建 Paper 服务器",
                "使用推荐版本、自动检测 Java 和资源预设",
                Some("n"),
                &["new", "create", "paper", "新建", "创建"],
            ));
            entries.push(CommandEntry::new(
                CommandId::ImportModpack,
                "创建",
                "导入整合包",
                "选择 .mrpack 并静态检查版本、加载器、Mod 和文件分流",
                Some("i"),
                &["import", "modpack", "mrpack", "整合包", "导入"],
            ));
            entries.push(CommandEntry::new(
                CommandId::OpenSelectedInstance,
                "导航",
                "打开所选实例",
                "进入实例详情，查看状态、日志和生命周期操作",
                Some("Enter"),
                &["open", "instance", "打开", "实例"],
            ));
            entries.push(CommandEntry::new(
                CommandId::OpenSelectedTask,
                "导航",
                "打开所选后台任务",
                "查看字节进度、事件游标和取消入口",
                Some("Enter"),
                &["open", "task", "打开", "任务"],
            ));
            entries.push(CommandEntry::new(
                CommandId::FocusInstances,
                "导航",
                "聚焦实例面板",
                "把选择和回车操作切换到实例列表",
                Some("1"),
                &["focus", "instance", "聚焦", "实例"],
            ));
            entries.push(CommandEntry::new(
                CommandId::FocusTasks,
                "导航",
                "聚焦后台任务面板",
                "把选择和回车操作切换到任务列表",
                Some("2"),
                &["focus", "task", "聚焦", "任务"],
            ));
            entries.push(CommandEntry::new(
                CommandId::TogglePanel,
                "导航",
                "切换实例/任务面板",
                "在实例列表和后台任务之间移动焦点",
                Some("Tab"),
                &["panel", "switch", "面板"],
            ));
            entries.push(CommandEntry::new(
                CommandId::ShowShell,
                "导航",
                "切换到会话终端",
                "打开 Claude Code 风格的会话日志与命令输入框",
                Some("t"),
                &["shell", "session", "terminal", "会话", "终端"],
            ));
            entries.push(CommandEntry::new(
                CommandId::Refresh,
                "实例",
                "刷新仪表盘",
                "重新读取实例和后台任务",
                Some("r"),
                &["refresh", "刷新"],
            ));
        }

        if state.wizard.is_some() {
            entries.push(CommandEntry::new(
                CommandId::CancelWizard,
                "导航",
                "回到仪表盘",
                "取消当前向导并返回仪表盘",
                None,
                &["dashboard", "home", "仪表盘", "首页"],
            ));
        } else if state.modpack_import_visible {
            entries.push(CommandEntry::new(
                CommandId::Back,
                "导航",
                "返回仪表盘",
                "关闭整合包检查页并回到实例列表",
                Some("Esc"),
                &["back", "dashboard", "返回"],
            ));
        } else if state.backup_manager_visible {
            entries.push(CommandEntry::new(
                CommandId::Back,
                "导航",
                "返回实例详情",
                "保留后台任务并回到服务器状态页",
                Some("Esc"),
                &["back", "返回"],
            ));
            entries.push(CommandEntry::new(
                CommandId::GoDashboard,
                "导航",
                "回到仪表盘",
                "关闭备份管理器并回到实例列表",
                Some("d"),
                &["dashboard", "home", "仪表盘"],
            ));
        } else if state.task_detail_visible {
            entries.push(CommandEntry::new(
                CommandId::Back,
                "导航",
                "返回后台任务列表",
                "关闭任务详情并保留持久任务",
                Some("Esc"),
                &["back", "返回"],
            ));
            entries.push(CommandEntry::new(
                CommandId::GoDashboard,
                "导航",
                "回到仪表盘",
                "关闭任务详情并回到实例列表",
                Some("d"),
                &["dashboard", "home", "仪表盘"],
            ));
        } else if state.instance_detail_visible {
            entries.push(CommandEntry::new(
                CommandId::Back,
                "导航",
                "返回实例列表",
                "离开当前实例详情",
                Some("Esc"),
                &["back", "返回"],
            ));
            entries.push(CommandEntry::new(
                CommandId::GoDashboard,
                "导航",
                "回到仪表盘",
                "关闭实例详情并回到实例列表",
                Some("d"),
                &["dashboard", "home", "仪表盘"],
            ));
        }

        entries.push(CommandEntry::new(
            CommandId::Theme,
            "外观",
            format!("主题：{}", crate::theme::current_theme().name()),
            "在 7 套主题之间循环切换；/theme 可列出全部主题",
            Some("F2"),
            &["theme", "color", "主题", "外观", "配色"],
        ));
        entries.push(CommandEntry::new(
            CommandId::Help,
            "帮助",
            "键盘快捷键与鼠标操作",
            "查看当前所有页面可用的按键、滚动和点击方式",
            Some("?"),
            &["help", "keys", "shortcut", "帮助", "快捷键"],
        ));
        entries.push(CommandEntry::new(
            CommandId::Quit,
            "应用",
            "退出 TUI",
            "持久任务会继续在后台运行",
            Some("q"),
            &["quit", "exit", "退出"],
        ));
        let mut palette = Self {
            query: String::new(),
            context,
            entries,
            filtered: Vec::new(),
            selected: 0,
        };
        palette.refresh_filter();
        palette
    }

    pub fn insert(&mut self, character: char) {
        if character.is_control() || self.query.len() >= 64 {
            return;
        }
        self.query.push(character);
        self.refresh_filter();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.refresh_filter();
    }

    pub fn select_next(&mut self) {
        if self.filtered.is_empty() {
            self.selected = 0;
        } else {
            self.selected = (self.selected + 1).min(self.filtered.len() - 1);
        }
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_edge(&mut self, last: bool) {
        self.selected = if last {
            self.filtered.len().saturating_sub(1)
        } else {
            0
        };
    }

    pub fn filtered_len(&self) -> usize {
        self.filtered.len()
    }

    pub fn selected_entry(&self) -> Option<&CommandEntry> {
        self.filtered
            .get(self.selected)
            .and_then(|index| self.entries.get(*index))
    }

    pub fn filtered_entries(&self) -> impl Iterator<Item = &CommandEntry> {
        self.filtered
            .iter()
            .filter_map(|index| self.entries.get(*index))
    }

    fn refresh_filter(&mut self) {
        let query = self.query.trim().to_lowercase();
        if query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
            return;
        }
        let mut matched: Vec<(usize, i32)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                fuzzy_score(&query, &entry.search).map(|score| (index, score))
            })
            .collect();
        matched.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| self.entries[left.0].title.cmp(&self.entries[right.0].title))
        });
        self.filtered = matched.into_iter().map(|(index, _)| index).collect();
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }
}

fn fuzzy_score(query: &str, haystack: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    if let Some(position) = haystack.find(query) {
        return Some(1000 - position.min(999) as i32);
    }
    let mut score = 0;
    let mut last = None;
    for character in query.chars() {
        let mut found = None;
        for (index, candidate) in haystack.char_indices() {
            if last.is_some_and(|last_index| index <= last_index) {
                continue;
            }
            if candidate == character {
                found = Some(index);
                break;
            }
        }
        let index = found?;
        if last.is_some_and(|last_index| index == last_index + 1) {
            score += 4;
        }
        score += 1;
        last = Some(index);
    }
    Some(score)
}

#[cfg(test)]
mod tests;
