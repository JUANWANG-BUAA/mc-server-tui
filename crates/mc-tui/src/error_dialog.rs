//! Error dialog component for displaying user-friendly error messages.

use mc_core::ErrorMessage;

#[derive(Debug, Clone)]
pub(crate) struct ErrorDialogState {
    /// The error message to display
    pub message: ErrorMessage,
    /// Whether to show technical details (collapsed by default)
    pub show_details: bool,
    /// Index of the selected action (for keyboard navigation)
    pub selected_action: usize,
}

impl ErrorDialogState {
    pub fn new(message: ErrorMessage) -> Self {
        Self {
            message,
            show_details: false,
            selected_action: 0,
        }
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    pub fn select_next_action(&mut self) {
        if !self.message.actions.is_empty() {
            self.selected_action = (self.selected_action + 1) % self.message.actions.len();
        }
    }

    pub fn select_previous_action(&mut self) {
        if !self.message.actions.is_empty() {
            if self.selected_action == 0 {
                self.selected_action = self.message.actions.len() - 1;
            } else {
                self.selected_action -= 1;
            }
        }
    }

    pub fn has_actions(&self) -> bool {
        !self.message.actions.is_empty()
    }

    pub fn diagnostic_report(&self) -> String {
        self.message.diagnostic_report()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_dialog_state_creation() {
        let msg = ErrorMessage::new("Test", "Reason");
        let state = ErrorDialogState::new(msg);
        assert!(!state.show_details);
        assert_eq!(state.selected_action, 0);
    }

    #[test]
    fn toggle_details() {
        let msg = ErrorMessage::new("Test", "Reason");
        let mut state = ErrorDialogState::new(msg);
        assert!(!state.show_details);
        state.toggle_details();
        assert!(state.show_details);
        state.toggle_details();
        assert!(!state.show_details);
    }

    #[test]
    fn action_navigation() {
        let msg = ErrorMessage::new("Test", "Reason")
            .with_actions(["Action 1", "Action 2", "Action 3"]);
        let mut state = ErrorDialogState::new(msg);

        assert_eq!(state.selected_action, 0);
        state.select_next_action();
        assert_eq!(state.selected_action, 1);
        state.select_next_action();
        assert_eq!(state.selected_action, 2);
        state.select_next_action();
        assert_eq!(state.selected_action, 0); // wraps around

        state.select_previous_action();
        assert_eq!(state.selected_action, 2); // wraps around
    }
}
