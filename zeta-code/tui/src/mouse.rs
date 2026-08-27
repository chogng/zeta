/// Declares whether the active TUI surface leaves drag selection to the terminal or receives mouse
/// events for its own click handling.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MouseMode {
    #[default]
    TerminalSelection,
    UiClick,
}
