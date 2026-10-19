//! Bounded portable packs. Imports stage and validate everything before adding files;
//! existing files are never overwritten, and worlds/identities stay out of packs.
use super::{invalid, io_error, safe_path};
use mc_core::{AppError, InstanceSummary};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
const LIMIT: u64 = 512 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Manifest {
    format: u32,
    provider: String,
    game_version: String,
    files: Vec<PackFile>,
}
#[derive(Serialize, Deserialize)]
struct PackFile {
    path: String,
    sha256: String,
}

fn allowed(path: &Path) -> bool {
    let parts = path
        .components()
        .filter_map(|c| {
            if let std::path::Component::Normal(v) = c {
                v.to_str()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    match parts.as_slice() {
        ["mods", name] => name.ends_with(".jar"),
        ["plugins", name] => name.ends_with(".jar"),
        ["config", ..] => parts.len() > 1,
        _ => false,
    }
}
fn archive_path(raw: &str) -> Result<PathBuf, AppError> {
    if raw.contains(['\\', ':']) || raw.chars().any(char::is_control) {
        return Err(invalid("压缩包包含不安全的文件路径。"));
    }
    let p = Path::new(raw);
    if p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(invalid("压缩包包含越界路径。"));
    }
    Ok(p.to_owned())
}
fn open(path: &Path) -> Result<ZipArchive<File>, AppError> {
    let archive = ZipArchive::new(File::open(path).map_err(io_error)?).map_err(io_error)?;
    if archive.len() > 10_000 {
        return Err(invalid("压缩包文件数超过 10000。"));
    }
    Ok(archive)
}
fn metadata(
    archive: &mut ZipArchive<File>,
    instance: &InstanceSummary,
) -> Result<Option<serde_json::Value>, AppError> {
    if let Ok(mut file) = archive.by_name("mc-pack.json") {
        let mut bytes = Vec::new();
        file.by_ref()
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(invalid("整合包清单过大。"));
        }
        let m: Manifest = serde_json::from_slice(&bytes).map_err(io_error)?;
        if m.format != 1
            || m.provider != instance.provider
            || m.game_version != instance.game_version
        {
            return Err(invalid(format!(
                "整合包需要 {} {}，当前实例为 {} {}。请创建匹配实例。",
                m.provider, m.game_version, instance.provider, instance.game_version
            )));
        }
        return Ok(None);
    }
    if let Ok(mut file) = archive.by_name("modrinth.index.json") {
        let mut bytes = Vec::new();
        file.by_ref()
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(invalid("Modrinth 清单过大。"));
        }
        let m: serde_json::Value = serde_json::from_slice(&bytes).map_err(io_error)?;
        if m["formatVersion"] != 1 || m["game"] != "minecraft" {
            return Err(invalid("不支持的 Modrinth 整合包格式。"));
        }
        let dependencies = &m["dependencies"];
        if dependencies["minecraft"] != instance.game_version {
            return Err(invalid(format!(
                "整合包需要 Minecraft {}；当前为 {}。",
                dependencies["minecraft"], instance.game_version
            )));
        }
        let provider = if dependencies.get("fabric-loader").is_some() {
            "fabric"
        } else if dependencies.get("forge").is_some() || dependencies.get("neoforge").is_some() {
            "forge"
        } else {
            "paper"
        };
        if provider != instance.provider {
            return Err(invalid(format!(
                "整合包需要 {provider}，当前为 {}。Fabric 包可先 /fabric <名称> {} --accept-eula 创建实例。",
                instance.provider, instance.game_version
            )));
        }
        return Ok(Some(m));
    }
    if archive.by_name("manifest.json").is_ok() {
        return Err(invalid(
            "这是 CurseForge 客户端清单包。请下载作者提供的 server pack ZIP；\
             它的文件 ID 需要 CurseForge API 授权，不能当作现成服务端文件导入。",
        ));
    }
    Ok(None)
}
/// Server-relevant archive entries. MRPACK `server-overrides/` replace the same
/// path from `overrides/` as the format specifies; any other duplicate or
/// case-only collision rejects the archive.
fn entries(archive: &mut ZipArchive<File>) -> Result<Vec<(usize, PathBuf)>, AppError> {
    let mut chosen: HashMap<String, (u8, usize, PathBuf)> = HashMap::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(io_error)?;
        let path = archive_path(file.name().trim_end_matches('/'))?;
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err(invalid("压缩包含符号链接，未导入。"));
        }
        if file.is_dir() {
            continue;
        }
        total = total
            .checked_add(file.size())
            .ok_or_else(|| invalid("文件大小溢出。"))?;
        if total > LIMIT || file.size() > 256 * 1024 * 1024 {
            return Err(invalid("整合包展开超过 512 MiB 或单文件超过 256 MiB。"));
        }
        let (priority, relative) = if let Ok(p) = path.strip_prefix("server-overrides") {
            (2, p.to_owned())
        } else if let Ok(p) = path.strip_prefix("overrides") {
            (1, p.to_owned())
        } else if path.starts_with("client-overrides") {
            // MRPACK client-only files never belong on a server.
            continue;
        } else if allowed(&path) {
            (0, path.clone())
        } else {
            // Common server packs wrap their payload in one directory.
            match path.to_str().and_then(|v| v.split_once('/')) {
                Some((_, tail)) => (0, PathBuf::from(tail)),
                None => continue,
            }
        };
        if !allowed(&relative) {
            continue;
        }
        let key = relative.to_string_lossy().to_lowercase();
        match chosen.get(&key) {
            Some((existing, ..)) if *existing == priority => {
                return Err(invalid("压缩包含重复或大小写冲突路径。"));
            }
            Some((existing, ..)) if *existing > priority => {}
            _ => {
                chosen.insert(key, (priority, i, relative));
            }
        }
    }
    let mut entries: Vec<_> = chosen.into_values().map(|(_, i, p)| (i, p)).collect();
    entries.sort_by_key(|(i, _)| *i);
    Ok(entries)
}

