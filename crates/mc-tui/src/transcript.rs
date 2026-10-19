#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TranscriptRole {
    User,
    Assistant,
    Success,
    Warning,
    Error,
    Muted,
}

#[derive(Debug)]
pub(crate) struct TranscriptEntry {
    pub role: TranscriptRole,
    pub text: String,
}

impl TranscriptEntry {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: TranscriptRole::User,
            text: text.into(),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: TranscriptRole::Assistant,
            text: text.into(),
        }
    }

    pub fn success(text: impl Into<String>) -> Self {
        Self {
            role: TranscriptRole::Success,
            text: text.into(),
        }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            role: TranscriptRole::Warning,
            text: text.into(),
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            role: TranscriptRole::Error,
            text: text.into(),
        }
    }

    pub fn muted(text: impl Into<String>) -> Self {
        Self {
            role: TranscriptRole::Muted,
            text: text.into(),
        }
    }

    pub fn line_count(&self) -> usize {
        self.text.lines().count().max(1)
    }
}
