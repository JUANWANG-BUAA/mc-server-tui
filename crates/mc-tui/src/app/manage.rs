use super::AppState;
use crate::{instance_detail::InstanceCommand, transcript::TranscriptEntry};
use mc_core::InstanceOperation;
impl AppState {
    pub(super) fn manage(&mut self, action: InstanceOperation) {
        if self.instance_operation_pending {
            self.push_transcript(TranscriptEntry::warning("上一项操作正在提交，请稍候。"));
            return;
        }
        let id = if matches!(action, InstanceOperation::CreateFabric { .. }) {
            uuid::Uuid::nil()
        } else {
            let Some(instance) = self.instances.get(self.selected_instance) else {
                self.push_transcript(TranscriptEntry::warning(
                    "先 /create 或 /fabric 创建实例，再 /use 选择。",
                ));
                return;
            };
            instance.id
        };
        self.instance_operation_pending = true;
        self.instance_command_requested = Some(InstanceCommand::Manage {
            instance_id: id,
            action,
        });
        self.status = "正在处理请求…".to_owned();
    }
    pub(super) fn manage_shell(&mut self, name: &str, args: &str) {
        use InstanceOperation::*;
        let action = match name {
            "connect" => Connection,
            "monitor" => Monitor,
            "rg" => {
                let mut pattern = args;
                let mut regex = false;
                let mut ignore_case = false;
                loop {
                    if let Some(tail) = pattern.strip_prefix("-i ") {
                        ignore_case = true;
                        pattern = tail.trim_start();
                    } else if let Some(tail) = pattern
                        .strip_prefix("-e ")
                        .or_else(|| pattern.strip_prefix("--regex "))
                    {
                        regex = true;
                        pattern = tail.trim_start();
                    } else {
                        break;
                    }
                }
                if pattern.is_empty() {
                    self.push_transcript(TranscriptEntry::assistant(
                        "/rg [-i] [-e] <关键词或正则>\n\
                         示例：/rg -i error · /rg -e WARN|ERROR\n\
                         默认按文字搜索；-e 启用正则。扫描当前和上一份轮转日志，返回文件名:行号。",
                    ));
                    return;
                }
                SearchLogs {
                    pattern: unquote(pattern).to_owned(),
                    regex,
                    ignore_case,
                }
            }
            "players" => {
                let mut parts = args.splitn(3, char::is_whitespace);
                Players {
                    command: parts
                        .next()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("show")
                        .to_owned(),
                    player: parts.next().map(str::to_owned),
                    reason: parts.next().map(str::to_owned),
                }
            }
            "mods" => {
                let mut parts = args.splitn(2, char::is_whitespace);
                let command = parts.next().unwrap_or("");
                let value = parts.next().unwrap_or("").trim();
                match command {
                    "" | "list" => ListAddons,
                    "search" => SearchAddons {
                        query: value.to_owned(),
                    },
                    "install" if !value.is_empty() => InstallAddon {
                        project: value.to_owned(),
                    },
                    _ => {
                        self.push_transcript(TranscriptEntry::warning(
                            "/mods list · /mods search <名称> · /mods install <Modrinth slug>",
                        ));
                        return;
                    }
                }
            }
            "export" => {
                if args.is_empty() {
                    self.push_transcript(TranscriptEntry::warning(
                        "/export <输出路径.zip> · 停服导出服务端扩展与配置，不含世界和玩家数据。",
                    ));
                    return;
                }
                ExportPack {
                    path: unquote(args).into(),
                }
            }
            "import" => {
                let (path, apply) = if let Some(path) = args.strip_suffix(" --apply") {
                    (path, true)
                } else {
                    (args, false)
                };
                ImportPack {
                    path: unquote(path).into(),
                    apply,
                }
            }
            "fabric" => {
                let parts = args.split_whitespace().collect::<Vec<_>>();
                if parts.len() < 2 {
                    self.push_transcript(TranscriptEntry::assistant(
                        "/fabric <名称> <Minecraft版本> --accept-eula\n\
                         例如 /fabric friends 1.21.11 --accept-eula\n\
                         阅读 https://www.minecraft.net/eula 后再明确接受。\
                         自动选稳定 Fabric Loader 和兼容 Java，默认内存 1–4 GiB。",
                    ));
                    return;
                }
                CreateFabric {
                    name: parts[0].to_owned(),
                    game_version: parts[1].to_owned(),
                    accept_eula: parts.contains(&"--accept-eula"),
                }
            }
            _ => return,
        };
        self.manage(action);
    }
}
fn unquote(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
}
