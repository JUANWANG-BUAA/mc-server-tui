#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellCommandSpec {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub args: &'static str,
    pub summary: &'static str,
}

pub(crate) const SHELL_COMMANDS: &[ShellCommandSpec] = &[
    ShellCommandSpec {
        name: "help",
        aliases: &["?", "keys"],
        args: "",
        summary: "列出所有会话命令与快捷键",
    },
    ShellCommandSpec {
        name: "status",
        aliases: &["info"],
        args: "",
        summary: "查看当前实例、运行状态和后台任务",
    },
    ShellCommandSpec {
        name: "instances",
        aliases: &["ls"],
        args: "",
        summary: "列出本机服务器实例",
    },
    ShellCommandSpec {
        name: "use",
        aliases: &["select", "cd"],
        args: "<序号|名称>",
        summary: "选择当前操作的实例",
    },
    ShellCommandSpec {
        name: "open",
        aliases: &["detail"],
        args: "",
        summary: "打开实例详情：完整日志与交互式控制台",
    },
    ShellCommandSpec {
        name: "logs",
        aliases: &["tail"],
        args: "",
        summary: "在会话中查看最近的控制台日志",
    },
    ShellCommandSpec {
        name: "start",
        aliases: &["run"],
        args: "",
        summary: "启动当前实例的 Minecraft 服务端",
    },
    ShellCommandSpec {
        name: "stop",
        aliases: &["shutdown"],
        args: "",
        summary: "优雅停止当前实例",
    },
    ShellCommandSpec {
        name: "console",
        aliases: &["cmd"],
        args: "<命令>",
        summary: "向当前实例发送控制台命令",
    },
    ShellCommandSpec {
        name: "tasks",
        aliases: &["jobs", "ps"],
        args: "",
        summary: "查看后台任务与字节进度",
    },
    ShellCommandSpec {
        name: "task",
        aliases: &["job"],
        args: "<序号>",
        summary: "打开后台任务详情",
    },
    ShellCommandSpec {
        name: "cancel",
        aliases: &["abort", "stop-task"],
        args: "[序号]",
        summary: "取消下载任务并保留可续传文件",
    },
    ShellCommandSpec {
        name: "create",
        aliases: &["new", "paper"],
        args: "",
        summary: "新建 Paper 服务器向导",
    },
    ShellCommandSpec {
        name: "import",
        aliases: &["modpack", "mrpack"],
        args: "[路径.zip|.mrpack] [--apply]",
        summary: "预览并停服导入 ZIP / Modrinth 整合包",
    },
    ShellCommandSpec {
        name: "backup",
        aliases: &["snapshot"],
        args: "",
        summary: "为当前实例创建停机世界备份",
    },
    ShellCommandSpec {
        name: "backups",
        aliases: &["restore"],
        args: "",
        summary: "打开世界与备份管理器",
    },
    ShellCommandSpec {
        name: "refresh",
        aliases: &["r"],
        args: "",
        summary: "立即刷新实例与任务",
    },
    ShellCommandSpec {
        name: "clear",
        aliases: &["cls"],
        args: "",
        summary: "清空当前会话记录",
    },
    ShellCommandSpec {
        name: "palette",
        aliases: &["commands"],
        args: "",
        summary: "打开完整命令面板（Ctrl-K）",
    },
    ShellCommandSpec {
        name: "theme",
        aliases: &["appearance", "color"],
        args: "[主题名]",
        summary: "列出或指定配色主题；F2 循环",
    },
    ShellCommandSpec {
        name: "classic",
        aliases: &["dashboard"],
        args: "",
        summary: "切换到经典双面板视图",
    },
    ShellCommandSpec {
        name: "quit",
        aliases: &["exit", "q"],
        args: "",
        summary: "退出 TUI；后台任务会继续运行",
    },
    ShellCommandSpec {
        name: "connect",
        aliases: &["join", "address"],
        args: "",
        summary: "本机、局域网与外网联机地址说明",
    },
    ShellCommandSpec {
        name: "monitor",
        aliases: &["health"],
        args: "",
        summary: "游戏状态、在线人数与实例监控",
    },
    ShellCommandSpec {
        name: "rg",
        aliases: &["grep", "search-logs"],
        args: "[-i] [-e] <模式>",
        summary: "搜索当前及轮转日志，显示文件名和行号",
    },
    ShellCommandSpec {
        name: "players",
        aliases: &["player", "permissions"],
        args: "[操作] [玩家]",
        summary: "OP、白名单、封禁、解封、踢出与在线玩家",
    },
    ShellCommandSpec {
        name: "mods",
        aliases: &["plugins", "addons"],
        args: "[list|search|install] [名称]",
        summary: "搜索、校验并安装兼容插件或模组与依赖",
    },
    ShellCommandSpec {
        name: "export",
        aliases: &["pack-export"],
        args: "<路径.zip>",
        summary: "停服导出整合包、配置与校验清单",
    },
    ShellCommandSpec {
        name: "fabric",
        aliases: &["create-fabric"],
        args: "<名称> <版本> --accept-eula",
        summary: "自动安装稳定 Fabric 与兼容 Java，用于模组服务器",
    },
    ShellCommandSpec {
        name: "pages",
        aliases: &["windows", "tabs"],
        args: "",
        summary: "列出工作区页面、稳定编号与当前页面",
    },
    ShellCommandSpec {
        name: "page",
        aliases: &["window"],
        args: "<页面编号>",
        summary: "按稳定编号切换页面，保留日志位置",
    },
    ShellCommandSpec {
        name: "close",
        aliases: &["close-page"],
        args: "[页面编号]",
        summary: "关闭页面，后台服务器继续运行",
    },
];

