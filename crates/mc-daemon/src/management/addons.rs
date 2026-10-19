use super::{invalid, io_error, safe_path};
use mc_core::{AppError, ErrorCode, InstanceSummary, TaskPhase, TaskProgress, TransferProgress};
use serde_json::Value;
use sha2::{Digest, Sha512};
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const METADATA_HOSTS: [&str; 2] = ["api.modrinth.com", "meta.fabricmc.net"];
const MAX_METADATA_BYTES: usize = 4 * 1024 * 1024;

/// HTTPS client for the exact metadata and CDN hosts the add-on flows trust.
/// It carries the configured User-Agent so upstreams can identify the operator.
#[derive(Clone)]
pub(crate) struct Upstream {
    client: reqwest::Client,
}

impl Upstream {
    pub(crate) fn new(user_agent: &str) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(io_error)?;
        Ok(Self { client })
    }

    /// Bounded body from a Modrinth or Fabric metadata endpoint.
    pub(super) async fn bytes(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, AppError> {
        let url = url::Url::parse(url).map_err(io_error)?;
        if url.scheme() != "https" || !url.host_str().is_some_and(|h| METADATA_HOSTS.contains(&h)) {
            return Err(invalid("不受支持的扩展元数据来源。"));
        }
        let mut response = self
            .client
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(io_error)?
            .error_for_status()
            .map_err(io_error)?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(io_error)? {
            if bytes.len() + chunk.len() > max_bytes {
                return Err(invalid(format!(
                    "上游响应超过 {} MiB。",
                    max_bytes / 1024 / 1024
                )));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    pub(super) async fn json(&self, url: &str) -> Result<Value, AppError> {
        serde_json::from_slice(&self.bytes(url, MAX_METADATA_BYTES).await?).map_err(io_error)
    }

    /// Stream one file from the official Modrinth CDN, enforcing the declared
    /// size and SHA-512 before the caller may commit it.
    pub(super) async fn download_file(
        &self,
        url: &str,
        expected_size: u64,
        sha512: &str,
        path: &Path,
        reporter: &mc_task::TaskReporter,
    ) -> Result<(), AppError> {
        let parsed = url::Url::parse(url).map_err(io_error)?;
        if parsed.scheme() != "https"
            || parsed.host_str() != Some("cdn.modrinth.com")
            || parsed.port_or_known_default() != Some(443)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(invalid("下载仅允许 Modrinth 官方 HTTPS CDN。"));
        }
        if expected_size == 0
            || expected_size > 256 * 1024 * 1024
            || sha512.len() != 128
            || !sha512.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid("扩展缺少有效大小或 SHA-512，无法校验。"));
        }
        let mut response = self
            .client
            .get(parsed)
            .send()
            .await
            .map_err(io_error)?
            .error_for_status()
            .map_err(io_error)?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await
            .map_err(io_error)?;
        let mut hash = Sha512::new();
        let mut downloaded = 0u64;
        let started = std::time::Instant::now();
        let mut last = std::time::Instant::now();
        while let Some(chunk) = response.chunk().await.map_err(io_error)? {
            if reporter.is_cancelled() {
                return Err(AppError::new(ErrorCode::TaskCancelled, "扩展下载已取消。"));
            }
            downloaded += chunk.len() as u64;
            if downloaded > expected_size {
                return Err(invalid("扩展文件超过声明大小。"));
            }
            hash.update(&chunk);
            file.write_all(&chunk).await.map_err(io_error)?;
            if last.elapsed() > Duration::from_millis(200) {
                let elapsed = started.elapsed().as_secs_f64().max(0.001);
                reporter
                    .set_phase(
                        TaskPhase::DownloadingArtifact,
                        "下载扩展并校验 SHA-512",
                        Some(TaskProgress {
                            completed_steps: 0,
                            total_steps: 1,
                            transfer: Some(TransferProgress {
                                downloaded_bytes: downloaded,
                                total_bytes: expected_size,
                                bytes_per_second: Some((downloaded as f64 / elapsed) as u64),
                                source: "Modrinth 官方 CDN".to_owned(),
                            }),
                        }),
                    )
                    .await?;
                last = std::time::Instant::now();
            }
        }
        let digest = format!("{:x}", hash.finalize());
        if downloaded != expected_size || !digest.eq_ignore_ascii_case(sha512) {
            return Err(AppError::new(
                ErrorCode::HashMismatch,
                "扩展的 SHA-512 或文件大小不匹配；未安装。",
            ));
        }
        file.sync_all().await.map_err(io_error)?;
        Ok(())
    }
}

