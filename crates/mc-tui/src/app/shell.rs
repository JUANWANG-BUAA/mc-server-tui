use mc_core::TaskStatus;
use uuid::Uuid;

use crate::{
    instance_detail::InstanceDetailState,
    shell::{self, ShellCommandSpec},
    transcript::TranscriptEntry,
    wizard::{WizardAction, WizardStep},
};

use super::{Action, AppState, Panel};

const MAX_TRANSCRIPT_ENTRIES: usize = 300;
const MAX_HISTORY: usize = 100;

impl AppState {
    pub(crate) fn paste_text(&mut self, text: &str) {
        if self.help_visible || self.error_dialog.is_some() {
            return;
        }

        let characters = normalize_pasted_text(text);
        if characters.is_empty() {
            return;
        }

        let mut pasted = false;
        if let Some(palette) = self.command_palette.as_mut() {
            for character in characters.iter().copied() {
                palette.insert(character);
            }
            pasted = true;
        } else if self.modpack_import_visible || self.task_detail_visible {
            return;
        } else if let Some(wizard) = self.wizard.as_mut() {
            if matches!(
                (wizard.step, wizard.focused_field),
                (WizardStep::Server, 0) | (WizardStep::Runtime, 2)
            ) {
                for character in characters.iter().copied() {
                    wizard.handle(WizardAction::Insert(character));
                }
                pasted = true;
            }
        } else if self.backup_manager_visible {
            if let Some(manager) = self
                .backup_manager
                .as_mut()
                .filter(|manager| manager.editing_confirmation)
            {
                for character in characters.iter().copied() {
                    manager.insert_confirmation_char(character);
                }
                pasted = true;
            }
        } else if self.server_properties_visible {
            if let Some(editor) = self.server_properties.as_mut() {
                if editor.phase == crate::server_properties::EditorPhase::Edit {
                    for character in characters.iter().copied() {
                        editor.insert(character);
                    }
                    pasted = true;
                }
            }
        } else if self.instance_detail_visible {
            if let Some(detail) = self
                .instance_detail
                .as_mut()
                .filter(|detail| detail.editing_console || detail.editing_filter)
            {
                for character in characters.iter().copied() {
                    detail.insert_console_char(character);
                }
                pasted = true;
            }
        } else if self.shell_visible {
            for character in characters.iter().copied() {
                self.shell_insert(character);
            }
            pasted = true;
        }

        if pasted && has_line_break(text) {
            self.status = "多行粘贴已合并为单行输入，请检查当前目标和内容后再提交。".to_owned();
        }
    }

