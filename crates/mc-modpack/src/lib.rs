//! Bounded, read-only analysis of Modrinth `.mrpack` server compatibility.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::File,
    io::{Read, Seek},
    path::{Component, Path},
};

use mc_core::{AppError, ErrorCode};
pub use mc_core::{
    ModpackAnalysisReport, ModpackFileDisposition, ModpackFilePlan, ModpackIssue, ModpackLoader,
    ModpackSeverity, ModpackStatus,
};
use serde::Deserialize;
use zip::ZipArchive;

const MAX_INDEX_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_REPORTED_FILES: usize = 1_000;
const MAX_REPORTED_ISSUES: usize = 100;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ModrinthIndex {
    format_version: u32,
    game: String,
    version_id: String,
    name: String,
    files: Vec<ModrinthFile>,
    dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ModrinthFile {
    path: String,
    #[serde(default)]
    hashes: HashMap<String, String>,
    #[serde(default)]
    env: Option<ModrinthEnvironment>,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default)]
    file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModrinthEnvironment {
    #[serde(default)]
    client: Option<String>,
    #[serde(default)]
    server: Option<String>,
}

pub fn analyze(path: impl AsRef<Path>) -> Result<ModpackAnalysisReport, AppError> {
    let path = path.as_ref();
    if path.extension().and_then(|value| value.to_str()) != Some("mrpack") {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Only Modrinth .mrpack files can be analyzed in this flow.",
        )
        .with_action("Choose a .mrpack file exported by Modrinth or a compatible launcher."));
    }
    let file = File::open(path).map_err(|error| {
        AppError::new(
            ErrorCode::InvalidInput,
            "The modpack file could not be opened.",
        )
        .with_evidence(error.to_string())
        .with_action("Check that the file exists and is readable.")
    })?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        AppError::new(
            ErrorCode::InvalidInput,
            "The modpack is not a valid ZIP archive.",
        )
        .with_evidence(error.to_string())
    })?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(AppError::new(
            ErrorCode::RequestTooLarge,
            "The modpack contains too many archive entries for safe analysis.",
        ));
    }
    let index_text = read_index(&mut archive)?;
    let index: ModrinthIndex = serde_json::from_str(&index_text).map_err(|error| {
        AppError::new(
            ErrorCode::SourceSchemaChanged,
            "modrinth.index.json does not match the supported schema.",
        )
        .with_evidence(error.to_string())
        .with_action("Use a Modrinth format version 1 export and update the importer if needed.")
    })?;
    let mut issues = Vec::new();
    if index.format_version != 1 {
        issues.push(blocker(
            "unsupported_format_version",
            "不支持的整合包格式",
            format!("formatVersion={}；当前只支持 1。", index.format_version),
            "使用 Modrinth 兼容格式版本 1 重新导出。",
        ));
    }
    if index.game != "minecraft" {
        issues.push(blocker(
            "not_minecraft",
            "不是 Minecraft 整合包",
            format!("game={}；只能分析 game=minecraft 的包。", index.game),
            "选择 Minecraft 整合包。",
        ));
    }
    let minecraft_version = index.dependencies.get("minecraft").cloned();
    if minecraft_version.is_none() {
        issues.push(blocker(
            "missing_minecraft_version",
            "缺少 Minecraft 版本",
            "dependencies.minecraft 不存在，无法判断服务端兼容性。".to_owned(),
            "在整合包元数据中锁定精确 Minecraft 版本。",
        ));
    }
    let loaders: Vec<_> = ["fabric-loader", "quilt-loader", "forge", "neoforge"]
        .into_iter()
        .filter_map(|dependency| {
            index
                .dependencies
                .get(dependency)
                .map(|version| ModpackLoader {
                    dependency: dependency.to_owned(),
                    version: version.clone(),
                })
        })
        .collect();
    if loaders.len() > 1 {
        issues.push(blocker(
            "multiple_loaders",
            "检测到多个加载器",
            loaders
                .iter()
                .map(|loader| format!("{} {}", loader.dependency, loader.version))
                .collect::<Vec<_>>()
                .join("、"),
            "只保留一个服务端加载器依赖，再重新导出整合包。",
        ));
    } else if loaders.is_empty() {
        issues.push(ModpackIssue {
            severity: ModpackSeverity::Warning,
            code: "missing_loader".to_owned(),
            title: "没有声明模组加载器".to_owned(),
            detail: "该包可能是 Vanilla 或缺少服务端加载器信息。".to_owned(),
            action: Some("确认作者提供的服务端包，或手动选择加载器后重新分析。".to_owned()),
        });
    }
    let mut seen_paths = BTreeSet::new();
    let mut hosts = BTreeSet::new();
    let mut files = Vec::new();
    let mut files_truncated = false;
    let mut install_files = 0;
    let mut review_files = 0;
    let mut excluded_files = 0;
    let mut mod_files = 0;
    let mut total_install_bytes = 0u64;
    for entry in &index.files {
        if !validate_relative_path(&entry.path) {
            issues.push(blocker(
                "unsafe_path",
                "整合包包含不安全路径",
                format!("路径越界或不是相对路径：{}", entry.path),
                "删除异常条目后重新导出。",
            ));
            continue;
        }
        if !seen_paths.insert(entry.path.clone()) {
            issues.push(blocker(
                "duplicate_path",
                "整合包包含重复路径",
                entry.path.clone(),
                "移除重复文件条目后重新导出。",
            ));
            continue;
        }
        let has_hash = has_supported_hash(&entry.hashes);
        let download_urls = entry
            .downloads
            .iter()
            .filter_map(|value| url::Url::parse(value).ok())
            .filter(|url| url.scheme() == "https" && url.host_str().is_some())
            .collect::<Vec<_>>();
        for url in &download_urls {
            if let Some(host) = url.host_str() {
                hosts.insert(host.to_owned());
            }
        }
        let verified_download = has_hash && !download_urls.is_empty();
        let (disposition, reason) = classify_file(entry);
        if disposition == ModpackFileDisposition::Install && !verified_download {
            issues.push(blocker(
                "unverifiable_download",
                "服务端必需文件无法安全下载",
                format!("{} 缺少支持的哈希或 HTTPS 下载地址。", entry.path),
                "使用带 sha1/sha512 和 HTTPS downloads 的 Modrinth 文件条目。",
            ));
        }
        match disposition {
            ModpackFileDisposition::Install => {
                install_files += 1;
                total_install_bytes =
                    total_install_bytes.saturating_add(entry.file_size.unwrap_or(0));
            }
            ModpackFileDisposition::Review => {
                review_files += 1;
                total_install_bytes =
                    total_install_bytes.saturating_add(entry.file_size.unwrap_or(0));
            }
            ModpackFileDisposition::Exclude => excluded_files += 1,
        }
        if entry.path.starts_with("mods/") && entry.path.ends_with(".jar") {
            mod_files += 1;
        }
        if files.len() < MAX_REPORTED_FILES {
            files.push(ModpackFilePlan {
                path: entry.path.clone(),
                disposition,
                reason,
                size: entry.file_size,
                has_verified_download: verified_download,
            });
        } else {
            files_truncated = true;
        }
    }
    let (server_override_files, _) = count_overrides(&mut archive)?;
    if review_files > 0 {
        issues.push(ModpackIssue {
            severity: ModpackSeverity::Warning,
            code: "server_compatibility_unknown".to_owned(),
            title: format!("{} 个文件需要人工确认", review_files),
            detail: "这些 JAR 没有明确声明 server=required，不能安全地自动安装。".to_owned(),
            action: Some("检查项目页面的服务端支持信息，或选择作者提供的服务端包。".to_owned()),
        });
    }
    if !hosts.is_empty() {
        issues.push(ModpackIssue {
            severity: ModpackSeverity::Info,
            code: "download_hosts".to_owned(),
            title: "下载域名已列出".to_owned(),
            detail: hosts.into_iter().collect::<Vec<_>>().join("、"),
            action: Some("真正下载前仍会对每个文件校验元数据哈希。".to_owned()),
        });
    }
    let blocked = issues
        .iter()
        .any(|issue| issue.severity == ModpackSeverity::Blocker);
    issues.sort_by_key(|issue| match issue.severity {
        ModpackSeverity::Blocker => 0,
        ModpackSeverity::Warning => 1,
        ModpackSeverity::Info => 2,
    });
    if issues.len() > MAX_REPORTED_ISSUES {
        let omitted = issues.len() - MAX_REPORTED_ISSUES;
        issues.truncate(MAX_REPORTED_ISSUES);
        issues.push(ModpackIssue {
            severity: ModpackSeverity::Info,
            code: "issues_truncated".to_owned(),
            title: format!("还有 {omitted} 条问题未展开"),
            detail: "问题列表已按严重级别排序并截断；摘要计数仍基于完整索引。".to_owned(),
            action: Some("先修复已列出的阻断项，再重新分析。".to_owned()),
        });
    }
    let status = if blocked {
        ModpackStatus::Blocked
    } else if review_files > 0
        || issues
            .iter()
            .any(|issue| issue.severity == ModpackSeverity::Warning)
    {
        ModpackStatus::Review
    } else {
        ModpackStatus::Ready
    };
    Ok(ModpackAnalysisReport {
        path: path.to_path_buf(),
        name: index.name,
        version_id: index.version_id,
        minecraft_version,
        loader: loaders.into_iter().next(),
        status,
        install_files,
        review_files,
        excluded_files,
        mod_files,
        total_install_bytes,
        server_override_files,
        issues,
        files,
        files_truncated,
    })
}