fn loader(instance: &InstanceSummary) -> &str {
    if instance.provider == "fabric" {
        "fabric"
    } else {
        "paper"
    }
}

pub(super) async fn search(
    upstream: &Upstream,
    instance: &InstanceSummary,
    query: &str,
) -> Result<String, AppError> {
    if query.len() > 256 {
        return Err(invalid("搜索词过长。"));
    }
    let mut url = url::Url::parse("https://api.modrinth.com/v2/search").map_err(io_error)?;
    let facets = serde_json::json!([
        [format!("versions:{}", instance.game_version)],
        [format!("categories:{}", loader(instance))]
    ])
    .to_string();
    url.query_pairs_mut()
        .append_pair("query", query)
        .append_pair("facets", &facets)
        .append_pair("limit", "8");
    let data = upstream.json(url.as_str()).await?;
    let mut report = format!(
        "Modrinth · {} {} · 仅显示兼容版本\n",
        loader(instance),
        instance.game_version
    );
    for hit in data["hits"].as_array().into_iter().flatten() {
        report.push_str(&format!(
            "{} · {}\n  {}\n",
            hit["slug"].as_str().unwrap_or(""),
            hit["title"].as_str().unwrap_or(""),
            hit["description"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(160)
                .collect::<String>()
        ));
    }
    report.push_str(
        "\n/mods install <项目 slug> 下载并校验，需停服；下次启动加载。\n\
         Paper 使用 plugins；Fabric 使用 mods，客户端专用模组不会安装。",
    );
    Ok(report)
}

pub(super) fn list(instance: &InstanceSummary) -> Result<String, AppError> {
    let root = instance.directory.join("server");
    let mut report = format!("{} · 已安装扩展（{}）\n", instance.name, loader(instance));
    for folder in ["plugins", "mods"] {
        let path = safe_path(&root, Path::new(folder))?;
        report.push_str(&format!("\n{folder}/\n"));
        if path.exists() {
            let mut files = std::fs::read_dir(path)
                .map_err(io_error)?
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|v| v == "jar"))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            files.sort();
            for f in files {
                report.push_str(&format!("  {f}\n"));
            }
        }
    }
    report.push_str(
        "\n/mods search <名称> 搜索 · /mods install <slug> 安装\n\
         权限组：安装 LuckPerms 后，在控制台使用 lp user <玩家> parent set <组>。",
    );
    Ok(report)
}

