/// Declares whether the active TUI surface leaves pointer input to the terminal or receives it for
/// screen selection and click handling.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MouseMode {
    #[default]
    TerminalSelection,
    TuiCapture,
}

/// Keeps pointer hover separate from keyboard selection and click activation.
///
/// A pointer move may change only `hovered`. Components continue to own their keyboard cursor,
/// while a click carries its resolved target directly to the component activation path.
/// Rendering may give hover and keyboard selection the same highlight or different highlights;
/// that visual choice must never merge their state or make hover affect keyboard behavior.
#[derive(Debug)]
pub(crate) struct PointerInteraction<T> {
    hovered: Option<T>,
}

impl<T> Default for PointerInteraction<T> {
    fn default() -> Self {
        Self { hovered: None }
    }
}

impl<T> PointerInteraction<T> {
    pub(crate) fn update_hover(&mut self, target: Option<T>) {
        self.hovered = target;
    }

    pub(crate) fn clear_hover(&mut self) {
        self.hovered = None;
    }

    pub(crate) fn hovered(&self) -> Option<&T> {
        self.hovered.as_ref()
    }
}

#[cfg(test)]
#[path = "mouse_tests.rs"]
mod tests;
