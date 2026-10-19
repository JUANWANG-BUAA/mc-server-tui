use ratatui::layout::{Position, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HitTarget {
    Instance(usize),
    Task(usize),
    Version(usize),
    Java(usize),
    Memory(usize),
    WizardField(usize),
    PaletteEntry(usize),
    Backup(usize),
    Modpack(usize),
    ToggleEula,
    HelpClose,
    ShellSuggestion(usize),
}

/// Regions recorded during the last draw pass. Input handling reads this map
/// instead of duplicating layout math in the event loop.
#[derive(Debug, Default)]
pub(crate) struct HitMap {
    regions: Vec<(Rect, HitTarget)>,
}

impl HitMap {
    pub(crate) fn push(&mut self, area: Rect, target: HitTarget) {
        if area.width > 0 && area.height > 0 {
            self.regions.push((area, target));
        }
    }

    pub(crate) fn target_at(&self, column: u16, row: u16) -> Option<HitTarget> {
        let position = Position::new(column, row);
        self.regions
            .iter()
            .rev()
            .find(|(area, _)| area.contains(position))
            .map(|(_, target)| *target)
    }
}