    pub(crate) fn shell_suggestions(&self) -> Vec<&'static ShellCommandSpec> {
        shell::suggestions(&self.shell_input)
    }

    pub(super) fn shell_insert(&mut self, character: char) {
        if character.is_control() || self.shell_input.chars().count() >= 512 {
            return;
        }
        if self.shell_input.is_empty() {
            self.shell_draft_target = self.current_instance().map(|instance| instance.id);
        }
        let cursor = self.shell_cursor.min(self.shell_input.len());
        self.shell_input.insert(cursor, character);
        self.shell_cursor = cursor + character.len_utf8();
        self.shell_suggestion_index = 0;
        self.shell_history_pos = None;
    }

    pub(super) fn shell_backspace(&mut self) {
        if self.shell_cursor == 0 {
            return;
        }
        let previous = self
            .shell_input
            .get(..self.shell_cursor)
            .and_then(|text| text.char_indices().last().map(|(index, _)| index))
            .unwrap_or(0);
        self.shell_input
            .replace_range(previous..self.shell_cursor, "");
        self.shell_cursor = previous;
        self.shell_suggestion_index = 0;
        self.shell_history_pos = None;
    }

    pub(super) fn shell_delete(&mut self) {
        let cursor = self.shell_cursor.min(self.shell_input.len());
        let Some(character) = self
            .shell_input
            .get(cursor..)
            .and_then(|text| text.chars().next())
        else {
            return;
        };
        self.shell_input
            .replace_range(cursor..cursor + character.len_utf8(), "");
        self.shell_suggestion_index = 0;
        self.shell_history_pos = None;
    }

    pub(super) fn shell_cursor_left(&mut self) {
        self.shell_cursor = self
            .shell_input
            .get(..self.shell_cursor)
            .and_then(|text| text.char_indices().last().map(|(index, _)| index))
            .unwrap_or(0);
    }

    pub(super) fn shell_cursor_right(&mut self) {
        let cursor = self.shell_cursor.min(self.shell_input.len());
        self.shell_cursor = self
            .shell_input
            .get(cursor..)
            .and_then(|text| text.chars().next())
            .map(|character| cursor + character.len_utf8())
            .unwrap_or(cursor);
    }

    pub(super) fn shell_cursor_home(&mut self) {
        self.shell_cursor = 0;
    }

    pub(super) fn shell_cursor_end(&mut self) {
        self.shell_cursor = self.shell_input.len();
    }

    pub(super) fn shell_up(&mut self) {
        let suggestions = self.shell_suggestions();
        if suggestions.is_empty() {
            self.shell_history_previous();
        } else {
            let length = suggestions.len();
            self.shell_suggestion_index = (self.shell_suggestion_index + length - 1) % length;
        }
    }

    pub(super) fn shell_down(&mut self) {
        let suggestions = self.shell_suggestions();
        if suggestions.is_empty() {
            self.shell_history_next();
        } else {
            let length = suggestions.len();
            self.shell_suggestion_index = (self.shell_suggestion_index + 1) % length;
        }
    }

    pub(super) fn shell_complete(&mut self) {
        let suggestions = self.shell_suggestions();
        let Some(command) = suggestions.get(
            self.shell_suggestion_index
                .min(suggestions.len().saturating_sub(1)),
        ) else {
            return;
        };
        self.shell_input = format!("/{} ", command.name);
        self.shell_cursor = self.shell_input.len();
        self.shell_suggestion_index = 0;
        self.shell_history_pos = None;
    }

    pub(super) fn shell_escape(&mut self) {
        self.shell_draft_target = None;
        self.shell_input.clear();
        self.shell_cursor = 0;
        self.shell_suggestion_index = 0;
        self.shell_history_pos = None;
    }

    pub(super) fn shell_clear(&mut self) {
        self.transcript.clear();
        self.shell_scroll = 0;
        self.push_transcript(TranscriptEntry::muted("会话已清空。"));
    }

    pub(super) fn shell_submit(&mut self) {
        let raw = self.shell_input.trim().to_owned();
        if raw.is_empty() {
            return;
        }
        let console = !raw.starts_with('/')
            || raw
                .split_whitespace()
                .next()
                .and_then(|name| name.strip_prefix('/'))
                .and_then(shell::find)
                .is_some_and(|command| command.name == "console");
        if console
            && self.shell_draft_target.is_some_and(|id| {
                self.current_instance()
                    .is_none_or(|instance| instance.id != id)
            })
        {
            self.status =
                "控制台草稿属于之前的服务器；切回原实例发送，或 Esc 清空后重新输入。".to_owned();
            self.push_transcript(TranscriptEntry::warning(self.status.clone()));
            return;
        }
        if raw.starts_with('/') && !raw.contains(char::is_whitespace) {
            let name = raw.trim_start_matches('/');
            if shell::find(name).is_none() {
                let suggestions = self.shell_suggestions();
                if let Some(command) = suggestions.get(
                    self.shell_suggestion_index
                        .min(suggestions.len().saturating_sub(1)),
                ) {
                    self.shell_input = format!("/{}", command.name);
                    self.shell_cursor = self.shell_input.len();
                    self.shell_suggestion_index = 0;
                    self.shell_history_pos = None;
                    self.status = format!("已补全为 {}；再次按 Enter 执行。", self.shell_input);
                    return;
                }
            }
        }
        self.shell_input.clear();
        self.shell_draft_target = None;
        self.shell_cursor = 0;
        self.shell_suggestion_index = 0;
        self.shell_history_pos = None;
        if self.shell_history.last() != Some(&raw) {
            self.shell_history.push(raw.clone());
            if self.shell_history.len() > MAX_HISTORY {
                self.shell_history.remove(0);
            }
        }
        self.shell_scroll = 0;
        self.push_transcript(TranscriptEntry::user(&raw));
        if raw.starts_with('/') {
            self.execute_shell_command(&raw);
        } else {
            self.execute_console_text(&raw);
        }
    }

    pub(super) fn show_shell(&mut self) {
        self.shell_visible = true;
        self.command_palette = None;
        self.help_visible = false;
        self.push_transcript(TranscriptEntry::assistant("已切换到会话终端。"));
        self.shell_scroll = 0;
    }

    pub(super) fn show_classic(&mut self) {
        self.shell_visible = false;
        self.status = "已切换到经典面板；按 t 或命令面板可返回会话终端。".to_owned();
    }

    pub(super) fn scroll_shell(&mut self, delta: isize) {
        let max = self.shell_max_scroll.get() as isize;
        let next = (self.shell_scroll as isize + delta).clamp(0, max.max(0));
        self.shell_scroll = next as usize;
    }

    pub(super) fn jump_shell(&mut self, latest: bool) {
        self.shell_scroll = if latest {
            0
        } else {
            self.shell_max_scroll.get()
        };
    }

    pub(crate) fn push_transcript(&mut self, entry: TranscriptEntry) {
        let added_lines = entry.line_count(self.shell_text_width.get()) + 1;
        if self.shell_scroll > 0 {
            self.shell_scroll = self.shell_scroll.saturating_add(added_lines);
        }
        self.transcript.push(entry);
        if self.transcript.len() > MAX_TRANSCRIPT_ENTRIES {
            let discard = self.transcript.len() - MAX_TRANSCRIPT_ENTRIES;
            self.transcript.drain(..discard);
        }
    }

    fn transcript_line_count(&self) -> usize {
        self.transcript
            .iter()
            .map(|entry| entry.line_count(self.shell_text_width.get()))
            .sum::<usize>()
            + self.transcript.len().saturating_sub(1)
    }

    fn push_transcript_reveal_top(&mut self, entry: TranscriptEntry) {
        let before_total = self.transcript_line_count();
        let start = if self.transcript.is_empty() {
            0
        } else {
            before_total + 1
        };
        let added_lines = entry.line_count(self.shell_text_width.get()) + 1;
        self.transcript.push(entry);
        if self.transcript.len() > MAX_TRANSCRIPT_ENTRIES {
            let discard = self.transcript.len() - MAX_TRANSCRIPT_ENTRIES;
            self.transcript.drain(..discard);
        }
        let total = self.transcript_line_count();
        let viewport = self.shell_viewport.get().max(1);
        self.shell_scroll = total
            .saturating_sub(start + viewport)
            .min(total.saturating_sub(viewport));
        let _ = added_lines;
    }

    fn shell_history_previous(&mut self) {
        if self.shell_history.is_empty() {
            return;
        }
        let next = match self.shell_history_pos {
            Some(index) => index.saturating_sub(1),
            None => self.shell_history.len() - 1,
        };
        self.shell_history_pos = Some(next);
        self.shell_input = self.shell_history[next].clone();
        self.shell_draft_target = self.current_instance().map(|instance| instance.id);
        self.shell_cursor = self.shell_input.len();
    }

    fn shell_history_next(&mut self) {
        let Some(index) = self.shell_history_pos else {
            return;
        };
        if index + 1 < self.shell_history.len() {
            self.shell_history_pos = Some(index + 1);
            self.shell_input = self.shell_history[index + 1].clone();
            self.shell_draft_target = self.current_instance().map(|instance| instance.id);
            self.shell_cursor = self.shell_input.len();
        } else {
            self.shell_history_pos = None;
            self.shell_input.clear();
            self.shell_cursor = 0;
        }
    }

    fn current_instance(&self) -> Option<&mc_core::InstanceSummary> {
        self.instances.get(self.selected_instance)
    }

    fn execute_console_text(&mut self, text: &str) {
        let Some(name) = self
            .current_instance()
            .map(|instance| instance.name.clone())
        else {
            self.push_transcript(TranscriptEntry::error(
                "当前没有选中的实例。先输入 /create 新建，或 /use <序号> 选择。",
            ));
            return;
        };
        self.push_transcript(TranscriptEntry::assistant(format!(
            "发送到「{name}」控制台：{text}"
        )));
        self.queue_console_command(text.to_owned());
        self.push_transcript(TranscriptEntry::muted(self.status.clone()));
    }

    fn execute_shell_command(&mut self, raw: &str) {
        let without_slash = raw.trim_start_matches('/');
        let mut parts = without_slash.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or_default();
        let args = parts.next().unwrap_or_default().trim();
        let Some(command) = shell::find(name) else {
            let suggestions = shell::suggestions(raw)
                .iter()
                .map(|command| format!("/{}", command.name))
                .collect::<Vec<_>>();
            let hint = if suggestions.is_empty() {
                "输入 /help 查看所有命令。".to_owned()
            } else {
                format!("你是不是想用：{}", suggestions.join(" · "))
            };
            self.push_transcript(TranscriptEntry::error(format!("未知命令 /{name}。{hint}")));
            self.push_transcript(TranscriptEntry::muted(
                "提示：Minecraft 控制台命令不加前导 /，例如直接输入 say hello。",
            ));
            return;
        };

        match command.name {
            "pages" | "page" | "close" => self.workspace_command(command.name, args),
            "connect" | "monitor" | "rg" | "players" | "mods" | "export" | "fabric" => {
                self.manage_shell(command.name, args)
            }
            "help" => {
                self.push_transcript_reveal_top(TranscriptEntry::assistant(shell::help_text()))
            }
            "status" => {
                let text = self.shell_status_text();
                self.push_transcript_reveal_top(TranscriptEntry::assistant(text));
            }
            "instances" => {
                let text = self.shell_instances_text();
                self.push_transcript_reveal_top(TranscriptEntry::assistant(text));
            }
            "use" => self.shell_use(args),
            "open" => {
                if self.current_instance().is_none() {
                    self.push_transcript(TranscriptEntry::error(
                        "当前没有实例。先输入 /create 新建，或 /instances 查看。",
                    ));
                    return;
                }
                self.focused_panel = Panel::Instances;
                self.update(Action::OpenInstanceDetail);
            }
            "config" => self.update(Action::OpenServerProperties),
            "logs" => {
                if args.is_empty() {
                    self.shell_tail_logs()
                } else {
                    self.manage_shell("rg", args)
                }
            }
            "start" => self.shell_server_command(false),
            "stop" => self.shell_server_command(true),
            "restart" => self.shell_restart_command(),
            "console" => {
                if args.is_empty() {
                    self.push_transcript(TranscriptEntry::warning("用法：/console <命令>"));
                    return;
                }
                self.execute_console_text(args);
            }
            "tasks" => {
                let text = self.shell_tasks_text();
                self.push_transcript_reveal_top(TranscriptEntry::assistant(text));
            }
            "task" => self.shell_open_task(args),
            "cancel" => self.shell_cancel_task(args),
            "create" => self.update(Action::OpenCreate),
            "import" if !args.is_empty() => self.manage_shell("import", args),
            "import" => self.update(Action::OpenModpackImport),
            "backup" => {
                if self.current_instance().is_none() {
                    self.push_transcript(TranscriptEntry::error("当前没有选中的实例。"));
                    return;
                }
                self.update(Action::CreateBackup);
                self.push_transcript(TranscriptEntry::muted(self.status.clone()));
            }
            "backups" => {
                if self.current_instance().is_none() {
                    self.push_transcript(TranscriptEntry::error("当前没有选中的实例。"));
                    return;
                }
                self.focused_panel = Panel::Instances;
                self.update(Action::OpenInstanceDetail);
                self.update(Action::OpenBackupManager);
                self.push_transcript(TranscriptEntry::assistant(
                    "已打开世界与备份；Esc 逐层返回会话终端。",
                ));
            }
            "refresh" => {
                self.update(Action::Refresh);
                self.push_transcript(TranscriptEntry::muted("已请求刷新实例与后台任务。"));
            }
            "clear" => self.shell_clear(),
            "palette" => self.update(Action::OpenCommandPalette),
            "theme" => {
                if args.is_empty() {
                    let current = crate::theme::current_theme();
                    let mut text = String::from("可用主题（F2 循环，/theme <name> 选择）\n");
                    for theme in crate::theme::ThemeId::ALL {
                        let marker = if theme == current { "›" } else { " " };
                        text.push_str(&format!(
                            "{marker} {:<9} ({})\n",
                            theme.name(),
                            theme.slug()
                        ));
                    }
                    self.push_transcript_reveal_top(TranscriptEntry::assistant(text));
                } else if let Some(theme) = crate::theme::ThemeId::from_slug(args) {
                    self.update(Action::SetTheme(theme));
                } else {
                    let names = crate::theme::ThemeId::ALL
                        .iter()
                        .map(|theme| theme.slug())
                        .collect::<Vec<_>>()
                        .join("|");
                    self.push_transcript(TranscriptEntry::warning(format!(
                        "未知主题「{args}」。可用：{names}"
                    )));
                }
            }
            "classic" => self.update(Action::ShowClassic),
            "quit" => self.update(Action::Quit),
            _ => {}
        }
    }

    fn shell_tail_logs(&mut self) {
        let Some(detail) = self.instance_detail.as_ref() else {
            self.push_transcript(TranscriptEntry::warning(
                "尚未读取到实例状态。先 /use 选择实例，或 /open 打开完整实例页。",
            ));
            return;
        };
        let logs = detail.logs.trim();
        if logs.is_empty() {
            self.push_transcript(TranscriptEntry::warning(
                "还没有收到控制台输出。服务器启动后，这里会显示最近 12 行日志。",
            ));
            return;
        }
        let lines = logs.lines().rev().take(12).collect::<Vec<_>>();
        let mut text = String::from("最近控制台日志\n");
        for line in lines.into_iter().rev() {
            text.push_str(line);
            text.push('\n');
        }
        text.push_str("输入 /open 打开完整实例页；直接输入文本可发送控制台命令。");
        self.push_transcript(TranscriptEntry::assistant(text));
    }

    fn shell_server_command(&mut self, stop: bool) {
        let Some(instance) = self.current_instance().cloned() else {
            self.push_transcript(TranscriptEntry::error("当前没有选中的实例。"));
            return;
        };
        let verb = if stop { "停止" } else { "启动" };
        self.push_transcript(TranscriptEntry::assistant(format!(
            "正在{verb}「{}」…",
            instance.name
        )));
        if stop {
            self.update(Action::StopServer);
        } else {
            self.update(Action::StartServer);
        }
        let status = self.status.clone();
        if status.contains("上一项") {
            self.push_transcript(TranscriptEntry::warning(status));
        }
    }

    fn shell_restart_command(&mut self) {
        let Some(instance) = self.current_instance().cloned() else {
            self.push_transcript(TranscriptEntry::error("当前没有选中的实例。"));
            return;
        };
        self.push_transcript(TranscriptEntry::assistant(format!(
            "正在受控重启「{}」：先优雅停止，确认旧进程退出后再启动…",
            instance.name
        )));
        self.update(Action::RestartServer);
        let status = self.status.clone();
        if status.contains("上一项") || status.contains("当前没有") {
            self.push_transcript(TranscriptEntry::warning(status));
        }
    }

    fn shell_use(&mut self, args: &str) {
        if self.instances.is_empty() {
            self.push_transcript(TranscriptEntry::error(
                "还没有服务器。输入 /create 新建 Paper 服务器。",
            ));
            return;
        }
        let arg = args.trim();
        if arg.is_empty() {
            self.push_transcript(TranscriptEntry::warning(
                "用法：/use <序号|名称>；输入 /instances 查看列表。",
            ));
            return;
        }
        let index = arg
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .filter(|index| *index < self.instances.len())
            .or_else(|| {
                self.instances
                    .iter()
                    .position(|instance| instance.name.eq_ignore_ascii_case(arg))
            })
            .or_else(|| {
                let needle = arg.to_lowercase();
                self.instances
                    .iter()
                    .position(|instance| instance.name.to_lowercase().contains(&needle))
            });
        let Some(index) = index else {
            self.push_transcript(TranscriptEntry::error(format!(
                "找不到实例「{arg}」。输入 /instances 查看可选实例。"
            )));
            return;
        };
        let instance = self.instances[index].clone();
        self.selected_instance = index;
        self.focused_panel = Panel::Instances;
        self.select_shell_instance(&instance.id);
        self.push_transcript(TranscriptEntry::success(format!(
            "已选择 #{}「{}」· {} {} · :{}",
            index + 1,
            instance.name,
            instance.provider.to_ascii_lowercase(),
            instance.game_version,
            instance.port
        )));
    }

    pub(crate) fn select_shell_instance(&mut self, instance_id: &Uuid) {
        if self.instance_detail_id != Some(*instance_id) {
            self.instance_detail_id = Some(*instance_id);
            if let Some(previous) = self.instance_detail.take() {
                self.instance_cache.insert(previous.instance_id, previous);
            }
            self.instance_detail = Some(
                self.instance_cache
                    .remove(instance_id)
                    .unwrap_or_else(|| InstanceDetailState::loading(*instance_id)),
            );
        }
        self.instance_detail_refresh_requested = true;
    }

    fn shell_cancel_task(&mut self, args: &str) {
        let index = args
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .unwrap_or_else(|| self.selected_task.min(self.tasks.len().saturating_sub(1)));
        let Some(task) = self.tasks.get(index).cloned() else {
            self.push_transcript(TranscriptEntry::warning("当前没有可取消的后台任务。"));
            return;
        };
        if !crate::task_presentation::is_cancelable_download(&task) {
            self.push_transcript(TranscriptEntry::warning(format!(
                "任务 #{} 不是可取消的下载任务；用 /tasks 查看状态。",
                index + 1
            )));
            return;
        }
        self.task_cancel_requested = Some(task.id);
        self.push_transcript(TranscriptEntry::warning(format!(
            "已请求取消任务 #{}「{}」；已完成的临时文件会保留用于续传。",
            index + 1,
            crate::task_presentation::kind_short(task.kind)
        )));
    }

    fn shell_open_task(&mut self, args: &str) {
        if self.tasks.is_empty() {
            self.push_transcript(TranscriptEntry::warning("当前没有后台任务。"));
            return;
        }
        let index = args
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .filter(|index| *index < self.tasks.len())
            .unwrap_or(self.selected_task.min(self.tasks.len() - 1));
        self.focused_panel = Panel::Tasks;
        self.selected_task = index;
        self.push_transcript(TranscriptEntry::assistant(format!(
            "正在打开后台任务 #{}；Esc 返回会话。",
            index + 1
        )));
        self.update(Action::OpenTaskDetail);
    }

    fn shell_instances_text(&self) -> String {
        if self.instances.is_empty() {
            return "还没有服务器。输入 /create 新建 Paper 服务器。".to_owned();
        }
        let mut text = format!("本机实例 · {}\n", self.instances.len());
        for (index, instance) in self.instances.iter().enumerate() {
            let selected = if index == self.selected_instance {
                "›"
            } else {
                " "
            };
            text.push_str(&format!(
                "{selected} {}. {}  ·  {} {}  ·  :{}  ·  内存 {}-{} MiB\n",
                index + 1,
                instance.name,
                instance.provider.to_ascii_lowercase(),
                instance.game_version,
                instance.port,
                instance.min_memory_mb,
                instance.max_memory_mb
            ));
        }
        text.push_str("用 /use <序号|名称> 选择，/open 打开详情。");
        text
    }

    fn shell_status_text(&self) -> String {
        let Some(instance) = self.current_instance() else {
            return "还没有实例。/create 新建 Paper；/fabric 新建模组服务器。".to_owned();
        };
        // The open detail page may belong to another instance; prefer it only
        // when it matches, otherwise use the dashboard's per-instance snapshot.
        let server = self
            .instance_detail
            .as_ref()
            .and_then(|detail| detail.detail.as_ref())
            .map(|detail| &detail.server)
            .filter(|server| server.instance_id == instance.id)
            .or_else(|| self.servers.get(&instance.id));
        let state = server
            .map(|server| crate::ui::server_state_label(server.state).to_owned())
            .unwrap_or_else(|| "未加载".to_owned());
        let address = server
            .map(|server| format!(" · {}", server.address))
            .unwrap_or_default();
        let active = self
            .tasks
            .iter()
            .filter(|task| is_active(task.status))
            .count();
        let mut text = format!(
            "当前实例：{}（#{}）\n运行状态：{}{}\n版本：{} {} · 端口 {} · 内存 {}-{} MiB\n后台任务：{} 个，进行中 {}\n",
            instance.name,
            self.selected_instance + 1,
            state,
            address,
            instance.provider.to_ascii_lowercase(),
            instance.game_version,
            instance.port,
            instance.min_memory_mb,
            instance.max_memory_mb,
            self.tasks.len(),
            active
        );
        for (index, task) in self
            .tasks
            .iter()
            .filter(|task| is_active(task.status))
            .take(3)
            .enumerate()
        {
            text.push_str(&format!("  {}. {}\n", index + 1, task_summary_line(task)));
        }
        text.push_str("输入 /tasks 查看全部，/open 打开控制台与日志。");
        text
    }

    fn shell_tasks_text(&self) -> String {
        if self.tasks.is_empty() {
            return "当前没有后台任务。".to_owned();
        }
        let mut text = format!("后台任务 · {}\n", self.tasks.len());
        for (index, task) in self.tasks.iter().take(12).enumerate() {
            let marker = if index == self.selected_task {
                "›"
            } else {
                " "
            };
            text.push_str(&format!(
                "{marker} {}. {}\n",
                index + 1,
                task_summary_line(task)
            ));
        }
        if self.tasks.len() > 12 {
            text.push_str(&format!("… 另有 {} 个任务\n", self.tasks.len() - 12));
        }
        text.push_str("用 /task <序号> 打开任务详情。");
        text
    }
}

