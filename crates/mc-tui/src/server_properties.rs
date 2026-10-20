use mc_core::{AppError, ServerDifficulty, ServerPropertiesSnapshot, ServerPropertiesValues};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorPhase {
    Loading,
    LoadFailed,
    Edit,
    Review,
    Saving,
    Saved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerPropertiesForm {
    pub motd: String,
    pub max_players: String,
    pub difficulty: ServerDifficulty,
    pub view_distance: String,
}

impl From<&ServerPropertiesValues> for ServerPropertiesForm {
    fn from(values: &ServerPropertiesValues) -> Self {
        Self {
            motd: values.motd.clone(),
            max_players: values.max_players.to_string(),
            difficulty: values.difficulty,
            view_distance: values.view_distance.to_string(),
        }
    }
}

impl ServerPropertiesForm {
    pub fn values(&self) -> Result<ServerPropertiesValues, String> {
        let max_players = self
            .max_players
            .parse::<u16>()
            .map_err(|_| "人数上限请输入 1–1000。".to_owned())?;
        let view_distance = self
            .view_distance
            .parse::<u8>()
            .map_err(|_| "视距请输入 3–32 个区块。".to_owned())?;
        let values = ServerPropertiesValues {
            motd: self.motd.clone(),
            max_players,
            difficulty: self.difficulty,
            view_distance,
        };
        values.validate().map_err(|error| error.reason)?;
        Ok(values)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ServerPropertiesEditor {
    pub instance_id: uuid::Uuid,
    pub request_id: uuid::Uuid,
    pub phase: EditorPhase,
    pub revision: Option<String>,
    pub original: Option<ServerPropertiesValues>,
    pub draft: ServerPropertiesForm,
    pub focused_field: usize,
    pub cursors: [usize; 4],
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub conflict: bool,
    pub saved: bool,
}

impl ServerPropertiesEditor {
    pub fn loading(instance_id: uuid::Uuid) -> Self {
        Self {
            instance_id,
            request_id: uuid::Uuid::new_v4(),
            phase: EditorPhase::Loading,
            revision: None,
            original: None,
            draft: ServerPropertiesForm::from(&ServerPropertiesValues::default()),
            focused_field: 0,
            cursors: [0; 4],
            warnings: Vec::new(),
            error: None,
            conflict: false,
            saved: false,
        }
    }

    pub fn request_reload(&mut self) -> uuid::Uuid {
        self.request_id = uuid::Uuid::new_v4();
        self.phase = EditorPhase::Loading;
        self.revision = None;
        self.original = None;
        self.warnings.clear();
        self.error = None;
        self.conflict = false;
        self.saved = false;
        self.request_id
    }

    pub fn accept_load(
        &mut self,
        request_id: uuid::Uuid,
        result: Result<ServerPropertiesSnapshot, AppError>,
    ) {
        if request_id != self.request_id || self.phase != EditorPhase::Loading {
            return;
        }
        match result {
            Ok(snapshot) if snapshot.instance_id == self.instance_id => {
                self.revision = Some(snapshot.revision);
                self.original = Some(snapshot.values.clone());
                self.draft = ServerPropertiesForm::from(&snapshot.values);
                self.warnings = snapshot.warnings;
                self.phase = EditorPhase::Edit;
                self.focused_field = 0;
                self.cursors = [
                    self.draft.motd.len(),
                    self.draft.max_players.len(),
                    0,
                    self.draft.view_distance.len(),
                ];
                self.error = None;
                self.conflict = false;
            }
            Ok(_) => {
                self.phase = EditorPhase::LoadFailed;
                self.error = Some("后台返回了其他实例的配置，已忽略。".to_owned());
            }
            Err(error) => {
                self.phase = EditorPhase::LoadFailed;
                self.error = Some(error.reason);
            }
        }
    }

    pub fn begin_review(&mut self) {
        if self.phase != EditorPhase::Edit || self.conflict {
            return;
        }
        match self.draft.values() {
            Ok(_) => {
                self.error = None;
                self.phase = EditorPhase::Review;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn begin_save(&mut self) -> Option<(uuid::Uuid, String, ServerPropertiesValues)> {
        if self.phase != EditorPhase::Review {
            return None;
        }
        let values = self.draft.values().ok()?;
        let original = self.original.as_ref()?;
        if &values == original {
            self.phase = EditorPhase::Edit;
            self.error = Some("没有待保存的变化。".to_owned());
            return None;
        }
        let revision = self.revision.clone()?;
        self.request_id = uuid::Uuid::new_v4();
        self.phase = EditorPhase::Saving;
        self.error = None;
        Some((self.request_id, revision, values))
    }

    pub fn accept_save(
        &mut self,
        request_id: uuid::Uuid,
        result: Result<ServerPropertiesSnapshot, AppError>,
    ) {
        if request_id != self.request_id || self.phase != EditorPhase::Saving {
            return;
        }
        match result {
            Ok(snapshot) if snapshot.instance_id == self.instance_id => {
                self.revision = Some(snapshot.revision);
                self.original = Some(snapshot.values.clone());
                self.draft = ServerPropertiesForm::from(&snapshot.values);
                self.warnings = snapshot.warnings;
                self.phase = EditorPhase::Saved;
                self.error = None;
                self.conflict = false;
                self.saved = true;
            }
            Ok(_) => {
                self.phase = EditorPhase::Edit;
                self.error = Some("后台返回了其他实例的配置；草稿仍保留。".to_owned());
            }
            Err(error) => {
                self.phase = EditorPhase::Edit;
                self.conflict = error.code == mc_core::ErrorCode::ConcurrentModification;
                self.error = Some(error.reason);
            }
        }
    }

    pub fn move_focus(&mut self, forward: bool) {
        self.focused_field = if forward {
            (self.focused_field + 1) % 4
        } else {
            (self.focused_field + 3) % 4
        };
        self.error = None;
    }

    pub fn insert(&mut self, character: char) {
        if character.is_control() || self.phase != EditorPhase::Edit || self.conflict {
            return;
        }
        let index = self.focused_field;
        if index != 0 && !character.is_ascii_digit() {
            return;
        }
        let max_chars = match index {
            0 => 256,
            1 => 4,
            3 => 2,
            _ => 0,
        };
        let cursor = self.cursors[index];
        let Some(next_cursor) = (|| {
            let text = self.text_field_mut(index)?;
            if text.chars().count() >= max_chars {
                return None;
            }
            let cursor = cursor.min(text.len());
            text.insert(cursor, character);
            Some(cursor + character.len_utf8())
        })() else {
            return;
        };
        self.cursors[index] = next_cursor;
        self.error = None;
    }

    pub fn backspace(&mut self) {
        if self.phase != EditorPhase::Edit || self.conflict {
            return;
        }
        let index = self.focused_field;
        let cursor = self.cursors[index];
        let Some(previous) = (|| {
            let text = self.text_field_mut(index)?;
            let cursor = cursor.min(text.len());
            if cursor == 0 {
                return None;
            }
            let previous = text[..cursor]
                .char_indices()
                .next_back()
                .map(|(index, _)| index)
                .unwrap_or(cursor);
            text.replace_range(previous..cursor, "");
            Some(previous)
        })() else {
            return;
        };
        self.cursors[index] = previous;
        self.error = None;
    }

    pub fn delete(&mut self) {
        if self.phase != EditorPhase::Edit || self.conflict {
            return;
        }
        let index = self.focused_field;
        let cursor = self.cursors[index];
        let Some(()) = (|| {
            let text = self.text_field_mut(index)?;
            let cursor = cursor.min(text.len());
            let next_char = text[cursor..].chars().next()?;
            let end = cursor + next_char.len_utf8();
            text.replace_range(cursor..end, "");
            Some(())
        })() else {
            return;
        };
        self.error = None;
    }

    pub fn move_cursor(&mut self, forward: bool) {
        let index = self.focused_field;
        if index == 2 {
            self.draft.difficulty = self.draft.difficulty.step(forward);
            return;
        }
        let cursor = self.cursors[index];
        let Some(next_cursor) = (|| {
            let text = self.text_field_mut(index)?;
            let cursor = cursor.min(text.len());
            Some(if forward {
                text[cursor..]
                    .chars()
                    .next()
                    .map(|character| cursor + character.len_utf8())
                    .unwrap_or(text.len())
            } else {
                text[..cursor]
                    .char_indices()
                    .next_back()
                    .map(|(offset, _)| offset)
                    .unwrap_or(0)
            })
        })() else {
            if index == 2 {
                self.draft.difficulty = self.draft.difficulty.step(forward);
            }
            return;
        };
        self.cursors[index] = next_cursor;
    }

    pub fn cursor_home(&mut self) {
        if self.phase == EditorPhase::Edit && !self.conflict {
            self.cursors[self.focused_field] = 0;
        }
    }

    pub fn cursor_end(&mut self) {
        if self.phase != EditorPhase::Edit || self.conflict {
            return;
        }
        let index = self.focused_field;
        let end = self.text_field_mut(index).map(|text| text.len());
        if let Some(end) = end {
            self.cursors[index] = end;
        }
    }

    pub fn adjust(&mut self, increase: bool) {
        match self.focused_field {
            1 => {
                let current = self.draft.max_players.parse::<i32>().unwrap_or(20);
                self.draft.max_players = (current + if increase { 1 } else { -1 })
                    .clamp(1, 1000)
                    .to_string();
                self.cursors[1] = self.draft.max_players.len();
            }
            2 => self.draft.difficulty = self.draft.difficulty.step(increase),
            3 => {
                let current = self.draft.view_distance.parse::<i32>().unwrap_or(10);
                self.draft.view_distance = (current + if increase { 1 } else { -1 })
                    .clamp(3, 32)
                    .to_string();
                self.cursors[3] = self.draft.view_distance.len();
            }
            _ => {}
        }
        self.error = None;
    }

    pub fn field_text(&self, index: usize) -> Option<&str> {
        match index {
            0 => Some(&self.draft.motd),
            1 => Some(&self.draft.max_players),
            3 => Some(&self.draft.view_distance),
            _ => None,
        }
    }

    fn text_field_mut(&mut self, index: usize) -> Option<&mut String> {
        match index {
            0 => Some(&mut self.draft.motd),
            1 => Some(&mut self.draft.max_players),
            3 => Some(&mut self.draft.view_distance),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ServerPropertiesOperation {
    Read {
        instance_id: uuid::Uuid,
    },
    Save {
        instance_id: uuid::Uuid,
        expected_revision: String,
        values: ServerPropertiesValues,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct ServerPropertiesRequest {
    pub request_id: uuid::Uuid,
    pub operation: ServerPropertiesOperation,
}

#[derive(Debug)]
pub(crate) struct ServerPropertiesUpdate {
    pub request_id: uuid::Uuid,
    pub result: Result<ServerPropertiesSnapshot, AppError>,
}
