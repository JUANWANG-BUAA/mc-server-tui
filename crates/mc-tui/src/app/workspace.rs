use super::{Action, AppState, Panel};
use crate::{
    instance_detail::InstanceDetailState, task_detail::TaskDetailState, transcript::TranscriptEntry,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageKind {
    Session,
    Dashboard,
    Instance(uuid::Uuid),
    Task(uuid::Uuid),
    Create,
    Import,
}
#[derive(Debug, Clone)]
pub(crate) struct Page {
    pub id: u32,
    pub kind: PageKind,
}
impl AppState {
    pub fn sync_pages(&mut self) {
        let kind = if self.wizard.is_some() {
            PageKind::Create
        } else if self.modpack_import_visible {
            PageKind::Import
        } else if self.task_detail_visible {
            self.task_detail
                .as_ref()
                .map(|t| PageKind::Task(t.task.id))
                .unwrap_or(PageKind::Session)
        } else if self.instance_detail_visible || self.backup_manager_visible {
            self.instance_detail_id
                .map(PageKind::Instance)
                .unwrap_or(PageKind::Session)
        } else if self.shell_visible {
            PageKind::Session
        } else {
            PageKind::Dashboard
        };
        let id = if let Some(page) = self.pages.iter().find(|p| p.kind == kind) {
            page.id
        } else {
            let id = self.next_page_id;
            self.next_page_id += 1;
            self.pages.push(Page { id, kind });
            id
        };
        self.active_page = id;
    }
    pub fn page_title(&self, page: &Page) -> String {
        match page.kind {
            PageKind::Session => "会话".to_owned(),
            PageKind::Dashboard => "实例总览".to_owned(),
            PageKind::Create => "新建服务器".to_owned(),
            PageKind::Import => "整合包".to_owned(),
            PageKind::Instance(id) => format!(
                "实例:{}{}",
                self.instances
                    .iter()
                    .find(|v| v.id == id)
                    .map(|v| v.name.as_str())
                    .unwrap_or("未知"),
                if self.backup_manager_visible && self.instance_detail_id == Some(id) {
                    " / 备份"
                } else {
                    ""
                }
            ),
            PageKind::Task(id) => format!("任务:{}", &id.to_string()[..8]),
        }
    }
    pub fn switch_page(&mut self, id: u32) {
        let Some(kind) = self.pages.iter().find(|p| p.id == id).map(|p| p.kind) else {
            self.push_transcript(TranscriptEntry::warning("没有这个页面编号；/pages 查看。"));
            return;
        };
        if self.backup_manager.as_ref().is_some_and(|m| m.is_busy()) {
            self.status = "备份事务运行中，完成后再切换页面。".to_owned();
            return;
        }
        if self.wizard.is_some() {
            self.parked_wizard = self.wizard.take();
        }
        self.command_palette = None;
        self.help_visible = false;
        self.modpack_import_visible = false;
        self.task_detail_visible = false;
        self.instance_detail_visible = false;
        self.backup_manager_visible = false;
        match kind {
            PageKind::Session => self.shell_visible = true,
            PageKind::Dashboard => self.shell_visible = false,
            PageKind::Create => {
                self.wizard = self.parked_wizard.take();
                if self.wizard.is_none() {
                    self.update(Action::OpenCreate);
                }
            }
            PageKind::Import => {
                if self.modpack_import.is_some() {
                    self.modpack_import_visible = true;
                } else {
                    self.update(Action::OpenModpackImport);
                }
            }
            PageKind::Instance(instance_id) => {
                if let Some(index) = self.instances.iter().position(|v| v.id == instance_id) {
                    self.selected_instance = index;
                }
                if self.instance_detail_id != Some(instance_id) {
                    if let Some(previous) = self.instance_detail.take() {
                        self.instance_cache.insert(previous.instance_id, previous);
                    }
                    self.instance_detail = Some(
                        self.instance_cache
                            .remove(&instance_id)
                            .unwrap_or_else(|| InstanceDetailState::loading(instance_id)),
                    );
                }
                self.instance_detail_id = Some(instance_id);
                self.instance_detail_visible = true;
                self.instance_detail_refresh_requested = true;
                self.focused_panel = Panel::Instances;
            }
            PageKind::Task(task_id) => {
                if self
                    .task_detail
                    .as_ref()
                    .is_some_and(|t| t.task.id != task_id)
                {
                    if let Some(previous) = self.task_detail.take() {
                        self.task_cache.insert(previous.task.id, previous);
                    }
                }
                if self
                    .task_detail
                    .as_ref()
                    .is_none_or(|t| t.task.id != task_id)
                {
                    self.task_detail = self.task_cache.remove(&task_id).or_else(|| {
                        self.tasks
                            .iter()
                            .find(|t| t.id == task_id)
                            .cloned()
                            .map(TaskDetailState::new)
                    });
                }
                self.task_detail_visible = self.task_detail.is_some();
                self.task_detail_refresh_requested = true;
            }
        }
        self.active_page = id;
    }
    pub fn cycle_page(&mut self, forward: bool) {
        self.sync_pages();
        let index = self
            .pages
            .iter()
            .position(|p| p.id == self.active_page)
            .unwrap_or(0);
        let next = if forward {
            (index + 1) % self.pages.len()
        } else {
            (index + self.pages.len() - 1) % self.pages.len()
        };
        let id = self.pages[next].id;
        self.switch_page(id);
    }
    pub(super) fn workspace_command(&mut self, name: &str, args: &str) {
        self.sync_pages();
        match name {
            "pages" => {
                let mut report = format!(
                    "工作区 · {} 个页面 · 当前 #{}\n",
                    self.pages.len(),
                    self.active_page
                );
                for p in &self.pages {
                    report.push_str(&format!(
                        "{} #{} {}\n",
                        if p.id == self.active_page { "›" } else { " " },
                        p.id,
                        self.page_title(p)
                    ));
                }
                report.push_str("/page <编号> 跳转 · /close <编号> 关闭 · Ctrl-B n/p 切换，Ctrl-B w 列表\n实例日志滚动和过滤条件会保留；关闭页面不会停止服务端。");
                self.push_transcript(TranscriptEntry::assistant(report));
            }
            "page" => {
                if let Ok(id) = args.parse() {
                    self.switch_page(id);
                } else {
                    self.push_transcript(TranscriptEntry::warning(
                        "/page <页面编号>；/pages 查看列表。",
                    ));
                }
            }
            "close" => {
                let id = args.parse().unwrap_or(self.active_page);
                self.close_page(id);
            }
            _ => {}
        }
    }
    pub fn close_page(&mut self, id: u32) {
        if id == 1 {
            self.status = "会话主页 #1 保留；/quit 退出程序。".to_owned();
            return;
        }
        let Some(index) = self.pages.iter().position(|p| p.id == id) else {
            return;
        };
        if self.active_page == id {
            self.switch_page(1);
            if self.active_page != 1 {
                return;
            }
        }
        let page = self.pages.remove(index);
        match page.kind {
            PageKind::Create => self.parked_wizard = None,
            PageKind::Instance(id) => {
                self.instance_cache.remove(&id);
            }
            PageKind::Task(id) => {
                self.task_cache.remove(&id);
            }
            _ => {}
        }
        self.status = format!("已关闭页面 #{id}；后台服务继续运行。");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_ids_are_stable_and_closing_does_not_reuse_ids() {
        let mut state = AppState::default();
        state.sync_pages();
        assert_eq!(state.active_page, 1);
        state.shell_visible = false;
        state.sync_pages();
        let dashboard = state.active_page;
        state.switch_page(1);
        state.switch_page(dashboard);
        state.sync_pages();
        assert_eq!(state.active_page, dashboard);
        state.close_page(dashboard);
        assert_eq!(state.active_page, 1);
        state.shell_visible = false;
        state.sync_pages();
        assert!(state.active_page > dashboard);
    }
    #[test]
    fn switching_away_from_a_draft_preserves_its_contents() {
        let mut state = AppState::default();
        state.update(Action::OpenCreate);
        state.wizard.as_mut().expect("draft").name = "朋友服".to_owned();
        state.sync_pages();
        let draft = state.active_page;
        state.switch_page(1);
        assert!(state.wizard.is_none());
        state.switch_page(draft);
        assert_eq!(state.wizard.as_ref().expect("draft").name, "朋友服");
    }
}