pub(super) async fn install(
    upstream: &Upstream,
    supervisor: &mc_supervisor::ProcessSupervisor,
    id: uuid::Uuid,
    project: &str,
    reporter: mc_task::TaskReporter,
) -> Result<String, AppError> {
    if project.is_empty()
        || project.len() > 128
        || !project
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("请输入 Modrinth 项目 slug 或 ID。"));
    }
    let guard = supervisor.lock_stopped_instance(id).await?;
    let instance = guard.instance();
    let server = instance.directory.join("server");
    let fabric = instance.provider == "fabric";
    let folder = if fabric { "mods" } else { "plugins" };
    let target = safe_path(&server, Path::new(folder))?;
    std::fs::create_dir_all(&target).map_err(io_error)?;
    let stage = tempfile::tempdir_in(&instance.directory).map_err(io_error)?;
    let mut queue = VecDeque::from([(project.to_owned(), None::<String>)]);
    let mut requested = HashSet::new();
    // Dependencies arrive as project IDs while JAR metadata yields slugs, so
    // resolved projects are deduplicated by their canonical Modrinth ID.
    let mut resolved = HashSet::new();
    let mut fabric_api_queued = false;
    let mut installs: Vec<(PathBuf, PathBuf)> = Vec::new();
    while let Some((project, version_id)) = queue.pop_front() {
        if !requested.insert((project.clone(), version_id.clone())) {
            continue;
        }
        if requested.len() > 32 {
            return Err(invalid("扩展依赖超过 32 项，未安装。"));
        }
        let info = upstream
            .json(&format!("https://api.modrinth.com/v2/project/{project}"))
            .await?;
        let project_id = info["id"].as_str().unwrap_or(&project).to_owned();
        if !resolved.insert(project_id.clone()) {
            continue;
        }
        if info["slug"] == "fabric-api" {
            fabric_api_queued = true;
        }
        if info["server_side"] == "unsupported" {
            return Err(invalid(format!(
                "{project} 是客户端专用模组，不能安装到服务端。"
            )));
        }
        let version = if let Some(version) = version_id {
            upstream
                .json(&format!("https://api.modrinth.com/v2/version/{version}"))
                .await?
        } else {
            let mut url = url::Url::parse(&format!(
                "https://api.modrinth.com/v2/project/{project_id}/version"
            ))
            .map_err(io_error)?;
            url.query_pairs_mut()
                .append_pair(
                    "loaders",
                    &serde_json::json!([loader(instance)]).to_string(),
                )
                .append_pair(
                    "game_versions",
                    &serde_json::json!([instance.game_version]).to_string(),
                );
            let versions = upstream.json(url.as_str()).await?;
            versions
                .as_array()
                .and_then(|vs| vs.iter().find(|v| v["version_type"] == "release"))
                .cloned()
                .ok_or_else(|| {
                    invalid(format!(
                        "{project} 没有兼容 {} {} 的正式服务端版本。",
                        loader(instance),
                        instance.game_version
                    ))
                })?
        };
        if !version["loaders"]
            .as_array()
            .is_some_and(|xs| xs.iter().any(|v| v == loader(instance)))
            || !version["game_versions"]
                .as_array()
                .is_some_and(|xs| xs.iter().any(|v| v == &instance.game_version))
        {
            return Err(invalid("扩展依赖的加载器或 Minecraft 版本不兼容，未安装。"));
        }
        for dependency in version["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| v["dependency_type"] == "required")
        {
            let p = dependency["project_id"]
                .as_str()
                .ok_or_else(|| invalid("依赖缺少项目 ID，未安装。"))?;
            queue.push_back((
                p.to_owned(),
                dependency["version_id"].as_str().map(str::to_owned),
            ));
        }
        let files = version["files"]
            .as_array()
            .ok_or_else(|| invalid("版本没有下载文件。"))?;
        let file = files
            .iter()
            .find(|v| v["primary"] == true)
            .or(files.first())
            .ok_or_else(|| invalid("扩展没有主文件。"))?;
        let filename = file["filename"]
            .as_str()
            .ok_or_else(|| invalid("文件名缺失。"))?;
        if Path::new(filename).components().count() != 1
            || !filename.ends_with(".jar")
            || filename.contains(['/', '\\'])
        {
            return Err(invalid("扩展文件名无效。"));
        }
        let destination = safe_path(&server, &Path::new(folder).join(filename))?;
        if installs.iter().any(|(_, to)| to == &destination) {
            continue;
        }
        let temporary = stage.path().join(filename);
        let expected = file["hashes"]["sha512"].as_str().unwrap_or("").to_owned();
        if destination.exists() {
            let existing = destination.clone();
            let temp = temporary.clone();
            tokio::task::spawn_blocking(move || {
                if !sha512_file(&existing)?.eq_ignore_ascii_case(&expected) {
                    return Err(invalid("已有同名扩展内容不同；未覆盖。"));
                }
                std::fs::copy(existing, temp).map_err(io_error)?;
                Ok::<_, AppError>(())
            })
            .await
            .map_err(io_error)??;
        } else {
            upstream
                .download_file(
                    file["url"].as_str().unwrap_or(""),
                    file["size"].as_u64().unwrap_or(0),
                    &expected,
                    &temporary,
                    &reporter,
                )
                .await?;
        }
        let jar = temporary.clone();
        let needs_api = tokio::task::spawn_blocking(move || {
            packs_validate_jar(&jar, fabric)?;
            if fabric {
                requires_fabric_api(&jar)
            } else {
                Ok(false)
            }
        })
        .await
        .map_err(io_error)??;
        if needs_api && !fabric_api_queued {
            fabric_api_queued = true;
            queue.push_back(("fabric-api".to_owned(), None));
        }
        installs.push((temporary, destination));
    }
    if reporter.is_cancelled() {
        return Err(AppError::new(
            ErrorCode::TaskCancelled,
            "扩展安装已取消，未写入实例。",
        ));
    }
    let commit = installs.clone();
    let installed = tokio::task::spawn_blocking(move || super::packs::commit_files(&commit))
        .await
        .map_err(io_error)??;
    let names = installs
        .iter()
        .map(|(_, p)| p.file_name().unwrap_or_default().to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "已核验 {} 个兼容扩展（含必需依赖），新增 {installed} 个，SHA-512 校验通过。\n\
         {names}\n/start 启动后加载；已有相同内容会跳过。",
        installs.len()
    ))
}

fn sha512_file(path: &Path) -> Result<String, AppError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(io_error)?;
    let mut hash = Sha512::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(io_error)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn packs_validate_jar(path: &Path, fabric: bool) -> Result<(), AppError> {
    let file = std::fs::File::open(path).map_err(io_error)?;
    let mut zip = zip::ZipArchive::new(file).map_err(io_error)?;
    let compatible = if fabric {
        zip.by_name("fabric.mod.json").is_ok()
    } else {
        zip.by_name("paper-plugin.yml").is_ok() || zip.by_name("plugin.yml").is_ok()
    };
    if !compatible {
        return Err(invalid(if fabric {
            "此 JAR 没有 Fabric 模组描述，未安装。"
        } else {
            "此 JAR 不是 Paper/Bukkit 插件；Fabric/Forge 模组不能装入 Paper。"
        }));
    }
    Ok(())
}