fn read_index<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<String, AppError> {
    let mut entry = archive.by_name("modrinth.index.json").map_err(|error| {
        AppError::new(
            ErrorCode::InvalidInput,
            "The archive does not contain modrinth.index.json.",
        )
        .with_evidence(error.to_string())
    })?;
    if entry.size() > MAX_INDEX_BYTES {
        return Err(AppError::new(
            ErrorCode::RequestTooLarge,
            "modrinth.index.json exceeds the supported size.",
        ));
    }
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).map_err(|error| {
        AppError::new(
            ErrorCode::InvalidInput,
            "Could not read modrinth.index.json.",
        )
        .with_evidence(error.to_string())
    })?;
    String::from_utf8(bytes).map_err(|error| {
        AppError::new(
            ErrorCode::SourceSchemaChanged,
            "modrinth.index.json is not valid UTF-8.",
        )
        .with_evidence(error.to_string())
    })
}

fn validate_relative_path(path: &str) -> bool {
    if path.is_empty() || path.contains('\\') || path.contains('\0') || path.starts_with('/') {
        return false;
    }
    let mut has_normal = false;
    for component in Path::new(path).components() {
        match component {
            Component::Normal(value) if !value.is_empty() => has_normal = true,
            Component::CurDir => {}
            _ => return false,
        }
    }
    has_normal
}

