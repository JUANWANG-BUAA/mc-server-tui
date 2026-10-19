//! Player-facing operations. No shell strings are executed by this module.
mod addons;
mod fabric;
mod packs;

use crate::state::DaemonState;
use mc_core::{
    ApiResponse, AppError, ErrorCode, InstanceOperation, InstanceSummary, ServerState, TaskKind,
    TaskResult,
};
use std::{
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};
use uuid::Uuid;

/// Configured upstream identity and cache shared by the background operations.
#[derive(Clone)]
pub(crate) struct Context {
    upstream: addons::Upstream,
    user_agent: String,
    cache_dir: PathBuf,
}

impl Context {
    fn new(state: &DaemonState) -> Result<Self, AppError> {
        Ok(Self {
            upstream: addons::Upstream::new(&state.user_agent)?,
            user_agent: state.user_agent.clone(),
            cache_dir: state.cache_dir.clone(),
        })
    }
}

pub(crate) fn invalid(reason: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::InvalidInput, reason)
}
pub(crate) fn io_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::Internal, format!("文件操作失败：{error}"))
}

/// Resolve only regular files beneath the managed server directory.
pub(crate) fn safe_path(root: &Path, relative: &Path) -> Result<PathBuf, AppError> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(invalid("文件路径必须位于服务端目录内。"));
    }
    let root = root.canonicalize().map_err(io_error)?;
    let mut path = root.clone();
    for part in relative.components() {
        path.push(part);
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if meta.file_type().is_symlink() {
                return Err(invalid("不允许通过符号链接读写实例文件。"));
            }
        }
    }
    Ok(path)
}

pub(crate) fn read_small(path: &Path, max: u64) -> Result<String, AppError> {
    let f = std::fs::File::open(path).map_err(io_error)?;
    if !f.metadata().map_err(io_error)?.is_file() {
        return Err(invalid("只能读取普通文件。"));
    }
    let mut text = String::new();
    f.take(max + 1)
        .read_to_string(&mut text)
        .map_err(io_error)?;
    if text.len() as u64 > max {
        return Err(invalid("文件过大，未载入。"));
    }
    Ok(text)
}

pub(crate) async fn dispatch(
    state: &DaemonState,
    id: Uuid,
    action: InstanceOperation,
) -> ApiResponse {
    match execute(state, id, action).await {
        Ok(response) => response,
        Err(error) => ApiResponse::Error(error),
    }
}

async fn execute(
    state: &DaemonState,
    id: Uuid,
    action: InstanceOperation,
) -> Result<ApiResponse, AppError> {
    use InstanceOperation::*;
    if matches!(
        action,
        CreateFabric { .. }
            | InstallAddon { .. }
            | ExportPack { .. }
            | ImportPack { apply: true, .. }
    ) {
        let context = Context::new(state)?;
        let instances = state.instances.clone();
        let supervisor = state.supervisor.clone();
        let completed = move |report| TaskResult::ManagementCompleted {
            instance_id: id,
            report,
        };
        let task = state
            .tasks
            .submit(
                TaskKind::ManageInstance,
                (!id.is_nil()).then_some(id),
                move |reporter| async move {
                    match action {
                        CreateFabric {
                            name,
                            game_version,
                            accept_eula,
                        } => {
                            fabric::create(
                                instances,
                                &context,
                                name,
                                game_version,
                                accept_eula,
                                reporter,
                            )
                            .await
                        }
                        InstallAddon { project } => {
                            addons::install(&context.upstream, &supervisor, id, &project, reporter)
                                .await
                                .map(completed)
                        }
                        ExportPack { path } => {
                            let guard = supervisor.lock_stopped_instance(id).await?;
                            tokio::task::spawn_blocking(move || packs::export(guard, &path))
                                .await
                                .map_err(io_error)?
                                .map(completed)
                        }
                        ImportPack { path, .. } => {
                            packs::import(&context.upstream, &supervisor, id, &path, reporter)
                                .await
                                .map(completed)
                        }
                        _ => Err(invalid("不支持的后台操作。")),
                    }
                },
            )
            .await?;
        return Ok(ApiResponse::TaskQueued(task));
    }
    let instance = state.instances.get_async(id).await?;
    let report = match action {
        Connection => connection(&instance),
        Monitor => {
            let detail = state.supervisor.detail(id).await?;
            let ping = addons::server_ping(instance.port)
                .await
                .unwrap_or_else(|error| format!("游戏状态查询未就绪：{}", error.reason));
            format!(
                "{} · {}\n{}\n{ping}\n\
                 实时 CPU、内存趋势：/open；Paper TPS：/console tps；\
                 性能分析：/console spark healthreport",
                instance.name,
                state_label(detail.server.state),
                connection(&instance),
            )
        }
        SearchLogs {
            pattern,
            regex,
            ignore_case,
        } => tokio::task::spawn_blocking(move || {
            search_logs(&instance, &pattern, regex, ignore_case)
        })
        .await
        .map_err(io_error)??,
        Players {
            command,
            player,
            reason,
        } => {
            if command == "show" {
                tokio::task::spawn_blocking(move || player_lists(&instance))
                    .await
                    .map_err(io_error)??
            } else {
                let command = player_command(&command, player.as_deref(), reason.as_deref())?;
                let detail = state.supervisor.detail(id).await?;
                if detail.server.state != ServerState::Running {
                    return Err(invalid("玩家管理需要服务器已就绪。先 /start，再重试。"));
                }
                // An offset past the end makes the reader report the current log
                // length, so only output produced after the command is shown.
                let before = state
                    .supervisor
                    .read_logs(id, u64::MAX, 1)
                    .await?
                    .next_offset;
                state.supervisor.send_console_command(id, &command).await?;
                // Most replies arrive within a few hundred milliseconds, but an
                // online-mode server first looks the name up with Mojang, which
                // can take many seconds; wait briefly and say so instead of hanging.
                let mut output = String::new();
                for _ in 0..30 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let logs = state.supervisor.read_logs(id, before, 8192).await?;
                    if !logs.content.trim().is_empty() {
                        output = logs.content;
                        break;
                    }
                }
                if output.is_empty() {
                    output =
                        "服务端尚未回应：在线模式会先向 Mojang 查询玩家档案，可能需要十几秒。\n\
                              稍后用 /players 查看名单，或 /rg <玩家名> 搜索结果。"
                            .to_owned();
                }
                format!("已发送：{command}\n{output}")
            }
        }
        SearchAddons { query } => {
            let upstream = addons::Upstream::new(&state.user_agent)?;
            addons::search(&upstream, &instance, &query).await?
        }
        ListAddons => tokio::task::spawn_blocking(move || addons::list(&instance))
            .await
            .map_err(io_error)??,
        ImportPack { path, .. } => {
            tokio::task::spawn_blocking(move || packs::preview(&instance, &path))
                .await
                .map_err(io_error)??
        }
        _ => return Err(invalid("不支持的操作。")),
    };
    Ok(ApiResponse::ManagementReport(report))
}