const MAX_PASTED_CHARACTERS: usize = 512;

fn normalize_pasted_text(text: &str) -> Vec<char> {
    let mut source = text.chars().take(MAX_PASTED_CHARACTERS).peekable();
    let mut normalized = Vec::with_capacity(MAX_PASTED_CHARACTERS.min(text.len()));
    while let Some(character) = source.next() {
        let character = match character {
            '\r' => {
                if source.peek() == Some(&'\n') {
                    source.next();
                }
                ' '
            }
            '\n' | '\t' | '\u{2028}' | '\u{2029}' => ' ',
            character if character.is_control() => continue,
            character => character,
        };
        normalized.push(character);
    }
    normalized
}

fn has_line_break(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '\r' | '\n' | '\u{2028}' | '\u{2029}'))
}

fn is_active(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Queued | TaskStatus::Running)
}

fn task_summary_line(task: &mc_core::TaskSnapshot) -> String {
    let ratio = crate::ui::task_ratio(task);
    let percent = (ratio * 100.0).round() as u8;
    let bar = shell::progress_text(ratio, 12);
    format!(
        "{} · {}  {} {:>3}%  {}/{}  {}",
        crate::task_presentation::kind_short(task.kind),
        crate::task_presentation::phase_label(task.phase),
        bar,
        percent,
        task.progress
            .as_ref()
            .map(|progress| progress.completed_steps)
            .unwrap_or(0),
        task.progress
            .as_ref()
            .map(|progress| progress.total_steps)
            .unwrap_or(0),
        crate::task_presentation::status_label(task.status)
    )
}
