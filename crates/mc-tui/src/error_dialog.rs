//! Presentation state for an operation failure. Recovery stays explicit.
use mc_core::ErrorMessage;
use std::cell::Cell;

#[derive(Debug, Clone)]
pub(crate) struct ErrorDialogState {
    pub message: ErrorMessage,
    pub show_details: bool,
    pub scroll: usize,
    pub max_scroll: Cell<usize>,
}

impl ErrorDialogState {
    pub fn new(message: ErrorMessage) -> Self {
        Self {
            message,
            show_details: false,
            scroll: 0,
            max_scroll: Cell::new(0),
        }
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
        self.scroll = 0;
    }
}
