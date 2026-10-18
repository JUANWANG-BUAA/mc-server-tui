use crate::app::{AppState, Panel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandId {
    OpenCreate,
    ImportModpack,
    Refresh,
    TogglePanel,
    OpenBackupManager,
    StartServer,
    StopServer,
    CreateBackup,
    VerifyBackup,
    PreviewRestore,
    LoadMoreBackups,
    CancelTask,
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
        let context = if state.backup_manager_visible {
            "世界与备份"
        } else if state.task_detail_visible {
            "任务详情"
        } else if state.instance_detail_visible {
            "实例详情"
        } else if state.focused_panel == Panel::Tasks {
            "后台任务"
        } else {
            "实例列表"
        };
        let mut entries = Vec::new();
        if state.backup_manager_visible {
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
            entries.push(CommandEntry::new(
                CommandId::Back,
                "导航",
                "返回实例详情",
                "保留后台任务并回到服务器状态页",
                Some("Esc"),
                &["back", "返回"],
            ));
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
            entries.push(CommandEntry::new(
                CommandId::Back,
                "导航",
                "返回后台任务列表",
                "关闭任务详情并保留持久任务",
                Some("Esc"),
                &["back", "返回"],
            ));
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
            entries.push(CommandEntry::new(
                CommandId::Back,
                "导航",
                "返回实例列表",
                "离开当前实例详情",
                Some("Esc"),
                &["back", "返回"],
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
                None,
                &["import", "modpack", "mrpack", "整合包", "导入"],
            ));
            entries.push(CommandEntry::new(
                CommandId::Refresh,
                "实例",
                "刷新仪表盘",
                "重新读取实例和后台任务",
                Some("r"),
                &["refresh", "刷新"],
            ));
            entries.push(CommandEntry::new(
                CommandId::TogglePanel,
                "导航",
                "切换实例/任务面板",
                "在实例列表和后台任务之间移动焦点",
                Some("Tab"),
                &["panel", "switch", "面板"],
            ));
        }
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