fn state_label(state: ServerState) -> &'static str {
    match state {
        ServerState::Running => "运行中",
        ServerState::Starting => "启动中",
        ServerState::Stopping => "停止中",
        ServerState::Failed => "异常退出",
        ServerState::Stopped => "已停止",
        ServerState::Installed => "已安装，尚未启动",
        ServerState::Unknown => "状态未知",
    }
}

fn connection(instance: &InstanceSummary) -> String {
    let port = instance.port;
    let addresses = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .filter_map(|interface| match interface.ip() {
            std::net::IpAddr::V4(ip) if ip.is_private() => {
                Some(format!("{ip}:{port} ({})", interface.name))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let lan = if addresses.is_empty() {
        "未检测到私有 IPv4 地址；请检查网络接口或使用联机隧道地址。".to_owned()
    } else {
        addresses.join(" · ")
    };
    format!(
        "加入 {} · Java 版 {}（客户端版本需匹配）\n\
         本机：127.0.0.1:{port}（仅这台电脑可用）\n\
         局域网候选地址：{lan}\n\
         朋友需与对应接口网络互通；VPN/虚拟网卡地址不等于物理局域网。\n\
         外网朋友：需配置路由器 TCP {port} 端口转发，或使用可信的联机隧道；不要把 127.0.0.1 发给朋友。\n\
         外网连通性尚未验证；防火墙需允许 Java/TCP {port}。",
        instance.name, instance.game_version,
    )
}

fn player_lists(instance: &InstanceSummary) -> Result<String, AppError> {
    let root = instance.directory.join("server");
    let mut report = String::from("玩家与权限 · /players list 在线玩家\n");
    for (label, file) in [
        ("管理员 OP", "ops.json"),
        ("白名单", "whitelist.json"),
        ("封禁玩家", "banned-players.json"),
        ("封禁 IP", "banned-ips.json"),
    ] {
        let path = safe_path(&root, Path::new(file))?;
        let text = if path.exists() {
            read_small(&path, 1024 * 1024)?
        } else {
            "[]".to_owned()
        };
        let rows: serde_json::Value = serde_json::from_str(&text).map_err(io_error)?;
        report.push_str(&format!("\n{label}\n"));
        if let Some(rows) = rows.as_array() {
            if rows.is_empty() {
                report.push_str("  无\n");
            }
            for row in rows.iter().take(100) {
                report.push_str(&format!(
                    "  {}{}\n",
                    row["name"]
                        .as_str()
                        .or(row["ip"].as_str())
                        .unwrap_or("未知"),
                    row["level"]
                        .as_u64()
                        .map(|v| format!(" · 等级 {v}"))
                        .unwrap_or_default()
                ));
            }
        }
    }
    report.push_str(
        "\n/players op|deop|ban|pardon|kick <玩家> [原因]\n\
         /players allow|deny <玩家> · /players whitelist-on|whitelist-off\n\
         /players group <玩家> <组> · grant|revoke <玩家> <权限节点>（需 LuckPerms）\n\
         OP 赋予全面管理权限；细粒度权限需 LuckPerms 等兼容扩展。\n\
         可先 /mods search luckperms，再 /mods install luckperms。",
    );
    Ok(report)
}

pub(crate) fn player_command(
    action: &str,
    player: Option<&str>,
    reason: Option<&str>,
) -> Result<String, AppError> {
    let fixed = match action {
        "list" => Some("list"),
        "banlist" => Some("banlist players"),
        "whitelist-on" => Some("whitelist on"),
        "whitelist-off" => Some("whitelist off"),
        _ => None,
    };
    if let Some(command) = fixed {
        return Ok(command.to_owned());
    }
    let verb = match action {
        "group" | "grant" | "revoke" => "lp",
        "op" => "op",
        "deop" => "deop",
        "ban" => "ban",
        "pardon" => "pardon",
        "kick" => "kick",
        "allow" => "whitelist add",
        "deny" => "whitelist remove",
        _ => return Err(invalid("未知玩家操作；使用 /players 查看用法。")),
    };
    let name = player.ok_or_else(|| invalid("请输入玩家名。"))?;
    if name.is_empty()
        || name.len() > 16
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(invalid("Java 玩家名应为 1–16 位英文字母、数字或下划线。"));
    }
    let reason = reason.unwrap_or("");
    if reason.chars().any(char::is_control) || reason.len() > 200 {
        return Err(invalid("原因不得包含控制字符，且不可超过 200 字节。"));
    }
    if matches!(action, "group" | "grant" | "revoke") {
        if reason.is_empty()
            || reason.len() > 128
            || !reason
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'*'))
        {
            return Err(invalid(
                "请输入有效的权限组或权限节点（英文字母、数字、点、横线、星号）。",
            ));
        }
        return Ok(if action == "group" {
            format!("lp user {name} parent set {reason}")
        } else {
            format!(
                "lp user {name} permission set {reason} {}",
                action == "grant"
            )
        });
    }
    Ok(if matches!(action, "ban" | "kick") && !reason.is_empty() {
        format!("{verb} {name} {reason}")
    } else {
        format!("{verb} {name}")
    })
}