pub(crate) fn find(name: &str) -> Option<&'static ShellCommandSpec> {
    let name = name.trim_start_matches('/').to_ascii_lowercase();
    SHELL_COMMANDS
        .iter()
        .find(|command| command.name == name || command.aliases.iter().any(|alias| *alias == name))
}

/// Suggest commands while the user is still typing the command word.
pub(crate) fn suggestions(input: &str) -> Vec<&'static ShellCommandSpec> {
    let Some(rest) = input.strip_prefix('/') else {
        return Vec::new();
    };
    if rest.contains(char::is_whitespace) {
        return Vec::new();
    }
    let query = rest.trim().to_ascii_lowercase();
    let mut matches: Vec<(u8, usize, &'static ShellCommandSpec)> = SHELL_COMMANDS
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            match_rank(command, &query).map(|rank| (rank, index, command))
        })
        .collect();
    matches.sort_by_key(|(rank, index, _)| (*rank, *index));
    matches
        .into_iter()
        .take(6)
        .map(|(_, _, command)| command)
        .collect()
}

/// Exact names beat name prefixes, which beat alias substrings. Without this
/// ordering, `/console` could offer `/open` first because its alias was
/// `console-page`; Tab would then complete the wrong command.
fn match_rank(command: &ShellCommandSpec, query: &str) -> Option<u8> {
    if command.name == query {
        Some(0)
    } else if command.name.starts_with(query) {
        Some(1)
    } else if command.aliases.contains(&query) {
        Some(2)
    } else if command.aliases.iter().any(|alias| alias.starts_with(query)) {
        Some(3)
    } else if command.name.contains(query) {
        Some(4)
    } else if command.aliases.iter().any(|alias| alias.contains(query)) {
        Some(5)
    } else {
        None
    }
}

pub(crate) fn help_text() -> String {
    let mut text = String::from("可用命令（直接输入文本会发送到当前实例控制台）\n");
    for command in SHELL_COMMANDS {
        let name = if command.args.is_empty() {
            format!("/{}", command.name)
        } else {
            format!("/{} {}", command.name, command.args)
        };
        text.push_str(&format!("{name:<18} {}\n", command.summary));
    }
    text.push_str("\n快捷键  Enter 发送 · ←→ 光标 · Tab 补全 · F2 主题 · F1 帮助");
    text
}

pub(crate) fn progress_text(ratio: f64, width: usize) -> String {
    let width = width.max(1);
    let filled = ((ratio.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

pub(crate) fn welcome() -> String {
    format!(
        "MC SERVER TUI v{}\n输入 /help 查看命令；直接输入文本会作为当前实例的控制台命令。\n按 F1 查看完整键盘帮助，Ctrl-K 打开命令面板。",
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_match_names_and_aliases() {
        assert!(suggestions("").is_empty());
        assert!(suggestions("/")
            .iter()
            .any(|command| command.name == "status"));
        let matched = suggestions("/sta");
        assert_eq!(matched.first().map(|command| command.name), Some("status"));
        assert!(suggestions("/ls")
            .iter()
            .any(|command| command.name == "instances"));
        assert_eq!(
            suggestions("/console").first().map(|command| command.name),
            Some("console")
        );
        assert_eq!(
            suggestions("/conso").first().map(|command| command.name),
            Some("console")
        );
        assert!(suggestions("/use 1").is_empty());
        assert!(find("say").is_none());
        assert!(find("list").is_none());
    }

    #[test]
    fn every_command_is_findable_by_alias() {
        for command in SHELL_COMMANDS {
            assert_eq!(
                find(command.name).map(|found| found.name),
                Some(command.name)
            );
            for alias in command.aliases {
                assert_eq!(find(alias).map(|found| found.name), Some(command.name));
            }
        }
    }

    #[test]
    fn help_lists_every_command() {
        let help = help_text();
        for command in SHELL_COMMANDS {
            assert!(help.contains(&format!("/{}", command.name)));
        }
    }
}
