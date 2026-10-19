use std::path::PathBuf;

use mc_core::{AppError, ModpackAnalysisReport};

use crate::modpack_import::ModpackImportState;

use super::AppState;

impl AppState {
    pub(super) fn open_modpack_import(&mut self) {
        self.modpack_import = Some(ModpackImportState::discover());
        self.modpack_import_visible = true;
        self.status = "已扫描 Downloads、桌面和当前目录中的 .mrpack / .zip 文件。".to_owned();
    }

    pub(super) fn close_modpack_import(&mut self) {
        let Some(import) = self.modpack_import.as_mut() else {
            self.modpack_import_visible = false;
            return;
        };
        if import.loading {
            self.status = "整合包静态检查仍在运行，请稍候。".to_owned();
            return;
        }
        if import.report.is_some() {
            import.close_report();
            return;
        }
        self.modpack_import_visible = false;
        self.modpack_import = None;
        self.status = "已取消整合包导入。".to_owned();
    }

    pub(super) fn analyze_selected_modpack(&mut self) {
        if let Some(path) = self
            .modpack_import
            .as_ref()
            .and_then(|state| state.selected_path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
            })
        {
            self.modpack_import_visible = false;
            self.shell_visible = true;
            self.manage(mc_core::InstanceOperation::ImportPack { path, apply: false });
            return;
        }
        let Some(import) = self.modpack_import.as_mut() else {
            return;
        };
        if let Some(path) = import.begin_analysis() {
            self.modpack_analysis_requested = Some(path);
            self.status = "正在读取整合包清单并检查版本、加载器和 Mod 分流…".to_owned();
        } else if import.loading {
            self.status = "整合包检查仍在运行。".to_owned();
        } else if import.report.is_some() {
            self.status = "按 Esc 返回文件列表后可选择其他整合包。".to_owned();
        } else {
            self.status = "没有可分析的 .mrpack / .zip 文件；按 r 重新扫描。".to_owned();
        }
    }

    pub(super) fn refresh_modpack_import(&mut self) {
        let Some(import) = self.modpack_import.as_mut() else {
            return;
        };
        if import.loading {
            return;
        }
        import.refresh();
        self.status = if import.candidates.is_empty() {
            "仍未找到 .mrpack；请将文件放到 Downloads、桌面或当前目录。".to_owned()
        } else {
            format!("已重新扫描，找到 {} 个整合包。", import.candidates.len())
        };
    }

    pub fn apply_modpack_analysis_result(
        &mut self,
        result: Result<ModpackAnalysisReport, AppError>,
    ) {
        let Some(import) = self.modpack_import.as_mut() else {
            return;
        };
        import.apply_result(result);
        if let Some(error) = &import.error {
            self.status = format!("整合包检查失败：{error}");
        } else if let Some(report) = &import.report {
            self.status = format!(
                "整合包静态检查完成：{} · {} 个安装文件 · {} 个待确认",
                report.name, report.install_files, report.review_files
            );
        }
    }

    pub fn take_modpack_analysis_request(&mut self) -> Option<PathBuf> {
        self.modpack_analysis_requested.take()
    }
}