/// Minecraft Server List Ping over loopback: proves a real game server answers,
/// not merely that the TCP port is open.
pub(super) async fn server_ping(port: u16) -> Result<String, AppError> {
    async fn read_varint(s: &mut tokio::net::TcpStream) -> Result<usize, AppError> {
        let mut n = 0;
        for i in 0..5 {
            let b = s.read_u8().await.map_err(io_error)?;
            n |= ((b & 127) as usize) << (7 * i);
            if b < 128 {
                return Ok(n);
            }
        }
        Err(invalid("状态包长度无效。"))
    }
    let query = async {
        let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .map_err(io_error)?;
        // Handshake: packet 0, protocol version (status accepts any), host,
        // port, next state 1 (status); then the empty status request.
        let mut handshake = vec![0, 0xff, 0xff, 0xff, 0xff, 0x07, 9];
        handshake.extend(b"127.0.0.1");
        handshake.extend(port.to_be_bytes());
        handshake.push(1);
        let mut data = vec![handshake.len() as u8];
        data.extend(handshake);
        data.extend([1, 0]);
        socket.write_all(&data).await.map_err(io_error)?;
        let length = read_varint(&mut socket).await?;
        if length > 1024 * 1024 {
            return Err(invalid("状态响应过大。"));
        }
        let _ = read_varint(&mut socket).await?;
        let size = read_varint(&mut socket).await?;
        if size > length {
            return Err(invalid("状态响应损坏。"));
        }
        let mut bytes = vec![0; size];
        socket.read_exact(&mut bytes).await.map_err(io_error)?;
        let data: Value = serde_json::from_slice(&bytes).map_err(io_error)?;
        Ok(format!(
            "游戏状态查询成功 · {} · 在线 {}/{}",
            data["version"]["name"].as_str().unwrap_or("Minecraft"),
            data["players"]["online"],
            data["players"]["max"]
        ))
    };
    tokio::time::timeout(Duration::from_secs(3), query)
        .await
        .map_err(|_| invalid("游戏状态查询超时。"))?
}

/// Fabric API is a bundle of modules; projects sometimes omit this dependency
/// from their catalog metadata while declaring the modules inside the JAR.
fn requires_fabric_api(path: &Path) -> Result<bool, AppError> {
    use std::io::Read;
    let mut zip =
        zip::ZipArchive::new(std::fs::File::open(path).map_err(io_error)?).map_err(io_error)?;
    let file = zip.by_name("fabric.mod.json").map_err(io_error)?;
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > 1024 * 1024 {
        return Err(invalid("Fabric 描述文件过大。"));
    }
    let metadata: Value = serde_json::from_slice(&bytes).map_err(io_error)?;
    // Fabric API's own descriptor names itself; it never depends on itself.
    if metadata["id"] == "fabric-api" || metadata["id"] == "fabric" {
        return Ok(false);
    }
    Ok(metadata["depends"].as_object().is_some_and(|deps| {
        deps.keys().any(|id| {
            id == "fabric-api"
                || id == "fabric"
                || id == "fabric-api-base"
                || id.starts_with("fabric-command-")
                || id.starts_with("fabric-lifecycle-")
                || id.starts_with("fabric-networking-")
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fabric_jar(descriptor: &[u8]) -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().expect("temp");
        let mut zip = zip::ZipWriter::new(file.reopen().expect("open"));
        zip.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default())
            .expect("entry");
        zip.write_all(descriptor).expect("write");
        zip.finish().expect("finish");
        file
    }

    #[test]
    fn jar_metadata_detects_fabric_api_even_when_catalog_does_not() {
        let file =
            fabric_jar(br#"{"id":"example","depends":{"minecraft":"*","fabric-api-base":"*"}}"#);
        assert!(requires_fabric_api(file.path()).expect("parse"));
        assert!(packs_validate_jar(file.path(), true).is_ok());
        assert!(packs_validate_jar(file.path(), false).is_err());
    }

    #[test]
    fn fabric_api_does_not_require_itself() {
        let file =
            fabric_jar(br#"{"id":"fabric-api","depends":{"fabric-api-base":"*","minecraft":"*"}}"#);
        assert!(!requires_fabric_api(file.path()).expect("parse"));
    }
}
