use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use mc_core::{AppError, ModpackAnalysisReport};

const MAX_CANDIDATES: usize = 200;

#[derive(Debug)]
pub(crate) struct ModpackImportState {
    pub candidates: Vec<PathBuf>,
    pub selected: usize,
    pub report: Option<ModpackAnalysisReport>,
    pub error: Option<String>,
    pub loading: bool,
}

impl ModpackImportState {
    pub fn discover() -> Self {
        let candidates = discover_candidates();
        let error = if candidates.is_empty() {
            Some("Downloads、桌面和当前目录中未找到 .mrpack / .zip；按 r 重新扫描。".to_owned())
        } else {
            None
        };
        Self {
            candidates,
            selected: 0,
            report: None,
            error,
            loading: false,
        }
    }

    pub fn refresh(&mut self) {
        self.candidates = discover_candidates();
        self.selected = self.selected.min(self.candidates.len().saturating_sub(1));
        self.report = None;
        self.loading = false;
        self.error = if self.candidates.is_empty() {
            Some("没有找到可分析的 .mrpack / .zip 文件。".to_owned())
        } else {
            None
        };
    }

    pub fn select_previous(&mut self) {
        if self.report.is_none() && !self.loading {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub fn select_next(&mut self) {
        if self.report.is_none() && !self.loading && !self.candidates.is_empty() {
            self.selected = (self.selected + 1).min(self.candidates.len() - 1);
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.candidates.get(self.selected).cloned()
    }

    pub fn begin_analysis(&mut self) -> Option<PathBuf> {
        if self.loading || self.report.is_some() {
            return None;
        }
        let path = self.selected_path()?;
        self.loading = true;
        self.error = None;
        Some(path)
    }

    pub fn apply_result(&mut self, result: Result<ModpackAnalysisReport, AppError>) {
        self.loading = false;
        match result {
            Ok(report) => {
                self.report = Some(report);
                self.error = None;
            }
            Err(error) => {
                self.report = None;
                self.error = Some(match error.next_action {
                    Some(action) => format!("{} · 下一步：{}", error.reason, action),
                    None => error.reason,
                });
            }
        }
    }

    pub fn close_report(&mut self) {
        self.report = None;
        self.error = None;
    }
}

fn discover_candidates() -> Vec<PathBuf> {
    discover_candidates_from(default_roots())
}

pub(crate) fn discover_candidates_from(roots: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for root in roots {
        collect_mrpack_files(&root, 0, &mut candidates);
        if candidates.len() >= MAX_CANDIDATES {
            break;
        }
    }
    candidates.sort_by_key(|left| std::cmp::Reverse(modified(left)));
    candidates.truncate(MAX_CANDIDATES);
    candidates
}

fn default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        roots.push(current);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join("Downloads"));
        roots.push(home.join("Desktop"));
    }
    roots
}

fn collect_mrpack_files(directory: &Path, depth: usize, output: &mut Vec<PathBuf>) {
    if depth > 2 || output.len() >= MAX_CANDIDATES {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if output.len() >= MAX_CANDIDATES {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if !name.starts_with('.') {
                collect_mrpack_files(&path, depth + 1, output);
            }
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| {
                ext.eq_ignore_ascii_case("mrpack") || ext.eq_ignore_ascii_case("zip")
            })
        {
            output.push(path);
        }
    }
}

fn modified(path: &PathBuf) -> SystemTime {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}