pub(super) fn preview(instance: &InstanceSummary, path: &Path) -> Result<String, AppError> {
    let mut zip = open(path)?;
    let metadata = metadata(&mut zip, instance)?;
    let entries = entries(&mut zip)?;
    if entries.is_empty() && metadata.is_none() {
        return Err(invalid(
            "未找到 mods/*.jar、plugins/*.jar 或 config/。\
             只导入兼容的扩展和配置，不执行安装脚本、不覆盖世界或服务端核心。",
        ));
    }
    let downloads = metadata
        .as_ref()
        .and_then(|v| v["files"].as_array())
        .map(|v| {
            v.iter()
                .filter(|f| f["env"]["server"] != "unsupported")
                .count()
        })
        .unwrap_or(0);
    let mut text = format!(
        "导入预览 · {}\n目标：{} · {} {}\n本地文件 {} 个 · Modrinth 服务端下载 {} 个\n",
        path.display(),
        instance.name,
        instance.provider,
        instance.game_version,
        entries.len(),
        downloads
    );
    for (_, p) in entries.iter().take(40) {
        text.push_str(&format!("  {}\n", p.display()));
    }
    text.push_str(&format!(
        "\n确认后停服，再输入 /import {} --apply\n\
         已有同内容文件跳过；不同内容冲突会拒绝整次导入。\
         世界、玩家数据、server.jar 和 server.properties 不会导入。",
        path.display()
    ));
    Ok(text)
}

