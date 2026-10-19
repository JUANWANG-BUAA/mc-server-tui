//! Fabric meta publishes no checksum for its generated launch JAR, so the first
//! official HTTPS fetch fixes the expected SHA-256 and the copy installed
//! through the verified artifact cache must match those exact bytes.
use super::{invalid, io_error, Context};
use mc_core::{AppError, ArtifactPlan, Checksum, ReleaseChannel, TaskPhase, TaskResult};
use sha2::{Digest, Sha256};

pub(super) async fn create(
    instances: mc_instance::InstanceService,
    context: &Context,
    name: String,
    game_version: String,
    accept_eula: bool,
    reporter: mc_task::TaskReporter,
) -> Result<TaskResult, AppError> {
    if !accept_eula {
        return Err(invalid(
            "请先阅读 https://www.minecraft.net/eula，再明确添加 --accept-eula。",
        ));
    }
    if game_version.is_empty()
        || game_version.len() > 32
        || !game_version
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'.')
    {
        return Err(invalid(
            "Fabric 版本需为正式 Minecraft 版本，如 1.21.11 或 26.1。",
        ));
    }
    reporter
        .set_phase(
            TaskPhase::ResolvingArtifact,
            "从 Fabric 官方解析稳定 Loader 与 Installer",
            None,
        )
        .await?;
    let loaders = context
        .upstream
        .json(&format!(
            "https://meta.fabricmc.net/v2/versions/loader/{game_version}"
        ))
        .await?;
    let loader = loaders
        .as_array()
        .and_then(|xs| xs.iter().find(|v| v["loader"]["stable"] == true))
        .and_then(|v| v["loader"]["version"].as_str())
        .ok_or_else(|| invalid("此 Minecraft 版本没有稳定 Fabric Loader。"))?;
    let installers = context
        .upstream
        .json("https://meta.fabricmc.net/v2/versions/installer")
        .await?;
    let installer = installers
        .as_array()
        .and_then(|xs| xs.iter().find(|v| v["stable"] == true))
        .and_then(|v| v["version"].as_str())
        .ok_or_else(|| invalid("Fabric 未返回稳定安装器。"))?;
    for version in [loader, installer] {
        if !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
        {
            return Err(invalid("Fabric 元数据包含无效版本。"));
        }
    }
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader}/{installer}/server/jar"
    );
    let bytes = context.upstream.bytes(&url, 16 * 1024 * 1024).await?;
    if bytes.is_empty() {
        return Err(invalid("Fabric 启动器为空。"));
    }
    let checksum = Checksum::sha256(format!("{:x}", Sha256::digest(&bytes)))?;
    let plan = ArtifactPlan::new(
        "fabric",
        "fabric",
        &game_version,
        1,
        ReleaseChannel("STABLE".to_owned()),
        format!("fabric-{game_version}-{loader}.jar"),
        url::Url::parse(&url).map_err(io_error)?,
        url::Url::parse(&url).map_err(io_error)?,
        bytes.len() as u64,
        checksum,
        chrono::Utc::now(),
    )?;
    let required = mc_core::minecraft_java_requirement(&game_version);
    let doctor = mc_system::run_doctor().await;
    let java = doctor
        .java_runtimes
        .iter()
        .filter(|v| v.major_version.is_some_and(|m| m >= required))
        .min_by_key(|v| v.major_version)
        .ok_or_else(|| {
            invalid(format!(
                "Minecraft {game_version} 需要 Java {required} 或更高版本。"
            ))
        })?;
    let reserved = instances
        .list_async()
        .await?
        .into_iter()
        .map(|v| v.port)
        .collect::<std::collections::HashSet<_>>();
    let port = (25565..25665)
        .find(|port| {
            !reserved.contains(port) && std::net::TcpListener::bind(("127.0.0.1", *port)).is_ok()
        })
        .ok_or_else(|| invalid("未找到可用的服务端端口。"))?;
    let spec = mc_instance::InstanceSpec {
        name,
        java_path: java.executable.clone().into(),
        min_memory_mb: 1024,
        max_memory_mb: 4096,
        port,
        accept_eula,
    };
    spec.validate_for_creation_async().await?;
    let config = mc_download::DownloadConfig {
        cache_root: context.cache_dir.clone(),
        user_agent: context.user_agent.clone(),
        allowed_hosts: ["meta.fabricmc.net".to_owned()].into_iter().collect(),
        max_artifact_size: 16 * 1024 * 1024,
    };
    let downloader = mc_download::DownloadManager::new(config)?;
    let blob = crate::dispatch::download_with_progress(&downloader, &plan, &reporter).await?;
    if reporter.is_cancelled() {
        return Err(mc_core::AppError::new(
            mc_core::ErrorCode::TaskCancelled,
            "Fabric 创建已取消。",
        ));
    }
    reporter
        .set_phase(
            TaskPhase::InstallingInstance,
            "安装 Fabric 实例；启动时 Fabric 会下载所需运行库",
            None,
        )
        .await?;
    Ok(TaskResult::InstanceCreated(
        instances.create(&plan, &blob, &spec).await?,
    ))
}
