//! Java discovery and bounded runtime probing.

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use mc_core::JavaRuntime;
use tokio::{process::Command, time::timeout};

const JAVA_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const JAVA_PROBE_CONCURRENCY: usize = 4;

pub(super) async fn discover_java_runtimes() -> Vec<JavaRuntime> {
    let mut candidates = tokio::task::spawn_blocking(java_candidates)
        .await
        .unwrap_or_default()
        .into_iter();
    let mut probes = tokio::task::JoinSet::new();
    for _ in 0..JAVA_PROBE_CONCURRENCY {
        let Some(executable) = candidates.next() else {
            break;
        };
        probes.spawn(async move { probe_java(&executable).await });
    }
    let mut runtimes = Vec::new();
    while let Some(result) = probes.join_next().await {
        if let Ok(Some(runtime)) = result {
            runtimes.push(runtime);
        }
        if let Some(executable) = candidates.next() {
            probes.spawn(async move { probe_java(&executable).await });
        }
    }
    runtimes.sort_by(|left, right| left.executable.cmp(&right.executable));
    runtimes
}

fn java_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = env::var_os("PATH") {
        candidates.extend(env::split_paths(&path).map(|directory| directory.join("java")));
    }
    if let Some(java_home) = env::var_os("JAVA_HOME") {
        candidates.push(PathBuf::from(java_home).join("bin/java"));
    }

    for root in [
        PathBuf::from("/Library/Java/JavaVirtualMachines"),
        PathBuf::from("/usr/lib/jvm"),
    ] {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let home = entry.path();
                candidates.push(home.join("bin/java"));
                candidates.push(home.join("Contents/Home/bin/java"));
            }
        }
    }
    if let Some(home) = env::var_os("HOME") {
        for root in [
            PathBuf::from(&home).join(".sdkman/candidates/java"),
            PathBuf::from(&home).join(".local/share/mise/installs/java"),
        ] {
            if let Ok(entries) = fs::read_dir(root) {
                for entry in entries.flatten() {
                    candidates.push(entry.path().join("bin/java"));
                }
            }
        }
    }
    let mut seen = BTreeSet::new();
    candidates.retain(|path| {
        if !path.is_file() {
            return false;
        }
        let normalized = path.canonicalize().unwrap_or_else(|_| path.clone());
        seen.insert(normalized)
    });
    candidates
}

async fn probe_java(executable: &Path) -> Option<JavaRuntime> {
    let command = Command::new(executable)
        .arg("-XshowSettings:properties")
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output();
    let output = timeout(JAVA_PROBE_TIMEOUT, command).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_java_properties(executable, &text)
}

fn parse_java_properties(executable: &Path, text: &str) -> Option<JavaRuntime> {
    let property = |key: &str| {
        text.lines()
            .find_map(|line| line.trim().strip_prefix(&format!("{key} = ")))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let version = property("java.version").or_else(|| {
        text.lines().find_map(|line| {
            let (_, version) = line.split_once("version ")?;
            version
                .trim()
                .trim_matches('"')
                .split_whitespace()
                .next()
                .map(str::to_owned)
        })
    })?;
    let major_version = property("java.specification.version")
        .or_else(|| Some(version.clone()))
        .and_then(|version| parse_java_major(&version));

    Some(JavaRuntime {
        executable: executable.display().to_string(),
        version,
        major_version,
        architecture: property("os.arch"),
        vendor: property("java.vendor"),
    })
}

fn parse_java_major(version: &str) -> Option<u16> {
    let version = version.strip_prefix("1.").unwrap_or(version);
    let major = version.split('.').next()?.parse::<u16>().ok()?;
    (major > 0).then_some(major)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_java_major_architecture_and_vendor() {
        let output = "    java.specification.version = 25\n    java.version = 25.0.1\n    os.arch = aarch64\n    java.vendor = Eclipse Adoptium\n";
        let runtime = parse_java_properties(Path::new("/opt/java/bin/java"), output)
            .expect("valid Java properties");

        assert_eq!(runtime.major_version, Some(25));
        assert_eq!(runtime.architecture.as_deref(), Some("aarch64"));
        assert_eq!(runtime.vendor.as_deref(), Some("Eclipse Adoptium"));
    }

    #[test]
    fn parses_legacy_java_version() {
        assert_eq!(parse_java_major("1.8.0_412"), Some(8));
        assert_eq!(parse_java_major("21.0.2"), Some(21));
        assert_eq!(parse_java_major("not-java"), None);
    }
}