fn search_logs(
    instance: &InstanceSummary,
    pattern: &str,
    is_regex: bool,
    ignore_case: bool,
) -> Result<String, AppError> {
    if pattern.is_empty() || pattern.len() > 512 {
        return Err(invalid("搜索内容需为 1–512 字节。"));
    }
    let escaped = if is_regex {
        pattern.to_owned()
    } else {
        regex::escape(pattern)
    };
    let matcher = regex::RegexBuilder::new(&escaped)
        .case_insensitive(ignore_case)
        .size_limit(1024 * 1024)
        .build()
        .map_err(|e| invalid(format!("正则表达式无效：{e}")))?;
    let root = instance.directory.join("logs");
    let mut matches = std::collections::VecDeque::new();
    let mut count = 0;
    for file in ["console.log.1", "console.log"] {
        let path = safe_path(&root, Path::new(file))?;
        if !path.exists() {
            continue;
        }
        let reader = BufReader::new(
            std::fs::File::open(path)
                .map_err(io_error)?
                .take(64 * 1024 * 1024),
        );
        for (i, line) in reader.split(b'\n').enumerate() {
            let line = line.map_err(io_error)?;
            let text = String::from_utf8_lossy(&line);
            if matcher.is_match(&text) {
                count += 1;
                matches.push_back(format!(
                    "{file}:{}: {}",
                    i + 1,
                    text.chars().take(1000).collect::<String>()
                ));
                if matches.len() > 100 {
                    matches.pop_front();
                }
            }
        }
    }
    Ok(format!(
        "日志搜索：{pattern} · {count} 个匹配（最多显示最近 100 条；每文件扫描上限 64 MiB）\n{}",
        matches.into_iter().collect::<Vec<_>>().join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn player_commands_cannot_inject_console_lines() {
        assert!(player_command("op", Some("friend\nstop"), None).is_err());
        assert!(player_command("ban", Some("friend"), Some("bad\nstop")).is_err());
        assert_eq!(
            player_command("allow", Some("Steve_1"), None).expect("valid"),
            "whitelist add Steve_1"
        );
        assert!(player_command("execute", Some("Steve"), None).is_err());
    }
    #[test]
    fn file_paths_cannot_escape_root() {
        let dir = tempfile::tempdir().expect("temp");
        assert!(safe_path(dir.path(), Path::new("../ops.json")).is_err());
    }
}