fn has_supported_hash(hashes: &HashMap<String, String>) -> bool {
    ["sha512", "sha1"].into_iter().any(|algorithm| {
        hashes.get(algorithm).is_some_and(|value| {
            let expected = if algorithm == "sha512" { 128 } else { 40 };
            value.len() == expected && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    })
}

fn classify_file(file: &ModrinthFile) -> (ModpackFileDisposition, String) {
    let server = file
        .env
        .as_ref()
        .and_then(|env| env.server.as_deref())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let client = file
        .env
        .as_ref()
        .and_then(|env| env.client.as_deref())
        .unwrap_or("unknown");
    match server.as_str() {
        "required" => (
            ModpackFileDisposition::Install,
            "Modrinth env.server=required".to_owned(),
        ),
        "optional" => (
            ModpackFileDisposition::Review,
            "Modrinth env.server=optional；需确认是否要启用".to_owned(),
        ),
        "unsupported" => (
            ModpackFileDisposition::Exclude,
            "Modrinth env.server=unsupported".to_owned(),
        ),
        _ if file.path.starts_with("shaderpacks/") || file.path.starts_with("resourcepacks/") => (
            ModpackFileDisposition::Exclude,
            "客户端资源目录，默认不进入服务端".to_owned(),
        ),
        _ if file.path.starts_with("mods/") && file.path.ends_with(".jar") => (
            ModpackFileDisposition::Review,
            "模组没有声明 env.server（client=".to_owned() + client + "），必须结合项目元数据确认",
        ),
        _ => (
            ModpackFileDisposition::Install,
            "客户端未排除，默认进入服务端审查".to_owned(),
        ),
    }
}

fn count_overrides<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<(usize, u64), AppError> {
    let mut count = 0usize;
    let mut bytes = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| {
            AppError::new(
                ErrorCode::InvalidInput,
                "Could not inspect an archive entry.",
            )
            .with_evidence(error.to_string())
        })?;
        if !entry.is_dir()
            && (entry.name().starts_with("server-overrides/")
                || entry.name().starts_with("overrides/"))
        {
            count += 1;
            bytes = bytes.saturating_add(entry.size());
        }
    }
    let _ = bytes;
    Ok((count, bytes))
}

fn blocker(code: &str, title: &str, detail: String, action: &str) -> ModpackIssue {
    ModpackIssue {
        severity: ModpackSeverity::Blocker,
        code: code.to_owned(),
        title: title.to_owned(),
        detail,
        action: Some(action.to_owned()),
    }
}
