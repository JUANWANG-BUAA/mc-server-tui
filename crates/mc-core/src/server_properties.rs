use serde::{Deserialize, Serialize};

use crate::{AppError, ErrorCode};

/// The small, deliberately bounded set of server.properties fields exposed in
/// the first editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPropertiesValues {
    pub motd: String,
    pub max_players: u16,
    pub difficulty: ServerDifficulty,
    pub view_distance: u8,
}

impl Default for ServerPropertiesValues {
    fn default() -> Self {
        Self {
            motd: "A Minecraft Server".to_owned(),
            max_players: 20,
            difficulty: ServerDifficulty::Easy,
            view_distance: 10,
        }
    }
}

impl ServerPropertiesValues {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.motd.chars().count() > 256 || self.motd.chars().any(char::is_control) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "MOTD 最多 256 个字符，且不能包含换行或控制字符。",
            ));
        }
        if !(1..=1000).contains(&self.max_players) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "人数上限请输入 1–1000。",
            ));
        }
        if !(3..=32).contains(&self.view_distance) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "视距请输入 3–32 个区块。",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerDifficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl ServerDifficulty {
    pub const ALL: [Self; 4] = [Self::Peaceful, Self::Easy, Self::Normal, Self::Hard];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Peaceful => "peaceful",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Peaceful => "和平",
            Self::Easy => "简单",
            Self::Normal => "普通",
            Self::Hard => "困难",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "peaceful" => Some(Self::Peaceful),
            "easy" => Some(Self::Easy),
            "normal" => Some(Self::Normal),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }

    pub fn step(self, forward: bool) -> Self {
        let index = Self::ALL.iter().position(|item| *item == self).unwrap_or(0);
        let next = if forward {
            (index + 1) % Self::ALL.len()
        } else {
            (index + Self::ALL.len() - 1) % Self::ALL.len()
        };
        Self::ALL[next]
    }
}

/// A revision identifies the exact bytes read from server.properties. The
/// daemon requires it again before saving so a concurrent external edit is
/// never silently overwritten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPropertiesSnapshot {
    pub instance_id: uuid::Uuid,
    pub revision: String,
    pub values: ServerPropertiesValues,
    #[serde(default)]
    pub warnings: Vec<String>,
}