pub(super) async fn import(
    upstream: &super::addons::Upstream,
    supervisor: &mc_supervisor::ProcessSupervisor,
    id: uuid::Uuid,
    path: &Path,
    reporter: mc_task::TaskReporter,
) -> Result<String, AppError> {
    let guard = supervisor.lock_stopped_instance(id).await?;
    let instance = guard.instance().clone();
    let path = path.to_owned();
    let (stage, mut files, metadata) = tokio::task::spawn_blocking(move || {
        let stage = tempfile::tempdir_in(&instance.directory).map_err(io_error)?;
        let mut zip = open(&path)?;
        let metadata = metadata(&mut zip, &instance)?;
        let entries = entries(&mut zip)?;
        let mut files = Vec::new();
        for (i, p) in entries {
            if (instance.provider == "paper" && p.starts_with("mods"))
                || (instance.provider == "fabric" && p.starts_with("plugins"))
            {
                return Err(invalid("包内的 mods/plugins 与当前服务端加载器不兼容。"));
            }
            let temp = stage.path().join(&p);
            std::fs::create_dir_all(temp.parent().ok_or_else(|| invalid("路径无父目录。"))?)
                .map_err(io_error)?;
            let mut file = zip.by_index(i).map_err(io_error)?;
            let mut out = File::create(&temp).map_err(io_error)?;
            let copied = std::io::copy(&mut file.by_ref().take(256 * 1024 * 1024 + 1), &mut out)
                .map_err(io_error)?;
            if copied > 256 * 1024 * 1024 {
                return Err(invalid("展开文件超过限制。"));
            }
            out.sync_all().map_err(io_error)?;
            if p.extension().is_some_and(|v| v == "jar") {
                super::addons::packs_validate_jar(&temp, instance.provider == "fabric")?;
            }
            let target = safe_path(&instance.directory.join("server"), &p)?;
            files.push((temp, target));
        }
        if let Ok(mut manifest) = zip.by_name("mc-pack.json") {
            let mut bytes = Vec::new();
            manifest
                .by_ref()
                .take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(io_error)?;
            let m: Manifest = serde_json::from_slice(&bytes).map_err(io_error)?;
            for declared in m.files {
                let p = archive_path(&declared.path)?;
                if !allowed(&p) {
                    return Err(invalid("清单引用了不允许的文件。"));
                }
                let file = stage.path().join(p);
                if hash(&file)? != declared.sha256 {
                    return Err(invalid("整合包 SHA-256 校验不匹配。"));
                }
            }
        }
        Ok::<_, AppError>((stage, files, metadata))
    })
    .await
    .map_err(io_error)??;
    if let Some(metadata) = metadata {
        let mut total = 0u64;
        for item in metadata["files"].as_array().into_iter().flatten() {
            if item["env"]["server"] == "unsupported" {
                continue;
            }
            let p = archive_path(item["path"].as_str().unwrap_or(""))?;
            if !allowed(&p) {
                return Err(invalid("Modrinth 清单引用了不允许的服务端路径。"));
            }
            let instance = guard.instance();
            if (instance.provider == "paper" && p.starts_with("mods"))
                || (instance.provider == "fabric" && p.starts_with("plugins"))
            {
                return Err(invalid("Modrinth 文件的加载器不兼容。"));
            }
            let size = item["fileSize"].as_u64().unwrap_or(0);
            total = total.saturating_add(size);
            if total > LIMIT {
                return Err(invalid("整合包下载超过 512 MiB。"));
            }
            let temp = stage.path().join(&p);
            if temp.exists() {
                return Err(invalid("清单和覆盖文件路径冲突。"));
            }
            std::fs::create_dir_all(temp.parent().ok_or_else(|| invalid("路径无父目录。"))?)
                .map_err(io_error)?;
            let url = item["downloads"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .find(|v| {
                    url::Url::parse(v).is_ok_and(|u| {
                        u.scheme() == "https" && u.host_str() == Some("cdn.modrinth.com")
                    })
                })
                .ok_or_else(|| invalid("整合包下载缺少可信的 Modrinth CDN 地址。"))?;
            upstream
                .download_file(
                    url,
                    size,
                    item["hashes"]["sha512"].as_str().unwrap_or(""),
                    &temp,
                    &reporter,
                )
                .await?;
            if p.extension().is_some_and(|v| v == "jar") {
                super::addons::packs_validate_jar(&temp, instance.provider == "fabric")?;
            }
            files.push((temp, safe_path(&instance.directory.join("server"), &p)?));
        }
    }
    if reporter.is_cancelled() {
        return Err(AppError::new(
            mc_core::ErrorCode::TaskCancelled,
            "导入已取消，未写入实例。",
        ));
    }
    if files.is_empty() {
        return Err(invalid("整合包没有可导入的服务端扩展或配置。"));
    }
    let commit = files.clone();
    let added = tokio::task::spawn_blocking(move || commit_files(&commit))
        .await
        .map_err(io_error)??;
    Ok(format!(
        "整合包导入完成：新增 {added} 个文件，同内容跳过 {} 个。\n/start 启动并 /logs 检查加载结果。",
        files.len() - added
    ))
}

fn hash(path: &Path) -> Result<String, AppError> {
    let mut f = File::open(path).map_err(io_error)?;
    let mut h = Sha256::new();
    let mut b = [0; 65536];
    loop {
        let n = f.read(&mut b).map_err(io_error)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

pub(super) fn commit_files(files: &[(PathBuf, PathBuf)]) -> Result<usize, AppError> {
    let mut unique = HashSet::new();
    for (from, to) in files {
        if !unique.insert(to.to_string_lossy().to_lowercase()) {
            return Err(invalid("导入目标重复。"));
        }
        if to.exists() && hash(from)? != hash(to)? {
            return Err(invalid(format!(
                "文件冲突：{}。未覆盖任何现有文件；请使用新实例导入。",
                to.display()
            )));
        }
    }
    let mut committed = Vec::new();
    for (from, to) in files {
        if to.exists() {
            continue;
        }
        let result = (|| {
            std::fs::create_dir_all(to.parent().ok_or_else(|| invalid("目标无父目录。"))?)
                .map_err(io_error)?;
            std::fs::hard_link(from, to).map_err(io_error)
        })();
        if let Err(error) = result {
            for p in committed {
                let _ = std::fs::remove_file(p);
            }
            return Err(error);
        }
        committed.push(to.clone());
    }
    Ok(committed.len())
}

pub(super) fn export(
    guard: mc_supervisor::StoppedInstanceGuard,
    path: &Path,
) -> Result<String, AppError> {
    let instance = guard.instance();
    let root = instance.directory.join("server");
    let parent = path
        .parent()
        .filter(|v| !v.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(io_error)?;
    if parent.starts_with(instance.directory.canonicalize().map_err(io_error)?) {
        return Err(invalid("请将整合包导出到实例目录之外。"));
    }
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    let mut zip = ZipWriter::new(staged.as_file_mut());
    let mut manifest = Manifest {
        format: 1,
        provider: instance.provider.clone(),
        game_version: instance.game_version.clone(),
        files: Vec::new(),
    };
    fn walk(root: &Path, path: &Path, files: &mut Vec<PathBuf>) -> Result<(), AppError> {
        if !path.exists() {
            return Ok(());
        }
        if std::fs::symlink_metadata(path)
            .map_err(io_error)?
            .file_type()
            .is_symlink()
        {
            return Err(invalid("导出内容含符号链接。"));
        }
        if path.is_dir() {
            for item in std::fs::read_dir(path).map_err(io_error)? {
                walk(root, &item.map_err(io_error)?.path(), files)?;
            }
        } else if let Ok(p) = path.strip_prefix(root) {
            if allowed(p) {
                files.push(p.to_owned());
            }
        }
        if files.len() > 10_000 {
            return Err(invalid("导出文件数超过 10000。"));
        }
        Ok(())
    }
    let mut files = Vec::new();
    for folder in ["mods", "plugins", "config"] {
        walk(&root, &root.join(folder), &mut files)?;
    }
    files.sort();
    let mut total = 0u64;
    for p in files {
        let source = safe_path(&root, &p)?;
        let size = source.metadata().map_err(io_error)?.len();
        total = total.saturating_add(size);
        if total > LIMIT {
            return Err(invalid("导出内容超过 512 MiB。"));
        }
        let name = p.to_string_lossy().replace('\\', "/");
        manifest.files.push(PackFile {
            path: name.clone(),
            sha256: hash(&source)?,
        });
        zip.start_file(
            &name,
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .map_err(io_error)?;
        std::io::copy(&mut File::open(source).map_err(io_error)?, &mut zip).map_err(io_error)?;
    }
    zip.start_file("mc-pack.json", SimpleFileOptions::default())
        .map_err(io_error)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest).map_err(io_error)?)
        .map_err(io_error)?;
    zip.finish().map_err(io_error)?;
    staged.as_file().sync_all().map_err(io_error)?;
    staged.persist_noclobber(path).map_err(io_error)?;
    Ok(format!(
        "已导出 {} 个文件：{}\n\
         包含兼容扩展、config 配置及 SHA-256 清单；\
         不含世界、玩家、管理员名单、服务端核心或 server.properties。\n\
         这是服务端整合包，不是客户端启动器格式。",
        manifest.files.len(),
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_and_loader_content_are_restricted() {
        for p in ["../x", "/tmp/x", "C:\\x", "mods/../../x"] {
            assert!(archive_path(p).is_err());
        }
        assert!(!allowed(Path::new("server.properties")));
        assert!(!allowed(Path::new("world/level.dat")));
        assert!(allowed(Path::new("mods/example.jar")));
    }
    #[test]
    fn server_overrides_replace_overrides_and_same_level_duplicates_fail() {
        fn archive(names: &[&str]) -> tempfile::NamedTempFile {
            let file = tempfile::NamedTempFile::new().expect("temp");
            let mut zip = ZipWriter::new(file.reopen().expect("open"));
            for name in names {
                zip.start_file(*name, SimpleFileOptions::default())
                    .expect("entry");
                zip.write_all(name.as_bytes()).expect("write");
            }
            zip.finish().expect("finish");
            file
        }
        let pack = archive(&[
            "overrides/config/a.toml",
            "server-overrides/config/a.toml",
            "client-overrides/config/b.toml",
            "overrides/mods/x.jar",
        ]);
        let mut zip = open(pack.path()).expect("open");
        let chosen = entries(&mut zip).expect("entries");
        assert_eq!(chosen.len(), 2);
        let (index, path) = &chosen[0];
        assert_eq!(path, Path::new("config/a.toml"));
        assert_eq!(
            zip.by_index(*index).expect("entry").name(),
            "server-overrides/config/a.toml"
        );
        let duplicate = archive(&["mods/x.jar", "mods/X.jar"]);
        assert!(entries(&mut open(duplicate.path()).expect("open")).is_err());
    }
    #[test]
    fn conflicts_do_not_partially_import() {
        let dir = tempfile::tempdir().expect("temp");
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        let existing = dir.path().join("existing");
        let new = dir.path().join("new");
        std::fs::write(&a, "new").expect("write");
        std::fs::write(&b, "replacement").expect("write");
        std::fs::write(&existing, "original").expect("write");
        assert!(commit_files(&[(a, new.clone()), (b, existing.clone())]).is_err());
        assert!(!new.exists());
        assert_eq!(std::fs::read_to_string(existing).expect("read"), "original");
    }
}
