use mc_core::{AppError, ServerPropertiesSnapshot, ServerPropertiesValues};

use crate::{
    server_properties::{
        EditorPhase, ServerPropertiesEditor, ServerPropertiesOperation, ServerPropertiesRequest,
    },
    transcript::TranscriptEntry,
};

use super::AppState;

impl AppState {
    pub(super) fn open_server_properties(&mut self) {
        let Some(instance_id) = self.selected_instance_id() else {
            self.push_transcript(TranscriptEntry::warning(
                "当前没有选中的实例。先输入 /use <序号>，或从实例详情按 c 打开配置。",
            ));
            return;
        };
        self.open_server_properties_for(instance_id);
    }

    pub(super) fn open_server_properties_for(&mut self, instance_id: uuid::Uuid) {
        self.server_properties_visible = true;
        self.backup_manager_visible = false;
        self.help_visible = false;
        self.command_palette = None;
        if let Some(index) = self
            .instances
            .iter()
            .position(|instance| instance.id == instance_id)
        {
            self.selected_instance = index;
        }

        let reuse_draft = self.server_properties.as_ref().is_some_and(|editor| {
            editor.instance_id == instance_id && !matches!(editor.phase, EditorPhase::Saved)
        });
        if reuse_draft {
            return;
        }
        self.begin_server_properties_load(instance_id);
    }

    fn begin_server_properties_load(&mut self, instance_id: uuid::Uuid) {
        let editor = ServerPropertiesEditor::loading(instance_id);
        let request_id = editor.request_id;
        self.server_properties = Some(editor);
        self.server_properties_requested = Some(ServerPropertiesRequest {
            request_id,
            operation: ServerPropertiesOperation::Read { instance_id },
        });
    }

    pub(super) fn close_server_properties(&mut self) {
        if !self.server_properties_visible {
            return;
        }
        self.server_properties_visible = false;
        self.status = "配置草稿已保留；再次打开可继续编辑。".to_owned();
    }

    pub(super) fn submit_server_properties(&mut self) {
        let Some(editor) = self.server_properties.as_mut() else {
            return;
        };
        match editor.phase {
            EditorPhase::Edit => editor.begin_review(),
            EditorPhase::Review => {
                let Some((request_id, expected_revision, values)) = editor.begin_save() else {
                    return;
                };
                self.server_properties_requested = Some(ServerPropertiesRequest {
                    request_id,
                    operation: ServerPropertiesOperation::Save {
                        instance_id: editor.instance_id,
                        expected_revision,
                        values,
                    },
                });
                self.status = "正在原子保存 server.properties…".to_owned();
            }
            EditorPhase::Saved => self.close_server_properties(),
            EditorPhase::LoadFailed => self.reload_server_properties(),
            EditorPhase::Loading | EditorPhase::Saving => {}
        }
    }

    pub(super) fn edit_server_properties(&mut self) {
        if let Some(editor) = self.server_properties.as_mut() {
            if editor.phase == EditorPhase::Review {
                editor.phase = EditorPhase::Edit;
                editor.error = None;
            }
        }
    }

    pub(super) fn server_properties_focus(&mut self, forward: bool) {
        if let Some(editor) = self
            .server_properties
            .as_mut()
            .filter(|editor| editor.phase == EditorPhase::Edit && !editor.conflict)
        {
            editor.move_focus(forward);
        }
    }

    pub(super) fn server_properties_insert(&mut self, character: char) {
        if let Some(editor) = self.server_properties.as_mut() {
            editor.insert(character);
        }
    }

    pub(super) fn server_properties_backspace(&mut self) {
        if let Some(editor) = self.server_properties.as_mut() {
            editor.backspace();
        }
    }

    pub(super) fn server_properties_delete(&mut self) {
        if let Some(editor) = self.server_properties.as_mut() {
            editor.delete();
        }
    }

    pub(super) fn server_properties_move_cursor(&mut self, forward: bool) {
        if let Some(editor) = self
            .server_properties
            .as_mut()
            .filter(|editor| editor.phase == EditorPhase::Edit && !editor.conflict)
        {
            editor.move_cursor(forward);
        }
    }

    pub(super) fn server_properties_cursor_home(&mut self) {
        if let Some(editor) = self.server_properties.as_mut() {
            editor.cursor_home();
        }
    }

    pub(super) fn server_properties_cursor_end(&mut self) {
        if let Some(editor) = self.server_properties.as_mut() {
            editor.cursor_end();
        }
    }

    pub(super) fn server_properties_adjust(&mut self, increase: bool) {
        if let Some(editor) = self
            .server_properties
            .as_mut()
            .filter(|editor| editor.phase == EditorPhase::Edit && !editor.conflict)
        {
            editor.adjust(increase);
        }
    }

    pub(super) fn reload_server_properties(&mut self) {
        let Some(editor) = self.server_properties.as_mut() else {
            return;
        };
        if editor.phase != EditorPhase::LoadFailed
            && !(editor.phase == EditorPhase::Edit && editor.conflict)
        {
            return;
        }
        let instance_id = editor.instance_id;
        let request_id = editor.request_reload();
        self.server_properties_requested = Some(ServerPropertiesRequest {
            request_id,
            operation: ServerPropertiesOperation::Read { instance_id },
        });
        self.status = "正在重新读取配置…".to_owned();
    }

    pub(crate) fn take_server_properties_request(&mut self) -> Option<ServerPropertiesRequest> {
        self.server_properties_requested.take()
    }

    pub(crate) fn apply_server_properties_result(
        &mut self,
        request_id: uuid::Uuid,
        result: Result<ServerPropertiesSnapshot, AppError>,
    ) {
        let Some(editor) = self.server_properties.as_mut() else {
            return;
        };
        if editor.request_id != request_id {
            return;
        }
        let failure = result.as_ref().err().cloned();
        let was_saving = editor.phase == EditorPhase::Saving;
        editor.accept_result(request_id, result);
        self.status = if was_saving && editor.phase == EditorPhase::Saved {
            "配置已保存；新值将在下次启动时生效。".to_owned()
        } else if editor.phase == EditorPhase::LoadFailed {
            editor
                .error
                .clone()
                .unwrap_or_else(|| "读取配置失败。".to_owned())
        } else {
            "配置已读取。".to_owned()
        };
        if let Some(error) = failure {
            self.show_error(&error);
        }
    }
}

impl ServerPropertiesEditor {
    fn accept_result(
        &mut self,
        request_id: uuid::Uuid,
        result: Result<ServerPropertiesSnapshot, AppError>,
    ) {
        match self.phase {
            EditorPhase::Loading => self.accept_load(request_id, result),
            EditorPhase::Saving => self.accept_save(request_id, result),
            _ => {}
        }
    }

    pub fn current_draft_values(&self) -> Result<ServerPropertiesValues, String> {
        self.draft.values()
    }
}
